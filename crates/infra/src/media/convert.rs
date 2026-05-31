//! Media conversion utilities.
//!
//! This module provides async conversion functions for
//! audio and video files using external tools.

use super::audio::{AudioPreset, get_audio_process_cmd};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::fs;
use tokio::process::Command;
use tracing::info;

/// Execute a shell command string cross-platform, capturing stderr on failure
async fn execute_shell_command_with_stderr(
    cmd_str: &str,
) -> Result<(bool, String, String), std::io::Error> {
    let (shell, shell_arg) = if std::env::consts::OS == "windows" {
        ("cmd", "/C")
    } else {
        ("sh", "-c")
    };

    let output = Command::new(shell)
        .args([shell_arg, cmd_str])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    Ok((success, stdout, stderr))
}

/// Policy for removing the original file after audio conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OriginRemoval {
    /// Never remove the original file.
    Never,
    /// Remove the original file only after successful conversion.
    #[default]
    OnSuccess,
    /// Remove the original file only when conversion fails.
    OnFailure,
    /// Always remove the original file (on success or failure).
    Always,
}

/// Options for controlling audio transfer behavior.
pub struct TransferOptions {
    /// Policy for removing the original file.
    pub origin_removal: OriginRemoval,
    /// Remove existing target files before conversion.
    pub remove_existing_target_file: bool,
    /// Stop processing on the first error.
    pub stop_on_error: bool,
}

async fn collect_tasks(dir: &Path, input_exts: &[&str]) -> Vec<(PathBuf, usize)> {
    let mut tasks: Vec<(PathBuf, usize)> = Vec::new();
    if let Ok(mut entries) = fs::read_dir(dir).await {
        while let Some(entry) = entries.next_entry().await.unwrap_or(None) {
            let path = entry.path();
            if path.is_file()
                && let Some(ext) = path.extension()
                && input_exts
                    .iter()
                    .any(|e| e.to_lowercase() == ext.to_string_lossy().to_lowercase())
            {
                tasks.push((path, 0));
            }
        }
    }
    tasks
}

async fn remove_existing_target(output: &Path, remove: bool) {
    if remove
        && output.is_file()
        && let Err(e) = fs::remove_file(output).await
    {
        tracing::info!("Failed to remove existing target file {output:?}: {e}");
    }
}

type TaskResult = (
    PathBuf,
    usize,
    Result<String, std::io::Error>,
    Vec<AudioPreset>,
    String,
);

fn spawn_conversion_task(
    input: PathBuf,
    preset_idx: usize,
    preset: AudioPreset,
    presets: Vec<AudioPreset>,
) -> tokio::task::JoinHandle<TaskResult> {
    tokio::spawn(async move {
        let stem = input.file_stem().unwrap_or_default().to_string_lossy();
        let output_ext = &preset.output_format;
        let output = input
            .parent()
            .expect("file path should have parent")
            .join(format!("{stem}.{output_ext}"));

        let cmd_str = get_audio_process_cmd(&input, &output, &preset);
        let (result, cmd_stdout) = if cmd_str.is_empty() {
            (
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Unknown exec: {}", preset.exec),
                )),
                String::new(),
            )
        } else {
            info!("Running: {}", cmd_str);
            match execute_shell_command_with_stderr(&cmd_str).await {
                Ok((true, stdout, _)) => (Ok(String::new()), stdout),
                Ok((false, stdout, stderr)) => (
                    Err(std::io::Error::other(format!(
                        "Conversion failed: stderr={stderr}"
                    ))),
                    stdout,
                ),
                Err(e) => (Err(e), String::new()),
            }
        };
        let stderr_msg = result
            .as_ref()
            .err()
            .map(std::string::ToString::to_string)
            .unwrap_or_default();
        (
            input,
            preset_idx,
            result.map(|_| stderr_msg),
            presets,
            cmd_stdout,
        )
    })
}

async fn should_skip_output(output: &Path, remove_existing: bool) -> bool {
    if output.is_file()
        && let Ok(metadata) = fs::metadata(output).await
        && metadata.len() > 0
        && !remove_existing
    {
        tracing::info!("File {output:?} exists! Skipping...");
        return true;
    }
    remove_existing_target(output, remove_existing).await;
    false
}

type HandleEntry = (tokio::task::JoinHandle<TaskResult>, bool);

/// Spawn conversion tasks from the queue until the handle list reaches capacity.
async fn spawn_tasks_until_capacity(
    task_queue: &mut VecDeque<(PathBuf, usize)>,
    handles: &mut Vec<HandleEntry>,
    presets: &[AudioPreset],
    options: &TransferOptions,
    capacity: usize,
) {
    while handles.len() < capacity {
        let Some((input, preset_idx)) = task_queue.pop_front() else {
            break;
        };
        let preset = presets
            .get(preset_idx)
            .expect("preset_idx < presets.len()")
            .clone();
        let stem = input.file_stem().unwrap_or_default().to_string_lossy();
        let output_ext = &preset.output_format;
        let output = input
            .parent()
            .expect("file path should have parent")
            .join(format!("{stem}.{output_ext}"));

        if should_skip_output(&output, options.remove_existing_target_file).await {
            continue;
        }

        let handle = spawn_conversion_task(input, preset_idx, preset, presets.to_vec());
        handles.push((handle, true));
    }
}

/// Collect fallback candidates from completed conversion tasks.
///
/// Drains `handles`, separating unfinished tasks into the returned vector
/// and producing a list of failed (input, `preset_idx`) pairs to retry.
async fn collect_fallback_candidates(
    handles: &mut Vec<HandleEntry>,
    options: &TransferOptions,
) -> (
    Vec<HandleEntry>,
    Vec<(PathBuf, usize)>,
    String,
    String,
    String,
) {
    let old_handles = std::mem::take(handles);
    let mut new_handles = Vec::new();
    let mut switch_next_list = Vec::new();
    let mut err_file_path = String::new();
    let mut err_stderr = String::new();
    let mut err_stdout = String::new();

    for (handle, is_process) in old_handles {
        if !is_process {
            new_handles.push((handle, false));
            continue;
        }
        if !handle.is_finished() {
            new_handles.push((handle, true));
            continue;
        }
        let result = handle.await;
        if let Ok((input, preset_idx, res, _presets_vec, cmd_stdout)) = result {
            match res {
                Ok(_stderr_msg) => {
                    if matches!(
                        options.origin_removal,
                        OriginRemoval::OnSuccess | OriginRemoval::Always
                    ) && input.is_file()
                        && let Err(e) = fs::remove_file(&input).await
                    {
                        tracing::info!("Failed to remove origin file {input:?}: {e}");
                    }
                }
                Err(e) => {
                    let stderr_str = e.to_string();
                    tracing::info!("Conversion failed for {input:?}: {e}");
                    switch_next_list.push((input.clone(), preset_idx));
                    err_file_path = input.to_string_lossy().to_string();
                    err_stderr = stderr_str;
                    err_stdout = cmd_stdout;
                }
            }
        }
    }

    (
        new_handles,
        switch_next_list,
        err_file_path,
        err_stderr,
        err_stdout,
    )
}

/// Process fallback entries: push retries to the task queue or mark errors.
///
/// Returns `Some(anyhow::Error)` if `stop_on_error` is set (caller should return early).
async fn process_fallback_list(
    switch_next_list: Vec<(PathBuf, usize)>,
    task_queue: &mut VecDeque<(PathBuf, usize)>,
    presets: &[AudioPreset],
    options: &TransferOptions,
    has_error: &mut bool,
    fallback_file_names: &mut Vec<(String, usize)>,
    err_stderr: &str,
) -> Option<anyhow::Error> {
    for (input, preset_idx) in switch_next_list {
        let next_idx = preset_idx + 1;
        if next_idx >= presets.len() {
            *has_error = true;
            if matches!(
                options.origin_removal,
                OriginRemoval::OnFailure | OriginRemoval::Always
            ) && input.is_file()
                && let Err(e) = fs::remove_file(&input).await
            {
                tracing::info!("Failed to remove failed origin file {input:?}: {e}");
            }
            if options.stop_on_error {
                return Some(anyhow::anyhow!(
                    "Conversion failed for {}: {err_stderr}",
                    input.display(),
                ));
            }
            continue;
        }
        fallback_file_names.push((
            input
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            next_idx,
        ));
        task_queue.push_back((input, next_idx));
    }
    None
}

/// Transfer audio files in a directory using format presets with fallback.
///
/// Supports preset fallback: when first preset fails, tries next one.
/// Unlimited fallback levels via task queue. Handles `OriginRemoval` policy.
/// Uses bounded concurrency based on disk type.
/// Captures stderr on failure and prints fallback statistics.
///
/// # Errors
///
/// Returns `std::io::Error` if `stop_on_error` is set and a conversion
/// fails with all presets, or if a spawned task panics.
///
/// # Panics
///
/// May panic if a spawned task panics, which propagates through
/// the `JoinHandle`.
pub async fn transfer_audio_by_format_in_dir(
    dir: &Path,
    input_exts: &[&str],
    presets: &[AudioPreset],
    options: &TransferOptions,
) -> anyhow::Result<()> {
    if presets.is_empty() {
        return Ok(());
    }

    let cpu_count = std::thread::available_parallelism().map_or(4, std::num::NonZero::get);

    let initial_tasks = collect_tasks(dir, input_exts).await;
    let file_count = initial_tasks.len();
    tracing::info!("Found {file_count} files to convert in {dir:?}");

    if initial_tasks.is_empty() {
        return Ok(());
    }

    tracing::info!("Entering dir: {dir:?} Input ext: {input_exts:?}");

    let mut task_queue: VecDeque<(PathBuf, usize)> = initial_tasks.into_iter().collect();
    let mut handles: Vec<HandleEntry> = Vec::new();
    let mut has_error = false;
    let mut err_file_path = String::new();
    let mut err_stdout = String::new();
    let mut err_stderr = String::new();
    let mut fallback_file_names: Vec<(String, usize)> = Vec::new();

    // Fill the initial batch of tasks up to cpu_count
    spawn_tasks_until_capacity(&mut task_queue, &mut handles, presets, options, cpu_count).await;

    loop {
        if handles.is_empty() && task_queue.is_empty() {
            break;
        }

        // Process completed handles and collect fallback candidates
        let (new_handles, switch_next_list, new_err_path, new_err_stderr, new_err_stdout) =
            collect_fallback_candidates(&mut handles, options).await;

        // Carry forward last error info
        if !new_err_path.is_empty() {
            err_file_path = new_err_path;
            err_stderr = new_err_stderr;
            err_stdout = new_err_stdout;
        }

        // Handle fallbacks: retry with next preset or mark error
        if let Some(err) = process_fallback_list(
            switch_next_list,
            &mut task_queue,
            presets,
            options,
            &mut has_error,
            &mut fallback_file_names,
            &err_stderr,
        )
        .await
        {
            return Err(err);
        }

        // Replenish handles
        handles = new_handles;
        spawn_tasks_until_capacity(&mut task_queue, &mut handles, presets, options, cpu_count)
            .await;

        if !handles.is_empty() {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }
    }

    // Summary reporting
    if has_error {
        tracing::info!("Has Error!");
        tracing::info!("- Err file_path: {err_file_path}");
        tracing::info!("- Err stdout: {err_stdout}");
        tracing::info!("- Err stderr: {err_stderr}");
        if matches!(
            options.origin_removal,
            OriginRemoval::OnFailure | OriginRemoval::Always
        ) {
            tracing::info!("The failed origin file has been removed.");
        }
    }

    if file_count > 0 {
        tracing::info!("Parsed {file_count} file(s).");
    }
    if !fallback_file_names.is_empty() {
        tracing::info!(
            "Fallback: {:?}. Totally {} files.",
            fallback_file_names,
            fallback_file_names.len()
        );
    }

    if has_error {
        Err(anyhow::anyhow!(
            "Audio conversion had errors in {}",
            dir.display()
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_transfer_audio_empty_dir() {
        let dir = TempDir::new().unwrap();
        let result = transfer_audio_by_format_in_dir(
            dir.path(),
            &["wav"],
            std::slice::from_ref(&super::super::audio::AUDIO_PRESET_FLAC),
            &TransferOptions {
                origin_removal: OriginRemoval::OnSuccess,
                remove_existing_target_file: true,
                stop_on_error: false,
            },
        )
        .await;
        assert!(result.is_ok(), "empty dir should succeed");
    }

    #[tokio::test]
    async fn test_transfer_audio_skips_wrong_ext() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("test.txt"), "not audio")
            .await
            .unwrap();
        let result = transfer_audio_by_format_in_dir(
            dir.path(),
            &["wav"],
            std::slice::from_ref(&super::super::audio::AUDIO_PRESET_FLAC),
            &TransferOptions {
                origin_removal: OriginRemoval::OnSuccess,
                remove_existing_target_file: true,
                stop_on_error: false,
            },
        )
        .await;
        assert!(result.is_ok(), "wrong ext files should be skipped");
    }
}

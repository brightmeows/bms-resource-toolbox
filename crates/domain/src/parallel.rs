//! Parallel work-directory processing utilities.
//!
//! Provides two helpers that eliminate ~200 lines of boilerplate across the
//! crate: [`collect_subdirs`] for enumerating immediate subdirectories, and
//! [`run_parallel`] for processing them concurrently with bounded concurrency.

use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::Semaphore;

/// Collect immediate child directory paths of `root_dir`.
///
/// Returns `Ok(vec![])` if the directory has no subdirectories. Propagates I/O
/// errors from `fs::read_dir` (e.g. `root_dir` is not a directory).
pub(crate) async fn collect_subdirs(root_dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut dirs = Vec::new();
    let mut read_dir = fs::read_dir(root_dir).await?;
    while let Some(entry) = read_dir.next_entry().await? {
        if entry.path().is_dir() {
            dirs.push(entry.path());
        }
    }
    Ok(dirs)
}

/// Process independent work directories in parallel with bounded concurrency.
///
/// For each entry in `dirs`, spawns a Tokio task that acquires a semaphore
/// permit and then calls `f(dir_path)`.  At most `available_parallelism()`
/// tasks run concurrently.
///
/// # Error handling
///
/// If `stop_on_error` is `true`, the function returns the first error as soon
/// as it is observed; remaining tasks are **not** cancelled but their results
/// are discarded.  If `false`, all tasks complete and the first error (if any)
/// is returned after all tasks finish — see caveat below.
///
/// A task panic (extremely rare) is converted to [`std::io::Error`] and then
/// to `E` via `E: From<std::io::Error>`.
///
/// # Panics
///
/// Panics if the internal concurrency semaphore is closed, which should not
/// happen under normal operation.
pub(crate) async fn run_parallel<F, Fut, E>(
    dirs: Vec<PathBuf>,
    stop_on_error: bool,
    f: F,
) -> Result<(), E>
where
    F: Fn(PathBuf) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), E>> + Send + 'static,
    E: 'static + Send + From<std::io::Error>,
{
    let cpu_count = std::thread::available_parallelism().map_or(4, std::num::NonZero::get);
    let sem = Arc::new(Semaphore::new(cpu_count));
    let mut handles = Vec::with_capacity(dirs.len());
    let f = Arc::new(f);

    for dir_path in dirs {
        let sem_clone = sem.clone();
        let f_clone = f.clone();
        handles.push(tokio::spawn(async move {
            let _permit = sem_clone.acquire().await.expect("semaphore not closed");
            f_clone(dir_path).await
        }));
    }

    let mut first_error: Option<E> = None;
    for handle in handles {
        match handle.await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                if first_error.is_none() {
                    first_error = Some(e);
                }
                if stop_on_error {
                    break;
                }
            }
            Err(e) => {
                let io_err = std::io::Error::other(format!("Task join error: {e}"));
                let typed_err = E::from(io_err);
                if first_error.is_none() {
                    first_error = Some(typed_err);
                }
                if stop_on_error {
                    break;
                }
            }
        }
    }

    match first_error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_collect_subdirs_empty_root() {
        let dir = TempDir::new().unwrap();
        let dirs = collect_subdirs(dir.path()).await.unwrap();
        assert!(dirs.is_empty());
    }

    #[tokio::test]
    async fn test_collect_subdirs_filters_files() {
        let dir = TempDir::new().unwrap();
        let sub = dir.path().join("sub");
        tokio::fs::create_dir_all(&sub).await.unwrap();
        tokio::fs::write(dir.path().join("f.txt"), "")
            .await
            .unwrap();
        let dirs = collect_subdirs(dir.path()).await.unwrap();
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].file_name().unwrap(), "sub");
    }

    #[tokio::test]
    async fn test_collect_subdirs_nonexistent_path() {
        let result = collect_subdirs(Path::new("/nonexistent/path")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_run_parallel_empty_input() {
        let result: Result<(), std::io::Error> =
            run_parallel(vec![], false, |_| async move { Ok(()) }).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_parallel_all_succeed() {
        let dirs: Vec<PathBuf> = vec![
            PathBuf::from("/a"),
            PathBuf::from("/b"),
            PathBuf::from("/c"),
        ];
        let counter = Arc::new(std::sync::atomic::AtomicI32::new(0));
        let counter_clone = counter.clone();
        let result: Result<(), std::io::Error> = run_parallel(dirs, false, move |_dir| {
            let counter = counter_clone.clone();
            async move {
                counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            }
        })
        .await;
        assert!(result.is_ok());
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_run_parallel_stop_on_error() {
        let dirs: Vec<PathBuf> = vec![
            PathBuf::from("/ok"),
            PathBuf::from("/err"),
            PathBuf::from("/never"),
        ];
        let result: Result<(), std::io::Error> = run_parallel(dirs, true, |dir| async move {
            if dir.as_os_str() == "/err" {
                Err(std::io::Error::other("oops"))
            } else {
                Ok(())
            }
        })
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_run_parallel_collect_all_errors() {
        let dirs: Vec<PathBuf> = vec![
            PathBuf::from("/err1"),
            PathBuf::from("/ok"),
            PathBuf::from("/err2"),
        ];
        let result: Result<(), std::io::Error> = run_parallel(dirs, false, |dir| async move {
            if dir.as_os_str() == "/ok" {
                Ok(())
            } else {
                Err(std::io::Error::other(format!("{dir:?}")))
            }
        })
        .await;
        // Returns the first error we observe (order is non-deterministic)
        assert!(result.is_err());
    }
}

//! Async tests for `fs::walk` module.

use bms_resource_toolbox::infra::fs::name::bms_dir_similarity;
use bms_resource_toolbox::infra::fs::pack_move::DEFAULT_MOVE_OPTIONS;
use bms_resource_toolbox::infra::fs::pack_move::REPLACE_OPTION_UPDATE_PACK;
use bms_resource_toolbox::infra::fs::pack_move::ReplaceAction;
use bms_resource_toolbox::infra::fs::pack_move::ReplaceOptions;
use bms_resource_toolbox::infra::fs::pack_move::move_elements_across_dir;
use bms_resource_toolbox::infra::fs::walk::remove_empty_dirs;
use std::collections::HashMap;
use std::path::PathBuf;

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join("bms_toolbox_tests")
        .join(format!("{prefix}_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[tokio::test]
async fn test_remove_empty_dirs_removes_leaf() {
    let dir = unique_temp_dir("rm_empty");
    let empty = dir.join("empty_sub");
    tokio::fs::create_dir_all(&empty).await.unwrap();

    let result = remove_empty_dirs(&dir).await;
    assert!(result.is_ok());
    assert!(!empty.exists());
    let _ = tokio::fs::remove_dir_all(&dir).await;
}

#[tokio::test]
async fn test_remove_empty_dirs_removes_nested_empty() {
    let dir = unique_temp_dir("rm_nested");
    let nested = dir.join("a").join("b").join("c");
    tokio::fs::create_dir_all(&nested).await.unwrap();

    let result = remove_empty_dirs(&dir).await;
    assert!(result.is_ok());
    assert!(!dir.join("a").exists());
    let _ = tokio::fs::remove_dir_all(&dir).await;
}

#[tokio::test]
async fn test_remove_empty_dirs_preserves_non_empty() {
    let dir = unique_temp_dir("rm_preserve");
    let sub = dir.join("has_file");
    tokio::fs::create_dir_all(&sub).await.unwrap();
    tokio::fs::write(sub.join("data.txt"), "content")
        .await
        .unwrap();

    let result = remove_empty_dirs(&dir).await;
    assert!(result.is_ok());
    assert!(sub.exists());
    assert!(sub.join("data.txt").exists());
    let _ = tokio::fs::remove_dir_all(&dir).await;
}

#[tokio::test]
async fn test_remove_empty_dirs_on_nonexistent_path() {
    let path = std::env::temp_dir()
        .join("bms_toolbox_tests")
        .join("nonexistent_rm_7291");
    let result = remove_empty_dirs(&path).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_similarity_identical_media() {
    let dir_a = unique_temp_dir("sim_a");
    let dir_b = unique_temp_dir("sim_b");
    tokio::fs::write(dir_a.join("song.wav"), "data")
        .await
        .unwrap();
    tokio::fs::write(dir_a.join("readme.txt"), "info")
        .await
        .unwrap();
    tokio::fs::write(dir_b.join("song.flac"), "data")
        .await
        .unwrap();
    tokio::fs::write(dir_b.join("readme.txt"), "info")
        .await
        .unwrap();
    let sim = bms_dir_similarity(&dir_a, &dir_b).await;
    assert!((sim - 1.0).abs() < 1e-6);
    let _ = tokio::fs::remove_dir_all(&dir_a).await;
    let _ = tokio::fs::remove_dir_all(&dir_b).await;
}

#[tokio::test]
async fn test_similarity_no_overlap() {
    let dir_a = unique_temp_dir("sim_no1");
    let dir_b = unique_temp_dir("sim_no2");
    tokio::fs::write(dir_a.join("a.wav"), "data").await.unwrap();
    tokio::fs::write(dir_a.join("readme.txt"), "info")
        .await
        .unwrap();
    tokio::fs::write(dir_b.join("b.wav"), "data").await.unwrap();
    tokio::fs::write(dir_b.join("readme.txt"), "info")
        .await
        .unwrap();
    let sim = bms_dir_similarity(&dir_a, &dir_b).await;
    assert!((sim - 0.0).abs() < 1e-6);
    let _ = tokio::fs::remove_dir_all(&dir_a).await;
    let _ = tokio::fs::remove_dir_all(&dir_b).await;
}

#[tokio::test]
async fn test_similarity_empty() {
    let dir_a = unique_temp_dir("sim_e1");
    let dir_b = unique_temp_dir("sim_e2");
    // add non-media to both to pass the non_media check
    tokio::fs::write(dir_a.join(".gitkeep"), "").await.unwrap();
    tokio::fs::write(dir_b.join(".gitkeep"), "").await.unwrap();
    assert!((bms_dir_similarity(&dir_a, &dir_b).await - 0.0).abs() < 1e-6);
    let _ = tokio::fs::remove_dir_all(&dir_a).await;
    let _ = tokio::fs::remove_dir_all(&dir_b).await;
}

#[tokio::test]
async fn test_similarity_partial() {
    let dir_a = unique_temp_dir("sim_p1");
    let dir_b = unique_temp_dir("sim_p2");
    tokio::fs::write(dir_a.join("c.wav"), "d").await.unwrap();
    tokio::fs::write(dir_a.join("u1.wav"), "d").await.unwrap();
    tokio::fs::write(dir_a.join("readme.txt"), "info")
        .await
        .unwrap();
    tokio::fs::write(dir_b.join("c.flac"), "d").await.unwrap();
    tokio::fs::write(dir_b.join("u2.wav"), "d").await.unwrap();
    tokio::fs::write(dir_b.join("readme.txt"), "info")
        .await
        .unwrap();
    let sim = bms_dir_similarity(&dir_a, &dir_b).await;
    assert!((sim - 0.5).abs() < 1e-6);
    let _ = tokio::fs::remove_dir_all(&dir_a).await;
    let _ = tokio::fs::remove_dir_all(&dir_b).await;
}

#[tokio::test]
async fn test_move_nonexistent_src() {
    let parent = std::env::temp_dir().join("bms_toolbox_tests");
    let dst = unique_temp_dir("mv_nope_dst");
    let nonexistent = parent.join("mv_nope");
    move_elements_across_dir(
        &nonexistent,
        &dst,
        DEFAULT_MOVE_OPTIONS,
        &ReplaceOptions::default(),
    )
    .await
    .unwrap();
    let _ = tokio::fs::remove_dir_all(&dst).await;
}

#[tokio::test]
async fn test_move_to_nonexistent_dst() {
    let parent = std::env::temp_dir().join("bms_toolbox_tests");
    let src = unique_temp_dir("mv_src");
    let dst = parent.join("mv_dst_new");
    tokio::fs::write(src.join("f.txt"), "content")
        .await
        .unwrap();
    move_elements_across_dir(&src, &dst, DEFAULT_MOVE_OPTIONS, &ReplaceOptions::default())
        .await
        .unwrap();
    assert!(dst.join("f.txt").is_file());
    assert!(!src.exists());
    let _ = tokio::fs::remove_dir_all(&dst).await;
}

#[tokio::test]
async fn test_move_replace_strategy() {
    let src = unique_temp_dir("mv_rp_src");
    let dst = unique_temp_dir("mv_rp_dst");
    tokio::fs::write(src.join("f.txt"), "new").await.unwrap();
    tokio::fs::write(dst.join("f.txt"), "old").await.unwrap();
    move_elements_across_dir(&src, &dst, DEFAULT_MOVE_OPTIONS, &ReplaceOptions::default())
        .await
        .unwrap();
    assert_eq!(
        tokio::fs::read_to_string(dst.join("f.txt")).await.unwrap(),
        "new"
    );
    let _ = tokio::fs::remove_dir_all(&dst).await;
}

#[tokio::test]
async fn test_move_skip_strategy() {
    let src = unique_temp_dir("mv_sk_src");
    let dst = unique_temp_dir("mv_sk_dst");
    tokio::fs::write(src.join("f.txt"), "new").await.unwrap();
    tokio::fs::write(dst.join("f.txt"), "old").await.unwrap();
    let opts = ReplaceOptions {
        ext: HashMap::new(),
        default: ReplaceAction::Skip,
    };
    move_elements_across_dir(&src, &dst, DEFAULT_MOVE_OPTIONS, &opts)
        .await
        .unwrap();
    assert_eq!(
        tokio::fs::read_to_string(dst.join("f.txt")).await.unwrap(),
        "old"
    );
    let _ = tokio::fs::remove_dir_all(&dst).await;
}

#[tokio::test]
async fn test_checkreplace_same_content_moves() {
    let src = unique_temp_dir("cr_src");
    let dst = unique_temp_dir("cr_dst");
    tokio::fs::write(src.join("song.bms"), "same")
        .await
        .unwrap();
    tokio::fs::write(dst.join("song.bms"), "same")
        .await
        .unwrap();
    move_elements_across_dir(
        &src,
        &dst,
        DEFAULT_MOVE_OPTIONS,
        &REPLACE_OPTION_UPDATE_PACK,
    )
    .await
    .unwrap();
    assert!(dst.join("song.bms").is_file());
    let _ = tokio::fs::remove_dir_all(&dst).await;
}

#[tokio::test]
async fn test_checkreplace_diff_content_renames() {
    let src = unique_temp_dir("cr2_src");
    let dst = unique_temp_dir("cr2_dst");
    tokio::fs::write(src.join("song.bms"), "v2").await.unwrap();
    tokio::fs::write(dst.join("song.bms"), "v1").await.unwrap();
    move_elements_across_dir(
        &src,
        &dst,
        DEFAULT_MOVE_OPTIONS,
        &REPLACE_OPTION_UPDATE_PACK,
    )
    .await
    .unwrap();
    assert!(dst.join("song.bms").is_file());
    assert!(dst.join("song.0.bms").is_file() || dst.join("song.1.bms").is_file());
    let _ = tokio::fs::remove_dir_all(&dst).await;
}

#[tokio::test]
async fn test_move_with_subdirectories() {
    let src = unique_temp_dir("rec_src");
    let dst = unique_temp_dir("rec_dst");
    tokio::fs::create_dir_all(src.join("sub")).await.unwrap();
    tokio::fs::write(src.join("sub").join("f.txt"), "data")
        .await
        .unwrap();
    move_elements_across_dir(&src, &dst, DEFAULT_MOVE_OPTIONS, &ReplaceOptions::default())
        .await
        .unwrap();
    assert!(dst.join("sub").join("f.txt").is_file());
    let _ = tokio::fs::remove_dir_all(&dst).await;
}

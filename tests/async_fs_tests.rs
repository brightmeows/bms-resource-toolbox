//! Async tests for `fs::walk` module.

use bms_resource_toolbox::infra::fs::name::bms_dir_similarity;
use bms_resource_toolbox::infra::fs::pack_move::DEFAULT_MOVE_OPTIONS;
use bms_resource_toolbox::infra::fs::pack_move::REPLACE_OPTION_UPDATE_PACK;
use bms_resource_toolbox::infra::fs::pack_move::ReplaceAction;
use bms_resource_toolbox::infra::fs::pack_move::ReplaceOptions;
use bms_resource_toolbox::infra::fs::pack_move::move_elements_across_dir;
use bms_resource_toolbox::infra::fs::walk::remove_empty_dirs;
use std::collections::HashMap;
use tempfile::TempDir;

#[tokio::test]
async fn test_remove_empty_dirs_removes_leaf() {
    let dir = TempDir::new().unwrap();
    let empty = dir.path().join("empty_sub");
    tokio::fs::create_dir_all(&empty).await.unwrap();

    let result = remove_empty_dirs(dir.path()).await;
    assert!(result.is_ok());
    assert!(!empty.exists());
}

#[tokio::test]
async fn test_remove_empty_dirs_removes_nested_empty() {
    let dir = TempDir::new().unwrap();
    let nested = dir.path().join("a").join("b").join("c");
    tokio::fs::create_dir_all(&nested).await.unwrap();

    let result = remove_empty_dirs(dir.path()).await;
    assert!(result.is_ok());
    assert!(!dir.path().join("a").exists());
}

#[tokio::test]
async fn test_remove_empty_dirs_preserves_non_empty() {
    let dir = TempDir::new().unwrap();
    let sub = dir.path().join("has_file");
    tokio::fs::create_dir_all(&sub).await.unwrap();
    tokio::fs::write(sub.join("data.txt"), "content")
        .await
        .unwrap();

    let result = remove_empty_dirs(dir.path()).await;
    assert!(result.is_ok());
    assert!(sub.exists());
    assert!(sub.join("data.txt").exists());
}

#[tokio::test]
async fn test_remove_empty_dirs_on_nonexistent_path() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("nonexistent");
    let result = remove_empty_dirs(&path).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_similarity_identical_media() {
    let dir_a = TempDir::new().unwrap();
    let dir_b = TempDir::new().unwrap();
    tokio::fs::write(dir_a.path().join("song.wav"), "data")
        .await
        .unwrap();
    tokio::fs::write(dir_a.path().join("readme.txt"), "info")
        .await
        .unwrap();
    tokio::fs::write(dir_b.path().join("song.flac"), "data")
        .await
        .unwrap();
    tokio::fs::write(dir_b.path().join("readme.txt"), "info")
        .await
        .unwrap();
    let sim = bms_dir_similarity(dir_a.path(), dir_b.path()).await;
    assert!((sim - 1.0).abs() < 1e-6);
}

#[tokio::test]
async fn test_similarity_no_overlap() {
    let dir_a = TempDir::new().unwrap();
    let dir_b = TempDir::new().unwrap();
    tokio::fs::write(dir_a.path().join("a.wav"), "data")
        .await
        .unwrap();
    tokio::fs::write(dir_a.path().join("readme.txt"), "info")
        .await
        .unwrap();
    tokio::fs::write(dir_b.path().join("b.wav"), "data")
        .await
        .unwrap();
    tokio::fs::write(dir_b.path().join("readme.txt"), "info")
        .await
        .unwrap();
    let sim = bms_dir_similarity(dir_a.path(), dir_b.path()).await;
    assert!((sim - 0.0).abs() < 1e-6);
}

#[tokio::test]
async fn test_similarity_empty() {
    let dir_a = TempDir::new().unwrap();
    let dir_b = TempDir::new().unwrap();
    // add non-media to both to pass the non_media check
    tokio::fs::write(dir_a.path().join(".gitkeep"), "")
        .await
        .unwrap();
    tokio::fs::write(dir_b.path().join(".gitkeep"), "")
        .await
        .unwrap();
    assert!((bms_dir_similarity(dir_a.path(), dir_b.path()).await - 0.0).abs() < 1e-6);
}

#[tokio::test]
async fn test_similarity_partial() {
    let dir_a = TempDir::new().unwrap();
    let dir_b = TempDir::new().unwrap();
    tokio::fs::write(dir_a.path().join("c.wav"), "d")
        .await
        .unwrap();
    tokio::fs::write(dir_a.path().join("u1.wav"), "d")
        .await
        .unwrap();
    tokio::fs::write(dir_a.path().join("readme.txt"), "info")
        .await
        .unwrap();
    tokio::fs::write(dir_b.path().join("c.flac"), "d")
        .await
        .unwrap();
    tokio::fs::write(dir_b.path().join("u2.wav"), "d")
        .await
        .unwrap();
    tokio::fs::write(dir_b.path().join("readme.txt"), "info")
        .await
        .unwrap();
    let sim = bms_dir_similarity(dir_a.path(), dir_b.path()).await;
    assert!((sim - 0.5).abs() < 1e-6);
}

#[tokio::test]
async fn test_move_nonexistent_src() {
    let parent = TempDir::new().unwrap();
    let dst = TempDir::new().unwrap();
    let nonexistent = parent.path().join("mv_nope");
    move_elements_across_dir(
        &nonexistent,
        dst.path(),
        DEFAULT_MOVE_OPTIONS,
        &ReplaceOptions::default(),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn test_move_to_nonexistent_dst() {
    let parent = TempDir::new().unwrap();
    let src = parent.path().join("mv_src");
    tokio::fs::create_dir_all(&src).await.unwrap();
    let dst = parent.path().join("mv_dst_new");
    tokio::fs::write(src.join("f.txt"), "content")
        .await
        .unwrap();
    move_elements_across_dir(&src, &dst, DEFAULT_MOVE_OPTIONS, &ReplaceOptions::default())
        .await
        .unwrap();
    assert!(dst.join("f.txt").is_file());
    assert!(!src.exists());
}

#[tokio::test]
async fn test_move_replace_strategy() {
    let src = TempDir::new().unwrap();
    let dst = TempDir::new().unwrap();
    tokio::fs::write(src.path().join("f.txt"), "new")
        .await
        .unwrap();
    tokio::fs::write(dst.path().join("f.txt"), "old")
        .await
        .unwrap();
    move_elements_across_dir(
        src.path(),
        dst.path(),
        DEFAULT_MOVE_OPTIONS,
        &ReplaceOptions::default(),
    )
    .await
    .unwrap();
    assert_eq!(
        tokio::fs::read_to_string(dst.path().join("f.txt"))
            .await
            .unwrap(),
        "new"
    );
}

#[tokio::test]
async fn test_move_skip_strategy() {
    let src = TempDir::new().unwrap();
    let dst = TempDir::new().unwrap();
    tokio::fs::write(src.path().join("f.txt"), "new")
        .await
        .unwrap();
    tokio::fs::write(dst.path().join("f.txt"), "old")
        .await
        .unwrap();
    let opts = ReplaceOptions {
        ext: HashMap::new(),
        default: ReplaceAction::Skip,
    };
    move_elements_across_dir(src.path(), dst.path(), DEFAULT_MOVE_OPTIONS, &opts)
        .await
        .unwrap();
    assert_eq!(
        tokio::fs::read_to_string(dst.path().join("f.txt"))
            .await
            .unwrap(),
        "old"
    );
}

#[tokio::test]
async fn test_checkreplace_same_content_moves() {
    let src = TempDir::new().unwrap();
    let dst = TempDir::new().unwrap();
    tokio::fs::write(src.path().join("song.bms"), "same")
        .await
        .unwrap();
    tokio::fs::write(dst.path().join("song.bms"), "same")
        .await
        .unwrap();
    move_elements_across_dir(
        src.path(),
        dst.path(),
        DEFAULT_MOVE_OPTIONS,
        &REPLACE_OPTION_UPDATE_PACK,
    )
    .await
    .unwrap();
    assert!(dst.path().join("song.bms").is_file());
}

#[tokio::test]
async fn test_checkreplace_diff_content_renames() {
    let src = TempDir::new().unwrap();
    let dst = TempDir::new().unwrap();
    tokio::fs::write(src.path().join("song.bms"), "v2")
        .await
        .unwrap();
    tokio::fs::write(dst.path().join("song.bms"), "v1")
        .await
        .unwrap();
    move_elements_across_dir(
        src.path(),
        dst.path(),
        DEFAULT_MOVE_OPTIONS,
        &REPLACE_OPTION_UPDATE_PACK,
    )
    .await
    .unwrap();
    assert!(dst.path().join("song.bms").is_file());
    assert!(dst.path().join("song.0.bms").is_file() || dst.path().join("song.1.bms").is_file());
}

#[tokio::test]
async fn test_move_with_subdirectories() {
    let src = TempDir::new().unwrap();
    let dst = TempDir::new().unwrap();
    tokio::fs::create_dir_all(src.path().join("sub"))
        .await
        .unwrap();
    tokio::fs::write(src.path().join("sub").join("f.txt"), "data")
        .await
        .unwrap();
    move_elements_across_dir(
        src.path(),
        dst.path(),
        DEFAULT_MOVE_OPTIONS,
        &ReplaceOptions::default(),
    )
    .await
    .unwrap();
    assert!(dst.path().join("sub").join("f.txt").is_file());
}

//! Async tests for `bms::encoding` and `bms::parse` modules.

use bms_resource_toolbox::domain::bms::encoding::get_bms_file_str;
use bms_resource_toolbox::domain::bms::info::get_dir_bms_info;
use bms_resource_toolbox::domain::bms::parse::parse_bms_content;
use bms_resource_toolbox::domain::bms::types::BMSDifficulty;
use bms_resource_toolbox::infra::adapters::fs::TokioFsAdapter;
use tempfile::TempDir;

#[tokio::test]
async fn test_read_bms_file_utf8() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.bms");
    tokio::fs::write(&path, "#TITLE テスト曲\n#ARTIST テスト\n")
        .await
        .unwrap();

    let bytes = tokio::fs::read(&path).await.unwrap();
    let content = get_bms_file_str(&bytes, None);
    assert!(content.contains("#TITLE"));
    assert!(content.contains("テスト曲"));
}

#[tokio::test]
async fn test_read_bms_file_ascii() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.bms");
    tokio::fs::write(&path, "#TITLE Test Song\n#ARTIST Artist\n")
        .await
        .unwrap();

    let bytes = tokio::fs::read(&path).await.unwrap();
    let content = get_bms_file_str(&bytes, None);
    assert!(content.contains("#TITLE Test Song"));
}

#[tokio::test]
async fn test_read_bms_file_nonexistent() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("nonexistent.bms");
    let result = tokio::fs::read(&path).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_parse_bms_file_full_metadata() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.bms");
    let content = "\
#TITLE Test Song
#ARTIST Test Artist
#GENRE Genre
#PLAYLEVEL 7
#DIFFICULTY 4
#TOTAL 200.5
#STAGEFILE bg.png
";
    tokio::fs::write(&path, content).await.unwrap();

    let bytes = tokio::fs::read(&path).await.unwrap();
    let file_str = get_bms_file_str(&bytes, None);
    let info = parse_bms_content(&file_str);
    assert_eq!(info.title, "Test Song");
    assert_eq!(info.artist, "Test Artist");
    assert_eq!(info.genre, "Genre");
    assert_eq!(info.playlevel, 7);
    assert_eq!(info.difficulty, BMSDifficulty::Another);
    assert_eq!(info.total, None);
    assert_eq!(info.stage_file, None);
}

#[tokio::test]
async fn test_parse_bms_file_minimal() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.bms");
    tokio::fs::write(&path, "#TITLE Only Title\n")
        .await
        .unwrap();

    let bytes = tokio::fs::read(&path).await.unwrap();
    let file_str = get_bms_file_str(&bytes, None);
    let info = parse_bms_content(&file_str);
    assert_eq!(info.title, "Only Title");
    assert_eq!(info.artist, "");
    assert_eq!(info.playlevel, 0);
    assert_eq!(info.total, None);
}

#[tokio::test]
async fn test_parse_bms_file_nonexistent() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("nonexistent.bms");
    let result = tokio::fs::read(&path).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_dir_bms_info_basic() {
    let dir = TempDir::new().unwrap();
    tokio::fs::write(
        dir.path().join("test.bms"),
        "#TITLE Title\n#ARTIST Artist\n#GENRE Genre\n",
    )
    .await
    .unwrap();
    let fs = TokioFsAdapter;
    let info = get_dir_bms_info(&fs, dir.path()).await.unwrap();
    assert_eq!(info.title, "Title");
    assert_eq!(info.artist, "Artist");
    assert_eq!(info.genre, "Genre");
}

#[tokio::test]
async fn test_get_dir_bms_info_none() {
    let dir = TempDir::new().unwrap();
    let fs = TokioFsAdapter;
    assert!(get_dir_bms_info(&fs, dir.path()).await.is_none());
}

#[tokio::test]
async fn test_get_dir_bms_info_multiple() {
    let dir = TempDir::new().unwrap();
    tokio::fs::write(
        dir.path().join("d1.bms"),
        "#TITLE Song [Easy]\n#ARTIST Common\n",
    )
    .await
    .unwrap();
    tokio::fs::write(
        dir.path().join("d2.bms"),
        "#TITLE Song [Hard]\n#ARTIST Common\n",
    )
    .await
    .unwrap();
    let fs = TokioFsAdapter;
    let info = get_dir_bms_info(&fs, dir.path()).await.unwrap();
    assert_eq!(info.title, "Song");
    assert_eq!(info.artist, "Common");
}

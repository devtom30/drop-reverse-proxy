use mock::repository::artist::ArtistRepoMock;
use mock::repository::drop::DropRepoMock;
use mock::repository::playlist::PlaylistRepoMock;
use drop_reverse_proxy::repository::artist::Artist;
use drop_reverse_proxy::service::drop::{DropRequest, DropService, DropServiceT, ImportError, PLAYLIST_DIR_PREFIX, TRACK_FILE_PREFIX};
use std::fs;
use tempfile::TempDir;
use drop_reverse_proxy::repository::Repo;

#[path = "../mock.rs"]
mod mock;

#[tokio::test]
async fn test_create_drop_success_with_artist_id() {
    let artist_repo = ArtistRepoMock::new();
    let drop_repo = DropRepoMock::new();
    let playlist_repo = PlaylistRepoMock::new();

    let artist_id = 10;
    artist_repo.map_by_id().write().unwrap().insert(artist_id, Artist::new(artist_id, "Artist Name".to_string()));

    let service = DropService::new(drop_repo, artist_repo, playlist_repo);

    let temp_import_dir = TempDir::new().unwrap();
    let import_path = temp_import_dir.path().to_str().unwrap().to_string();
    fs::write(temp_import_dir.path().join("track1.mp3"), "content1").unwrap();

    let temp_web_server_dir = TempDir::new().unwrap();
    let web_server_path = temp_web_server_dir.path().to_str().unwrap().to_string();

    let drop_request = DropRequest::new(
        Some(artist_id),
        None,
        "Playlist Name".to_string(),
        vec!["track1.mp3".to_string()]
    );

    let result: Result<(), ImportError> = service.create_drop(&import_path, drop_request, &web_server_path).await;
    assert!(result.is_ok());

    // Verify playlist directory and file
    // PlaylistRepoMock returns entity.id() on save. Playlist::new(0, ...) has id 0.
    let playlist_dir = temp_web_server_dir.path().join(format!("{}{}", PLAYLIST_DIR_PREFIX, 0));
    assert!(playlist_dir.exists());
    assert!(playlist_dir.join(format!("{}{}", TRACK_FILE_PREFIX, 1)).exists());

    let drop_result = service.drop_repository().get(0).await;
    assert!(drop_result.is_ok());
    assert_eq!(drop_result.unwrap().artist_id(), artist_id);
}

#[tokio::test]
async fn test_create_drop_success_with_artist_name() {
    let artist_repo = ArtistRepoMock::new();
    let drop_repo = DropRepoMock::new();
    let playlist_repo = PlaylistRepoMock::new();

    let artist_id = 10;
    let artist_name = "Artist Name";
    artist_repo.map_by_name().write().unwrap().insert(artist_name.to_string(), Artist::new(artist_id, artist_name.to_string()));

    let service = DropService::new(drop_repo, artist_repo, playlist_repo);

    let temp_import_dir = TempDir::new().unwrap();
    let import_path = temp_import_dir.path().to_str().unwrap().to_string();
    fs::write(temp_import_dir.path().join("track1.mp3"), "content1").unwrap();

    let temp_web_server_dir = TempDir::new().unwrap();
    let web_server_path = temp_web_server_dir.path().to_str().unwrap().to_string();

    let drop_request = DropRequest::new(
        None,
        Some(artist_name.to_string()),
        "Playlist Name".to_string(),
        vec!["track1.mp3".to_string()]
    );

    let result: Result<(), ImportError> = service.create_drop(&import_path, drop_request, &web_server_path).await;
    assert!(result.is_ok());

    let playlist_dir = temp_web_server_dir.path().join(format!("{}{}", PLAYLIST_DIR_PREFIX, 0));
    assert!(playlist_dir.exists());
}

#[tokio::test]
async fn test_create_drop_error_both_artist_id_and_name() {
    let artist_repo = ArtistRepoMock::new();
    let drop_repo = DropRepoMock::new();
    let playlist_repo = PlaylistRepoMock::new();

    let service = DropService::new(drop_repo, artist_repo, playlist_repo);

    let drop_request = DropRequest::new(
        Some(1),
        Some("Name".to_string()),
        "Playlist".to_string(),
        vec![]
    );

    let result = service.create_drop(&"import".to_string(), drop_request, &"web".to_string()).await;
    assert!(matches!(result, Err(ImportError::ArtistIdAndArtistNameAreBothPresent)));
}

#[tokio::test]
async fn test_create_drop_error_artist_id_not_found() {
    let artist_repo = ArtistRepoMock::new();
    let drop_repo = DropRepoMock::new();
    let playlist_repo = PlaylistRepoMock::new();

    let service = DropService::new(drop_repo, artist_repo, playlist_repo);

    let drop_request = DropRequest::new(
        Some(999),
        None,
        "Playlist".to_string(),
        vec![]
    );

    let result = service.create_drop(&"import".to_string(), drop_request, &"web".to_string()).await;
    assert!(matches!(result, Err(ImportError::InvalidArtistId)));
}

#[tokio::test]
async fn test_create_drop_error_artist_name_not_found() {
    let artist_repo = ArtistRepoMock::new();
    let drop_repo = DropRepoMock::new();
    let playlist_repo = PlaylistRepoMock::new();

    let service = DropService::new(drop_repo, artist_repo, playlist_repo);

    let drop_request = DropRequest::new(
        None,
        Some("Unknown".to_string()),
        "Playlist".to_string(),
        vec![]
    );

    let result = service.create_drop(&"import".to_string(), drop_request, &"web".to_string()).await;
    assert!(matches!(result, Err(ImportError::CantCreateArtistFromArtistName)));
}

#[tokio::test]
async fn test_create_drop_error_missing_track_file() {
    let artist_repo = ArtistRepoMock::new();
    let drop_repo = DropRepoMock::new();
    let playlist_repo = PlaylistRepoMock::new();

    let artist_id = 1;
    artist_repo.map_by_id().write().unwrap().insert(artist_id, Artist::new(artist_id, "Artist".to_string()));

    let service = DropService::new(drop_repo, artist_repo, playlist_repo);

    let temp_import_dir = TempDir::new().unwrap();
    let import_path = temp_import_dir.path().to_str().unwrap().to_string();
    // Do NOT create the track file

    let temp_web_server_dir = TempDir::new().unwrap();
    let web_server_path = temp_web_server_dir.path().to_str().unwrap().to_string();

    let drop_request = DropRequest::new(
        Some(artist_id),
        None,
        "Playlist".to_string(),
        vec!["missing.mp3".to_string()]
    );

    let result = service.create_drop(&import_path, drop_request, &web_server_path).await;
    assert!(matches!(result, Err(ImportError::CantCopyTrackFileToPlaylistDirectory)));
}

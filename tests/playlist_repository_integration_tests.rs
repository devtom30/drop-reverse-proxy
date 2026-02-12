use crate::utils::{create_default_db_config, start_postgres_container};
use drop_reverse_proxy::repository::playlist::{Playlist, PlaylistRepo};
use drop_reverse_proxy::repository::Repo;

mod utils;

#[tokio::test]
async fn test_playlist_repo_integration() {
    // 1. Start Postgres container
    let db_name = "drop_of_culture";
    let user = "drop_of_culture";
    let password = "drop_of_culture";
    let (_container_guard, host, port) = start_postgres_container(
        db_name,
        user,
        password,
    ).await.expect("Failed to start Postgres container");

    // 2. Setup database pool
    let db_config = create_default_db_config(host, port, db_name, user, password);

    let pool = drop_reverse_proxy::config::db::create_pool(&db_config)
        .await
        .expect("Failed to create database pool");

    // 3. Initialize schema
    sqlx::query(
        r#"
        CREATE TABLE "playlist" (
            id SERIAL PRIMARY KEY,
            name VARCHAR(255) NOT NULL
        )
        "#
    )
    .execute(&pool)
    .await
    .expect("Failed to create table");

    let repo = PlaylistRepo::new(&db_config)
        .await
        .expect("Failed to create playlist repository");

    // 4. Test save_or_update
    let new_playlist = Playlist {
        id: 0,
        name: "Test Playlist".to_string(),
    };

    <PlaylistRepo as Repo<Playlist>>::save_or_update(&repo, &new_playlist).await.expect("Failed to save playlist");

    // 5. Test get
    let saved_playlist = <PlaylistRepo as Repo<Playlist>>::get(&repo, 1).await.expect("Failed to get playlist");
    
    assert_eq!(saved_playlist.name, "Test Playlist");
    assert_eq!(saved_playlist.id, 1);
}

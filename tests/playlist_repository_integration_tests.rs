use drop_reverse_proxy::repository::Repo;
use drop_reverse_proxy::repository::playlist::{Playlist, PlaylistRepo};
use sqlx::postgres::PgPoolOptions;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

#[tokio::test]
async fn test_playlist_repo_integration() {
    // 1. Start Postgres container
    let postgres_container = Postgres::default().start().await.expect("Failed to start Postgres container");
    let host = postgres_container.get_host().await.expect("Failed to get host");
    let port = postgres_container.get_host_port_ipv4(5432).await.expect("Failed to get port");
    let connection_string = format!("postgres://postgres:postgres@{}:{}/postgres", host, port);

    // 2. Setup database pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&connection_string)
        .await
        .expect("Failed to connect to Postgres");

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

    let repo = PlaylistRepo::new(pool);

    // 4. Test save_or_update
    let new_playlist = Playlist {
        id: 0,
        name: "Test Playlist".to_string(),
    };

    <PlaylistRepo as Repo<Playlist>>::save_or_update(&repo, &new_playlist).await.expect("Failed to save playlist");

    // 5. Test get
    let saved_playlist = <PlaylistRepo as Repo<Playlist>>::get(&repo, "1").await.expect("Failed to get playlist");
    
    assert_eq!(saved_playlist.name, "Test Playlist");
    assert_eq!(saved_playlist.id, 1);
}

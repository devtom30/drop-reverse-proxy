use crate::utils::{create_default_db_config, start_postgres_container};
use drop_reverse_proxy::repository::artist::{Artist, ArtistRepo};
use drop_reverse_proxy::repository::{RepoByName};
use std::sync::Arc;

mod utils;

#[tokio::test]
async fn test_artist_repo_integration() {
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
        CREATE TABLE "artist" (
            id SERIAL PRIMARY KEY,
            name VARCHAR(255) NOT NULL
        )
        "#
    )
    .execute(&pool)
    .await
    .expect("Failed to create table");

    let repo = Arc::new(ArtistRepo::new(&db_config)
        .await
        .expect("Failed to create artist repository"));

    // 4. Test save_or_update
    let new_artist = Artist::new(0, "Test Artist".to_string());

    let artist_id = repo.save_or_update(&new_artist).await.expect("Failed to save artist");

    // 5. Test get
    let saved_artist = repo.get(artist_id).await.expect("Failed to get artist");
    
    assert_eq!(saved_artist.name(), "Test Artist");
    assert_eq!(saved_artist.id(), 1);
}

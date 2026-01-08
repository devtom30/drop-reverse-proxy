use drop_reverse_proxy::repository::{Drop, DropRepo, Repo};
use sqlx::postgres::PgPoolOptions;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

#[tokio::test]
async fn should_insert_data() {
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
        CREATE TABLE "drop" (
            id SERIAL PRIMARY KEY,
            artist_id INTEGER NOT NULL,
            artwork_id INTEGER NOT NULL,
            type_id SMALLINT NOT NULL
        )
        "#
    )
    .execute(&pool)
    .await
    .expect("Failed to create table");

    let repo = DropRepo::new(pool);

    // 4. Test save_or_update
    let new_drop = Drop {
        id: 0, // Serial will override this
        artist_id: 1,
        artwork_id: 10,
        type_id: 2,
    };

    <DropRepo as Repo<Drop>>::save_or_update(&repo, &new_drop).await.expect("Failed to save drop");

    // 5. Test get
    // Since it's the first insert, id should be 1
    let saved_drop = <DropRepo as Repo<Drop>>::get(&repo, "1").await.expect("Failed to get drop");
    
    assert_eq!(saved_drop.artist_id, 1);
    assert_eq!(saved_drop.artwork_id, 10);
    assert_eq!(saved_drop.type_id, 2);
    assert_eq!(saved_drop.id, 1);
}
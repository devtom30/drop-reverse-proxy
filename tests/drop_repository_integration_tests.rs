use drop_reverse_proxy::config::db::DatabaseConfig;
use drop_reverse_proxy::repository::drop::{Drop, DropRepo};
use drop_reverse_proxy::repository::Repo;
use std::time::Duration;
use testcontainers::runners::AsyncRunner;
use testcontainers::ContainerAsync;
use testcontainers_modules::postgres::Postgres;

struct ContainerGuard(ContainerAsync<Postgres>);

async fn start_postgres_container(
    db_name: &str,
    user: &str,
    password: &str
) -> Result<(ContainerAsync<Postgres>, String, u16), Box<dyn std::error::Error>> {

    let postgres_container = Postgres::default()
        .with_db_name(db_name)
        .with_user(user)
        .with_password(password)
        .start().await.expect("Failed to start Postgres container");
    match postgres_container.get_host().await {
        Ok(host) => match postgres_container.get_host_port_ipv4(5432).await {
            Ok(port) => Ok((postgres_container, host.to_string(), port)),
            Err(e) => Err(Box::from(e))
        },
        Err(e) => Err(Box::from(e))
    }
}

#[tokio::test]
async fn should_insert_data() {
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
    /*let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&connection_string)
        .await
        .expect("Failed to connect to Postgres");
*/
    let db_config = DatabaseConfig {
        host: host.clone(),
        port: port as u16,
        database: db_name.to_string(),
        username: user.to_string(),
        password: password.to_string(),
        max_connections: 10,
        min_connections: 1,
        connect_timeout: Duration::from_secs(5),
        idle_timeout: Duration::from_secs(100),
        max_lifetime: Duration::from_secs(1800)
    };

    let pool = drop_reverse_proxy::config::db::create_pool(&db_config)
        .await
        .expect("Failed to create database pool");

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

    let repo = DropRepo::new(&db_config).await.expect("Failed to create drop repository");

    // 4. Test save_or_update
    let new_drop = Drop::new(0, 1, 2, 10);

    <DropRepo as Repo<Drop>>::save_or_update(&repo, &new_drop).await.expect("Failed to save drop");

    // 5. Test get
    // Since it's the first insert, id should be 1
    let saved_drop = <DropRepo as Repo<Drop>>::get(&repo, "1").await.expect("Failed to get drop");
    
    assert_eq!(saved_drop.artist_id(), 1);
    assert_eq!(saved_drop.artwork_id(), 10);
    assert_eq!(saved_drop.type_id(), 2);
    assert_eq!(saved_drop.id(), 1);
}
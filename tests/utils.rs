use std::time::Duration;
use testcontainers::ContainerAsync;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;
use drop_reverse_proxy::config::db::DatabaseConfig;

pub async fn start_postgres_container(
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

pub fn create_default_db_config(host: String, port: u16, db_name: &str, user: &str, password: &str) -> DatabaseConfig {
    DatabaseConfig {
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
    }
}
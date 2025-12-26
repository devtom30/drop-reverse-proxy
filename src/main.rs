use app_properties::AppProperties;
use chrono::NaiveDateTime;
use drop_reverse_proxy::{app, create_conf_from_toml_file, AppState, Conf, InMemoryIpRepo, InMemoryTagRepo, InMemoryTokenRepo, Tag, TagRepo};
use std::net::SocketAddr;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let conf = create_conf_from_toml_file("app.toml")
        .expect("can't load conf from toml file");

    let listener = tokio::net::TcpListener::bind(conf.bind_addr()).await.unwrap();
    let token_repo = InMemoryTokenRepo::default();
    let tag_repo = InMemoryTagRepo::default();
    ["jdznjevb", "xurnxenyoawltkky", "tag3"].iter()
        .for_each(|t| tag_repo.save(&Tag::new(t.to_string(), NaiveDateTime::default())));
    let ip_repo = InMemoryIpRepo::default();
    //tag_repo.save(&drop_reverse_proxy::Tag::new("tag1".to_string(), chrono::NaiveDateTime::default()));
    let app_state = AppState {
        token_repo: Arc::new(token_repo.clone()),
        tag_repo: Arc::new(tag_repo.clone()),
        ip_repo: Arc::new(ip_repo),
        conf
    };
    axum::serve(
        listener,
        app(app_state).into_make_service_with_connect_info::<SocketAddr>()
    ).await.unwrap();
}

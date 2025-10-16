use drop_reverse_proxy::{app, AppState, InMemoryTokenRepo};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    let token_repo = InMemoryTokenRepo::default();
    let app_state = AppState {
        token_repo: Arc::new(token_repo.clone()),
    };
    axum::serve(listener, app(app_state)).await.unwrap();
}

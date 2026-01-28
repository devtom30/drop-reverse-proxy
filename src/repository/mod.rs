pub mod drop;

use sqlx::{Pool, Postgres};

pub trait Entity {
    fn id(&self) -> String;
}
#[derive(Debug)]
pub enum RepositoryError {
    EntityNotFound,
    EntityNotSaved
}
pub trait Repo<E: Entity> {
    async fn get(&self, id: &str) -> Result<E, RepositoryError>;
    async fn save_or_update(&self, entity: &E) -> Result<(), RepositoryError>;
}



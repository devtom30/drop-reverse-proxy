pub mod drop;
pub mod artist;
pub mod playlist;

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

pub trait RepoByName<E: Entity> {
    async fn get_by_name(&self, name: &str) -> Result<E, RepositoryError>;
}

#[derive(Clone)]
pub enum RepoType {
    Artist(std::sync::Arc<crate::repository::artist::ArtistRepo>),
    Drop(std::sync::Arc<crate::repository::drop::DropRepo>),
    Playlist(std::sync::Arc<crate::repository::playlist::PlaylistRepo>),
}



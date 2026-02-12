use std::rc::Rc;
use std::sync::Arc;
use async_trait::async_trait;
use crate::repository::artist::Artist;
use crate::repository::drop::Drop;
use crate::repository::playlist::Playlist;
use crate::repository::{Entity, Repo, RepoByName, RepositoryError};
use crate::service::drop::{DropRequest, DropService, ImportError};

pub mod drop;

pub enum ServiceEnum {
    DropService(
        DropService<
            Arc<dyn Repo<Drop>>,
            Arc<dyn Repo<Artist>>,
            Arc<dyn Repo<Playlist>>,
        >,
    ),
}

pub trait ArtistRepoTrait: Repo<Artist> + RepoByName<Artist> + Send + Sync {}
impl<T: Repo<Artist> + RepoByName<Artist> + Send + Sync> ArtistRepoTrait for T {}

#[async_trait]
impl Repo<Artist> for Box<dyn ArtistRepoTrait> {
    async fn get(&self, id: i32) -> Result<Artist, RepositoryError> {
        self.as_ref().get(id).await
    }

    async fn save_or_update(&self, entity: &Artist) -> Result<i32, RepositoryError> {
        self.as_ref().save_or_update(entity).await
    }
}

#[async_trait]
impl RepoByName<Artist> for Box<dyn ArtistRepoTrait> {
    async fn get_by_name(&self, name: &str) -> Result<Artist, RepositoryError> {
        self.as_ref().get_by_name(name).await
    }
}

#[async_trait]
pub trait DropServiceT {
    async fn create_drop(
        &self,
        drop_import_path: String,
        drop_request: DropRequest,
    ) -> Result<(), ImportError>;
}

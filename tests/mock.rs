use async_trait::async_trait;
use drop_reverse_proxy::repository::drop::Drop;
use drop_reverse_proxy::repository::{Repo, RepositoryError};
use drop_reverse_proxy::repository::artist::Artist;
use drop_reverse_proxy::repository::playlist::Playlist;

pub struct DropRepoMock;
#[async_trait]
impl Repo<Drop> for DropRepoMock {
    async fn get(&self, id: i32) -> Result<Drop, RepositoryError> {
        match id {
            1..3 => Ok(Drop::new(
                id,
                1,
                1,
                1
            )),
            _ => Err(RepositoryError::EntityNotFound)
        }
    }

    async fn save_or_update(&self, entity: &Drop) -> Result<i32, RepositoryError> {
        Ok(1)
    }
}

pub struct ArtistRepoMock;
#[async_trait]
impl Repo<Artist> for ArtistRepoMock {
    async fn get(&self, id: i32) -> Result<Artist, RepositoryError> {
        match id {
            1 => Ok(Artist::new(1, "Artist 1".to_string())),
            2 => Ok(Artist::new(2, "Artist 2".to_string())),
            _ => Err(RepositoryError::EntityNotFound)
        }
    }

    async fn save_or_update(&self, entity: &Artist) -> Result<i32, RepositoryError> {
        Ok(1)
    }
}

pub struct PlaylistRepoMock;
#[async_trait]
impl Repo<Playlist> for PlaylistRepoMock {
    async fn get(&self, id: i32) -> Result<Playlist, RepositoryError> {
        match id {
            1 => Ok(Playlist::new(1, "Playlist 1".to_string())),
            _ => Err(RepositoryError::EntityNotFound)
        }
    }

    async fn save_or_update(&self, entity: &Playlist) -> Result<i32, RepositoryError> {
        Ok(1)
    }
}

use async_trait::async_trait;
use drop_reverse_proxy::repository::playlist::Playlist;
use drop_reverse_proxy::repository::{Repo, RepositoryError};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct PlaylistRepoMock {
    map: Arc<RwLock<HashMap<i32, Playlist>>>
}
impl PlaylistRepoMock {
    pub fn new() -> Self {
        Self { map: Arc::new(RwLock::new(HashMap::new())) }
    }

    pub fn map(&self) -> &Arc<RwLock<HashMap<i32, Playlist>>> {
        &self.map
    }
}

#[async_trait]
impl Repo<Playlist> for PlaylistRepoMock {
    async fn get(&self, id: i32) -> Result<Playlist, RepositoryError> {
        match self.map().read().unwrap().get(&id) {
            Some(drop) => Ok(drop.clone()),
            None => Err(RepositoryError::EntityNotFound)
        }
    }

    async fn save_or_update(&self, entity: &Playlist) -> Result<i32, RepositoryError> {
        self.map.write().unwrap().insert(entity.id(), entity.clone());
        Ok(entity.id())
    }
}
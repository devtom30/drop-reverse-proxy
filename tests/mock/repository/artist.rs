use std::sync::{Arc, RwLock};
use drop_reverse_proxy::repository::artist::Artist;
use drop_reverse_proxy::repository::{RepoByName, RepositoryError};
use std::collections::HashMap;

#[derive(Clone)]
pub struct ArtistRepoMock {
    map_by_id: Arc<RwLock<HashMap<i32, Artist>>>,
    map_by_name: Arc<RwLock<HashMap<String, Artist>>>,
}

impl ArtistRepoMock {
    pub fn new() -> Self {
        Self {
            map_by_id: Arc::new(RwLock::new(HashMap::new())),
            map_by_name: Arc::new(RwLock::new(HashMap::new()))
        }
    }

    pub fn map_by_id(&self) -> &Arc<RwLock<HashMap<i32, Artist>>> {
        &self.map_by_id
    }
    pub fn map_by_name(&self) -> &Arc<RwLock<HashMap<String, Artist>>> {
        &self.map_by_name
    }
}

#[async_trait::async_trait]
impl RepoByName<Artist> for ArtistRepoMock {
    async fn get(&self, id: i32) -> Result<Artist, RepositoryError> {
        self.map_by_id.read().unwrap().get(&id).cloned().ok_or(RepositoryError::EntityNotFound)
    }

    async fn save_or_update(&self, entity: &Artist) -> Result<i32, RepositoryError> {
        self.map_by_id.write().unwrap().insert(entity.id(), entity.clone());
        self.map_by_name.write().unwrap().insert(entity.name().to_string(), entity.clone());
        Ok(entity.id())
    }

    async fn get_by_name(&self, name: &str) -> Result<Artist, RepositoryError> {
        self.map_by_name.read().unwrap().get(name).cloned().ok_or(RepositoryError::EntityNotFound)
    }
}
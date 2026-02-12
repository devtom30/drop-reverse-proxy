use std::sync::Arc;
use async_trait::async_trait;
use crate::config::db::{create_pool, DatabaseConfig};
use crate::repository::{Entity, Repo, RepositoryError};
use derive_new::new;
use sqlx::{Pool, Postgres};

#[derive(sqlx::FromRow, Debug, Clone, PartialEq, new)]
pub struct Playlist {
    pub id: i32,
    pub name: String,
}

impl Entity for Playlist {
    fn id(&self) -> String {
        self.id.to_string()
    }
}

#[derive(Clone, Debug)]
pub struct PlaylistRepo {
    pub pool: Pool<Postgres>,
}

impl PlaylistRepo {
    pub async fn new(database_config: &DatabaseConfig) -> Result<PlaylistRepo, RepositoryError> {
        match create_pool(database_config).await {
            Ok(pool) => Ok(Self { pool }),
            Err(err) => Err(RepositoryError::DatabaseError(err))
        }
    }
}

#[async_trait]
impl Repo<Playlist> for PlaylistRepo {
    async fn get(&self, id: i32) -> Result<Playlist, RepositoryError> {
        sqlx::query_as::<_, Playlist>("
SELECT id, name
FROM \"playlist\"
WHERE id = $1
LIMIT 1
")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|_| RepositoryError::EntityNotFound)
    }

    async fn save_or_update(&self, playlist: &Playlist) -> Result<i32, RepositoryError> {
        sqlx::query_scalar::<_, i32>("
INSERT INTO \"playlist\" (name)
VALUES ($1)
RETURNING id
    ")
            .bind(playlist.name.clone())
            .fetch_one(&self.pool)
            .await
            .map_err(|_| RepositoryError::EntityNotSaved)
    }
}

#[async_trait]
impl Repo<Playlist> for Arc<PlaylistRepo> {
    async fn get(&self, id: i32) -> Result<Playlist, RepositoryError> {
        self.as_ref().get(id).await
    }

    async fn save_or_update(&self, entity: &Playlist) -> Result<i32, RepositoryError> {
        self.as_ref().save_or_update(entity).await
    }
}


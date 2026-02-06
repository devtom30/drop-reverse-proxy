use crate::config::db::{create_pool, DatabaseConfig};
use crate::repository::{Entity, Repo, RepoByName, RepositoryError};
use derive_new::new;
use sqlx::{Pool, Postgres};

#[derive(sqlx::FromRow, Debug, Clone, PartialEq, new)]
pub struct Artist {
    id: i32,
    name: String,
}

impl Artist {
    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Entity for Artist {
    fn id(&self) -> String {
        self.id.to_string()
    }
}

#[derive(Debug, Clone)]
pub struct ArtistRepo {
    pub pool: Pool<Postgres>,
}

impl ArtistRepo {
    pub async fn new(database_config: &DatabaseConfig) -> Result<ArtistRepo, RepositoryError> {
        match create_pool(database_config).await {
            Ok(pool) => Ok(Self { pool }),
            Err(err) => Err(RepositoryError::DatabaseError(err))
        }
    }
}
impl Repo<Artist> for ArtistRepo {
    async fn get(&self, id: i32) -> Result<Artist, RepositoryError> {
        sqlx::query_as::<_, Artist>("
SELECT id, name
FROM \"artist\"
WHERE id = $1
LIMIT 1
")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|_| RepositoryError::EntityNotFound)
    }

    async fn save_or_update(&self, artist: &Artist) -> Result<i32, RepositoryError> {
        sqlx::query_scalar::<_, i32>("
INSERT INTO \"artist\" (name)
VALUES ($1)
RETURNING id
    ")
            .bind(artist.name.clone())
            .fetch_one(&self.pool)
            .await
            .map_err(|_| RepositoryError::EntityNotSaved)
    }
}

impl RepoByName<Artist> for ArtistRepo {
    async fn get_by_name(&self, name: &str) -> Result<Artist, RepositoryError> {
        sqlx::query_as::<_, Artist>("
SELECT id, name
FROM \"artist\"
WHERE name = $1
LIMIT 1
")
            .bind(name)
            .fetch_one(&self.pool)
            .await
            .map_err(|_| RepositoryError::EntityNotFound)
    }
}

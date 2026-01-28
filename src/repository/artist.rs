use derive_new::new;
use crate::repository::{Entity, Repo, RepoByName, RepositoryError};
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

pub struct ArtistRepo {
    pub pool: Pool<Postgres>,
}

impl ArtistRepo {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}
impl Repo<Artist> for ArtistRepo {
    async fn get(&self, id: &str) -> Result<Artist, RepositoryError> {
        let parsed_id = id.parse::<i32>().map_err(|_| RepositoryError::EntityNotFound)?;
        sqlx::query_as::<_, Artist>("
SELECT id, name
FROM \"artist\"
WHERE id = $1
LIMIT 1
")
            .bind(parsed_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|_| RepositoryError::EntityNotFound)
    }

    async fn save_or_update(&self, artist: &Artist) -> Result<(), RepositoryError> {
        sqlx::query("
INSERT INTO \"artist\" (name)
VALUES ($1)
    ")
            .bind(artist.name.clone())
            .execute(&self.pool)
            .await
            .map(|_| ())
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

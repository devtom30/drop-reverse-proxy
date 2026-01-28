use sqlx::{Pool, Postgres};
use crate::repository::{Entity, Repo, RepositoryError};

#[derive(sqlx::FromRow, Debug, Clone, PartialEq)]
pub struct Playlist {
    pub id: i32,
    pub name: String,
}

impl Entity for Playlist {
    fn id(&self) -> String {
        self.id.to_string()
    }
}

pub struct PlaylistRepo {
    pub pool: Pool<Postgres>,
}

impl PlaylistRepo {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

impl Repo<Playlist> for PlaylistRepo {
    async fn get(&self, id: &str) -> Result<Playlist, RepositoryError> {
        let parsed_id = id.parse::<i32>().map_err(|_| RepositoryError::EntityNotFound)?;
        sqlx::query_as::<_, Playlist>("
SELECT id, name
FROM \"playlist\"
WHERE id = $1
LIMIT 1
")
            .bind(parsed_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|_| RepositoryError::EntityNotFound)
    }

    async fn save_or_update(&self, playlist: &Playlist) -> Result<(), RepositoryError> {
        sqlx::query("
INSERT INTO \"playlist\" (name)
VALUES ($1)
    ")
            .bind(playlist.name.clone())
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|_| RepositoryError::EntityNotSaved)
    }
}


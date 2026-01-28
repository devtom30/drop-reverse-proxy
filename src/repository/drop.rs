use sqlx::{Pool, Postgres};
use crate::repository::{Entity, Repo, RepositoryError};

#[derive(sqlx::FromRow, Debug, Clone, PartialEq)]
pub struct Drop {
    pub id: i32,
    pub artist_id: i32,
    pub type_id: i16,
    pub artwork_id: i32,
}

impl Entity for Drop {
    fn id(&self) -> String {
        self.id.to_string()
    }
}

pub struct DropRepo {
    pub pool: Pool<Postgres>,
}

impl DropRepo {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

impl Repo<Drop> for DropRepo {
    async fn get(&self, id: &str) -> Result<Drop, RepositoryError> {
        let parsed_id = id.parse::<i32>().map_err(|_| RepositoryError::EntityNotFound)?;
        sqlx::query_as::<_, Drop>("
SELECT id, artist_id, artwork_id, type_id
FROM \"drop\"
WHERE id = $1
LIMIT 1
")
            .bind(parsed_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|_| RepositoryError::EntityNotFound)
    }

    async fn save_or_update(&self, drop: &Drop) -> Result<(), RepositoryError> {
        sqlx::query("
INSERT INTO \"drop\" (artist_id, artwork_id, type_id)
VALUES ($1, $2, $3)
    ")
            .bind(drop.artist_id)
            .bind(drop.artwork_id)
            .bind(drop.type_id)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|_| RepositoryError::EntityNotSaved)
    }
}

use sqlx::{sqlite::SqliteRow, FromRow, Row, SqlitePool};

use crate::error::AppError;

pub async fn fetch_user_by_username(
    pool: &SqlitePool,
    username: &str,
) -> Result<Option<User>, AppError> {
    let row = sqlx::query(
        "SELECT id, username, email, pwd_hash, created_at FROM users WHERE username = ?",
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|row| User::from_row(&row)).transpose()?)
}

pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub pwd_hash: String,
    pub created_at: String,
}

impl FromRow<'_, SqliteRow> for User {
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get("id")?,
            username: row.try_get("username")?,
            email: row.try_get("email")?,
            pwd_hash: row.try_get("pwd_hash")?,
            created_at: row.try_get("created_at")?,
        })
    }
}

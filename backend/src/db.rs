//! Schema del database (SQLite).

use shared::Announcement;
use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;

/// Riga della tabella `announcements` così com'è su disco.
#[derive(Debug, FromRow)]
pub struct AnnouncementRow {
    pub id: String,
    pub dog_name: String,
    pub breed: Option<String>,
    pub age: Option<String>,
    pub description: String,
    pub city: Option<String>,
    pub contact: String,
    pub image_path: String,
    pub created_at: String,
}

impl From<AnnouncementRow> for Announcement {
    fn from(row: AnnouncementRow) -> Self {
        Announcement {
            id: row.id,
            dog_name: row.dog_name,
            breed: row.breed,
            age: row.age,
            description: row.description,
            city: row.city,
            contact: row.contact,
            image_url: format!("/uploads/{}", row.image_path),
            created_at: row.created_at,
        }
    }
}

/// Migrazione minimale all'avvio: crea lo schema se non esiste.
pub async fn ensure_schema(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS announcements (
            id             TEXT PRIMARY KEY,
            dog_name       TEXT NOT NULL,
            breed          TEXT,
            age            TEXT,
            description    TEXT NOT NULL,
            city           TEXT,
            contact        TEXT NOT NULL,
            image_path     TEXT NOT NULL,
            created_at     TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;
    Ok(())
}

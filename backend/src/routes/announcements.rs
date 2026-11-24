//! Endpoint per gli annunci dei cani.

use axum::Json;
use axum::extract::multipart::Field;
use axum::extract::{Multipart, Path, State};
use axum::http::StatusCode;
use shared::Announcement;

use crate::AppState;
use crate::auth::AuthUser as Authenticated;
use crate::db::AnnouncementRow;
use crate::error::ApiError;

/// Dimensione massima dell'immagine: 10 MB.
const MAX_IMAGE_SIZE: usize = 10 * 1024 * 1024;

/// Immagine caricata dal client.
struct ImageUpload {
    content_type: String,
    bytes: Vec<u8>,
}

/// `GET /api/announcements` — elenco di tutti gli annunci, dal più recente.
pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<Announcement>>, ApiError> {
    let rows = sqlx::query_as::<_, AnnouncementRow>(
        "SELECT * FROM announcements ORDER BY created_at DESC",
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

/// `GET /api/announcements/{id}` — dettaglio di un singolo annuncio.
pub async fn get_by_id(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Announcement>, ApiError> {
    let row = sqlx::query_as::<_, AnnouncementRow>("SELECT * FROM announcements WHERE id = ?1")
        .bind(&id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(ApiError::NotFound)?;

    Ok(Json(row.into()))
}

/// `POST /api/announcements` — crea un annuncio (`multipart/form-data`).
///
/// Campi testuali: `dog_name`, `breed`, `age`, `city`, `description`, `contact`.
/// Campo binario: `image` (png/jpeg/webp/gif).
/// Richiede `Authorization: Bearer <token>`: la lettura resta pubblica, la scrittura no.
pub async fn create(
    State(state): State<AppState>,
    authenticated: Authenticated,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<Announcement>), ApiError> {
    let mut dog_name = String::new();
    let mut breed = None;
    let mut age = None;
    let mut city = None;
    let mut description = String::new();
    let mut contact = String::new();
    let mut image = None;

    while let Some(field) = multipart.next_field().await? {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "dog_name" => dog_name = field.text().await?.trim().to_string(),
            "breed" => breed = optional_text(field).await?,
            "age" => age = optional_text(field).await?,
            "city" => city = optional_text(field).await?,
            "description" => description = field.text().await?.trim().to_string(),
            "contact" => contact = field.text().await?.trim().to_string(),
            "image" => {
                let content_type = field.content_type().unwrap_or_default().to_string();
                let bytes = field.bytes().await?.to_vec();
                image = Some(ImageUpload {
                    content_type,
                    bytes,
                });
            }
            // Ignora eventuali campi extra inviati dal client.
            _ => {
                let _ = field.bytes().await?;
            }
        }
    }

    // Validazione dei campi obbligatori.
    if dog_name.is_empty() {
        return Err(ApiError::BadRequest(
            "Il nome del cane è obbligatorio".into(),
        ));
    }
    if description.is_empty() {
        return Err(ApiError::BadRequest("La descrizione è obbligatoria".into()));
    }
    if contact.is_empty() {
        return Err(ApiError::BadRequest("Un contatto è obbligatorio".into()));
    }

    let image = image.ok_or_else(|| ApiError::BadRequest("L'immagine è obbligatoria".into()))?;
    if image.bytes.is_empty() {
        return Err(ApiError::BadRequest("L'immagine è vuota".into()));
    }
    if image.bytes.len() > MAX_IMAGE_SIZE {
        return Err(ApiError::BadRequest("L'immagine supera i 10 MB".into()));
    }

    let extension = match image.content_type.as_str() {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        "image/gif" => "gif",
        other => {
            return Err(ApiError::BadRequest(format!(
                "Tipo di immagine non supportato: {other}"
            )));
        }
    };

    let id = uuid::Uuid::new_v4().to_string();
    let filename = format!("{id}.{extension}");
    let image_path = state.uploads_dir.join(&filename);
    tokio::fs::write(&image_path, &image.bytes).await?;

    let created_at = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO announcements
            (id, dog_name, breed, age, description, city, contact, image_path, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        "#,
    )
    .bind(&id)
    .bind(&dog_name)
    .bind(&breed)
    .bind(&age)
    .bind(&description)
    .bind(&city)
    .bind(&contact)
    .bind(&filename)
    .bind(&created_at)
    .execute(&state.pool)
    .await?;

    let announcement = Announcement {
        id,
        dog_name,
        breed,
        age,
        description,
        city,
        contact,
        image_url: format!("/uploads/{filename}"),
        created_at,
    };

    tracing::info!(
        "Creato annuncio {} ({}) da {}",
        announcement.id,
        announcement.dog_name,
        authenticated.email
    );

    Ok((StatusCode::CREATED, Json(announcement)))
}

/// Legge un campo testuale opzionale: il vuoto diventa `None`.
async fn optional_text(field: Field<'_>) -> Result<Option<String>, ApiError> {
    let value = field.text().await?.trim().to_string();
    Ok((!value.is_empty()).then_some(value))
}

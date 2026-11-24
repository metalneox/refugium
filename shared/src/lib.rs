//! Tipi condivisi tra backend (Axum) e frontend (Leptos/WASM).
//!
//! Questo crate viene compilato sia per target nativo sia per `wasm32`,
//! quindi deve contenere solo tipi e dipendenze portabili.

use serde::{Deserialize, Serialize};

/// Un annuncio di un cane pubblicato sul rifugio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Announcement {
    pub id: String,
    pub dog_name: String,
    pub breed: Option<String>,
    pub age: Option<String>,
    pub description: String,
    pub city: Option<String>,
    pub contact: String,

    /// URL pubblico dell'immagine, servito dal backend su `/uploads`.
    pub image_url: String,

    /// Timestamp di pubblicazione (RFC 3339).
    pub created_at: String,
}

/// Utente registrato (senza dati sensibili: mai la password).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    pub id: String,
    pub email: String,
    pub created_at: String,
}

/// Credenziali inviate dal frontend per registrazione e login.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub email: String,
    pub password: String,
}

/// Risposta di registrazione/login: token JWT + dati dell'utente.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: AuthUser,
}

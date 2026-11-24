//! Autenticazione: password con hash Argon2 e token JWT (HS256).
//!
//! La lettura degli annunci resta pubblica; solo la creazione richiede un token.

use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use chrono::Utc;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use shared::Credentials;

use crate::AppState;
use crate::error::ApiError;

/// Chiave di sviluppo usata solo se `REFUGIUM_JWT_SECRET` non è impostata.
const DEFAULT_SECRET: &str = "refugium-dev-secret-change-me";

/// Durata di validità del token: 7 giorni.
const TOKEN_TTL_SECONDS: i64 = 7 * 24 * 60 * 60;

/// Configurazione JWT, condivisa con tutti gli handler tramite `AppState`.
#[derive(Clone)]
pub struct Auth {
    secret: String,
}

impl Auth {
    /// Legge la chiave dall'ambiente; in assenza usa quella di sviluppo.
    pub fn from_env() -> Self {
        let secret = std::env::var("REFUGIUM_JWT_SECRET").unwrap_or_else(|_| {
            tracing::warn!(
                "REFUGIUM_JWT_SECRET non impostato: uso una chiave di sviluppo. Impostala in produzione."
            );
            DEFAULT_SECRET.to_string()
        });
        Self { secret }
    }

    /// Crea l'hash di una password con Argon2id e salt casuale.
    pub fn hash_password(password: &str) -> anyhow::Result<String> {
        let salt = argon2::password_hash::SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|error| {
                anyhow::anyhow!("Impossibile generare l'hash della password: {error}")
            })?;
        Ok(hash.to_string())
    }

    /// Verifica una password contro l'hash salvato (tempo costante).
    pub fn verify_password(password: &str, hash: &str) -> bool {
        match PasswordHash::new(hash) {
            Ok(parsed) => Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok(),
            Err(_) => false,
        }
    }

    /// Genera un JWT HS256 firmato con la chiave configurata.
    pub fn issue_token(&self, id: &str, email: &str) -> anyhow::Result<String> {
        let now = Utc::now().timestamp();
        let claims = Claims {
            sub: id.to_string(),
            email: email.to_string(),
            iat: now,
            exp: now + TOKEN_TTL_SECONDS,
        };
        Ok(encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )?)
    }

    /// Valida firma, scadenza e campi obbligatori di un JWT.
    pub fn verify_token(&self, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_required_spec_claims(&["sub", "email", "exp"]);
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &validation,
        )
        .map(|data| data.claims)
    }
}

/// Payload del token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Claims {
    /// ID dell'utente (`subject`).
    sub: String,
    email: String,
    /// `issued at`, secondi Unix.
    iat: i64,
    /// `expiration`, secondi Unix.
    exp: i64,
}

/// Dati dell'utente autenticato, ricavati dal token.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: String,
    pub email: String,
}

/// Estrattore Axum: rifiuta la richiesta senza `Authorization: Bearer <token>` valido.
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| ApiError::Unauthorized("Autenticazione richiesta".into()))?;

        let token = header.strip_prefix("Bearer ").unwrap_or_default().trim();
        if token.is_empty() {
            return Err(ApiError::Unauthorized("Token non valido".into()));
        }

        let claims = state.auth.verify_token(token).map_err(|_| {
            ApiError::Unauthorized("Token non valido o scaduto: accedi di nuovo".into())
        })?;

        Ok(AuthUser {
            id: claims.sub,
            email: claims.email,
        })
    }
}

/// Controlli sulle credenziali in fase di registrazione/login.
pub fn validate_credentials(credentials: &Credentials) -> Result<(), ApiError> {
    if !is_valid_email(credentials.email.trim()) {
        return Err(ApiError::BadRequest("Indirizzo email non valido".into()));
    }
    let length = credentials.password.chars().count();
    if !(8..=128).contains(&length) {
        return Err(ApiError::BadRequest(
            "La password deve contenere tra 8 e 128 caratteri".into(),
        ));
    }
    Ok(())
}

/// Validazione email minimale, allineata a quella del frontend.
pub fn is_valid_email(value: &str) -> bool {
    let mut parts = value.split('@');
    let local = parts.next().unwrap_or_default();
    let domain = parts.next().unwrap_or_default();
    if parts.next().is_some() || local.is_empty() || domain.is_empty() {
        return false;
    }
    let clean = |part: &str| !part.starts_with('.') && !part.ends_with('.') && !part.contains("..");
    clean(local) && domain.contains('.') && clean(domain)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_hash_roundtrip() {
        let hash = Auth::hash_password("correct horse battery staple").unwrap();
        assert!(Auth::verify_password("correct horse battery staple", &hash));
        assert!(!Auth::verify_password("wrong password", &hash));
        // Due hash della stessa password sono diversi (salt casuale).
        let other = Auth::hash_password("correct horse battery staple").unwrap();
        assert_ne!(hash, other);
    }

    #[test]
    fn token_roundtrip_and_tampering() {
        let auth = Auth {
            secret: "test-secret".into(),
        };
        let token = auth.issue_token("user-1", "mario@example.com").unwrap();
        let claims = auth.verify_token(&token).unwrap();
        assert_eq!(claims.sub, "user-1");
        assert_eq!(claims.email, "mario@example.com");

        // Firma con un'altra chiave o token troncato => rifiutato.
        let other = Auth {
            secret: "another-secret".into(),
        };
        assert!(other.verify_token(&token).is_err());
        assert!(auth.verify_token(&token[..token.len() - 4]).is_err());
    }

    #[test]
    fn credentials_validation_rules() {
        let valid = Credentials {
            email: "mario@example.com".into(),
            password: "12345678".into(),
        };
        assert!(validate_credentials(&valid).is_ok());

        let bad_email = Credentials {
            email: "mario@example".into(),
            password: "12345678".into(),
        };
        assert!(validate_credentials(&bad_email).is_err());

        let short_password = Credentials {
            email: "mario@example.com".into(),
            password: "short".into(),
        };
        assert!(validate_credentials(&short_password).is_err());
    }
}

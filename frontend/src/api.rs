//! Chiamate HTTP all'API del backend tramite `fetch` (via `web-sys`).

use serde::de::DeserializeOwned;
use serde_wasm_bindgen::from_value;
use shared::{Announcement, AuthResponse, AuthUser, Credentials};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

/// Chiave del JWT nel `localStorage` del browser.
const TOKEN_KEY: &str = "refugium_token";

/// Dati testuali di un nuovo annuncio, raccolti dal form.
pub struct NewAnnouncement {
    pub dog_name: String,
    pub breed: Option<String>,
    pub age: Option<String>,
    pub city: Option<String>,
    pub description: String,
    pub contact: String,
}

fn js_error(value: wasm_bindgen::JsValue) -> String {
    value
        .as_string()
        .unwrap_or_else(|| "Errore di rete sconosciuto".to_string())
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

/// JWT salvato dopo login/registrazione, se presente.
pub fn get_token() -> Option<String> {
    storage()?.get_item(TOKEN_KEY).ok().flatten()
}

/// Salva il JWT per le prossime richieste e per il ripristino della sessione.
pub fn set_token(token: &str) {
    if let Some(storage) = storage() {
        let _ = storage.set_item(TOKEN_KEY, token);
    }
}

/// Rimuove il JWT (logout o token non più valido).
pub fn clear_token() {
    if let Some(storage) = storage() {
        let _ = storage.remove_item(TOKEN_KEY);
    }
}

/// Intestazioni con Bearer token, quando c'è una sessione attiva.
fn auth_headers() -> Result<web_sys::Headers, String> {
    let headers = web_sys::Headers::new().map_err(js_error)?;
    if let Some(token) = get_token() {
        headers
            .append("Authorization", &format!("Bearer {token}"))
            .map_err(js_error)?;
    }
    Ok(headers)
}

async fn fetch_response(request: web_sys::Request) -> Result<web_sys::Response, String> {
    let window = web_sys::window().ok_or_else(|| "Browser non disponibile".to_string())?;
    let response = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(js_error)?;
    response
        .dyn_into::<web_sys::Response>()
        .map_err(|_| "Risposta non valida dal server".to_string())
}

async fn parse_json<T: DeserializeOwned>(response: web_sys::Response) -> Result<T, String> {
    let promise = response
        .json()
        .map_err(|_| "La risposta non contiene JSON".to_string())?;
    let value = JsFuture::from(promise).await.map_err(js_error)?;
    from_value(value).map_err(|error| format!("JSON non valido: {error}"))
}

async fn error_from_response(response: web_sys::Response) -> String {
    let status = response.status();
    let body = match response.text() {
        Ok(promise) => JsFuture::from(promise)
            .await
            .ok()
            .and_then(|value| value.as_string()),
        Err(_) => None,
    };
    match body {
        Some(text) if !text.is_empty() => format!("Errore {status}: {text}"),
        _ => format!("Richiesta fallita (HTTP {status})"),
    }
}

/// `GET /api/announcements` — elenco completo degli annunci (pubblico).
pub async fn list_announcements() -> Result<Vec<Announcement>, String> {
    let request = web_sys::Request::new_with_str("/api/announcements")
        .map_err(|_| "Impossibile preparare la richiesta".to_string())?;
    let response = fetch_response(request).await?;
    if !response.ok() {
        return Err(error_from_response(response).await);
    }
    parse_json(response).await
}

/// Costruisce una POST JSON con le credenziali (registrazione o login).
fn credentials_request(path: &str, credentials: &Credentials) -> Result<web_sys::Request, String> {
    let body = serde_json::to_string(credentials)
        .map_err(|error| format!("Impossibile preparare le credenziali: {error}"))?;

    let init = web_sys::RequestInit::new();
    init.set_method("POST");
    init.set_body(&JsValue::from_str(&body));

    let headers = auth_headers()?;
    headers
        .append("Content-Type", "application/json")
        .map_err(js_error)?;
    init.set_headers_headers(&headers);

    web_sys::Request::new_with_str_and_init(path, &init)
        .map_err(|_| "Impossibile preparare la richiesta".to_string())
}

async fn post_credentials(path: &str, credentials: &Credentials) -> Result<AuthResponse, String> {
    let request = credentials_request(path, credentials)?;
    let response = fetch_response(request).await?;
    if !response.ok() {
        return Err(error_from_response(response).await);
    }
    parse_json(response).await
}

/// `POST /api/auth/register` — crea un account e restituisce token + utente.
pub async fn register(credentials: &Credentials) -> Result<AuthResponse, String> {
    post_credentials("/api/auth/register", credentials).await
}

/// `POST /api/auth/login` — accede e restituisce token + utente.
pub async fn login(credentials: &Credentials) -> Result<AuthResponse, String> {
    post_credentials("/api/auth/login", credentials).await
}

/// `GET /api/auth/me` — ripristina l'utente dal token salvato.
pub async fn me() -> Result<AuthUser, String> {
    if get_token().is_none() {
        return Err("Nessuna sessione attiva".to_string());
    }
    let init = web_sys::RequestInit::new();
    init.set_method("GET");
    init.set_headers_headers(&auth_headers()?);
    let request = web_sys::Request::new_with_str_and_init("/api/auth/me", &init)
        .map_err(|_| "Impossibile preparare la richiesta".to_string())?;
    let response = fetch_response(request).await?;
    if !response.ok() {
        return Err(error_from_response(response).await);
    }
    parse_json(response).await
}

/// `POST /api/announcements` — pubblica un annuncio con dati e foto.
/// Il token JWT viene aggiunto automaticamente se l'utente ha effettuato l'accesso.
pub async fn create_announcement(
    data: &NewAnnouncement,
    image: web_sys::File,
) -> Result<Announcement, String> {
    let form = web_sys::FormData::new().map_err(|_| "Impossibile preparare il form".to_string())?;

    form.append_with_str("dog_name", &data.dog_name)
        .map_err(js_error)?;
    if let Some(value) = &data.breed {
        form.append_with_str("breed", value).map_err(js_error)?;
    }
    if let Some(value) = &data.age {
        form.append_with_str("age", value).map_err(js_error)?;
    }
    if let Some(value) = &data.city {
        form.append_with_str("city", value).map_err(js_error)?;
    }
    form.append_with_str("description", &data.description)
        .map_err(js_error)?;
    form.append_with_str("contact", &data.contact)
        .map_err(js_error)?;
    form.append_with_blob("image", &image).map_err(js_error)?;

    let init = web_sys::RequestInit::new();
    init.set_method("POST");
    init.set_body(&form);
    init.set_headers_headers(&auth_headers()?);

    let request = web_sys::Request::new_with_str_and_init("/api/announcements", &init)
        .map_err(|_| "Impossibile preparare la richiesta".to_string())?;
    let response = fetch_response(request).await?;
    if !response.ok() {
        return Err(error_from_response(response).await);
    }
    parse_json(response).await
}

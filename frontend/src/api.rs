//! Chiamate HTTP all'API del backend tramite `fetch` (via `web-sys`).

use serde::de::DeserializeOwned;
use serde_wasm_bindgen::from_value;
use shared::Announcement;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

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
        Ok(promise) => JsFuture::from(promise).await.ok().and_then(|value| value.as_string()),
        Err(_) => None,
    };
    match body {
        Some(text) if !text.is_empty() => format!("Errore {status}: {text}"),
        _ => format!("Richiesta fallita (HTTP {status})"),
    }
}

/// `GET /api/announcements` — elenco completo degli annunci.
pub async fn list_announcements() -> Result<Vec<Announcement>, String> {
    let request = web_sys::Request::new_with_str("/api/announcements")
        .map_err(|_| "Impossibile preparare la richiesta".to_string())?;
    let response = fetch_response(request).await?;
    if !response.ok() {
        return Err(error_from_response(response).await);
    }
    parse_json(response).await
}

/// `POST /api/announcements` — pubblica un annuncio con dati e foto.
pub async fn create_announcement(
    data: &NewAnnouncement,
    image: web_sys::File,
) -> Result<Announcement, String> {
    let form = web_sys::FormData::new()
        .map_err(|_| "Impossibile preparare il form".to_string())?;

    form.append_with_str("dog_name", &data.dog_name).map_err(js_error)?;
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
    form.append_with_str("contact", &data.contact).map_err(js_error)?;
    form.append_with_blob("image", &image).map_err(js_error)?;

    let init = web_sys::RequestInit::new();
    init.set_method("POST");
    init.set_body(&form);

    let request = web_sys::Request::new_with_str_and_init("/api/announcements", &init)
        .map_err(|_| "Impossibile preparare la richiesta".to_string())?;
    let response = fetch_response(request).await?;
    if !response.ok() {
        return Err(error_from_response(response).await);
    }
    parse_json(response).await
}

//! Frontend di Refugium: SPA in Leptos (WASM), compilata con Trunk.

mod api;
mod app;

use app::App;
use leptos::mount::mount_to_body;

fn main() {
    mount_to_body(App);
}

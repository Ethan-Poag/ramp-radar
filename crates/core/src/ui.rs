//! Embedded web UI (built from /web into /web/dist). Falls back to a stub page when the
//! frontend hasn't been built, so `cargo test` works without Node.

use axum::http::{header, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../../web/dist"]
#[allow_missing = true]
struct Assets;

const STUB: &str = "<!doctype html><title>Aircraft Monitor</title><body style='font-family:sans-serif;padding:2rem'>\
<h1>Aircraft Monitor</h1><p>The web UI was not built into this binary. Run <code>npm ci && npm run build</code> in <code>web/</code> and rebuild.</p>";

pub async fn static_handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    match Assets::get(path) {
        Some(f) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            let cache = if path.starts_with("assets/") { "public, max-age=31536000, immutable" } else { "no-cache" };
            ([(header::CONTENT_TYPE, mime.to_string()), (header::CACHE_CONTROL, cache.to_string())], f.data.into_owned()).into_response()
        }
        None if path.starts_with("api/") || path.contains('.') => StatusCode::NOT_FOUND.into_response(),
        None => match Assets::get("index.html") {
            Some(f) => ([(header::CONTENT_TYPE, "text/html".to_string())], f.data.into_owned()).into_response(),
            None => ([(header::CONTENT_TYPE, "text/html")], STUB).into_response(),
        },
    }
}

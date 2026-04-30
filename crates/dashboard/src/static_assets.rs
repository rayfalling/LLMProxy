//! Static SPA assets embedded into the dashboard binary.
//!
//! The frontend is built (`web/dist/`) by `cd web && npm run build`. The
//! `include_dir!` macro takes a snapshot of that directory at compile time so
//! the resulting binary is fully self-contained.
//!
//! Routing semantics (in `main.rs`):
//!
//! * `/api/*` is handled by the API router and never hits this fallback.
//! * Any other path is looked up inside the embedded directory; if found,
//!   the file's bytes are returned with a guessed mime type.
//! * If the path is not found, `index.html` is returned so React Router can
//!   resolve the SPA route on the client side.

use axum::{
    body::Body,
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use include_dir::{include_dir, Dir};

pub static WEB_DIST: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/../../web/dist");

pub async fn serve_spa(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');

    // 1. Exact asset hit (e.g. /assets/index-abc123.js).
    if !path.is_empty() {
        if let Some(file) = WEB_DIST.get_file(path) {
            return asset_response(path, file.contents()).into_response();
        }
    }

    // 2. SPA fallback to index.html so React Router can take over.
    match WEB_DIST.get_file("index.html") {
        Some(file) => asset_response("index.html", file.contents()).into_response(),
        None => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "dashboard frontend not embedded; rebuild after `cd web && npm run build`",
        )
            .into_response(),
    }
}

fn asset_response(path: &str, bytes: &'static [u8]) -> Response {
    let mime = mime_guess::from_path(path)
        .first_or_octet_stream()
        .as_ref()
        .to_string();
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .body(Body::from(bytes))
        .unwrap()
}

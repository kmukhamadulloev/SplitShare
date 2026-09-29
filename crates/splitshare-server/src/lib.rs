//! HTTP adapter and compile-time frontend assets. No file APIs exist in Phase 01.
use axum::{
    Json, Router,
    body::Body,
    http::{Method, StatusCode, Uri, header},
    response::{IntoResponse, Response},
};
use rust_embed::RustEmbed;
use serde::Serialize;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

#[derive(RustEmbed)]
#[folder = "../../web/dist/"]
struct Assets;

#[derive(Serialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}
#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: &'static str,
    details: Option<()>,
}

fn not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorEnvelope {
            error: ErrorBody {
                code: "NOT_FOUND",
                message: "The requested resource is unavailable.",
                details: None,
            },
        }),
    )
        .into_response()
}

async fn frontend(method: Method, uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    if path == "api" || path.starts_with("api/") || path == "j" || path.starts_with("j/") {
        return not_found();
    }
    if method != Method::GET && method != Method::HEAD {
        return (
            StatusCode::METHOD_NOT_ALLOWED,
            [(header::ALLOW, "GET, HEAD")],
        )
            .into_response();
    }
    // URI escapes and dot segments are never interpreted as filesystem paths.
    if path.contains('%') || path.contains('\\') || path.split('/').any(|p| p == "." || p == "..") {
        return not_found();
    }
    let name = if path.is_empty() { "index.html" } else { path };
    let (asset, content_type) = if let Some(asset) = Assets::get(name) {
        (
            asset,
            mime_guess::from_path(name)
                .first_or_octet_stream()
                .to_string(),
        )
    } else if !path.contains('.') {
        let Some(asset) = Assets::get("index.html") else {
            return not_found();
        };
        (asset, "text/html".to_owned())
    } else {
        return not_found();
    };
    let length = asset.data.len();
    let body = if method == Method::HEAD {
        Body::empty()
    } else {
        Body::from(asset.data.into_owned())
    };
    (
        [
            (header::CONTENT_TYPE, content_type),
            (header::CONTENT_LENGTH, length.to_string()),
            (header::CACHE_CONTROL, "no-cache".to_owned()),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff".to_owned()),
        ],
        body,
    )
        .into_response()
}

pub fn router() -> Router {
    Router::new().fallback(frontend)
}

/// The composition root owns cancellation; the HTTP adapter drains active requests.
pub async fn serve(listener: TcpListener, shutdown: CancellationToken) -> std::io::Result<()> {
    axum::serve(listener, router())
        .with_graceful_shutdown(shutdown.cancelled_owned())
        .await
}

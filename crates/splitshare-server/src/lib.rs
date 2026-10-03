//! HTTP adapter, streamed downloads, SSE and embedded frontend.
use axum::{
    Router,
    body::Body,
    http::{Method, StatusCode, Uri, header},
    response::{IntoResponse, Response},
};
use rust_embed::RustEmbed;
mod access;
mod api;
mod diagnostics;
mod error;
mod range;
mod transport;
mod uploads;
pub use api::ServerState;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

#[derive(RustEmbed)]
#[folder = "../../web/dist/"]
struct Assets;

fn not_found() -> Response {
    error::ApiError::new(
        StatusCode::NOT_FOUND,
        "NOT_FOUND",
        "The requested resource is unavailable.",
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

pub fn api_router(state: ServerState) -> Router {
    use axum::{
        extract::DefaultBodyLimit,
        middleware,
        routing::{get, post},
    };
    Router::new()
        .route("/j/{token}", get(access::join))
        .route("/api/v1/session/leave", post(access::leave))
        .route(
            "/api/v1/host/settings",
            get(access::settings).put(access::update),
        )
        .route(
            "/api/v1/host/logs",
            get(diagnostics::list).delete(diagnostics::clear),
        )
        .route(
            "/api/v1/host/logs/config",
            get(diagnostics::config).put(diagnostics::configure),
        )
        .route("/api/v1/host/logs/events", get(diagnostics::events))
        .route("/api/v1/host/network", get(access::network))
        .route(
            "/api/v1/host/setup",
            get(access::setup).put(access::configure_network),
        )
        .route("/api/v1/host/folder", post(access::choose_folder))
        .route("/api/v1/host/share-token/rotate", post(access::rotate))
        .route("/api/v1/status", get(api::status))
        .route("/api/v1/files", get(api::list).delete(api::delete))
        .route("/api/v1/directories", post(api::mkdir))
        .route("/api/v1/files/rename", post(api::rename))
        .route("/api/v1/files/download", get(api::download))
        .route("/api/v1/events", get(api::events))
        .route(
            "/api/v1/uploads",
            post(uploads::upload).layer(DefaultBodyLimit::disable()),
        )
        .route("/api/v1/transfers", get(uploads::list))
        .route(
            "/api/v1/transfers/{id}",
            axum::routing::delete(uploads::cancel),
        )
        .fallback(frontend)
        .method_not_allowed_fallback(|| async {
            error::ApiError::new(
                StatusCode::METHOD_NOT_ALLOWED,
                "METHOD_NOT_ALLOWED",
                "The method is not supported.",
            )
        })
        .layer(DefaultBodyLimit::max(16 * 1024))
        .layer(middleware::from_fn_with_state(state.clone(), api::guard))
        .with_state(state)
}

/// The composition root owns cancellation; SSE and file streams share this token.
pub async fn serve(listener: TcpListener, shutdown: CancellationToken) -> std::io::Result<()> {
    transport::serve(listener, router(), shutdown).await
}
pub async fn serve_api(listener: TcpListener, state: ServerState) -> std::io::Result<()> {
    let shutdown = state.shutdown.clone();
    transport::serve(listener, api_router(state), shutdown).await
}

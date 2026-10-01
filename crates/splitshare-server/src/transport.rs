//! Bounded HTTP/1 transport. Socket peer identity is attached here, never from headers.
use axum::{Extension, Router, extract::ConnectInfo};
use hyper_util::{
    rt::{TokioIo, TokioTimer},
    service::TowerToHyperService,
};
use std::{sync::Arc, time::Duration};
use tokio::{net::TcpListener, sync::Semaphore, task::JoinSet};
use tokio_util::sync::CancellationToken;
const CONNECTION_LIMIT: usize = 128;
const HEADER_TIMEOUT: Duration = Duration::from_secs(10);

pub async fn serve(
    listener: TcpListener,
    router: Router,
    shutdown: CancellationToken,
) -> std::io::Result<()> {
    let router = router.layer(axum::middleware::from_fn(header_budget));
    let slots = Arc::new(Semaphore::new(CONNECTION_LIMIT));
    let mut tasks = JoinSet::new();
    loop {
        tokio::select! {
            biased;
            _ = shutdown.cancelled() => break,
            Some(result) = tasks.join_next(), if !tasks.is_empty() => {
                if result.is_err() { tracing::warn!("HTTP connection task failed"); }
            },
            accepted = listener.accept() => {
                let (socket, peer) = accepted?;
                let Ok(permit) = slots.clone().try_acquire_owned() else {
                    // No unbounded task or request allocation when the transport is full.
                    drop(socket); continue;
                };
                // Small control responses and SSE must not wait for delayed ACKs.
                if socket.set_nodelay(true).is_err() {
                    tracing::warn!("Could not disable TCP coalescing; small requests may be delayed");
                }
                let service = TowerToHyperService::new(router.clone().layer(Extension(ConnectInfo(peer))));
                let shutdown = shutdown.clone();
                tasks.spawn(async move {
                    let _permit = permit;
                    let mut builder = hyper::server::conn::http1::Builder::new();
                    builder.timer(TokioTimer::new()).header_read_timeout(HEADER_TIMEOUT)
                        .max_headers(64).max_buf_size(32 * 1024);
                    let connection = builder.serve_connection(TokioIo::new(socket), service);
                    tokio::pin!(connection);
                    tokio::select! {
                        // Never log a raw HTTP parser error: it may contain client data.
                        _ = &mut connection => {},
                        _ = shutdown.cancelled() => {
                            connection.as_mut().graceful_shutdown();
                            let _ = tokio::time::timeout(Duration::from_secs(5), &mut connection).await;
                        }
                    }
                });
            }
        }
    }
    drop(listener);
    while let Some(result) = tasks.join_next().await {
        if result.is_err() {
            tracing::warn!("HTTP connection task failed during shutdown");
        }
    }
    Ok(())
}

async fn header_budget(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    let bytes = request.uri().to_string().len()
        + request.method().as_str().len()
        + 12
        + request
            .headers()
            .iter()
            .map(|(name, value)| name.as_str().len() + value.len() + 4)
            .sum::<usize>();
    if request.headers().len() > 64 || bytes > 16 * 1024 {
        return crate::error::ApiError::new(
            axum::http::StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE,
            "HEADER_LIMIT",
            "Request headers exceed the supported limit.",
        )
        .into_response();
    }
    next.run(request).await
}

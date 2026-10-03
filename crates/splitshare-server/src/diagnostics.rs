//! Thin host-only diagnostics HTTP adapter; the application owns history/control.
use crate::{ServerState, error::ApiError};
use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{
        IntoResponse, Response, Sse,
        sse::{Event, KeepAlive},
    },
};
use serde::Deserialize;
use splitshare_application::diagnostics::{LogConfig, LogEntry, LogLevel};
use std::{convert::Infallible, time::Duration};
pub async fn list(State(state): State<ServerState>) -> Json<Vec<LogEntry>> {
    Json(state.diagnostics.snapshot())
}
pub async fn clear(State(state): State<ServerState>) -> StatusCode {
    state.diagnostics.clear();
    StatusCode::NO_CONTENT
}
pub async fn config(State(state): State<ServerState>) -> Json<LogConfig> {
    Json(state.diagnostics.config())
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Configure {
    level: LogLevel,
}
pub async fn configure(
    State(state): State<ServerState>,
    body: Result<Json<Configure>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<LogConfig>, ApiError> {
    let body = body.map_err(|_| ApiError::invalid())?.0;
    state.diagnostics.configure(body.level).map_err(|message| {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "LOGGING_UNAVAILABLE",
            message,
        )
    })?;
    tracing::info!("Logging level changed");
    Ok(Json(state.diagnostics.config()))
}
pub async fn events(State(state): State<ServerState>) -> Result<Response, ApiError> {
    let permit = state.streams.clone().try_acquire_owned().map_err(|_| {
        ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "EVENT_LIMIT",
            "Too many event connections.",
        )
    })?;
    let mut receiver = state.diagnostics.subscribe();
    let stream = async_stream::stream! {
     let _permit=permit;
     yield Ok::<_,Infallible>(Event::default().event("logs.changed").data("{}"));
     loop {
      tokio::select! {
       _=state.shutdown.cancelled()=>break,
       event=receiver.recv()=>match event {
        Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_))=>yield Ok(Event::default().event("logs.changed").data("{}")),
        Err(tokio::sync::broadcast::error::RecvError::Closed)=>break,
       }
      }
     }
    };
    Ok(Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
        .into_response())
}

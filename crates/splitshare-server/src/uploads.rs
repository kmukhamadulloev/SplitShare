//! Upload HTTP adapter. Lifecycle, workers and concurrency belong to TransferManager.
use crate::{ServerState, error::ApiError};
use axum::{
    Json,
    body::Body,
    extract::{Path, Query, State, rejection::QueryRejection},
    http::{HeaderMap, StatusCode},
};
use futures_util::StreamExt;
use serde::Deserialize;
use splitshare_application::transfers::{TransferError, TransferManager, UploadRequest};
use splitshare_core::{ConflictPolicy, Transfer, VirtualPath};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UploadQuery {
    path: VirtualPath,
    #[serde(default)]
    policy: ConflictPolicy,
}
fn manager(state: &ServerState) -> Result<&TransferManager, ApiError> {
    state.transfers.as_ref().ok_or_else(|| {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "SHARE_NOT_CONFIGURED",
            "The host has not selected a shared folder.",
        )
    })
}
fn header<'a>(headers: &'a HeaderMap, name: &str) -> Result<&'a str, ApiError> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(ApiError::invalid)
}
impl From<TransferError> for ApiError {
    fn from(error: TransferError) -> Self {
        match error {
            TransferError::Storage(error) => error.into(),
            TransferError::Invalid => ApiError::invalid(),
            TransferError::Duplicate => Self::new(
                StatusCode::CONFLICT,
                "DUPLICATE_TRANSFER",
                "A transfer already uses that ID.",
            ),
            TransferError::Capacity => Self::new(
                StatusCode::TOO_MANY_REQUESTS,
                "TRANSFER_LIMIT",
                "The transfer queue is full.",
            ),
            TransferError::Missing => Self::new(
                StatusCode::NOT_FOUND,
                "TRANSFER_NOT_FOUND",
                "The transfer is unavailable.",
            ),
            TransferError::Forbidden => Self::new(
                StatusCode::FORBIDDEN,
                "TRANSFER_KEY_REQUIRED",
                "The transfer key is invalid.",
            ),
            TransferError::TooLate => Self::new(
                StatusCode::CONFLICT,
                "TRANSFER_FINALIZING",
                "The transfer is already finalizing or finished.",
            ),
            TransferError::QueueTimeout => Self::new(
                StatusCode::REQUEST_TIMEOUT,
                "UPLOAD_QUEUE_TIMEOUT",
                "Upload queue wait expired. Retry when capacity is available.",
            ),
            TransferError::IdleTimeout => Self::new(
                StatusCode::REQUEST_TIMEOUT,
                "UPLOAD_IDLE_TIMEOUT",
                "No upload data arrived for 60 seconds. Retry the upload.",
            ),
            TransferError::Cancelled => Self::new(
                StatusCode::CONFLICT,
                "TRANSFER_CANCELLED",
                "The transfer was cancelled.",
            ),
        }
    }
}
pub async fn upload(
    axum::Extension(access): axum::Extension<splitshare_application::sessions::Access>,
    State(state): State<ServerState>,
    query: Result<Query<UploadQuery>, QueryRejection>,
    headers: HeaderMap,
    body: Body,
) -> Result<(StatusCode, Json<Transfer>), ApiError> {
    let query = query.map_err(|_| ApiError::invalid())?.0;
    if header(&headers, "content-type")? != "application/octet-stream" {
        return Err(ApiError::new(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "INVALID_CONTENT_TYPE",
            "Send one raw file with application/octet-stream.",
        ));
    }
    let request = UploadRequest {
        id: header(&headers, "x-transfer-id")?.into(),
        key: header(&headers, "x-transfer-key")?.into(),
        path: query.path,
        total: headers
            .get("content-length")
            .map(|value| {
                value
                    .to_str()
                    .ok()
                    .and_then(|value| value.parse().ok())
                    .ok_or_else(ApiError::invalid)
            })
            .transpose()?,
        policy: query.policy,
    };
    let stream = body
        .into_data_stream()
        .map(|result| result.map_err(|_| std::io::Error::from(std::io::ErrorKind::UnexpectedEof)));
    let manager = manager(&state)?;
    let transfer = tokio::select! {
        biased;
        _ = access.invalidated() => return Err(splitshare_application::sessions::AccessError::Unauthorized.into()),
        result = manager.upload(request, stream) => result?,
    };
    Ok((StatusCode::CREATED, Json(transfer)))
}
pub async fn list(State(state): State<ServerState>) -> Result<Json<Vec<Transfer>>, ApiError> {
    Ok(Json(manager(&state)?.snapshot()))
}
pub async fn cancel(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    manager(&state)?.cancel(&id, header(&headers, "x-transfer-key")?)?;
    Ok(StatusCode::ACCEPTED)
}

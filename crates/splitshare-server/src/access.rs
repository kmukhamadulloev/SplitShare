//! Cookie and host configuration transport adapters.
use crate::{ServerState, error::ApiError};
use axum::{
    Json,
    extract::{Path, State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use splitshare_application::sessions::{AccessError, SESSION_SECONDS};
use splitshare_core::HostSettings;

impl From<AccessError> for ApiError {
    fn from(error: AccessError) -> Self {
        let (status, code, message) = match error {
            AccessError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "SESSION_REQUIRED",
                "Open a current share link from the host to connect.",
            ),
            AccessError::Forbidden => (
                StatusCode::FORBIDDEN,
                "PERMISSION_DENIED",
                "The host has disabled this action.",
            ),
            AccessError::Capacity => (
                StatusCode::TOO_MANY_REQUESTS,
                "SESSION_LIMIT",
                "The session limit has been reached.",
            ),
            AccessError::Invalid => (
                StatusCode::BAD_REQUEST,
                "INVALID_SETTINGS",
                "Host settings are invalid.",
            ),
            AccessError::Busy => (
                StatusCode::CONFLICT,
                "TRANSFERS_ACTIVE",
                "Wait for transfers to finish before changing concurrency.",
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "SETTINGS_FAILED",
                "Unable to update sharing state.",
            ),
        };
        Self::new(status, code, message)
    }
}
pub fn cookie(headers: &HeaderMap) -> Option<&str> {
    let mut found = None;
    for header in headers.get_all(header::COOKIE) {
        for pair in header.to_str().ok()?.split(';') {
            if let Some(value) = pair.trim().strip_prefix("splitshare_session=") {
                if found.is_some()
                    || value.len() != 64
                    || !value.bytes().all(|b| b.is_ascii_hexdigit())
                {
                    return None;
                }
                found = Some(value);
            }
        }
    }
    found
}
pub async fn join(
    State(state): State<ServerState>,
    Path(token): Path<String>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    state.service()?;
    let session = state.sessions.join(&token, cookie(&headers))?;
    Ok((StatusCode::SEE_OTHER, [
        (header::LOCATION, "/".to_owned()),
        (header::SET_COOKIE, format!("splitshare_session={session}; HttpOnly; SameSite=Strict; Path=/; Max-Age={SESSION_SECONDS}")),
    ]).into_response())
}
pub async fn leave(State(state): State<ServerState>, headers: HeaderMap) -> impl IntoResponse {
    state.sessions.leave(cookie(&headers));
    (
        StatusCode::NO_CONTENT,
        [(
            header::SET_COOKIE,
            "splitshare_session=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0",
        )],
    )
}
pub async fn settings(State(state): State<ServerState>) -> Json<HostSettings> {
    Json(state.sessions.settings())
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsRequest {
    share_mode: splitshare_core::ShareMode,
    permissions: splitshare_core::PermissionSet,
    parallel_uploads_enabled: bool,
    max_parallel_uploads: u8,
}
impl From<SettingsRequest> for HostSettings {
    fn from(value: SettingsRequest) -> Self {
        Self {
            share_mode: value.share_mode,
            permissions: value.permissions,
            parallel_uploads_enabled: value.parallel_uploads_enabled,
            max_parallel_uploads: value.max_parallel_uploads,
        }
    }
}
pub async fn update(
    State(state): State<ServerState>,
    body: Result<Json<SettingsRequest>, JsonRejection>,
) -> Result<Json<HostSettings>, ApiError> {
    let settings: HostSettings = body
        .map_err(|error| {
            ApiError::new(
                error.status(),
                "INVALID_REQUEST",
                "The JSON settings are invalid or too large.",
            )
        })?
        .0
        .into();
    tokio::task::spawn_blocking(move || {
        state.sessions.update(
            settings.clone(),
            state.settings_store.as_deref(),
            state.transfers.as_ref(),
        )?;
        Ok(Json(settings))
    })
    .await
    .map_err(|_| ApiError::from(AccessError::Save))?
}
pub async fn rotate(State(state): State<ServerState>) -> Result<StatusCode, ApiError> {
    state.sessions.rotate()?;
    Ok(StatusCode::NO_CONTENT)
}
pub async fn network(State(state): State<ServerState>) -> Json<serde_json::Value> {
    let token_mode = state.sessions.settings().share_mode == splitshare_core::ShareMode::TokenLink;
    let suffix = if token_mode {
        format!("/j/{}", state.sessions.share_token())
    } else {
        "/".into()
    };
    Json(
        serde_json::json!({ "candidates": state.candidates.iter().map(|candidate| serde_json::json!({
        "interface": candidate.interface, "address": candidate.address, "kind": candidate.kind,
        "url": format!("{}{}",candidate.base_url,suffix),
    })).collect::<Vec<_>>() }),
    )
}

fn host_control(
    state: &ServerState,
) -> Result<&splitshare_application::host_control::HostControl, ApiError> {
    state.host_control.as_ref().ok_or_else(|| {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "HOST_CONTROL_UNAVAILABLE",
            "Host setup is unavailable in this server. Use the native SplitShare application.",
        )
    })
}
pub async fn setup(
    State(state): State<ServerState>,
) -> Result<Json<splitshare_application::host_control::Snapshot>, ApiError> {
    Ok(Json(host_control(&state)?.snapshot()))
}
fn submit(
    state: &ServerState,
    action: splitshare_application::host_control::Action,
) -> Result<
    (
        StatusCode,
        Json<splitshare_application::host_control::Snapshot>,
    ),
    ApiError,
> {
    let snapshot = host_control(state)?
        .submit(action)
        .map_err(|message| ApiError::new(StatusCode::CONFLICT, "HOST_SETUP_REJECTED", message))?;
    Ok((StatusCode::ACCEPTED, Json(snapshot)))
}
pub async fn choose_folder(
    State(state): State<ServerState>,
    body: axum::body::Bytes,
) -> Result<impl IntoResponse, ApiError> {
    if !body.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_REQUEST",
            "Folder selection takes no browser paths or body.",
        ));
    }
    submit(
        &state,
        splitshare_application::host_control::Action::ChooseFolder,
    )
}
pub async fn configure_network(
    State(state): State<ServerState>,
    body: Result<Json<splitshare_application::host_control::NetworkSettings>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let settings = body
        .map_err(|_| {
            ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_NETWORK_SETTINGS",
                "Choose an IPv4 interface and a port from 1 to 65535.",
            )
        })?
        .0;
    submit(
        &state,
        splitshare_application::host_control::Action::Network(settings),
    )
}

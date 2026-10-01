use crate::{error::ApiError, range};
use axum::{
    Json,
    body::Body,
    extract::{
        ConnectInfo, Query, Request, State,
        rejection::{JsonRejection, QueryRejection},
    },
    http::{HeaderMap, Method, StatusCode, header},
    middleware::Next,
    response::{
        IntoResponse, Response, Sse,
        sse::{Event, KeepAlive},
    },
};
use serde::{Deserialize, Serialize};
use splitshare_application::FileService;
use splitshare_application::sessions::{Access, SessionManager, SettingsStore};
use splitshare_core::{Capability, HostSettings, PermissionSet, VirtualPath};
use splitshare_network::LocalAddresses;
use std::{convert::Infallible, net::SocketAddr, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncSeekExt},
    sync::Semaphore,
};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct ServerState {
    pub files: Option<FileService>,
    pub transfers: Option<splitshare_application::transfers::TransferManager>,
    pub sessions: SessionManager,
    pub settings_store: Option<Arc<dyn SettingsStore>>,
    pub candidates: Vec<splitshare_network::AddressCandidate>,
    pub local_addresses: LocalAddresses,
    pub authorities: Vec<String>,
    pub dev_origin: Option<String>,
    pub shutdown: CancellationToken,
    streams: Arc<Semaphore>,
    downloads: Arc<Semaphore>,
}
impl ServerState {
    pub fn new(
        files: Option<FileService>,
        mut settings: HostSettings,
        open_lan: bool,
        local_addresses: LocalAddresses,
        authorities: Vec<String>,
        shutdown: CancellationToken,
    ) -> Self {
        if open_lan {
            settings.share_mode = splitshare_core::ShareMode::OpenLan;
        }
        let sessions =
            SessionManager::new(settings.clone()).expect("Secure random source required");
        let transfers = files.clone().map(|files| {
            splitshare_application::transfers::TransferManager::new(
                files,
                &settings,
                shutdown.clone(),
            )
            .expect("validated host settings")
        });
        Self {
            files,
            transfers,
            sessions,
            settings_store: None,
            candidates: vec![],
            local_addresses,
            authorities,
            dev_origin: None,
            shutdown,
            streams: Arc::new(Semaphore::new(32)),
            downloads: Arc::new(Semaphore::new(32)),
        }
    }
    pub(crate) fn service(&self) -> Result<&FileService, ApiError> {
        self.files.as_ref().ok_or_else(|| {
            ApiError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "SHARE_NOT_CONFIGURED",
                "The host has not selected a shared folder.",
            )
        })
    }
}

async fn guard_inner(
    State(state): State<ServerState>,
    mut request: Request,
    next: Next,
) -> Response {
    let denied = || {
        ApiError::new(
            StatusCode::FORBIDDEN,
            "REQUEST_NOT_ALLOWED",
            "This request is not allowed.",
        )
        .into_response()
    };
    let Some(authority) = request
        .headers()
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
    else {
        return denied();
    };
    if !state
        .authorities
        .iter()
        .any(|known| known.eq_ignore_ascii_case(authority))
    {
        return denied();
    }
    if let Some(origin) = request.headers().get(header::ORIGIN) {
        let origin = origin.to_str().unwrap_or("");
        if origin != format!("http://{authority}") && state.dev_origin.as_deref() != Some(origin) {
            return denied();
        }
    }
    if request.uri().path().starts_with("/api/") {
        let Some(peer) = request.extensions().get::<ConnectInfo<SocketAddr>>() else {
            return denied();
        };
        let local = state.local_addresses.is_local(peer.0.ip());
        let path = request.uri().path();
        if path.starts_with("/api/v1/host/") && !local {
            return ApiError::new(
                StatusCode::FORBIDDEN,
                "HOST_CLIENT_REQUIRED",
                "Only the host can change or read host configuration.",
            )
            .into_response();
        }
        let capability = match (request.method(), path) {
            (&Method::GET | &Method::HEAD, "/api/v1/files") => Some(Capability::Browse),
            (&Method::GET | &Method::HEAD, "/api/v1/files/download") => Some(Capability::Download),
            (&Method::POST, "/api/v1/uploads") => Some(Capability::Upload),
            (&Method::POST, "/api/v1/directories") => Some(Capability::CreateDirectory),
            (&Method::POST, "/api/v1/files/rename") => Some(Capability::Rename),
            (&Method::DELETE, "/api/v1/files") => Some(Capability::Delete),
            (&Method::GET | &Method::HEAD, "/api/v1/transfers") => Some(Capability::Upload),
            _ => None,
        };
        // Leaving remains possible after expiry or revocation.
        if path != "/api/v1/session/leave" {
            let access = match state.sessions.authorize(
                local,
                crate::access::cookie(request.headers()),
                capability,
            ) {
                Ok(access) => access,
                Err(error) => return ApiError::from(error).into_response(),
            };
            request.extensions_mut().insert(access);
        }
        if !matches!(
            *request.method(),
            Method::GET | Method::HEAD | Method::OPTIONS
        ) && request
            .headers()
            .get("x-splitshare-request")
            .and_then(|v| v.to_str().ok())
            != Some("1")
        {
            return denied();
        }
    }
    // Metadata is small and has a total read deadline. Raw uploads retain streaming
    // and their own idle/queue limits; never buffer their bodies here.
    if request.uri().path() != "/api/v1/uploads"
        && !matches!(*request.method(), Method::GET | Method::HEAD)
    {
        let (parts, body) = request.into_parts();
        let bytes = match tokio::time::timeout(
            Duration::from_secs(10),
            axum::body::to_bytes(body, 16 * 1024),
        )
        .await
        {
            Ok(Ok(bytes)) => bytes,
            Ok(Err(_)) => {
                return ApiError::new(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "CONTROL_BODY_LIMIT",
                    "Control request body is invalid or exceeds 16 KiB.",
                )
                .into_response();
            }
            Err(_) => {
                return ApiError::new(
                    StatusCode::REQUEST_TIMEOUT,
                    "CONTROL_BODY_TIMEOUT",
                    "Control request body did not arrive within 10 seconds.",
                )
                .into_response();
            }
        };
        request = Request::from_parts(parts, Body::from(bytes));
    }
    next.run(request).await
}
pub async fn guard(state: State<ServerState>, request: Request, next: Next) -> Response {
    let mut response = guard_inner(state, request, next).await;
    response
        .headers_mut()
        .insert(header::X_CONTENT_TYPE_OPTIONS, "nosniff".parse().unwrap());
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    response
        .headers_mut()
        .insert(header::REFERRER_POLICY, "no-referrer".parse().unwrap());
    response
}

#[derive(Serialize)]
pub struct Status {
    version: &'static str,
    sharing: bool,
    root_label: &'static str,
    local_client: bool,
    share_mode: &'static str,
    upload_concurrency: usize,
    permissions: PermissionSet,
}
pub async fn status(
    State(state): State<ServerState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
) -> Json<Status> {
    let sharing = state.files.is_some();
    Json(Status {
        version: env!("CARGO_PKG_VERSION"),
        sharing,
        root_label: "Shared folder",
        local_client: state.local_addresses.is_local(peer.ip()),
        share_mode: match state.sessions.settings().share_mode {
            splitshare_core::ShareMode::OpenLan => "open_lan",
            _ => "token_link",
        },
        upload_concurrency: state
            .sessions
            .settings()
            .effective_upload_limit()
            .unwrap_or(1),
        permissions: if sharing {
            state
                .sessions
                .permissions(state.local_addresses.is_local(peer.ip()))
        } else {
            PermissionSet::all(false)
        },
    })
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PathQuery {
    #[serde(default = "VirtualPath::root")]
    path: VirtualPath,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PathBody {
    path: VirtualPath,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Create {
    parent: VirtualPath,
    name: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rename {
    from: VirtualPath,
    to: VirtualPath,
}
fn query(value: Result<Query<PathQuery>, QueryRejection>) -> Result<VirtualPath, ApiError> {
    value
        .map(|value| value.0.path)
        .map_err(|_| ApiError::invalid())
}
fn json<T>(value: Result<Json<T>, JsonRejection>) -> Result<T, ApiError> {
    value.map(|value| value.0).map_err(|error| {
        ApiError::new(
            error.status(),
            "INVALID_REQUEST",
            "The JSON request is invalid or too large.",
        )
    })
}
pub async fn list(
    State(state): State<ServerState>,
    request: Result<Query<PathQuery>, QueryRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let path = query(request)?;
    let entries = state.service()?.list(path.clone()).await?;
    Ok(Json(serde_json::json!({"path": path, "entries": entries})))
}
pub async fn mkdir(
    State(state): State<ServerState>,
    request: Result<Json<Create>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let body = json(request)?;
    let path = state.service()?.mkdir(body.parent, body.name).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({"path": path}))))
}
pub async fn rename(
    State(state): State<ServerState>,
    request: Result<Json<Rename>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    let body = json(request)?;
    state.service()?.rename(body.from, body.to).await?;
    Ok(StatusCode::NO_CONTENT)
}
pub async fn delete(
    State(state): State<ServerState>,
    request: Result<Json<PathBody>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    state.service()?.delete(json(request)?.path).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn download(
    axum::Extension(access): axum::Extension<Access>,
    State(state): State<ServerState>,
    method: Method,
    headers: HeaderMap,
    request: Result<Query<PathQuery>, QueryRejection>,
) -> Result<Response, ApiError> {
    let path = query(request)?;
    let permit = state.downloads.clone().try_acquire_owned().map_err(|_| {
        ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "DOWNLOAD_LIMIT",
            "Too many active downloads.",
        )
    })?;
    let handle = state.service()?.read(path.clone()).await?;
    let size = handle.len;
    let range = if method == Method::GET && !headers.contains_key(header::IF_RANGE) {
        if let Some(value) = headers.get(header::RANGE) {
            let parsed = value
                .to_str()
                .map_err(|_| ())
                .and_then(|value| range::parse(value, size));
            if headers.get_all(header::RANGE).iter().count() != 1 || parsed.is_err() {
                let mut response = ApiError::new(
                    StatusCode::RANGE_NOT_SATISFIABLE,
                    "INVALID_RANGE",
                    "The requested byte range is invalid or unsupported.",
                )
                .into_response();
                response.headers_mut().insert(
                    header::CONTENT_RANGE,
                    format!("bytes */{size}").parse().unwrap(),
                );
                return Ok(response);
            }
            Some(parsed.unwrap())
        } else {
            None
        }
    } else {
        None
    }; // HEAD ignores Range. Without validators If-Range always falls back to full.
    let (start, length, status) = range
        .as_ref()
        .map(|range| (range.start, range.length, StatusCode::PARTIAL_CONTENT))
        .unwrap_or((0, size, StatusCode::OK));
    let mut file = tokio::fs::File::from_std(handle.into_file());
    file.set_max_buf_size(64 * 1024);
    file.seek(std::io::SeekFrom::Start(start))
        .await
        .map_err(|_| ApiError::from(splitshare_core::StorageError::Io))?;
    let body = if method == Method::HEAD {
        Body::empty()
    } else {
        let shutdown = state.shutdown.clone();
        Body::from_stream(async_stream::stream! {
            let _permit = permit;
            let mut remaining = length;
            while remaining > 0 {
                let mut bytes = vec![0; remaining.min(64 * 1024) as usize];
                let count = tokio::select! { biased; _ = shutdown.cancelled() => break, _ = access.invalidated() => break, count = file.read(&mut bytes) => count };
                let count = match count { Ok(count) => count, Err(error) => { yield Err(error); break; } };
                if count == 0 { yield Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof)); break; }
                remaining -= count as u64;
                bytes.truncate(count);
                yield Ok::<_, std::io::Error>(bytes);
            }
        })
    };
    let filename = path.components().last().unwrap_or("download");
    let encoded =
        percent_encoding::utf8_percent_encode(filename, percent_encoding::NON_ALPHANUMERIC);
    let mut response = (status, body).into_response();
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_LENGTH, length.to_string().parse().unwrap());
    headers.insert(
        header::CONTENT_TYPE,
        "application/octet-stream".parse().unwrap(),
    );
    headers.insert(header::ACCEPT_RANGES, "bytes".parse().unwrap());
    headers.insert(
        header::CONTENT_DISPOSITION,
        format!("attachment; filename=\"download\"; filename*=UTF-8''{encoded}")
            .parse()
            .unwrap(),
    );
    if range.is_some() {
        headers.insert(
            header::CONTENT_RANGE,
            format!("bytes {start}-{}/{size}", start + length - 1)
                .parse()
                .unwrap(),
        );
    }
    Ok(response)
}

pub async fn events(
    State(state): State<ServerState>,
    axum::Extension(access): axum::Extension<Access>,
) -> Result<Response, ApiError> {
    let mut receiver = state.service()?.subscribe();
    let mut transfers = state
        .transfers
        .as_ref()
        .expect("configured share has manager")
        .subscribe();
    let permit = state.streams.clone().try_acquire_owned().map_err(|_| {
        ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "EVENT_LIMIT",
            "Too many event connections.",
        )
    })?;
    let shutdown = state.shutdown.clone();
    let stream = async_stream::stream! {
        let _permit = permit;
        yield Ok::<_, Infallible>(Event::default().event("filesystem.resync").data("{}"));
        yield Ok(Event::default().event("transfer.resync").data("{}"));
        loop {
            let event = tokio::select! {
                biased;
                _ = shutdown.cancelled() => break,
                _ = access.invalidated() => { yield Ok(Event::default().event("session.permissions_changed").data("{}")); break; },
                event = transfers.recv() => {
                    match event {
                        Ok(event) => if state.sessions.permissions(access.local).upload { yield Ok(Event::default().event(event.name).json_data(event.transfer).expect("serializable transfer")); },
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => yield Ok(Event::default().event("transfer.resync").data("{}")),
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                    continue;
                },
                event = receiver.recv() => event,
            };
            match event {
                Ok(event) => if state.sessions.permissions(access.local).browse { yield Ok(Event::default().event("filesystem.changed").json_data(event).expect("serializable virtual path event")); },
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => yield Ok(Event::default().event("filesystem.resync").data("{}")),
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    };
    Ok(Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
        .into_response())
}

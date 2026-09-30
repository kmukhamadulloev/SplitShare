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
use splitshare_core::{HostSettings, VirtualPath};
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
    pub settings: HostSettings,
    pub open_lan: bool,
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
        settings: HostSettings,
        open_lan: bool,
        local_addresses: LocalAddresses,
        authorities: Vec<String>,
        shutdown: CancellationToken,
    ) -> Self {
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
            settings,
            open_lan,
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

pub async fn guard(State(state): State<ServerState>, request: Request, next: Next) -> Response {
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
        if !state.open_lan && !state.local_addresses.is_local(peer.0.ip()) {
            return ApiError::new(
                StatusCode::FORBIDDEN,
                "LOCAL_ONLY",
                "Remote access is disabled by the host.",
            )
            .into_response();
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
    let mut response = next.run(request).await;
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
pub struct Permissions {
    browse: bool,
    download: bool,
    upload: bool,
    create_directory: bool,
    rename: bool,
    delete: bool,
}
#[derive(Serialize)]
pub struct Status {
    version: &'static str,
    sharing: bool,
    root_label: &'static str,
    local_client: bool,
    share_mode: &'static str,
    upload_concurrency: usize,
    permissions: Permissions,
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
        share_mode: if state.open_lan {
            "open_lan"
        } else {
            "local_only"
        },
        upload_concurrency: state.settings.effective_upload_limit().unwrap_or(1),
        permissions: Permissions {
            browse: sharing,
            download: sharing,
            upload: sharing,
            create_directory: sharing,
            rename: sharing,
            delete: sharing,
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
                let count = tokio::select! { _ = shutdown.cancelled() => break, count = file.read(&mut bytes) => count };
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

pub async fn events(State(state): State<ServerState>) -> Result<Response, ApiError> {
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
                _ = shutdown.cancelled() => break,
                event = transfers.recv() => {
                    match event {
                        Ok(event) => yield Ok(Event::default().event(event.name).json_data(event.transfer).expect("serializable transfer")),
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => yield Ok(Event::default().event("transfer.resync").data("{}")),
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                    continue;
                },
                event = receiver.recv() => event,
            };
            match event {
                Ok(event) => yield Ok(Event::default().event("filesystem.changed").json_data(event).expect("serializable virtual path event")),
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => yield Ok(Event::default().event("filesystem.resync").data("{}")),
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    };
    Ok(Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
        .into_response())
}

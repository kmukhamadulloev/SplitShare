use axum::{
    body::{Body, to_bytes},
    extract::ConnectInfo,
    http::{Request, StatusCode},
    response::Response,
};
use splitshare_application::FileService;
use splitshare_core::{HostSettings, PermissionSet, ShareMode};
use splitshare_network::LocalAddresses;
use splitshare_server::{ServerState, api_router};
use std::net::SocketAddr;
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;

fn state() -> (tempfile::TempDir, ServerState) {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("hello.txt"), b"hello").unwrap();
    let mut state = ServerState::new(
        Some(FileService::new(
            splitshare_storage::Storage::open(root.path()).unwrap(),
        )),
        HostSettings::default(),
        false,
        LocalAddresses::default(),
        vec!["127.0.0.1:8080".into()],
        CancellationToken::new(),
    );
    state.candidates = splitshare_network::candidates(
        &[
            ("wifi".into(), "192.168.1.20".parse().unwrap()),
            ("wg0".into(), "10.0.0.2".parse().unwrap()),
        ],
        "0.0.0.0:8080".parse().unwrap(),
    );
    (root, state)
}
async fn send(
    state: &ServerState,
    local: bool,
    method: &str,
    path: &str,
    cookie: &str,
    body: &str,
) -> Response {
    api_router(state.clone())
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("host", "127.0.0.1:8080")
                .header("origin", "http://127.0.0.1:8080")
                .header("x-forwarded-for", "127.0.0.1")
                .header("x-local-client", "true")
                .header("cookie", cookie)
                .header("x-splitshare-request", "1")
                .header("content-type", "application/json")
                .extension(ConnectInfo(
                    if local {
                        "127.0.0.1:2000"
                    } else {
                        "192.0.2.30:2000"
                    }
                    .parse::<SocketAddr>()
                    .unwrap(),
                ))
                .body(Body::from(body.to_owned()))
                .unwrap(),
        )
        .await
        .unwrap()
}
async fn json(response: Response) -> serde_json::Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap()
}
async fn join(state: &ServerState) -> String {
    let token = state.sessions.share_token();
    let response = send(state, false, "GET", &format!("/j/{token}"), "", "").await;
    assert_eq!(response.status(), 303);
    assert_eq!(response.headers()["location"], "/");
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(response.headers()["referrer-policy"], "no-referrer");
    let cookie = response.headers()["set-cookie"].to_str().unwrap();
    assert!(
        cookie.contains("HttpOnly")
            && cookie.contains("SameSite=Strict")
            && cookie.contains("Path=/")
            && !cookie.contains("Secure")
    );
    cookie.split(';').next().unwrap().into()
}
#[tokio::test]
async fn joins_rotate_leave_and_never_disclose_tokens() {
    let (_root, state) = state();
    assert_eq!(
        send(&state, false, "GET", "/api/v1/files", "", "")
            .await
            .status(),
        401
    );
    assert_eq!(
        send(&state, false, "GET", "/j/invalid", "", "")
            .await
            .status(),
        401
    );
    let token = state.sessions.share_token();
    let cookie = join(&state).await;
    let status = json(send(&state, false, "GET", "/api/v1/status", &cookie, "").await).await;
    assert_eq!(status["local_client"], false);
    assert_eq!(status["share_mode"], "token_link");
    assert!(!status.to_string().contains(&token));
    assert_eq!(
        send(&state, false, "GET", "/api/v1/files", &cookie, "")
            .await
            .status(),
        200
    );
    assert_eq!(
        send(
            &state,
            true,
            "POST",
            "/api/v1/host/share-token/rotate",
            "",
            ""
        )
        .await
        .status(),
        204
    );
    assert_eq!(
        send(&state, false, "GET", &format!("/j/{token}"), "", "")
            .await
            .status(),
        401
    );
    assert_eq!(
        send(&state, false, "GET", "/api/v1/files", &cookie, "")
            .await
            .status(),
        401
    );
    let cookie = join(&state).await;
    let response = send(&state, false, "POST", "/api/v1/session/leave", &cookie, "").await;
    assert_eq!(response.status(), 204);
    assert!(
        response.headers()["set-cookie"]
            .to_str()
            .unwrap()
            .contains("Max-Age=0")
    );
    assert_eq!(
        send(&state, false, "GET", "/api/v1/status", &cookie, "")
            .await
            .status(),
        401
    );
}
#[tokio::test]
async fn host_only_routes_and_every_capability_reject_manual_requests() {
    let (root, state) = state();
    let cookie = join(&state).await;
    for (method, path) in [
        ("GET", "/api/v1/host/settings"),
        ("PUT", "/api/v1/host/settings"),
        ("GET", "/api/v1/host/network"),
        ("POST", "/api/v1/host/share-token/rotate"),
    ] {
        let response = send(&state, false, method, path, &cookie, "{}").await;
        assert_eq!(response.status(), 403, "{path}");
        assert_eq!(
            json(response).await["error"]["code"],
            "HOST_CLIENT_REQUIRED"
        );
    }
    let settings = HostSettings {
        permissions: PermissionSet::all(false),
        ..HostSettings::default()
    };
    let response = send(
        &state,
        true,
        "PUT",
        "/api/v1/host/settings",
        "",
        &serde_json::to_string(&settings).unwrap(),
    )
    .await;
    assert_eq!(response.status(), 200);
    for (method, path, body) in [
        ("GET", "/api/v1/files", ""),
        ("HEAD", "/api/v1/files", ""),
        ("GET", "/api/v1/files/download?path=/hello.txt", ""),
        ("HEAD", "/api/v1/files/download?path=/hello.txt", ""),
        ("POST", "/api/v1/uploads?path=/new.txt", "bytes"),
        (
            "POST",
            "/api/v1/directories",
            r#"{"parent":"/","name":"new"}"#,
        ),
        (
            "POST",
            "/api/v1/files/rename",
            r#"{"from":"/hello.txt","to":"/renamed"}"#,
        ),
        ("DELETE", "/api/v1/files", r#"{"path":"/hello.txt"}"#),
        ("GET", "/api/v1/transfers", ""),
        ("HEAD", "/api/v1/transfers", ""),
    ] {
        assert_eq!(
            send(&state, false, method, path, &cookie, body)
                .await
                .status(),
            403,
            "{method} {path}"
        );
    }
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    assert_eq!(
        send(&state, true, "GET", "/api/v1/files", "", "")
            .await
            .status(),
        200
    );
    let network = json(send(&state, true, "GET", "/api/v1/host/network", "", "").await).await;
    assert_eq!(network["candidates"].as_array().unwrap().len(), 2);
    assert!(
        network["candidates"][0]["url"]
            .as_str()
            .unwrap()
            .starts_with("http://192.168.1.20:8080/j/")
    );
    let open = HostSettings {
        share_mode: ShareMode::OpenLan,
        ..settings
    };
    assert_eq!(
        send(
            &state,
            true,
            "PUT",
            "/api/v1/host/settings",
            "",
            &serde_json::to_string(&open).unwrap()
        )
        .await
        .status(),
        200
    );
    assert_eq!(
        send(&state, false, "GET", "/api/v1/status", "", "")
            .await
            .status(),
        200
    );
    assert_eq!(
        send(&state, false, "GET", "/api/v1/files", "", "")
            .await
            .status(),
        403
    );
    assert_eq!(
        send(&state, false, "PUT", "/api/v1/host/settings", "", "{}")
            .await
            .status(),
        403
    );
}
#[tokio::test]
async fn malformed_settings_and_csrf_cannot_change_policy() {
    let (_root, state) = state();
    let old = state.sessions.settings();
    for body in [
        "{}",
        r#"{"local_client":true}"#,
        r#"{"share_mode":"open_lan","parallel_uploads_enabled":false,"max_parallel_uploads":3}"#,
    ] {
        assert!(
            send(&state, true, "PUT", "/api/v1/host/settings", "", body)
                .await
                .status()
                .is_client_error()
        );
    }
    let mut settings = old.clone();
    settings.max_parallel_uploads = 0;
    assert_eq!(
        send(
            &state,
            true,
            "PUT",
            "/api/v1/host/settings",
            "",
            &serde_json::to_string(&settings).unwrap()
        )
        .await
        .status(),
        400
    );
    for (origin, marker) in [
        ("http://evil.example", true),
        ("http://127.0.0.1:8080", false),
    ] {
        let mut request = Request::builder()
            .method("POST")
            .uri("/api/v1/host/share-token/rotate")
            .header("host", "127.0.0.1:8080")
            .header("origin", origin)
            .extension(ConnectInfo("127.0.0.1:1".parse::<SocketAddr>().unwrap()));
        if marker {
            request = request.header("x-splitshare-request", "1");
        }
        assert_eq!(
            api_router(state.clone())
                .oneshot(request.body(Body::empty()).unwrap())
                .await
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN
        );
    }
    assert_eq!(state.sessions.settings(), old);
}
#[tokio::test]
async fn rotation_ends_existing_event_stream() {
    use futures_util::StreamExt;
    let (_root, state) = state();
    let cookie = join(&state).await;
    let response = send(&state, false, "GET", "/api/v1/events", &cookie, "").await;
    assert_eq!(response.status(), 200);
    let mut stream = response.into_body().into_data_stream();
    stream.next().await.unwrap().unwrap();
    state.sessions.rotate().unwrap();
    let rest = tokio::time::timeout(std::time::Duration::from_secs(1), async {
        let mut bytes = Vec::new();
        while let Some(chunk) = stream.next().await {
            bytes.extend(chunk.unwrap());
        }
        String::from_utf8(bytes).unwrap()
    })
    .await
    .unwrap();
    assert!(rest.contains("session.permissions_changed"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rotation_cancels_remote_upload_and_download() {
    use futures_util::StreamExt;
    let (root, state) = state();
    let cookie = join(&state).await;
    let (sender, receiver) = tokio::sync::mpsc::channel::<Result<Vec<u8>, std::io::Error>>(1);
    sender.send(Ok(vec![1; 65536])).await.unwrap();
    let stream = futures_util::stream::unfold(receiver, |mut receiver| async move {
        receiver.recv().await.map(|item| (item, receiver))
    });
    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/uploads?path=/partial")
        .header("host", "127.0.0.1:8080")
        .header("cookie", &cookie)
        .header("x-splitshare-request", "1")
        .header("content-type", "application/octet-stream")
        .header("x-transfer-id", "a".repeat(32))
        .header("x-transfer-key", "b".repeat(32))
        .extension(ConnectInfo(
            "192.0.2.30:2000".parse::<SocketAddr>().unwrap(),
        ))
        .body(Body::from_stream(stream))
        .unwrap();
    let router = api_router(state.clone());
    let task = tokio::spawn(async move { router.oneshot(request).await.unwrap() });
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            if state
                .transfers
                .as_ref()
                .unwrap()
                .snapshot()
                .iter()
                .any(|item| item.transferred_bytes == 65536)
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    let download = send(
        &state,
        false,
        "GET",
        "/api/v1/files/download?path=/hello.txt",
        &cookie,
        "",
    )
    .await;
    state.sessions.rotate().unwrap();
    assert_eq!(task.await.unwrap().status(), 401);
    let mut download = download.into_body().into_data_stream();
    assert!(download.next().await.is_none());
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            if state.transfers.as_ref().unwrap().snapshot()[0]
                .state
                .terminal()
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    drop(sender);
}

#[tokio::test]
async fn each_capability_is_independent() {
    let (_root, state) = state();
    let cookie = join(&state).await;
    let cases = [
        ("browse", "GET", "/api/v1/files"),
        ("download", "GET", "/api/v1/files/download?path=/hello.txt"),
        ("upload", "POST", "/api/v1/uploads?path=/new"),
        ("create_directory", "POST", "/api/v1/directories"),
        ("rename", "POST", "/api/v1/files/rename"),
        ("delete", "DELETE", "/api/v1/files"),
    ];
    for (disabled, _, _) in cases {
        let mut permissions = serde_json::to_value(PermissionSet::all(true)).unwrap();
        permissions[disabled] = false.into();
        let settings = HostSettings {
            permissions: serde_json::from_value(permissions).unwrap(),
            ..HostSettings::default()
        };
        state
            .sessions
            .update(settings, None, state.transfers.as_ref())
            .unwrap();
        for (capability, method, path) in cases {
            let response = send(&state, false, method, path, &cookie, "{}").await;
            if capability == disabled {
                assert_eq!(response.status(), 403, "{disabled}");
            } else {
                assert_ne!(
                    response.status(),
                    403,
                    "{disabled} must not disable {capability}"
                );
            }
        }
    }
}

#[tokio::test]
async fn native_setup_is_host_only_and_never_accepts_browser_paths() {
    use splitshare_application::host_control::{Action, HostControl, Interface, Snapshot};
    let (_root, mut state) = state();
    let (control, mut actions) = HostControl::new(Snapshot {
        bind_ip: "127.0.0.1".into(),
        port: 8080,
        folder_selected: false,
        interfaces: vec![Interface {
            address: "127.0.0.1".into(),
            label: "Local".into(),
        }],
        state: "ready",
        message: None,
        local_url: "http://127.0.0.1:8080/".into(),
    });
    state.host_control = Some(control.clone());
    let cookie = join(&state).await;
    for (method, path, body) in [
        ("GET", "/api/v1/host/setup", ""),
        (
            "PUT",
            "/api/v1/host/setup",
            r#"{"bind_ip":"127.0.0.1","port":8081}"#,
        ),
        ("POST", "/api/v1/host/folder", ""),
    ] {
        assert_eq!(
            send(&state, false, method, path, &cookie, body)
                .await
                .status(),
            403
        );
    }
    assert!(actions.try_recv().is_err());
    for body in [
        r#"{"bind_ip":"203.0.113.1","port":8081}"#,
        r#"{"bind_ip":"127.0.0.1","port":0}"#,
        r#"{"bind_ip":"127.0.0.1","port":8081,"root":"/private/path"}"#,
    ] {
        assert!(
            send(&state, true, "PUT", "/api/v1/host/setup", "", body)
                .await
                .status()
                .is_client_error()
        );
    }
    assert!(actions.try_recv().is_err());
    assert_eq!(
        send(
            &state,
            true,
            "POST",
            "/api/v1/host/folder",
            "",
            r#"{"root":"/private/path"}"#
        )
        .await
        .status(),
        400
    );
    assert!(actions.try_recv().is_err());
    let response = send(&state, true, "POST", "/api/v1/host/folder", "", "").await;
    assert_eq!(response.status(), 202);
    let payload = json(response).await;
    assert_eq!(payload["state"], "selecting");
    assert!(payload.get("root").is_none());
    assert!(matches!(actions.try_recv().unwrap(), Action::ChooseFolder));
    assert_eq!(
        send(&state, true, "POST", "/api/v1/host/folder", "", "")
            .await
            .status(),
        409
    );
}

#[tokio::test]
async fn diagnostics_are_host_only_bounded_and_do_not_revoke_sessions() {
    use std::{collections::BTreeMap, sync::Arc};
    let (_root, state) = state();
    let cookie = join(&state).await;
    for (method, path, body) in [
        ("GET", "/api/v1/host/logs", ""),
        ("HEAD", "/api/v1/host/logs", ""),
        ("DELETE", "/api/v1/host/logs", ""),
        ("GET", "/api/v1/host/logs/config", ""),
        ("PUT", "/api/v1/host/logs/config", r#"{"level":"debug"}"#),
        ("GET", "/api/v1/host/logs/events", ""),
    ] {
        assert_eq!(
            send(&state, false, method, path, &cookie, body)
                .await
                .status(),
            403
        );
    }
    state.diagnostics.install_control(Arc::new(|_| Ok(())));
    for _ in 0..510 {
        state
            .diagnostics
            .record("INFO", "test", "safe test entry", BTreeMap::new());
    }
    let response = send(&state, true, "GET", "/api/v1/host/logs", "", "").await;
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(json(response).await.as_array().unwrap().len(), 500);
    let configured = json(
        send(
            &state,
            true,
            "PUT",
            "/api/v1/host/logs/config",
            "",
            r#"{"level":"debug"}"#,
        )
        .await,
    )
    .await;
    assert_eq!(configured["level"], "debug");
    assert_eq!(
        send(&state, false, "GET", "/api/v1/files", &cookie, "")
            .await
            .status(),
        200
    );
    let invalid = send(
        &state,
        true,
        "PUT",
        "/api/v1/host/logs/config",
        "",
        r#"{"level":"trace"}"#,
    )
    .await;
    assert_eq!(invalid.status(), 400);
    assert_eq!(json(invalid).await["error"]["code"], "INVALID_REQUEST");
    let mut streams = vec![];
    for _ in 0..32 {
        let response = send(&state, true, "GET", "/api/v1/host/logs/events", "", "").await;
        assert_eq!(response.status(), 200);
        streams.push(response);
    }
    assert_eq!(
        send(&state, true, "GET", "/api/v1/host/logs/events", "", "")
            .await
            .status(),
        429
    );
    drop(streams);
    assert_eq!(
        send(&state, true, "DELETE", "/api/v1/host/logs", "", "")
            .await
            .status(),
        204
    );
    assert!(state.diagnostics.snapshot().is_empty());
    let response = api_router(state.clone())
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/host/logs")
                .header("host", "127.0.0.1:8080")
                .extension(ConnectInfo("127.0.0.1:2000".parse::<SocketAddr>().unwrap()))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 403);
}

#[tokio::test]
async fn preview_requires_download_permission_and_session() {
    let (_root, state) = state();
    for method in ["GET", "HEAD"] {
        assert_eq!(
            send(
                &state,
                false,
                method,
                "/api/v1/files/preview?path=/hello.txt",
                "",
                ""
            )
            .await
            .status(),
            401
        );
    }
    let cookie = join(&state).await;
    assert_eq!(
        send(
            &state,
            false,
            "GET",
            "/api/v1/files/preview?path=/hello.txt",
            &cookie,
            ""
        )
        .await
        .status(),
        200
    );
    let mut settings = state.sessions.settings();
    settings.permissions.download = false;
    state
        .sessions
        .update(settings, None, state.transfers.as_ref())
        .unwrap();
    let cookie = join(&state).await;
    for method in ["GET", "HEAD"] {
        assert_eq!(
            send(
                &state,
                false,
                method,
                "/api/v1/files/preview?path=/hello.txt",
                &cookie,
                ""
            )
            .await
            .status(),
            403
        );
    }
}

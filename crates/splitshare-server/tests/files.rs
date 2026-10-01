use axum::{body::Body, extract::ConnectInfo, http::Request};
use futures_util::StreamExt;
use reqwest::Client;
use serde_json::{Value, json};
use splitshare_application::FileService;
use splitshare_core::HostSettings;
use splitshare_network::LocalAddresses;
use splitshare_server::ServerState;
use splitshare_storage::Storage;
use std::{net::SocketAddr, time::Duration};
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;

struct Server {
    root: tempfile::TempDir,
    url: String,
    shutdown: CancellationToken,
    task: tokio::task::JoinHandle<std::io::Result<()>>,
    client: Client,
}
impl Server {
    async fn start() -> Self {
        Self::with_settings(HostSettings::default()).await
    }
    async fn with_settings(settings: HostSettings) -> Self {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("hello.txt"), b"0123456789").unwrap();
        std::fs::write(root.path().join("empty"), b"").unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let shutdown = CancellationToken::new();
        let state = ServerState::new(
            Some(FileService::new(Storage::open(root.path()).unwrap())),
            settings,
            true,
            LocalAddresses::default(),
            vec![address.to_string()],
            shutdown.clone(),
        );
        let task = tokio::spawn(splitshare_server::serve_api(listener, state));
        Self {
            root,
            url: format!("http://{address}"),
            shutdown,
            task,
            client: Client::builder().no_proxy().build().unwrap(),
        }
    }
    fn get(&self, path: &str) -> reqwest::RequestBuilder {
        self.client.get(format!("{}{path}", self.url))
    }
    fn mutation(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Value,
    ) -> reqwest::RequestBuilder {
        self.client
            .request(method, format!("{}{path}", self.url))
            .header("x-splitshare-request", "1")
            .header("origin", &self.url)
            .json(&body)
    }
    async fn stop(self) {
        self.shutdown.cancel();
        tokio::time::timeout(Duration::from_secs(3), self.task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }
}

#[tokio::test]
async fn real_http_file_lifecycle_and_safe_errors() {
    let server = Server::start().await;
    let status: Value = server
        .get("/api/v1/status")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(status["sharing"], true);
    assert_eq!(status["permissions"]["upload"], true);
    let list = server
        .get("/api/v1/files?path=/")
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(list.contains("hello.txt"));
    assert!(!list.contains(server.root.path().to_str().unwrap()));
    let created = server
        .mutation(
            reqwest::Method::POST,
            "/api/v1/directories",
            json!({"parent":"/","name":"folder"}),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(created.status(), 201);
    assert!(server.root.path().join("folder").is_dir());
    assert_eq!(
        server
            .mutation(
                reqwest::Method::POST,
                "/api/v1/files/rename",
                json!({"from":"/hello.txt","to":"/folder/renamed.txt"})
            )
            .send()
            .await
            .unwrap()
            .status(),
        204
    );
    assert_eq!(
        server
            .get("/api/v1/files/download?path=/folder/renamed.txt")
            .send()
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap(),
        "0123456789"
    );
    assert_eq!(
        server
            .mutation(
                reqwest::Method::DELETE,
                "/api/v1/files",
                json!({"path":"/folder"})
            )
            .send()
            .await
            .unwrap()
            .status(),
        409
    );
    for path in ["/folder/renamed.txt", "/folder"] {
        assert_eq!(
            server
                .mutation(
                    reqwest::Method::DELETE,
                    "/api/v1/files",
                    json!({"path":path})
                )
                .send()
                .await
                .unwrap()
                .status(),
            204
        );
    }
    assert_eq!(
        server
            .mutation(
                reqwest::Method::DELETE,
                "/api/v1/files",
                json!({"path":"/"})
            )
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    let response = server
        .get("/api/v1/files?path=/missing")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 404);
    assert_eq!(
        response.json::<Value>().await.unwrap()["error"]["code"],
        "PATH_NOT_FOUND"
    );
    server.stop().await;
}

#[tokio::test]
async fn byte_ranges_head_and_conditional_fallback() {
    let server = Server::start().await;
    for (range, content, header) in [
        ("bytes=2-4", "234", "bytes 2-4/10"),
        ("bytes=7-", "789", "bytes 7-9/10"),
        ("bytes=-3", "789", "bytes 7-9/10"),
        ("bytes=0-999", "0123456789", "bytes 0-9/10"),
    ] {
        let response = server
            .get("/api/v1/files/download?path=/hello.txt")
            .header("range", range)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 206);
        assert_eq!(response.headers()["content-range"], header);
        assert_eq!(
            response.headers()["content-length"],
            content.len().to_string()
        );
        assert_eq!(response.text().await.unwrap(), content);
    }
    for range in [
        "bytes=-0",
        "bytes=10-",
        "bytes=4-2",
        "bytes=0-1,3-4",
        "bad",
        "bytes=0-18446744073709551616",
    ] {
        let response = server
            .get("/api/v1/files/download?path=/hello.txt")
            .header("range", range)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 416, "{range}");
        assert_eq!(response.headers()["content-range"], "bytes */10");
    }
    let head = server
        .client
        .head(format!(
            "{}/api/v1/files/download?path=/hello.txt",
            server.url
        ))
        .header("range", "broken")
        .send()
        .await
        .unwrap();
    assert_eq!(head.status(), 200);
    assert_eq!(head.headers()["content-length"], "10");
    assert!(head.bytes().await.unwrap().is_empty());
    let response = server
        .get("/api/v1/files/download?path=/hello.txt")
        .header("range", "bytes=1-2")
        .header("if-range", "\"old\"")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.bytes().await.unwrap().len(), 10);
    assert_eq!(
        server
            .get("/api/v1/files/download?path=/empty")
            .send()
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap()
            .len(),
        0
    );
    assert_eq!(
        server
            .get("/api/v1/files/download?path=/empty")
            .header("range", "bytes=0-")
            .send()
            .await
            .unwrap()
            .status(),
        416
    );
    server.stop().await;
}

#[tokio::test]
async fn traversal_origin_host_body_limits_and_forwarding_headers() {
    let server = Server::start().await;
    for path in [
        "/../secret",
        "/%2e%2e/secret",
        "/%252e%252e/secret",
        "/C:/secret",
        "/a\\b",
        "/.splitshare-upload-x.part",
    ] {
        assert_eq!(
            server
                .get("/api/v1/files")
                .query(&[("path", path)])
                .send()
                .await
                .unwrap()
                .status(),
            400,
            "{path}"
        );
    }
    assert_eq!(
        server
            .get("/api/v1/status")
            .header("host", "evil.example")
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    assert_eq!(
        server
            .get("/api/v1/files")
            .header("origin", "https://evil.example")
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    assert_eq!(
        server
            .client
            .post(format!("{}/api/v1/directories", server.url))
            .json(&json!({"parent":"/","name":"bad"}))
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    let huge = "a".repeat(17000);
    assert_eq!(
        server
            .mutation(
                reqwest::Method::POST,
                "/api/v1/directories",
                json!({"parent":"/","name":huge})
            )
            .send()
            .await
            .unwrap()
            .status(),
        413
    );
    let body = server
        .get("/api/v1/files?path=/missing")
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(!body.contains(server.root.path().to_str().unwrap()));
    // Synthetic socket peers exercise policy separately from the actual TCP tests.
    let state = ServerState::new(
        None,
        HostSettings::default(),
        false,
        LocalAddresses::default(),
        vec!["127.0.0.1:8080".into()],
        CancellationToken::new(),
    );
    let response = splitshare_server::api_router(state)
        .oneshot(
            Request::builder()
                .uri("/api/v1/status")
                .header("host", "127.0.0.1:8080")
                .header("x-forwarded-for", "127.0.0.1")
                .extension(ConnectInfo(
                    "192.0.2.20:1000".parse::<SocketAddr>().unwrap(),
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 401);
    server.stop().await;
}

#[tokio::test]
async fn sse_emits_real_mutations_and_stops_on_shutdown() {
    let server = Server::start().await;
    let response = server.get("/api/v1/events").send().await.unwrap();
    assert_eq!(response.headers()["content-type"], "text/event-stream");
    let mut stream = response.bytes_stream();
    let initial = stream.next().await.unwrap().unwrap();
    assert!(String::from_utf8_lossy(&initial).contains("filesystem.resync"));
    server
        .mutation(
            reqwest::Method::POST,
            "/api/v1/directories",
            json!({"parent":"/","name":"live"}),
        )
        .send()
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let event = stream.next().await.unwrap().unwrap();
            if String::from_utf8_lossy(&event).contains("filesystem.changed") {
                break;
            }
        }
    })
    .await
    .unwrap();
    server.shutdown.cancel();
    assert!(
        tokio::time::timeout(Duration::from_secs(2), stream.next())
            .await
            .unwrap()
            .is_none()
    );
    server.stop().await;
}

#[cfg(unix)]
#[tokio::test]
async fn http_cannot_read_external_symlinks() {
    let server = Server::start().await;
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("secret"), b"private").unwrap();
    std::os::unix::fs::symlink(outside.path(), server.root.path().join("escape")).unwrap();
    assert_eq!(
        server
            .get("/api/v1/files/download?path=/escape/secret")
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    server.stop().await;
}

#[tokio::test]
async fn unicode_attachment_and_bounded_large_download() {
    use std::io::Write;
    let server = Server::start().await;
    let mut file = std::fs::File::create(server.root.path().join("資料.txt")).unwrap();
    let block = [b'x'; 65536];
    for _ in 0..128 {
        file.write_all(&block).unwrap();
    }
    drop(file);
    let response = server
        .get("/api/v1/files/download")
        .query(&[("path", "/資料.txt")])
        .send()
        .await
        .unwrap();
    assert_eq!(
        response.headers()["content-type"],
        "application/octet-stream"
    );
    assert!(
        response.headers()["content-disposition"]
            .to_str()
            .unwrap()
            .contains("filename*=UTF-8''%")
    );
    let mut stream = response.bytes_stream();
    let mut total = 0;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.unwrap();
        total += chunk.len();
        assert!(chunk.iter().all(|&byte| byte == b'x'));
    }
    assert_eq!(total, 8 * 1024 * 1024);
    server.stop().await;
}

#[tokio::test]
async fn event_connections_have_a_limit() {
    let server = Server::start().await;
    let mut streams = Vec::new();
    for _ in 0..32 {
        let response = server.get("/api/v1/events").send().await.unwrap();
        assert_eq!(response.status(), 200);
        streams.push(response);
    }
    assert_eq!(
        server.get("/api/v1/events").send().await.unwrap().status(),
        429
    );
    drop(streams);
    server.stop().await;
}

#[tokio::test]
async fn open_lan_peer_classification_ignores_browser_claims() {
    let state = ServerState::new(
        None,
        HostSettings::default(),
        true,
        LocalAddresses::default(),
        vec!["127.0.0.1:8080".into()],
        CancellationToken::new(),
    );
    let response = splitshare_server::api_router(state)
        .oneshot(
            Request::builder()
                .uri("/api/v1/status")
                .header("host", "127.0.0.1:8080")
                .header("x-forwarded-for", "127.0.0.1")
                .extension(ConnectInfo(
                    "192.0.2.20:1000".parse::<SocketAddr>().unwrap(),
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let bytes = axum::body::to_bytes(response.into_body(), 4096)
        .await
        .unwrap();
    let status: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(status["local_client"], false);
    assert_eq!(status["sharing"], false);
}

async fn transfer_snapshot(server: &Server) -> Vec<Value> {
    server
        .get("/api/v1/transfers")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_upload_concurrency_cancellation_and_secrets() {
    let server = Server::with_settings(HostSettings {
        parallel_uploads_enabled: true,
        max_parallel_uploads: 2,
        ..Default::default()
    })
    .await;
    let mut senders = Vec::new();
    let mut tasks = Vec::new();
    for id in 1..=3 {
        let (sender, receiver) = tokio::sync::mpsc::channel::<Result<Vec<u8>, std::io::Error>>(1);
        sender.send(Ok(vec![b'u'; 65536])).await.unwrap();
        let stream = futures_util::stream::unfold(receiver, |mut receiver| async move {
            receiver.recv().await.map(|bytes| (bytes, receiver))
        });
        let request = server
            .client
            .post(format!("{}/api/v1/uploads?path=/upload-{id}", server.url))
            .header("x-splitshare-request", "1")
            .header("content-type", "application/octet-stream")
            .header("x-transfer-id", format!("{id:032x}"))
            .header("x-transfer-key", "a".repeat(32))
            .body(reqwest::Body::wrap_stream(stream));
        tasks.push(tokio::spawn(async move { request.send().await }));
        senders.push(sender);
    }
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let items = transfer_snapshot(&server).await;
            if items.len() == 3
                && items
                    .iter()
                    .filter(|item| {
                        item["state"] == "uploading" && item["transferred_bytes"] == 65536
                    })
                    .count()
                    == 2
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let items = transfer_snapshot(&server).await;
    assert_eq!(
        items
            .iter()
            .filter(|item| item["state"] == "queued")
            .count(),
        1
    );
    assert!(
        !serde_json::to_string(&items)
            .unwrap()
            .contains(&"a".repeat(32))
    );
    let id = items
        .iter()
        .find(|item| item["state"] == "uploading")
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let url = format!("{}/api/v1/transfers/{id}", server.url);
    assert_eq!(
        server
            .client
            .delete(&url)
            .header("x-splitshare-request", "1")
            .header("x-transfer-key", "b".repeat(32))
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    assert_eq!(
        server
            .client
            .delete(&url)
            .header("x-splitshare-request", "1")
            .header("x-transfer-key", "a".repeat(32))
            .send()
            .await
            .unwrap()
            .status(),
        202
    );
    drop(senders);
    let mut completed = 0;
    let mut cancelled = 0;
    for task in tasks {
        match task.await.unwrap().unwrap().status().as_u16() {
            201 => completed += 1,
            409 => cancelled += 1,
            status => panic!("Unexpected {status}"),
        }
    }
    assert_eq!((completed, cancelled), (2, 1));
    assert!(
        !std::fs::read_dir(server.root.path())
            .unwrap()
            .any(|entry| entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".splitshare-"))
    );
    server.stop().await;
}

#[tokio::test]
async fn raw_upload_conflicts_retry_and_oversized_file_body() {
    let server = Server::start().await;
    for (id, policy, expected) in [
        (10, "ask", 409),
        (11, "replace", 201),
        (12, "auto_rename", 201),
    ] {
        let response = server
            .client
            .post(format!(
                "{}/api/v1/uploads?path=/hello.txt&policy={policy}",
                server.url
            ))
            .header("x-splitshare-request", "1")
            .header("content-type", "application/octet-stream")
            .header("x-transfer-id", format!("{id:032x}"))
            .header("x-transfer-key", "a".repeat(32))
            .body(vec![b'x'; 128 * 1024])
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
    }
    assert_eq!(
        std::fs::metadata(server.root.path().join("hello.txt"))
            .unwrap()
            .len(),
        128 * 1024
    );
    assert!(server.root.path().join("hello.txt (1)").exists());
    let response = server
        .client
        .post(format!(
            "{}/api/v1/uploads?path=/%252e%252e/secret",
            server.url
        ))
        .header("x-splitshare-request", "1")
        .header("content-type", "application/octet-stream")
        .header("x-transfer-id", format!("{:032x}", 13))
        .header("x-transfer-key", "a".repeat(32))
        .body("bad")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 400);
    server.stop().await;
}

#[tokio::test]
async fn control_body_deadline_rejects_stalled_mutations_without_publication() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let server = Server::start().await;
    let address = server.url.trim_start_matches("http://");
    let mut socket = tokio::net::TcpStream::connect(address).await.unwrap();
    socket.write_all(format!("POST /api/v1/directories HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/json\r\nContent-Length: 100\r\nConnection: close\r\nX-SplitShare-Request: 1\r\n\r\n{{").as_bytes()).await.unwrap();
    let mut bytes = Vec::new();
    let length = tokio::time::timeout(Duration::from_secs(12), socket.read_to_end(&mut bytes))
        .await
        .unwrap()
        .unwrap();
    let reply = String::from_utf8_lossy(&bytes[..length]);
    assert!(reply.starts_with("HTTP/1.1 408"));
    assert!(reply.contains("CONTROL_BODY_TIMEOUT"));
    assert_eq!(std::fs::read_dir(server.root.path()).unwrap().count(), 2);
    server.shutdown.cancel();
    server.task.await.unwrap().unwrap();
}

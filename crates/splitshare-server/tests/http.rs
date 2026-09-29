use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use tower::ServiceExt;

#[tokio::test]
async fn api_and_join_never_return_spa_or_private_details() {
    for path in [
        "/api",
        "/api/v1/files",
        "/api/v1/status",
        "/j",
        "/j/secret",
        "/%2e%2e/config.json",
    ] {
        let response = splitshare_server::router()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let bytes = to_bytes(response.into_body(), 4096).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["error"]["code"], "NOT_FOUND");
        assert!(!String::from_utf8_lossy(&bytes).contains("secret"));
    }
}

#[tokio::test]
async fn embedded_assets_and_methods() {
    for path in ["/", "/browser", "/logo.png"] {
        let response = splitshare_server::router()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["x-content-type-options"], "nosniff");
    }
    for (method, status) in [
        ("HEAD", StatusCode::OK),
        ("POST", StatusCode::METHOD_NOT_ALLOWED),
    ] {
        let response = splitshare_server::router()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri("/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
        assert!(
            to_bytes(response.into_body(), 4096)
                .await
                .unwrap()
                .is_empty()
        );
    }
}

#[tokio::test]
async fn cancellation_stops_listener() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let shutdown = tokio_util::sync::CancellationToken::new();
    let server = tokio::spawn(splitshare_server::serve(listener, shutdown.clone()));
    shutdown.cancel();
    tokio::time::timeout(std::time::Duration::from_secs(2), server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}

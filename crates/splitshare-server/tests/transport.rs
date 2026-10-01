//! Real socket regressions, including clients that never complete an HTTP request.
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use tokio_util::sync::CancellationToken;
async fn server() -> (
    std::net::SocketAddr,
    CancellationToken,
    tokio::task::JoinHandle<std::io::Result<()>>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let shutdown = CancellationToken::new();
    let task = tokio::spawn(splitshare_server::serve(listener, shutdown.clone()));
    (address, shutdown, task)
}
async fn response(socket: &mut TcpStream) -> Vec<u8> {
    let mut bytes = vec![0; 65536];
    let size = tokio::time::timeout(Duration::from_secs(12), socket.read(&mut bytes))
        .await
        .unwrap();
    bytes.truncate(size.unwrap_or(0));
    bytes
}
#[tokio::test]
async fn rejects_excess_headers_and_oversized_request_head() {
    let (address, shutdown, task) = server().await;
    for headers in [
        (0..70)
            .map(|n| format!("x-{n}: value\r\n"))
            .collect::<String>(),
        format!("x-large: {}\r\n", "a".repeat(40 * 1024)),
    ] {
        let mut socket = TcpStream::connect(address).await.unwrap();
        let _ = socket
            .write_all(format!("GET / HTTP/1.1\r\nHost: {address}\r\n{headers}\r\n").as_bytes())
            .await;
        let reply = response(&mut socket).await;
        assert!(
            reply.is_empty() || String::from_utf8_lossy(&reply).starts_with("HTTP/1.1 431"),
            "{}",
            String::from_utf8_lossy(&reply)
        );
    }
    shutdown.cancel();
    task.await.unwrap().unwrap();
}
#[tokio::test]
async fn incomplete_headers_expire_and_shutdown_does_not_wait_for_slow_clients() {
    let (address, shutdown, task) = server().await;
    let mut socket = TcpStream::connect(address).await.unwrap();
    socket.write_all(b"GET / HTTP/1.1\r\nHost:").await.unwrap();
    let reply = response(&mut socket).await;
    assert!(reply.is_empty() || String::from_utf8_lossy(&reply).starts_with("HTTP/1.1 408"));
    let mut other = TcpStream::connect(address).await.unwrap();
    other.write_all(b"GET / HTTP/1.1\r\nHost:").await.unwrap();
    shutdown.cancel();
    tokio::time::timeout(Duration::from_secs(6), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}
#[tokio::test]
async fn connection_limit_is_enforced_before_http_work_is_allocated() {
    let (address, shutdown, task) = server().await;
    let mut held = Vec::new();
    for _ in 0..128 {
        let mut socket = TcpStream::connect(address).await.unwrap();
        socket
            .write_all(format!("HEAD / HTTP/1.1\r\nHost: {address}\r\n\r\n").as_bytes())
            .await
            .unwrap();
        assert!(String::from_utf8_lossy(&response(&mut socket).await).starts_with("HTTP/1.1 200"));
        held.push(socket);
    }
    let mut rejected = TcpStream::connect(address).await.unwrap();
    let _ = rejected
        .write_all(format!("GET / HTTP/1.1\r\nHost: {address}\r\n\r\n").as_bytes())
        .await;
    assert!(response(&mut rejected).await.is_empty());
    drop(held);
    shutdown.cancel();
    task.await.unwrap().unwrap();
}

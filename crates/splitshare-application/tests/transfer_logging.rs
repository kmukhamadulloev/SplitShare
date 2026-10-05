use splitshare_application::{
    FileService,
    transfers::{TransferManager, UploadRequest},
};
use splitshare_core::{ConflictPolicy, HostSettings, VirtualPath};
use splitshare_storage::Storage;
use std::io;
use tokio_util::sync::CancellationToken;
fn request(id: u32, total: Option<u64>) -> UploadRequest {
    UploadRequest {
        id: format!("{id:032x}"),
        key: "a".repeat(32),
        path: VirtualPath::try_from(format!("/file-{id}")).unwrap(),
        total,
        policy: ConflictPolicy::Reject,
    }
}
fn setup(parallel: bool, limit: u8) -> (tempfile::TempDir, TransferManager, CancellationToken) {
    let root = tempfile::tempdir().unwrap();
    let settings = HostSettings {
        parallel_uploads_enabled: parallel,
        max_parallel_uploads: limit,
        ..Default::default()
    };
    let shutdown = CancellationToken::new();
    let manager = TransferManager::new(
        FileService::new(Storage::open(root.path()).unwrap()),
        &settings,
        shutdown.clone(),
    )
    .unwrap();
    (root, manager, shutdown)
}

#[derive(Clone, Default)]
struct LogCapture(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);
impl io::Write for LogCapture {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for LogCapture {
    type Writer = Self;
    fn make_writer(&'a self) -> Self {
        self.clone()
    }
}

#[tokio::test]
async fn upload_logs_correlate_failures_without_leaking_paths_keys_or_error_text() {
    let output = LogCapture::default();
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(output.clone())
        .finish();
    let (root, manager, shutdown) = setup(false, 1);
    tracing::subscriber::set_global_default(subscriber).unwrap();
    {
        manager
            .upload(
                request(101, Some(3)),
                futures_util::stream::iter([Ok::<_, io::Error>(b"abc".to_vec())]),
            )
            .await
            .unwrap_or_else(|error| {
                let log = output.0.lock().unwrap();
                panic!(
                    "Initial upload failed: {error:?}\nCaptured diagnostics:\n{}",
                    String::from_utf8_lossy(&log)
                );
            });
        let error = io::Error::new(
            io::ErrorKind::ConnectionReset,
            "SECRET /home/private/image.jpg token=secret",
        );
        assert!(
            manager
                .upload(
                    request(102, Some(6)),
                    futures_util::stream::iter([Ok(b"abc".to_vec()), Err(error)])
                )
                .await
                .is_err()
        );
        let mut conflict = request(103, Some(3));
        conflict.path = request(101, Some(3)).path;
        assert!(
            manager
                .upload(
                    conflict,
                    futures_util::stream::iter([Ok::<_, io::Error>(b"abc".to_vec())])
                )
                .await
                .is_err()
        );
        shutdown.cancel();
        assert!(
            manager
                .upload(
                    request(104, None),
                    futures_util::stream::pending::<Result<Vec<u8>, io::Error>>()
                )
                .await
                .is_err()
        );
    }
    let log = String::from_utf8(output.0.lock().unwrap().clone()).unwrap();
    for expected in [
        "Upload accepted",
        "Upload completed",
        "Upload failed before publication",
        "Upload cancelled",
        "bytes_written=3",
        "expected_bytes=Some(6)",
        "stage=\"receive\"",
        "stage=\"publish\"",
        "ConnectionReset",
        "Storage(Conflict)",
        "queue_wait_ms=",
        "elapsed_ms=",
        "concurrency_limit=1",
    ] {
        assert!(log.contains(expected), "Missing {expected} in {log}");
    }
    for id in [101, 102, 103, 104] {
        assert!(log.contains(&format!("{id:032x}")));
    }
    for secret in [
        "SECRET",
        "token=secret",
        "/home/private",
        "/file-101",
        &"a".repeat(32),
        root.path().to_str().unwrap(),
    ] {
        assert!(!log.contains(secret), "Logs exposed sensitive data");
    }
    assert!(
        !log.lines()
            .filter(|line| line.contains("Upload cancelled"))
            .any(|line| line.contains("WARN"))
    );
}

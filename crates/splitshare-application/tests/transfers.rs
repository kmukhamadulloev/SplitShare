use splitshare_application::{
    FileService,
    transfers::{TransferError, TransferManager, UploadRequest},
};
use splitshare_core::{ConflictPolicy, HostSettings, TransferState, VirtualPath};
use splitshare_storage::Storage;
use std::{io, time::Duration};
use tokio::sync::mpsc;
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
fn stream(
    receiver: mpsc::Receiver<Result<Vec<u8>, io::Error>>,
) -> impl futures_util::Stream<Item = Result<Vec<u8>, io::Error>> + Send {
    futures_util::stream::unfold(receiver, |mut receiver| async move {
        receiver.recv().await.map(|item| (item, receiver))
    })
}
async fn until(
    manager: &TransferManager,
    predicate: impl Fn(&[splitshare_core::Transfer]) -> bool,
) {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if predicate(&manager.snapshot()) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn semaphore_enforces_two_active_and_third_queued_even_for_modified_clients() {
    let (root, manager, _) = setup(true, 2);
    let mut senders = Vec::new();
    let mut tasks = Vec::new();
    for id in 1..=3 {
        let (sender, receiver) = mpsc::channel(1);
        sender.send(Ok(vec![id as u8])).await.unwrap();
        let manager = manager.clone();
        tasks.push(tokio::spawn(async move {
            manager.upload(request(id, Some(1)), stream(receiver)).await
        }));
        senders.push(sender);
    }
    until(&manager, |items| {
        items.len() == 3
            && items
                .iter()
                .filter(|item| {
                    item.state == TransferState::Uploading && item.transferred_bytes == 1
                })
                .count()
                == 2
    })
    .await;
    let snapshot = manager.snapshot();
    assert_eq!(
        snapshot
            .iter()
            .filter(|item| item.state == TransferState::Queued)
            .count(),
        1
    );
    assert_eq!(
        snapshot
            .iter()
            .map(|item| item.transferred_bytes)
            .sum::<u64>(),
        2
    );
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 2);
    drop(senders);
    for task in tasks {
        assert_eq!(task.await.unwrap().unwrap().state, TransferState::Completed);
    }
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 3);
}
#[tokio::test]
async fn disabled_parallel_forces_one_and_cancellation_cleans_before_releasing_slot() {
    let (root, manager, _) = setup(false, 8);
    assert_eq!(manager.limit(), 1);
    let (sender, receiver) = mpsc::channel(1);
    sender.send(Ok(vec![1; 65536])).await.unwrap();
    let first = manager.clone();
    let task = tokio::spawn(async move { first.upload(request(1, None), stream(receiver)).await });
    until(&manager, |items| {
        items.iter().any(|item| item.transferred_bytes == 65536)
    })
    .await;
    let (queued_sender, receiver) = mpsc::channel(1);
    let second = manager.clone();
    let second_task =
        tokio::spawn(async move { second.upload(request(2, None), stream(receiver)).await });
    until(&manager, |items| items.len() == 2).await;
    assert_eq!(
        manager
            .snapshot()
            .iter()
            .filter(|item| item.state == TransferState::Queued)
            .count(),
        1
    );
    assert_eq!(
        manager.cancel(&format!("{:032x}", 1), &"b".repeat(32)),
        Err(TransferError::Forbidden)
    );
    manager
        .cancel(&format!("{:032x}", 1), &"a".repeat(32))
        .unwrap();
    assert!(matches!(task.await.unwrap(), Err(TransferError::Cancelled)));
    manager
        .cancel(&format!("{:032x}", 2), &"a".repeat(32))
        .unwrap();
    assert!(matches!(
        second_task.await.unwrap(),
        Err(TransferError::Cancelled)
    ));
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    drop((sender, queued_sender));
}
#[tokio::test]
async fn disconnect_shutdown_and_length_failures_cannot_publish() {
    let (root, manager, shutdown) = setup(true, 2);
    let (sender, receiver) = mpsc::channel(1);
    sender.send(Ok(vec![1; 100])).await.unwrap();
    let first = manager.clone();
    let task = tokio::spawn(async move { first.upload(request(1, None), stream(receiver)).await });
    until(&manager, |items| {
        items.iter().any(|item| item.transferred_bytes == 100)
    })
    .await;
    task.abort();
    let _ = task.await;
    until(&manager, |items| items[0].state == TransferState::Cancelled).await;
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    drop(sender);
    let data = futures_util::stream::iter([Ok::<_, io::Error>(vec![1; 10])]);
    assert!(manager.upload(request(2, Some(100)), data).await.is_err());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    let (sender, receiver) = mpsc::channel(1);
    sender.send(Ok(vec![1; 100])).await.unwrap();
    let last = manager.clone();
    let task = tokio::spawn(async move { last.upload(request(3, None), stream(receiver)).await });
    until(&manager, |items| {
        items
            .iter()
            .any(|item| item.id == format!("{:032x}", 3) && item.transferred_bytes == 100)
    })
    .await;
    shutdown.cancel();
    assert!(task.await.unwrap().is_err());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}
#[tokio::test]
async fn conflicts_fail_without_overwrite_and_retry_starts_over() {
    let (root, manager, _) = setup(true, 2);
    std::fs::write(root.path().join("file-1"), b"original").unwrap();
    let bytes = || futures_util::stream::iter([Ok::<_, io::Error>(b"new".to_vec())]);
    assert_eq!(
        manager
            .upload(request(1, Some(3)), bytes())
            .await
            .unwrap_err(),
        TransferError::Storage(splitshare_core::StorageError::Conflict)
    );
    assert_eq!(
        std::fs::read(root.path().join("file-1")).unwrap(),
        b"original"
    );
    let mut retry = request(2, Some(3));
    retry.path = VirtualPath::try_from("/file-1").unwrap();
    retry.policy = ConflictPolicy::Replace;
    let transfer = manager.upload(retry, bytes()).await.unwrap();
    assert_eq!(transfer.state, TransferState::Completed);
    assert_eq!(transfer.transferred_bytes, 3);
    assert_eq!(std::fs::read(root.path().join("file-1")).unwrap(), b"new");
}

#[tokio::test]
async fn queued_disconnect_becomes_terminal_and_history_is_bounded() {
    let (root, manager, _) = setup(false, 1);
    let (sender, receiver) = mpsc::channel(1);
    sender.send(Ok(vec![1])).await.unwrap();
    let first = manager.clone();
    let active =
        tokio::spawn(async move { first.upload(request(1, None), stream(receiver)).await });
    until(&manager, |items| {
        items
            .iter()
            .any(|item| item.state == TransferState::Uploading)
    })
    .await;
    let (_sender, receiver) = mpsc::channel(1);
    let second = manager.clone();
    let queued =
        tokio::spawn(async move { second.upload(request(2, None), stream(receiver)).await });
    until(&manager, |items| items.len() == 2).await;
    queued.abort();
    let _ = queued.await;
    until(&manager, |items| {
        items
            .iter()
            .any(|item| item.id == format!("{:032x}", 2) && item.state == TransferState::Cancelled)
    })
    .await;
    manager
        .cancel(&format!("{:032x}", 1), &"a".repeat(32))
        .unwrap();
    assert!(active.await.unwrap().is_err());
    drop(sender);
    for id in 3..=132 {
        manager
            .upload(
                request(id, Some(0)),
                futures_util::stream::empty::<Result<Vec<u8>, io::Error>>(),
            )
            .await
            .unwrap();
    }
    assert_eq!(manager.snapshot().len(), 128);
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 130);
}

//! Bounded upload lifecycle. The worker owns its semaphore permit until cleanup.
use crate::FileService;
use futures_util::{Stream, StreamExt};
use splitshare_core::VirtualPath;
use splitshare_core::{ConflictPolicy, HostSettings, StorageError, Transfer, TransferState};
use std::{
    collections::BTreeMap,
    io,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::{Semaphore, broadcast, mpsc};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransferError {
    Invalid,
    Duplicate,
    Capacity,
    Missing,
    Forbidden,
    TooLate,
    Cancelled,
    Storage(StorageError),
}
impl From<StorageError> for TransferError {
    fn from(error: StorageError) -> Self {
        Self::Storage(error)
    }
}
#[derive(Clone)]
pub struct TransferEvent {
    pub name: &'static str,
    pub transfer: Transfer,
}
struct Record {
    transfer: Transfer,
    key: String,
    cancel: CancellationToken,
}
struct Inner {
    records: Mutex<BTreeMap<String, Record>>,
    slots: Arc<Semaphore>,
    events: broadcast::Sender<TransferEvent>,
    shutdown: CancellationToken,
    files: FileService,
    limit: std::sync::atomic::AtomicUsize,
}
#[derive(Clone)]
pub struct TransferManager(Arc<Inner>);
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
fn valid_token(value: &str) -> bool {
    value.len() == 32 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

pub struct UploadRequest {
    pub id: String,
    pub key: String,
    pub path: VirtualPath,
    pub total: Option<u64>,
    pub policy: ConflictPolicy,
}
enum Message {
    Data(Vec<u8>),
    Finish,
    Failed,
}
struct DisconnectGuard {
    manager: TransferManager,
    id: String,
}
impl Drop for DisconnectGuard {
    fn drop(&mut self) {
        let mut records = self
            .manager
            .0
            .records
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(record) = records.get_mut(&self.id)
            && !record.transfer.state.terminal()
            && record.transfer.state != TransferState::Publishing
        {
            record.cancel.cancel();
            if record.transfer.state == TransferState::Queued {
                record.transfer.state = TransferState::Cancelled;
                record.transfer.updated_unix_ms = now();
                self.manager.event("transfer.updated", &record.transfer);
            }
        }
    }
}
impl TransferManager {
    pub fn new(
        files: FileService,
        settings: &HostSettings,
        shutdown: CancellationToken,
    ) -> Result<Self, splitshare_core::DomainError> {
        let limit = settings.effective_upload_limit()?;
        let (events, _) = broadcast::channel(128);
        Ok(Self(Arc::new(Inner {
            records: Mutex::new(BTreeMap::new()),
            slots: Arc::new(Semaphore::new(limit)),
            events,
            shutdown,
            files,
            limit: std::sync::atomic::AtomicUsize::new(limit),
        })))
    }
    pub fn limit(&self) -> usize {
        self.0.limit.load(std::sync::atomic::Ordering::Relaxed)
    }
    /// Change limits only while idle; registration is excluded through persistence.
    pub fn configure_limit(
        &self,
        limit: usize,
        persist: impl FnOnce() -> Result<(), crate::sessions::AccessError>,
    ) -> Result<(), crate::sessions::AccessError> {
        if !(1..=32).contains(&limit) {
            return Err(crate::sessions::AccessError::Invalid);
        }
        let records = self.0.records.lock().unwrap();
        let old = self.limit();
        if old != limit
            && (records
                .values()
                .any(|record| !record.transfer.state.terminal())
                || self.0.slots.available_permits() != old)
        {
            return Err(crate::sessions::AccessError::Busy);
        }
        persist()?;
        if limit > old {
            self.0.slots.add_permits(limit - old);
        }
        if limit < old {
            self.0.slots.forget_permits(old - limit);
        }
        self.0
            .limit
            .store(limit, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
    pub fn subscribe(&self) -> broadcast::Receiver<TransferEvent> {
        self.0.events.subscribe()
    }
    pub fn snapshot(&self) -> Vec<Transfer> {
        self.0
            .records
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .values()
            .map(|record| record.transfer.clone())
            .collect()
    }
    fn event(&self, name: &'static str, transfer: &Transfer) {
        let _ = self.0.events.send(TransferEvent {
            name,
            transfer: transfer.clone(),
        });
    }
    fn update(
        &self,
        id: &str,
        state: TransferState,
        bytes: u64,
        started: Option<Instant>,
        failure: Option<&'static str>,
        emit: bool,
    ) -> Option<Transfer> {
        let mut records = self
            .0
            .records
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(record) = records.get_mut(id) {
            record.transfer.state = state;
            record.transfer.transferred_bytes = bytes;
            record.transfer.updated_unix_ms = now();
            record.transfer.failure = failure;
            record.transfer.bytes_per_second = started.and_then(|start| {
                let seconds = start.elapsed().as_secs_f64();
                (seconds > 0.0 && bytes > 0).then(|| (bytes as f64 / seconds) as u64)
            });
            record.transfer.eta_seconds = if state == TransferState::Uploading {
                record
                    .transfer
                    .total_bytes
                    .zip(record.transfer.bytes_per_second)
                    .filter(|(_, speed)| *speed > 0)
                    .map(|(total, speed)| total.saturating_sub(bytes).div_ceil(speed))
            } else {
                None
            };
            if emit {
                self.event("transfer.updated", &record.transfer);
            }
            Some(record.transfer.clone())
        } else {
            None
        }
    }
    pub fn cancel(&self, id: &str, key: &str) -> Result<(), TransferError> {
        let records = self
            .0
            .records
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let record = records.get(id).ok_or(TransferError::Missing)?;
        if record.key != key {
            return Err(TransferError::Forbidden);
        }
        if record.transfer.state.terminal() || record.transfer.state == TransferState::Publishing {
            return Err(TransferError::TooLate);
        }
        record.cancel.cancel();
        Ok(())
    }
    fn register(&self, request: &UploadRequest) -> Result<CancellationToken, TransferError> {
        if !valid_token(&request.id) || !valid_token(&request.key) || request.path.is_root() {
            return Err(TransferError::Invalid);
        }
        let mut records = self
            .0
            .records
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if records.contains_key(&request.id) {
            return Err(TransferError::Duplicate);
        }
        if records.len() >= 128 {
            let oldest = records
                .iter()
                .filter(|(_, record)| record.transfer.state.terminal())
                .min_by_key(|(_, record)| record.transfer.updated_unix_ms)
                .map(|(id, _)| id.clone());
            if let Some(oldest) = oldest {
                records.remove(&oldest);
            } else {
                return Err(TransferError::Capacity);
            }
        }
        let cancel = self.0.shutdown.child_token();
        let transfer = Transfer {
            id: request.id.clone(),
            path: request.path.clone(),
            direction: "upload",
            total_bytes: request.total,
            transferred_bytes: 0,
            state: TransferState::Queued,
            bytes_per_second: None,
            eta_seconds: None,
            failure: None,
            created_unix_ms: now(),
            updated_unix_ms: now(),
        };
        self.event("transfer.created", &transfer);
        records.insert(
            request.id.clone(),
            Record {
                transfer,
                key: request.key.clone(),
                cancel: cancel.clone(),
            },
        );
        Ok(cancel)
    }
    /// Input is a transport-neutral stream. At most two 64-KiB chunks are queued.
    pub async fn upload<S, B>(
        &self,
        request: UploadRequest,
        stream: S,
    ) -> Result<Transfer, TransferError>
    where
        S: Stream<Item = Result<B, io::Error>> + Send,
        B: AsRef<[u8]>,
    {
        let cancel = self.register(&request)?;
        let _disconnect = DisconnectGuard {
            manager: self.clone(),
            id: request.id.clone(),
        };
        let permit = tokio::select! {
            _ = cancel.cancelled() => { self.update(&request.id,TransferState::Cancelled,0,None,None,true); return Err(TransferError::Cancelled); },
            permit = self.0.slots.clone().acquire_owned() => permit.map_err(|_| TransferError::Cancelled)?,
        };
        let (sender, mut receiver) = mpsc::channel::<Message>(2);
        let manager = self.clone();
        let worker_cancel = cancel.clone();
        let id = request.id.clone();
        let request_id = id.clone();
        let task = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let started = Instant::now();
            let result = (|| -> Result<VirtualPath, TransferError> {
                if worker_cancel.is_cancelled() {
                    return Err(TransferError::Cancelled);
                }
                let mut upload = manager
                    .0
                    .files
                    .storage()
                    .begin_upload(&request.path, request.total)?;
                manager.update(&id, TransferState::Uploading, 0, Some(started), None, true);
                let mut last_event = Instant::now();
                loop {
                    let message = receiver.blocking_recv();
                    if worker_cancel.is_cancelled() {
                        return Err(TransferError::Cancelled);
                    }
                    match message {
                        Some(Message::Data(bytes)) => {
                            upload.write_chunk(&bytes)?;
                            let emit = last_event.elapsed() >= Duration::from_millis(100);
                            manager.update(
                                &id,
                                TransferState::Uploading,
                                upload.bytes_written(),
                                Some(started),
                                None,
                                emit,
                            );
                            if emit {
                                last_event = Instant::now();
                            }
                        }
                        Some(Message::Finish) => break,
                        Some(Message::Failed) => {
                            return Err(TransferError::Storage(StorageError::UploadFailed));
                        }
                        None => return Err(TransferError::Cancelled),
                    }
                }
                // Serialize cancellation against the publication boundary. Once publishing
                // begins, cancellation returns TooLate rather than claiming rollback.
                {
                    let mut records = manager
                        .0
                        .records
                        .lock()
                        .unwrap_or_else(|error| error.into_inner());
                    if worker_cancel.is_cancelled() {
                        return Err(TransferError::Cancelled);
                    }
                    let record = records.get_mut(&id).ok_or(TransferError::Missing)?;
                    record.transfer.state = TransferState::Publishing;
                    manager.event("transfer.updated", &record.transfer);
                }
                Ok(upload.publish(request.policy)?)
            })();
            let bytes = manager
                .snapshot()
                .into_iter()
                .find(|record| record.id == id)
                .map(|record| record.transferred_bytes)
                .unwrap_or(0);
            match result {
                Ok(path) => {
                    manager.0.files.changed(&path);
                    {
                        let mut records = manager
                            .0
                            .records
                            .lock()
                            .unwrap_or_else(|error| error.into_inner());
                        records.get_mut(&id).expect("active transfer").transfer.path = path;
                    }
                    manager
                        .update(
                            &id,
                            TransferState::Completed,
                            bytes,
                            Some(started),
                            None,
                            true,
                        )
                        .ok_or(TransferError::Missing)
                }
                Err(error) => {
                    let failure = match error {
                        TransferError::Storage(StorageError::Conflict) => "CONFLICT",
                        TransferError::Storage(StorageError::LengthMismatch) => "LENGTH_MISMATCH",
                        _ => "UPLOAD_FAILED",
                    };
                    manager.update(
                        &id,
                        if error == TransferError::Cancelled {
                            TransferState::Cancelled
                        } else {
                            TransferState::Failed
                        },
                        bytes,
                        Some(started),
                        Some(failure),
                        true,
                    );
                    Err(error)
                }
            }
        });
        futures_util::pin_mut!(stream);
        'input: loop {
            let next = tokio::select! {
                _ = cancel.cancelled() => break,
                _ = sender.closed() => break,
                next = tokio::time::timeout(Duration::from_secs(60),stream.next()) => next,
            };
            let message = match next {
                Ok(Some(Ok(bytes))) => {
                    for bytes in bytes.as_ref().chunks(64 * 1024) {
                        tokio::select! {
                            _ = cancel.cancelled() => break 'input,
                            result = sender.send(Message::Data(bytes.to_vec())) => if result.is_err() { break 'input; },
                        }
                    }
                    continue;
                }
                Ok(None) => Message::Finish,
                _ => Message::Failed,
            };
            tokio::select! { _ = cancel.cancelled() => {}, _ = sender.send(message) => {} }
            break;
        }
        drop(sender);
        match task.await {
            Ok(result) => result,
            Err(_) => {
                let bytes = self
                    .snapshot()
                    .into_iter()
                    .find(|transfer| transfer.id == request_id)
                    .map(|transfer| transfer.transferred_bytes)
                    .unwrap_or(0);
                self.update(
                    &request_id,
                    TransferState::Failed,
                    bytes,
                    None,
                    Some("WORKER_FAILED"),
                    true,
                );
                Err(TransferError::Storage(StorageError::Io))
            }
        }
    }
}

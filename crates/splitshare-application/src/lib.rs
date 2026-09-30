//! Application orchestration; no Axum dependency or native path serialization.
use serde::Serialize;
use splitshare_core::{FileEntry, StorageError, VirtualPath};
use splitshare_storage::{ReadHandle, Storage};
use std::sync::Arc;
use tokio::sync::{Semaphore, broadcast};

#[derive(Clone, Debug, Serialize)]
pub struct FilesystemChanged {
    pub paths: Vec<VirtualPath>,
}
#[derive(Clone)]
pub struct FileService {
    storage: Storage,
    jobs: Arc<Semaphore>,
    events: broadcast::Sender<FilesystemChanged>,
}
impl FileService {
    pub fn new(storage: Storage) -> Self {
        let (events, _) = broadcast::channel(128);
        Self {
            storage,
            jobs: Arc::new(Semaphore::new(16)),
            events,
        }
    }
    pub fn subscribe(&self) -> broadcast::Receiver<FilesystemChanged> {
        self.events.subscribe()
    }
    async fn run<T: Send + 'static>(
        &self,
        operation: impl FnOnce(Storage) -> Result<T, StorageError> + Send + 'static,
    ) -> Result<T, StorageError> {
        let permit = self
            .jobs
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| StorageError::Io)?;
        let storage = self.storage.clone();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            operation(storage)
        })
        .await
        .map_err(|_| StorageError::Io)?
    }
    pub async fn list(&self, path: VirtualPath) -> Result<Vec<FileEntry>, StorageError> {
        self.run(move |storage| storage.list(&path)).await
    }
    pub async fn read(&self, path: VirtualPath) -> Result<ReadHandle, StorageError> {
        self.run(move |storage| storage.read(&path)).await
    }
    async fn mutate(
        &self,
        paths: Vec<VirtualPath>,
        operation: impl FnOnce(Storage) -> Result<(), StorageError> + Send + 'static,
    ) -> Result<(), StorageError> {
        // Publish in the worker, even if the initiating HTTP request disconnects.
        let events = self.events.clone();
        self.run(move |storage| {
            operation(storage)?;
            // Having no active subscribers is normal; snapshots remain authoritative.
            let _ = events.send(FilesystemChanged { paths });
            tracing::debug!("Filesystem mutation completed");
            Ok(())
        })
        .await
    }
    pub async fn mkdir(
        &self,
        parent: VirtualPath,
        name: String,
    ) -> Result<VirtualPath, StorageError> {
        let destination = parent.join(&name)?;
        let path = destination.clone();
        self.mutate(vec![parent], move |storage| storage.mkdir(&path))
            .await?;
        Ok(destination)
    }
    pub async fn rename(&self, from: VirtualPath, to: VirtualPath) -> Result<(), StorageError> {
        self.mutate(vec![parent(&from), parent(&to)], move |storage| {
            storage.rename(&from, &to)
        })
        .await
    }
    pub async fn delete(&self, path: VirtualPath) -> Result<(), StorageError> {
        self.mutate(vec![parent(&path)], move |storage| storage.delete(&path))
            .await
    }
}
fn parent(path: &VirtualPath) -> VirtualPath {
    let prefix = path
        .as_str()
        .rsplit_once('/')
        .map(|pair| pair.0)
        .unwrap_or("");
    VirtualPath::try_from(if prefix.is_empty() { "/" } else { prefix })
        .expect("validated path parent")
}

pub mod transfers;
impl FileService {
    pub(crate) fn storage(&self) -> Storage {
        self.storage.clone()
    }
    pub(crate) fn changed(&self, path: &VirtualPath) {
        let _ = self.events.send(FilesystemChanged {
            paths: vec![parent(path)],
        });
    }
}

pub mod sessions;

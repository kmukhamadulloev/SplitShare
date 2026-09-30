//! Server-authoritative transfer snapshots; no secrets or native paths.
use crate::VirtualPath;
use serde::Serialize;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferState {
    Queued,
    Uploading,
    Publishing,
    Completed,
    Failed,
    Cancelled,
}
impl TransferState {
    pub fn terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Transfer {
    pub id: String,
    pub path: VirtualPath,
    pub direction: &'static str,
    pub total_bytes: Option<u64>,
    pub transferred_bytes: u64,
    pub state: TransferState,
    pub bytes_per_second: Option<u64>,
    pub eta_seconds: Option<u64>,
    pub failure: Option<&'static str>,
    pub created_unix_ms: u64,
    pub updated_unix_ms: u64,
}

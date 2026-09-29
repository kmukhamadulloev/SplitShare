//! Transport-independent host configuration and domain errors.
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShareMode {
    TokenLink,
    OpenLan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostSettings {
    pub share_mode: ShareMode,
    pub parallel_uploads_enabled: bool,
    pub max_parallel_uploads: u8,
}

impl Default for HostSettings {
    fn default() -> Self {
        Self {
            share_mode: ShareMode::TokenLink,
            parallel_uploads_enabled: false,
            max_parallel_uploads: 3,
        }
    }
}

impl HostSettings {
    pub fn validate(&self) -> Result<(), DomainError> {
        if !(1..=32).contains(&self.max_parallel_uploads) {
            return Err(DomainError::InvalidUploadLimit);
        }
        Ok(())
    }

    pub fn effective_upload_limit(&self) -> Result<usize, DomainError> {
        self.validate()?;
        Ok(if self.parallel_uploads_enabled {
            usize::from(self.max_parallel_uploads)
        } else {
            1
        })
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("The upload concurrency limit must be between 1 and 32.")]
    InvalidUploadLimit,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_require_token_and_serial_uploads() {
        let mut settings = HostSettings::default();
        assert_eq!(settings.share_mode, ShareMode::TokenLink);
        assert_eq!(settings.effective_upload_limit(), Ok(1));
        settings.parallel_uploads_enabled = true;
        assert_eq!(settings.effective_upload_limit(), Ok(3));
        for limit in [0, 33, 255] {
            settings.max_parallel_uploads = limit;
            assert_eq!(settings.validate(), Err(DomainError::InvalidUploadLimit));
        }
    }
}

pub mod storage;
pub use storage::{ConflictPolicy, EntryKind, FileEntry, StorageError, VirtualPath};

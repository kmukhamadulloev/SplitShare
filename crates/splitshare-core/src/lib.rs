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
    #[serde(default)]
    pub permissions: PermissionSet,
    pub parallel_uploads_enabled: bool,
    pub max_parallel_uploads: u8,
}

impl Default for HostSettings {
    fn default() -> Self {
        Self {
            share_mode: ShareMode::TokenLink,
            permissions: PermissionSet::default(),
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

pub mod transfer;
pub use transfer::{Transfer, TransferState};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PermissionSet {
    pub browse: bool,
    pub download: bool,
    pub upload: bool,
    pub create_directory: bool,
    pub rename: bool,
    pub delete: bool,
}
impl PermissionSet {
    pub const fn all(value: bool) -> Self {
        Self {
            browse: value,
            download: value,
            upload: value,
            create_directory: value,
            rename: value,
            delete: value,
        }
    }
    pub fn allows(self, capability: Capability) -> bool {
        match capability {
            Capability::Browse => self.browse,
            Capability::Download => self.download,
            Capability::Upload => self.upload,
            Capability::CreateDirectory => self.create_directory,
            Capability::Rename => self.rename,
            Capability::Delete => self.delete,
        }
    }
}
impl Default for PermissionSet {
    fn default() -> Self {
        Self {
            browse: true,
            download: true,
            upload: true,
            create_directory: true,
            rename: true,
            delete: false,
        }
    }
}
#[derive(Clone, Copy)]
pub enum Capability {
    Browse,
    Download,
    Upload,
    CreateDirectory,
    Rename,
    Delete,
}

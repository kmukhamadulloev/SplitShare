//! Validated virtual paths and storage contracts. No host paths or OS errors.
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct VirtualPath(String);

impl VirtualPath {
    pub fn root() -> Self { Self("/".into()) }
    pub fn as_str(&self) -> &str { &self.0 }
    pub fn is_root(&self) -> bool { self.0 == "/" }
    pub fn components(&self) -> impl Iterator<Item = &str> { self.0.split('/').filter(|s| !s.is_empty()) }
    pub fn join(&self, name: &str) -> Result<Self, StorageError> {
        if name.contains('/') { return Err(StorageError::InvalidPath); }
        Self::try_from(format!("{}{name}", if self.is_root() { "/".into() } else { format!("{}/", self.0) }))
    }
}

impl TryFrom<String> for VirtualPath {
    type Error = StorageError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value == "/" { return Ok(Self(value)); }
        if !value.starts_with('/') || value.len() > 4096 || value.ends_with('/') {
            return Err(StorageError::InvalidPath);
        }
        let mut count = 0;
        for name in value[1..].split('/') {
            count += 1;
            let stem = name.split('.').next().unwrap_or_default().to_uppercase();
            let device = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$")
                || ["COM", "LPT"].iter().any(|prefix| stem.strip_prefix(prefix).is_some_and(|n| matches!(n, "1"|"2"|"3"|"4"|"5"|"6"|"7"|"8"|"9"|"¹"|"²"|"³")));
            if name.is_empty() || name.len() > 255 || name == "." || name == ".."
                || name.ends_with(['.', ' ']) || device
                || name.to_ascii_lowercase().starts_with(".splitshare-")
                || name.chars().any(|c| c.is_control() || matches!(c, '\\' | ':' | '%' | '<' | '>' | '"' | '|' | '?' | '*'))
            { return Err(StorageError::InvalidPath); }
        }
        if count > 128 { return Err(StorageError::InvalidPath); }
        Ok(Self(value))
    }
}
impl TryFrom<&str> for VirtualPath {
    type Error = StorageError;
    fn try_from(value: &str) -> Result<Self, Self::Error> { Self::try_from(value.to_owned()) }
}
impl From<VirtualPath> for String { fn from(value: VirtualPath) -> Self { value.0 } }

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConflictPolicy { Ask, Reject, Replace, AutoRename }

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind { File, Directory }

#[derive(Clone, Debug, Serialize)]
pub struct FileEntry {
    pub path: VirtualPath,
    pub name: String,
    pub kind: EntryKind,
    pub size: Option<u64>,
    pub modified_unix_seconds: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Error, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StorageError {
    #[error("The virtual path is invalid.")] InvalidPath,
    #[error("The requested item does not exist.")] NotFound,
    #[error("An item already exists at that destination.")] Conflict,
    #[error("The virtual root cannot be changed.")] RootProtected,
    #[error("Links and special files are not supported.")] UnsupportedEntry,
    #[error("The requested item is not a file.")] NotFile,
    #[error("The requested item is not a directory.")] NotDirectory,
    #[error("The directory is not empty.")] DirectoryNotEmpty,
    #[error("Access was denied.")] PermissionDenied,
    #[error("The filesystem does not support this operation.")] UnsupportedOperation,
    #[error("The upload length does not match the expected length.")] LengthMismatch,
    #[error("The upload has failed and cannot be published.")] UploadFailed,
    #[error("The storage operation failed.")] Io,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_cross_platform_attack_paths_and_serde_bypass() {
        for path in ["", "relative", "../x", "/..", "/a/../b", "/./x", "//server/share", "C:/x", "/C:/x", "/a\\b", "/%2e%2e/x", "/%252e%252e/x", "/x\0", "/x\n", "/CON.txt", "/com1", "/LPT².log", "/x.", "/x ", "/a//b", "/a/", "/.splitshare-upload-x.part", "/.SPLITSHARE-hidden"] {
            assert!(VirtualPath::try_from(path).is_err(), "{path:?}");
            assert!(serde_json::from_str::<VirtualPath>(&serde_json::to_string(path).unwrap()).is_err());
        }
        assert!(VirtualPath::try_from(format!("/{}", "x".repeat(256))).is_err());
    }
    #[test]
    fn preserves_unicode_and_virtual_root() {
        for path in ["/", "/notes.md", "/資料/é.txt", "/e\u{301}.txt"] {
            let parsed = VirtualPath::try_from(path).unwrap();
            assert_eq!(parsed.as_str(), path);
            assert_eq!(serde_json::to_string(&parsed).unwrap(), serde_json::to_string(path).unwrap());
        }
        assert!(VirtualPath::root().join("nested/name").is_err());
    }
}

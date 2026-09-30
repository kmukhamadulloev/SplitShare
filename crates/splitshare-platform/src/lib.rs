//! Native configuration adapter. Host paths never enter HTTP response types.
use directories::BaseDirs;
use splitshare_core::HostSettings;
use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};
use thiserror::Error;

const MAX_CONFIG_BYTES: u64 = 64 * 1024;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Application data directory is unavailable.")]
    NoDataDirectory,
    #[error("Unable to access host configuration.")]
    Io(#[from] io::Error),
    #[error("Host configuration is malformed or unsupported.")]
    Invalid,
}

pub fn config_path() -> Result<PathBuf, ConfigError> {
    let dirs = BaseDirs::new().ok_or(ConfigError::NoDataDirectory)?;
    Ok(dirs.data_local_dir().join("SplitShare").join("config.json"))
}

// Explicit envelope keeps version checks ahead of deserializing version-specific settings.
fn decode(bytes: &[u8]) -> Result<HostSettings, ConfigError> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| ConfigError::Invalid)?;
    let object = value.as_object().ok_or(ConfigError::Invalid)?;
    if object.len() != 2 || object.get("version").and_then(|v| v.as_u64()) != Some(1) {
        return Err(ConfigError::Invalid);
    }
    let settings: HostSettings =
        serde_json::from_value(object.get("settings").ok_or(ConfigError::Invalid)?.clone())
            .map_err(|_| ConfigError::Invalid)?;
    settings.validate().map_err(|_| ConfigError::Invalid)?;
    Ok(settings)
}

pub fn load_or_create(path: &Path) -> Result<HostSettings, ConfigError> {
    match fs::File::open(path) {
        Ok(file) => {
            let mut bytes = Vec::new();
            file.take(MAX_CONFIG_BYTES + 1).read_to_end(&mut bytes)?;
            if bytes.len() as u64 > MAX_CONFIG_BYTES {
                return Err(ConfigError::Invalid);
            }
            decode(&bytes)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let settings = HostSettings::default();
            let parent = path.parent().ok_or(ConfigError::Invalid)?;
            fs::create_dir_all(parent)?;
            let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
            let bytes =
                serde_json::to_vec_pretty(&serde_json::json!({"version": 1, "settings": settings}))
                    .map_err(|_| ConfigError::Invalid)?;
            temporary.write_all(&bytes)?;
            temporary.as_file().sync_all()?;
            match temporary.persist_noclobber(path) {
                Ok(_) => Ok(settings),
                Err(error) if error.error.kind() == io::ErrorKind::AlreadyExists => {
                    load_or_create(path)
                }
                Err(error) => Err(ConfigError::Io(error.error)),
            }
        }
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn creates_and_reloads_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/config.json");
        assert_eq!(load_or_create(&path).unwrap(), HostSettings::default());
        assert_eq!(load_or_create(&path).unwrap(), HostSettings::default());
    }
    #[test]
    fn rejects_invalid_files_without_overwriting() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        for bytes in [
            b"broken".to_vec(),
            br#"{"version":2,"settings":{}}"#.to_vec(),
            vec![b' '; 65537],
        ] {
            fs::write(&path, &bytes).unwrap();
            assert!(load_or_create(&path).is_err());
            assert_eq!(fs::read(&path).unwrap(), bytes);
        }
    }
}

/// Atomically replace versioned configuration; no host path is returned to clients.
pub fn save(path: &Path, settings: &HostSettings) -> Result<(), ConfigError> {
    settings.validate().map_err(|_| ConfigError::Invalid)?;
    let parent = path.parent().ok_or(ConfigError::Invalid)?;
    fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    let bytes = serde_json::to_vec_pretty(&serde_json::json!({"version": 1, "settings": settings}))
        .map_err(|_| ConfigError::Invalid)?;
    temporary.write_all(&bytes)?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(path)
        .map_err(|error| ConfigError::Io(error.error))?;
    Ok(())
}

#[cfg(test)]
mod persistence_tests {
    use super::*;
    #[test]
    fn migrates_legacy_permissions_and_atomically_saves_settings() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("config.json");
        fs::write(&path, br#"{"version":1,"settings":{"share_mode":"token_link","parallel_uploads_enabled":false,"max_parallel_uploads":3}}"#).unwrap();
        let mut settings = load_or_create(&path).unwrap();
        assert!(!settings.permissions.delete);
        settings.permissions.upload = false;
        settings.share_mode = splitshare_core::ShareMode::OpenLan;
        save(&path, &settings).unwrap();
        assert_eq!(load_or_create(&path).unwrap(), settings);
        settings.max_parallel_uploads = 0;
        assert!(save(&path, &settings).is_err());
        assert_ne!(load_or_create(&path).unwrap(), settings);
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }
}

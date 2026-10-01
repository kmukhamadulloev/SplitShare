//! Private native startup settings. This file is never returned through HTTP.
use crate::ConfigError;
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    net::SocketAddrV4,
    path::{Path, PathBuf},
};
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Startup {
    pub root: Option<PathBuf>,
    pub bind: Option<SocketAddrV4>,
}
pub fn load(path: &Path) -> Result<Startup, ConfigError> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Startup::default()),
        Err(error) => return Err(error.into()),
    };
    let mut bytes = Vec::new();
    file.take(65537).read_to_end(&mut bytes)?;
    if bytes.len() > 65536 {
        return Err(ConfigError::Invalid);
    }
    let mut value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| ConfigError::Invalid)?;
    let object = value.as_object_mut().ok_or(ConfigError::Invalid)?;
    if object
        .remove("version")
        .and_then(|version| version.as_u64())
        != Some(1)
    {
        return Err(ConfigError::Invalid);
    }
    serde_json::from_value(value).map_err(|_| ConfigError::Invalid)
}
pub fn save(path: &Path, startup: &Startup) -> Result<(), ConfigError> {
    let parent = path.parent().ok_or(ConfigError::Invalid)?;
    std::fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(
        &serde_json::to_vec_pretty(
            &serde_json::json!({"version":1,"root":startup.root,"bind":startup.bind}),
        )
        .map_err(|_| ConfigError::Invalid)?,
    )?;
    file.as_file().sync_all()?;
    file.persist(path)
        .map_err(|error| ConfigError::Io(error.error))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn private_startup_settings_are_versioned_atomic_and_restricted() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("host.json");
        let startup = Startup {
            root: Some(root.path().join("chosen")),
            bind: Some("0.0.0.0:8080".parse().unwrap()),
        };
        save(&path, &startup).unwrap();
        assert_eq!(load(&path).unwrap().root, startup.root);
        assert_eq!(load(&path).unwrap().bind, startup.bind);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o077,
                0
            );
        }
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
        std::fs::write(&path, br#"{"version":99,"root":null,"bind":null}"#).unwrap();
        assert!(load(&path).is_err());
        std::fs::write(&path, vec![b' '; 65537]).unwrap();
        assert!(load(&path).is_err());
    }
}

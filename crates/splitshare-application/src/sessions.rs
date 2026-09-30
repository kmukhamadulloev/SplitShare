//! In-memory private-network access policy. Tokens never implement Debug or Serialize.
use splitshare_core::{Capability, HostSettings, PermissionSet, ShareMode};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;

pub const SESSION_SECONDS: u64 = 12 * 60 * 60;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessError {
    Unauthorized,
    Forbidden,
    Capacity,
    Random,
    Invalid,
    Save,
    Busy,
}
pub trait SettingsStore: Send + Sync {
    fn save(&self, settings: &HostSettings) -> Result<(), AccessError>;
}
struct Session {
    expires: Instant,
    revoked: CancellationToken,
}
struct Policy {
    settings: HostSettings,
    token: String,
    sessions: HashMap<String, Session>,
    changed: CancellationToken,
}
#[derive(Clone)]
pub struct SessionManager(Arc<Mutex<Policy>>);
#[derive(Clone)]
pub struct Access {
    pub local: bool,
    pub changed: CancellationToken,
    pub revoked: CancellationToken,
    pub expires: Instant,
}
impl Access {
    pub async fn invalidated(&self) {
        tokio::select! {
            _ = self.changed.cancelled() => {},
            _ = self.revoked.cancelled() => {},
            _ = tokio::time::sleep_until(self.expires.into()) => {},
        }
    }
}
fn secret() -> Result<String, AccessError> {
    let mut bytes = [0; 32];
    getrandom::fill(&mut bytes).map_err(|_| AccessError::Random)?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}
fn matches_secret(left: &str, right: &str) -> bool {
    left.len() == right.len()
        && left
            .bytes()
            .zip(right.bytes())
            .fold(0, |difference, (a, b)| difference | (a ^ b))
            == 0
}
impl SessionManager {
    pub fn new(settings: HostSettings) -> Result<Self, AccessError> {
        settings.validate().map_err(|_| AccessError::Invalid)?;
        Ok(Self(Arc::new(Mutex::new(Policy {
            settings,
            token: secret()?,
            sessions: HashMap::new(),
            changed: CancellationToken::new(),
        }))))
    }
    pub fn settings(&self) -> HostSettings {
        self.0.lock().unwrap().settings.clone()
    }
    pub fn share_token(&self) -> String {
        self.0.lock().unwrap().token.clone()
    }
    pub fn join(&self, token: &str, previous: Option<&str>) -> Result<String, AccessError> {
        let mut policy = self.0.lock().unwrap();
        if !matches_secret(&policy.token, token) {
            return Err(AccessError::Unauthorized);
        }
        policy
            .sessions
            .retain(|_, session| session.expires > Instant::now());
        if let Some(previous) = previous
            && let Some(session) = policy.sessions.remove(previous)
        {
            session.revoked.cancel();
        }
        if policy.sessions.len() >= 256 {
            return Err(AccessError::Capacity);
        }
        let id = secret()?;
        policy.sessions.insert(
            id.clone(),
            Session {
                expires: Instant::now() + Duration::from_secs(SESSION_SECONDS),
                revoked: CancellationToken::new(),
            },
        );
        tracing::info!("Share session joined");
        Ok(id)
    }
    pub fn authorize(
        &self,
        local: bool,
        cookie: Option<&str>,
        capability: Option<Capability>,
    ) -> Result<Access, AccessError> {
        let policy = self.0.lock().unwrap();
        let mut access = Access {
            local,
            changed: CancellationToken::new(),
            revoked: CancellationToken::new(),
            expires: Instant::now() + Duration::from_secs(SESSION_SECONDS),
        };
        if local {
            return Ok(access);
        }
        access.changed = policy.changed.clone();
        if policy.settings.share_mode == ShareMode::TokenLink {
            let session = cookie
                .and_then(|id| policy.sessions.get(id))
                .filter(|session| session.expires > Instant::now())
                .ok_or(AccessError::Unauthorized)?;
            access.revoked = session.revoked.clone();
            access.expires = session.expires;
        }
        if capability.is_some_and(|capability| !policy.settings.permissions.allows(capability)) {
            return Err(AccessError::Forbidden);
        }
        Ok(access)
    }
    pub fn permissions(&self, local: bool) -> PermissionSet {
        if local {
            PermissionSet::all(true)
        } else {
            self.settings().permissions
        }
    }
    pub fn leave(&self, cookie: Option<&str>) {
        if let Some(cookie) = cookie
            && let Some(session) = self.0.lock().unwrap().sessions.remove(cookie)
        {
            session.revoked.cancel();
        }
    }
    pub fn rotate(&self) -> Result<(), AccessError> {
        let token = secret()?;
        let mut policy = self.0.lock().unwrap();
        policy.token = token;
        for (_, session) in policy.sessions.drain() {
            session.revoked.cancel();
        }
        policy.changed.cancel();
        policy.changed = CancellationToken::new();
        tracing::info!("Share token rotated; remote sessions revoked");
        Ok(())
    }
    pub fn update(
        &self,
        settings: HostSettings,
        store: Option<&dyn SettingsStore>,
        transfers: Option<&crate::transfers::TransferManager>,
    ) -> Result<(), AccessError> {
        settings.validate().map_err(|_| AccessError::Invalid)?;
        let mut policy = self.0.lock().unwrap();
        let persist = || {
            if let Some(store) = store {
                store.save(&settings)?;
            }
            Ok(())
        };
        if let Some(transfers) = transfers {
            transfers.configure_limit(
                settings
                    .effective_upload_limit()
                    .map_err(|_| AccessError::Invalid)?,
                persist,
            )?;
        } else {
            persist()?;
        }
        if policy.settings.share_mode != settings.share_mode {
            for (_, session) in policy.sessions.drain() {
                session.revoked.cancel();
            }
        }
        policy.settings = settings;
        policy.changed.cancel();
        policy.changed = CancellationToken::new();
        tracing::info!("Host settings updated");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tokens_sessions_expiry_rotation_and_capacity() {
        let manager = SessionManager::new(HostSettings::default()).unwrap();
        let token = manager.share_token();
        assert_eq!(token.len(), 64);
        assert!(token.bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert_ne!(
            token,
            SessionManager::new(HostSettings::default())
                .unwrap()
                .share_token()
        );
        assert!(manager.join("invalid", None).is_err());
        let id = manager.join(&token, None).unwrap();
        assert_ne!(id, token);
        let grant = manager.authorize(false, Some(&id), None).unwrap();
        manager
            .0
            .lock()
            .unwrap()
            .sessions
            .get_mut(&id)
            .unwrap()
            .expires = Instant::now();
        assert!(manager.authorize(false, Some(&id), None).is_err());
        let id = manager.join(&token, None).unwrap();
        manager.rotate().unwrap();
        assert!(grant.changed.is_cancelled());
        assert!(manager.join(&token, None).is_err());
        assert!(manager.authorize(false, Some(&id), None).is_err());
        let token = manager.share_token();
        for _ in 0..256 {
            manager.join(&token, None).unwrap();
        }
        assert_eq!(manager.join(&token, None), Err(AccessError::Capacity));
    }
    #[test]
    fn capability_policy_and_failed_persistence_are_atomic() {
        struct Broken;
        impl SettingsStore for Broken {
            fn save(&self, _: &HostSettings) -> Result<(), AccessError> {
                Err(AccessError::Save)
            }
        }
        let manager = SessionManager::new(HostSettings::default()).unwrap();
        let id = manager.join(&manager.share_token(), None).unwrap();
        let grant = manager.authorize(false, Some(&id), None).unwrap();
        let settings = HostSettings {
            permissions: PermissionSet::all(false),
            ..manager.settings()
        };
        assert_eq!(
            manager.update(settings.clone(), Some(&Broken), None),
            Err(AccessError::Save)
        );
        assert!(!grant.changed.is_cancelled());
        manager.update(settings, None, None).unwrap();
        assert!(grant.changed.is_cancelled());
        for capability in [
            Capability::Browse,
            Capability::Download,
            Capability::Upload,
            Capability::CreateDirectory,
            Capability::Rename,
            Capability::Delete,
        ] {
            assert!(matches!(
                manager.authorize(false, Some(&id), Some(capability)),
                Err(AccessError::Forbidden)
            ));
            assert!(manager.authorize(true, None, Some(capability)).is_ok());
        }
        let settings = HostSettings {
            share_mode: ShareMode::OpenLan,
            ..manager.settings()
        };
        manager.update(settings, None, None).unwrap();
        assert!(manager.authorize(false, None, None).is_ok());
        manager.update(HostSettings::default(), None, None).unwrap();
        assert!(manager.authorize(false, Some(&id), None).is_err());
        let id = manager.join(&manager.share_token(), None).unwrap();
        let access = manager.authorize(false, Some(&id), None).unwrap();
        manager.leave(Some(&id));
        assert!(access.revoked.is_cancelled());
        assert!(manager.authorize(false, Some(&id), None).is_err());
    }
}

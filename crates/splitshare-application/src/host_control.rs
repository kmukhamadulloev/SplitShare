//! Host-only lifecycle commands. Native paths never enter these transport DTOs.
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

#[derive(Clone, Serialize)]
pub struct Interface {
    pub address: String,
    pub label: String,
}
#[derive(Clone, Serialize)]
pub struct Snapshot {
    pub bind_ip: String,
    pub port: u16,
    pub folder_selected: bool,
    pub setup_required: bool,
    pub interfaces: Vec<Interface>,
    pub state: &'static str,
    pub message: Option<String>,
    pub local_url: String,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkSettings {
    pub bind_ip: std::net::Ipv4Addr,
    pub port: u16,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InitialSetup {
    pub network: NetworkSettings,
    pub settings: splitshare_core::HostSettings,
}
pub enum Action {
    ChooseFolder,
    Complete(InitialSetup),
    Network(NetworkSettings),
}
#[derive(Clone)]
pub struct HostControl {
    snapshot: Arc<Mutex<Snapshot>>,
    commands: mpsc::Sender<Action>,
}
impl HostControl {
    pub fn new(snapshot: Snapshot) -> (Self, mpsc::Receiver<Action>) {
        let (commands, receiver) = mpsc::channel(1);
        (
            Self {
                snapshot: Arc::new(Mutex::new(snapshot)),
                commands,
            },
            receiver,
        )
    }
    pub fn snapshot(&self) -> Snapshot {
        self.snapshot.lock().unwrap().clone()
    }
    pub fn publish(&self, snapshot: Snapshot) {
        *self.snapshot.lock().unwrap() = snapshot;
    }
    pub fn submit(&self, action: Action) -> Result<Snapshot, &'static str> {
        let mut snapshot = self.snapshot.lock().unwrap();
        if matches!(snapshot.state, "selecting" | "applying") {
            return Err("Another host configuration change is in progress.");
        }
        let network = match &action {
            Action::Network(settings) => {
                if snapshot.setup_required {
                    return Err("Complete initial setup before changing network settings.");
                }
                Some(settings)
            }
            Action::Complete(setup) => {
                if !snapshot.setup_required || !snapshot.folder_selected {
                    return Err("Choose a folder before completing initial setup.");
                }
                setup
                    .settings
                    .validate()
                    .map_err(|_| "Invalid sharing settings.")?;
                Some(&setup.network)
            }
            Action::ChooseFolder => None,
        };
        if let Some(settings) = network
            && (settings.port == 0
                || !snapshot
                    .interfaces
                    .iter()
                    .any(|item| item.address == settings.bind_ip.to_string()))
        {
            return Err("Choose an available IPv4 interface and a port from 1 to 65535.");
        }
        let state = if matches!(action, Action::ChooseFolder) {
            "selecting"
        } else {
            "applying"
        };
        self.commands
            .try_send(action)
            .map_err(|_| "Host controls are unavailable. Restart SplitShare and try again.")?;
        snapshot.state = state;
        snapshot.message = None;
        Ok(snapshot.clone())
    }
}

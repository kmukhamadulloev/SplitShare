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
pub enum Action {
    ChooseFolder,
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
        if let Action::Network(settings) = &action
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

//! Listener lifecycle and desktop command policy; never exposed as remote commands.
use crate::{ConfigStore, Options, shutdown_signal};
use splitshare_core::{HostSettings, ShareMode};
use splitshare_platform::desktop::{Command, DesktopActions, Status};
use splitshare_server::ServerState;
use std::{net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};
use tokio::{
    net::TcpListener,
    sync::{mpsc, watch},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
type Error = Box<dyn std::error::Error + Send + Sync>;
struct Running {
    state: ServerState,
    task: JoinHandle<std::io::Result<()>>,
}
pub struct Host {
    options: Options,
    config_path: PathBuf,
    settings: HostSettings,
    files: Option<splitshare_application::FileService>,
    address: SocketAddr,
    running: Option<Running>,
    actions: DesktopActions,
}
impl Host {
    pub async fn new(options: Options) -> Result<Self, Error> {
        Self::configured(options, splitshare_platform::config_path()?).await
    }
    async fn configured(options: Options, config_path: PathBuf) -> Result<Self, Error> {
        let mut settings = splitshare_platform::load_or_create(&config_path)?;
        if let Some(limit) = options.parallel {
            settings.parallel_uploads_enabled = limit > 1;
            settings.max_parallel_uploads = limit;
            settings.validate()?;
        }
        if options.open_lan {
            settings.share_mode = ShareMode::OpenLan;
        }
        // Retain the opened sandbox capability across stops, never reopen a replaced root.
        let files = options
            .root
            .as_deref()
            .map(splitshare_storage::Storage::open)
            .transpose()?
            .map(splitshare_application::FileService::new);
        let mut host = Self {
            address: options.bind.into(),
            options,
            config_path,
            settings,
            files,
            running: None,
            actions: DesktopActions::default(),
        };
        host.start().await?;
        Ok(host)
    }
    async fn start(&mut self) -> Result<(), Error> {
        if self.running.is_some() {
            return Ok(());
        }
        let addresses = splitshare_network::LocalAddresses::discover()?;
        let listener = TcpListener::bind(self.address).await?;
        self.address = listener.local_addr()?; // Keep an OS-assigned port stable across Start/Stop.
        let mut authorities = addresses.authorities(self.address.port());
        if !self.address.ip().is_unspecified() {
            authorities.push(self.address.to_string());
        }
        let mut state = ServerState::new(
            self.files.clone(),
            self.settings.clone(),
            false,
            addresses.clone(),
            authorities,
            CancellationToken::new(),
        );
        state.candidates = addresses.candidates(self.address);
        state.settings_store = Some(Arc::new(ConfigStore(self.config_path.clone())));
        if self.options.dev {
            state.dev_origin = Some("http://127.0.0.1:5173".into());
        }
        let task = tokio::spawn(splitshare_server::serve_api(listener, state.clone()));
        self.running = Some(Running { state, task });
        tracing::info!(address = %self.address, sharing = self.files.is_some(), "SplitShare started");
        Ok(())
    }
    async fn stop(&mut self) -> Result<(), Error> {
        let Some(mut running) = self.running.take() else {
            return Ok(());
        };
        running.state.shutdown.cancel();
        // A timed-out drain is fatal: do not start a second listener with old workers alive.
        match tokio::time::timeout(Duration::from_secs(10), &mut running.task).await {
            Ok(result) => result??,
            Err(_) => {
                running.task.abort();
                return Err("Listener shutdown exceeded ten seconds".into());
            }
        }
        // Capture settings after in-flight host mutations have drained.
        self.settings = running.state.sessions.settings();
        tracing::info!("Server stopped");
        Ok(())
    }
    fn local_url(&self) -> String {
        let ip = if self.address.ip().is_unspecified() {
            std::net::Ipv4Addr::LOCALHOST.into()
        } else {
            self.address.ip()
        };
        format!("http://{ip}:{}/", self.address.port())
    }
    fn share_link(&self) -> Option<String> {
        let state = &self.running.as_ref()?.state;
        self.files.as_ref()?;
        let candidate = state
            .candidates
            .iter()
            .find(|item| item.kind != "loopback")
            .or_else(|| state.candidates.first())?;
        Some(
            if state.sessions.settings().share_mode == ShareMode::OpenLan {
                format!("{}/", candidate.base_url)
            } else {
                format!("{}/j/{}", candidate.base_url, state.sessions.share_token())
            },
        )
    }
    fn publish(&self, status: &watch::Sender<Status>, busy: bool, message: Option<String>) {
        status.send_replace(Status {
            listening: self.running.is_some(),
            configured: self.files.is_some(),
            busy,
            exiting: false,
            message,
        });
    }
    pub async fn run(
        mut self,
        mut commands: mpsc::UnboundedReceiver<Command>,
        status: &watch::Sender<Status>,
    ) -> Result<(), Error> {
        self.publish(status, false, None);
        let signal = shutdown_signal();
        tokio::pin!(signal);
        if self.options.open_browser {
            tokio::select! {
                result = &mut signal => { result?; return self.stop().await; },
                result = DesktopActions::open(self.local_url()) => {
                    if let Err(error) = result { tracing::warn!(error, "Browser open failed"); }
                }
            }
        }
        let mut tick = tokio::time::interval(Duration::from_millis(200));
        loop {
            tokio::select! {
                result = &mut signal => { result?; break; },
                _ = tick.tick() => {
                    if self.running.as_ref().is_some_and(|running| running.task.is_finished()) {
                        self.stop().await?;
                        return Err("Listener stopped unexpectedly".into());
                    }
                },
                command = commands.recv() => {
                    let Some(command) = command else { break; };
                    let result = match command {
                        Command::Quit => break,
                        Command::Toggle => {
                            self.publish(status, true, None);
                            if self.running.is_some() { self.stop().await?; Ok(()) }
                            else { self.start().await.map_err(|_| "Unable to restart listener; address may be in use") }
                        },
                        Command::Open | Command::Settings => {
                            if self.running.is_none() { Err("Start sharing to open the browser") }
                            else { DesktopActions::open(format!("{}{}",self.local_url(),if command == Command::Settings { "#settings" } else { "" })).await }
                        },
                        Command::OpenFolder => match &self.options.root { Some(path) => DesktopActions::folder(path).await, None => Err("No shared folder selected") },
                        Command::CopyLink => match self.share_link() { Some(link) => self.actions.copy(link), None => Err("No active share link") },
                    };
                    if let Err(error) = result { tracing::warn!(error, "Desktop action failed"); }
                    self.publish(status, false, result.err().map(str::to_owned));
                }
            }
        }
        self.publish(status, true, None);
        self.stop().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn options(root: PathBuf) -> Options {
        Options {
            root: Some(root),
            bind: "127.0.0.1:0".parse().unwrap(),
            open_lan: false,
            dev: false,
            parallel: None,
            no_tray: true,
            open_browser: false,
        }
    }
    async fn fixture() -> (tempfile::TempDir, Host) {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("share");
        std::fs::create_dir(&root).unwrap();
        let host = Host::configured(options(root), temp.path().join("config.json"))
            .await
            .unwrap();
        (temp, host)
    }
    #[tokio::test]
    async fn stop_restart_keeps_port_and_sandbox_but_revokes_links() {
        let (_temp, mut host) = fixture().await;
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        let old_link = host.share_link().unwrap();
        let address = host.address;
        let settings = HostSettings {
            max_parallel_uploads: 3,
            parallel_uploads_enabled: true,
            ..Default::default()
        };
        assert_eq!(
            client
                .put(format!("{}api/v1/host/settings", host.local_url()))
                .header("X-SplitShare-Request", "1")
                .json(&settings)
                .send()
                .await
                .unwrap()
                .status(),
            200
        );
        assert_eq!(client.get(&old_link).send().await.unwrap().status(), 303);
        host.stop().await.unwrap();
        assert!(host.share_link().is_none());
        assert!(client.get(host.local_url()).send().await.is_err());
        let occupied = TcpListener::bind(address).await.unwrap();
        assert!(host.start().await.is_err());
        assert!(host.running.is_none());
        drop(occupied);
        host.start().await.unwrap();
        assert_eq!(host.address, address);
        assert_eq!(
            host.running
                .as_ref()
                .unwrap()
                .state
                .sessions
                .settings()
                .max_parallel_uploads,
            3
        );
        assert_ne!(host.share_link().unwrap(), old_link);
        assert_eq!(client.get(old_link).send().await.unwrap().status(), 401);
        assert_eq!(
            client
                .get(format!("{}api/v1/files?path=/", host.local_url()))
                .send()
                .await
                .unwrap()
                .status(),
            200
        );
        host.stop().await.unwrap();
    }
    #[tokio::test]
    async fn stop_cancels_active_upload_and_removes_partial_file() {
        let (temp, mut host) = fixture().await;
        let url = format!(
            "{}api/v1/uploads?path=/unfinished.bin&policy=reject",
            host.local_url()
        );
        let body = futures_util::stream::once(async { Ok::<_, std::io::Error>(vec![7u8; 65536]) })
            .chain(futures_util::stream::pending());
        use futures_util::StreamExt;
        let upload = tokio::spawn(async move {
            reqwest::Client::builder()
                .no_proxy()
                .build()
                .unwrap()
                .post(url)
                .header("X-SplitShare-Request", "1")
                .header("Content-Type", "application/octet-stream")
                .header("X-Transfer-Id", "11111111111111111111111111111111")
                .header("X-Transfer-Key", "22222222222222222222222222222222")
                .body(reqwest::Body::wrap_stream(body))
                .send()
                .await
        });
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                if std::fs::read_dir(temp.path().join("share"))
                    .unwrap()
                    .count()
                    > 0
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        host.stop().await.unwrap();
        assert_eq!(
            std::fs::read_dir(temp.path().join("share"))
                .unwrap()
                .count(),
            0
        );
        upload.abort();
    }
    #[tokio::test]
    async fn quit_command_drains_listener() {
        let (_temp, host) = fixture().await;
        let address = host.address;
        let (commands, receiver) = mpsc::unbounded_channel();
        let (status, _) = watch::channel(Status::default());
        commands.send(Command::Quit).unwrap();
        host.run(receiver, &status).await.unwrap();
        assert!(TcpListener::bind(address).await.is_ok());
    }
}

//! Listener lifecycle and desktop command policy; never exposed as remote commands.
use crate::{ConfigStore, Options, shutdown_signal};
use splitshare_application::host_control::{Action, HostControl, Interface, Snapshot};
use splitshare_core::{HostSettings, ShareMode};
use splitshare_platform::desktop::{Command, DesktopActions, Status};
use splitshare_platform::startup::{self, Startup};
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
    control: HostControl,
    setup_commands: Option<mpsc::Receiver<Action>>,
}
impl Host {
    pub async fn new(options: Options) -> Result<Self, Error> {
        Self::configured(options, splitshare_platform::config_path()?).await
    }
    async fn configured(mut options: Options, config_path: PathBuf) -> Result<Self, Error> {
        let root_explicit = options.root.is_some();
        let persisted = startup::load(&config_path.with_file_name("host.json"))?;
        if options.root.is_none() {
            options.root = persisted.root;
        }
        if !options.bind_explicit
            && !options.dev
            && let Some(bind) = persisted.bind
        {
            options.bind = bind;
        }
        if options.dev && !options.bind.ip().is_loopback() {
            return Err("Development mode requires loopback binding".into());
        }
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
        let mut unavailable_root = false;
        let files = match options
            .root
            .as_deref()
            .map(splitshare_storage::Storage::open)
            .transpose()
        {
            Ok(storage) => storage.map(splitshare_application::FileService::new),
            Err(error) if root_explicit => return Err(error.into()),
            Err(_) => {
                options.root = None;
                unavailable_root = true;
                tracing::warn!(
                    "Saved shared folder is unavailable; choose a folder in host settings"
                );
                None
            }
        };
        let (control, setup_commands) = HostControl::new(Snapshot {
            bind_ip: options.bind.ip().to_string(),
            port: options.bind.port(),
            folder_selected: files.is_some(),
            interfaces: vec![],
            state: "ready",
            message: None,
            local_url: String::new(),
        });
        let mut host = Self {
            address: options.bind.into(),
            options,
            config_path,
            settings,
            files,
            running: None,
            actions: DesktopActions::default(),
            control,
            setup_commands: Some(setup_commands),
        };
        host.start().await?;
        host.setup_status(
            "ready",
            if unavailable_root {
                Some("The saved folder is unavailable. Choose a shared folder again.")
            } else {
                None
            },
        );
        Ok(host)
    }
    async fn start(&mut self) -> Result<(), Error> {
        self.start_with(None).await
    }
    async fn start_with(&mut self, reserved: Option<TcpListener>) -> Result<(), Error> {
        if self.running.is_some() {
            return Ok(());
        }
        let addresses = splitshare_network::LocalAddresses::discover()?;
        let listener = match reserved {
            Some(listener) => listener,
            None => TcpListener::bind(self.address).await?,
        };
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
        state.host_control = Some(self.control.clone());
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
    fn setup_status(&self, state: &'static str, message: Option<&str>) {
        let mut interfaces = vec![Interface {
            address: "127.0.0.1".into(),
            label: "This computer only".into(),
        }];
        if !self.options.dev {
            interfaces.push(Interface {
                address: "0.0.0.0".into(),
                label: "All interfaces (LAN / VPN)".into(),
            });
            if let Some(running) = &self.running {
                for item in running
                    .state
                    .local_addresses
                    .candidates("0.0.0.0:1".parse().unwrap())
                {
                    if item.address != "127.0.0.1" {
                        interfaces.push(Interface {
                            address: item.address,
                            label: format!("{} · {}", item.interface, item.kind),
                        });
                    }
                }
            }
        }
        self.control.publish(Snapshot {
            bind_ip: self.address.ip().to_string(),
            port: self.address.port(),
            folder_selected: self.files.is_some(),
            interfaces,
            state,
            message: message.map(str::to_owned),
            local_url: self.local_url(),
        });
    }
    async fn reconfigure(
        &mut self,
        root: Option<PathBuf>,
        files: Option<splitshare_application::FileService>,
        address: SocketAddr,
    ) -> Result<(), Error> {
        if self.options.dev && !address.ip().is_loopback() {
            self.setup_status(
                "failed",
                Some("Development mode requires loopback binding."),
            );
            return Ok(());
        }
        // Reserve non-overlapping listeners before touching the active share.
        let reserve = address.port() != self.address.port()
            || (!address.ip().is_unspecified()
                && !self.address.ip().is_unspecified()
                && address.ip() != self.address.ip());
        let listener = if reserve {
            match TcpListener::bind(address).await {
                Ok(listener) => Some(listener),
                Err(_) => {
                    self.setup_status(
                        "failed",
                        Some(
                            "That address or port is unavailable. The current share is unchanged.",
                        ),
                    );
                    return Ok(());
                }
            }
        } else {
            None
        };
        let path = self.config_path.with_file_name("host.json");
        let previous = match startup::load(&path) {
            Ok(previous) => previous,
            Err(_) => {
                self.setup_status(
                    "failed",
                    Some("Saved host setup cannot be read. The current share is unchanged."),
                );
                tracing::warn!(
                    "Host setup file is invalid or unavailable; refusing to overwrite it"
                );
                return Ok(());
            }
        };
        let SocketAddr::V4(bind) = address else {
            return Err("IPv4 required".into());
        };
        if startup::save(
            &path,
            &Startup {
                root: root.clone(),
                bind: Some(bind),
            },
        )
        .is_err()
        {
            self.setup_status(
                "failed",
                Some("Could not save host setup. The current share is unchanged."),
            );
            return Ok(());
        }
        self.setup_status("applying", None);
        if let Err(error) = self.stop().await {
            if startup::save(&path, &previous).is_err() {
                tracing::error!("Could not restore host setup after shutdown failure");
            }
            return Err(error);
        }
        let old_address = self.address;
        let old_root = self.options.root.clone();
        let old_files = self.files.clone();
        self.address = address;
        self.options.root = root;
        self.files = files;
        if self.start_with(listener).await.is_err() {
            self.address = old_address;
            self.options.root = old_root;
            self.files = old_files;
            startup::save(&path, &previous)?;
            self.start().await?;
            self.setup_status(
                "failed",
                Some("Could not apply host setup. The previous share has been restored."),
            );
            tracing::warn!("Host setup failed; previous listener and sandbox restored");
        } else {
            self.setup_status(
                "ready",
                Some("Host setup saved. Sharing is ready; use a new QR code or link."),
            );
            tracing::info!("Host folder/network setup applied; old sessions revoked");
        }
        Ok(())
    }
    async fn selected_folder(&mut self, root: PathBuf) -> Result<(), Error> {
        let selected = root.clone();
        let files =
            tokio::task::spawn_blocking(move || splitshare_storage::Storage::open(&selected)).await;
        match files {
            Ok(Ok(storage)) => {
                self.reconfigure(
                    Some(root),
                    Some(splitshare_application::FileService::new(storage)),
                    self.address,
                )
                .await
            }
            _ => {
                self.setup_status("failed", Some("That folder cannot be shared. Choose an accessible folder without symlink redirects."));
                Ok(())
            }
        }
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
        let mut setup_commands = self.setup_commands.take().expect("Host setup receiver");
        let mut picker: Option<JoinHandle<Option<PathBuf>>> = None;
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
                result = async { picker.as_mut().unwrap().await }, if picker.is_some() => {
                    picker = None;
                    match result {
                        Ok(Some(root)) => self.selected_folder(root).await?,
                        _ => self.setup_status("cancelled", Some("No folder selected. Selection was cancelled or the desktop picker is unavailable. Headless hosts can use --root DIRECTORY.")),
                    }
                    self.publish(status, false, None);
                },
                Some(action) = setup_commands.recv(), if picker.is_none() => {
                    match action {
                        Action::ChooseFolder => {
                            self.publish(status, true, None);
                            picker = Some(tokio::spawn(async { tokio::time::timeout(Duration::from_secs(300), DesktopActions::choose_folder()).await.ok().flatten() }));
                        },
                        Action::Network(settings) => {
                            self.reconfigure(self.options.root.clone(), self.files.clone(), std::net::SocketAddrV4::new(settings.bind_ip, settings.port).into()).await?;
                            self.publish(status, false, None);
                        },
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
        if let Some(picker) = picker {
            picker.abort();
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
            bind_explicit: true,
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
    #[tokio::test]
    async fn folder_selection_replaces_sandbox_persists_and_revokes_sessions() {
        let (temp, mut host) = fixture().await;
        let old_link = host.share_link().unwrap();
        let next_root = temp.path().join("next-share");
        std::fs::create_dir(&next_root).unwrap();
        std::fs::write(next_root.join("selected.txt"), "selected folder").unwrap();
        host.selected_folder(next_root.clone()).await.unwrap();
        assert_eq!(host.control.snapshot().state, "ready");
        assert_ne!(host.share_link().unwrap(), old_link);
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        assert_eq!(
            client
                .get(format!(
                    "{}api/v1/files/download?path=/selected.txt",
                    host.local_url()
                ))
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap(),
            "selected folder"
        );
        let saved = startup::load(&temp.path().join("host.json")).unwrap();
        assert_eq!(saved.root.as_ref(), Some(&next_root));
        assert!(
            !serde_json::to_string(&host.control.snapshot())
                .unwrap()
                .contains(&temp.path().to_string_lossy().to_string())
        );
        host.stop().await.unwrap();
        let mut restart = options(next_root);
        restart.root = None;
        restart.bind_explicit = false;
        let mut restarted = Host::configured(restart, temp.path().join("config.json"))
            .await
            .unwrap();
        assert!(restarted.files.is_some());
        assert_eq!(restarted.address, host.address);
        restarted.stop().await.unwrap();
    }
    #[tokio::test]
    async fn network_changes_rebind_and_occupied_ports_preserve_active_share() {
        let (temp, mut host) = fixture().await;
        let occupied = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = host.address;
        let link = host.share_link().unwrap();
        host.reconfigure(
            host.options.root.clone(),
            host.files.clone(),
            occupied.local_addr().unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(host.control.snapshot().state, "failed");
        assert_eq!(host.address, address);
        assert_eq!(host.share_link().unwrap(), link);
        assert!(!temp.path().join("host.json").exists());
        let new_address = occupied.local_addr().unwrap();
        drop(occupied);
        host.reconfigure(host.options.root.clone(), host.files.clone(), new_address)
            .await
            .unwrap();
        assert_eq!(host.control.snapshot().state, "ready");
        assert_eq!(host.address, new_address);
        assert!(TcpListener::bind(address).await.is_ok());
        // The common local-only -> LAN change reuses the port and refreshes QR candidates.
        let lan = SocketAddr::from(([0, 0, 0, 0], new_address.port()));
        host.reconfigure(host.options.root.clone(), host.files.clone(), lan)
            .await
            .unwrap();
        assert_eq!(host.address, lan);
        assert_eq!(host.control.snapshot().state, "ready");
        assert!(
            host.running
                .as_ref()
                .unwrap()
                .state
                .candidates
                .iter()
                .any(|item| item.address == "127.0.0.1")
        );
        host.stop().await.unwrap();
    }
    #[tokio::test]
    async fn missing_saved_folder_keeps_setup_available() {
        let temp = tempfile::tempdir().unwrap();
        startup::save(
            &temp.path().join("host.json"),
            &Startup {
                root: Some(temp.path().join("missing")),
                bind: None,
            },
        )
        .unwrap();
        let mut opts = options(temp.path().to_owned());
        opts.root = None;
        let mut host = Host::configured(opts, temp.path().join("config.json"))
            .await
            .unwrap();
        assert!(host.files.is_none());
        assert!(
            host.control
                .snapshot()
                .message
                .unwrap()
                .contains("unavailable")
        );
        host.stop().await.unwrap();
    }
}

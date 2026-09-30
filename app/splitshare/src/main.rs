//! Native composition root; tray and transfer managers attach to this lifecycle later.
use std::time::Duration;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "splitshare=info".into()),
        )
        .init();
    let options = options()?;
    let config_path = splitshare_platform::config_path()?;
    let mut settings = splitshare_platform::load_or_create(&config_path)?;
    if let Some(limit) = options.parallel {
        settings.parallel_uploads_enabled = limit > 1;
        settings.max_parallel_uploads = limit;
        settings.validate()?;
    }
    if options.open_lan {
        settings.share_mode = splitshare_core::ShareMode::OpenLan;
    }
    let storage = options
        .root
        .as_deref()
        .map(splitshare_storage::Storage::open)
        .transpose()?;
    let files = storage.map(splitshare_application::FileService::new);
    let addresses = splitshare_network::LocalAddresses::discover()?;
    let listener = tokio::net::TcpListener::bind(options.bind).await?;
    let address = listener.local_addr()?;
    tracing::info!(%address, sharing = files.is_some(), share_mode = ?settings.share_mode, "SplitShare started");
    let shutdown = CancellationToken::new();
    let mut authorities = addresses.authorities(address.port());
    if !address.ip().is_unspecified() {
        authorities.push(address.to_string());
    }
    let mut state = splitshare_server::ServerState::new(
        files,
        settings,
        options.open_lan,
        addresses.clone(),
        authorities,
        shutdown.clone(),
    );
    state.candidates = addresses.candidates(address);
    state.settings_store = Some(std::sync::Arc::new(ConfigStore(config_path)));
    if options.dev {
        state.dev_origin = Some("http://127.0.0.1:5173".into());
    }
    let server = splitshare_server::serve_api(listener, state);
    tokio::pin!(server);
    tokio::select! {
        result = &mut server => result?,
        result = shutdown_signal() => {
            shutdown.cancel();
            result?;
            tokio::time::timeout(Duration::from_secs(10), &mut server).await??;
        }
    }
    tracing::info!("Server stopped");
    Ok(())
}

async fn shutdown_signal() -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        tokio::select! { result = tokio::signal::ctrl_c() => result, _ = terminate.recv() => Ok(()) }
    }
    #[cfg(not(unix))]
    tokio::signal::ctrl_c().await
}

struct Options {
    root: Option<std::path::PathBuf>,
    bind: std::net::SocketAddrV4,
    open_lan: bool,
    dev: bool,
    parallel: Option<u8>,
}
fn options() -> Result<Options, Box<dyn std::error::Error>> {
    let mut result = Options {
        root: None,
        bind: "127.0.0.1:8080".parse()?,
        open_lan: false,
        dev: false,
        parallel: None,
    };
    let mut args = std::env::args_os().skip(1);
    while let Some(argument) = args.next() {
        match argument.to_str() {
            Some("--root") => {
                result.root = Some(args.next().ok_or("--root needs a directory")?.into())
            }
            Some("--bind") => {
                result.bind = args
                    .next()
                    .ok_or("--bind needs an IPv4 address:port")?
                    .to_str()
                    .ok_or("Invalid bind address")?
                    .parse()?
            }
            Some("--parallel-uploads") => {
                result.parallel = Some(
                    args.next()
                        .ok_or("--parallel-uploads needs a limit (1–32)")?
                        .to_str()
                        .ok_or("Invalid upload limit")?
                        .parse()?,
                )
            }
            Some("--serial-uploads") => result.parallel = Some(1),
            Some("--open-lan") => result.open_lan = true,
            Some("--dev") => result.dev = true,
            Some("--help") => {
                println!(
                    "splitshare [--root DIRECTORY] [--bind IPv4:PORT] [--open-lan] [--parallel-uploads 1-32 | --serial-uploads] [--dev]\nLAN binding uses token links by default. --open-lan disables the token requirement. Use only trusted local/private networks."
                );
                std::process::exit(0);
            }
            _ => return Err("Unknown argument; use --help".into()),
        }
    }
    if result.dev && !result.bind.ip().is_loopback() {
        return Err("Development origin is allowed only for loopback binding".into());
    }
    Ok(result)
}

struct ConfigStore(std::path::PathBuf);
impl splitshare_application::sessions::SettingsStore for ConfigStore {
    fn save(
        &self,
        settings: &splitshare_core::HostSettings,
    ) -> Result<(), splitshare_application::sessions::AccessError> {
        splitshare_platform::save(&self.0, settings).map_err(|_| {
            tracing::error!("Unable to persist host settings");
            splitshare_application::sessions::AccessError::Save
        })
    }
}

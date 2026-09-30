//! Native composition root. The OS event loop stays on the main thread.
mod host;
use splitshare_platform::desktop::{self, Status};
use tokio::sync::{mpsc, watch};
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "splitshare=info".into()),
        )
        .init();
    let options = options()?;
    let no_tray = options.no_tray;
    let runtime = tokio::runtime::Runtime::new()?;
    let host = runtime.block_on(host::Host::new(options))?;
    let (commands, receiver) = mpsc::unbounded_channel();
    let (status, updates) = watch::channel(Status::default());
    let task = runtime.spawn(async move {
        let result = host.run(receiver, &status).await;
        status.send_modify(|value| value.exiting = true);
        result
    });
    if !no_tray {
        let tray = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            desktop::run(commands.clone(), updates)
        }));
        match tray {
            Ok(Ok(())) => {}
            Ok(Err(_)) | Err(_) => tracing::warn!(
                "Tray unavailable; server continues. Use --no-tray for headless operation"
            ),
        }
    }
    // Keep the command channel alive after tray failure; terminal signals still quit.
    let result = runtime.block_on(task)?;
    drop(commands);
    runtime.shutdown_timeout(std::time::Duration::from_secs(10));
    result
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
    no_tray: bool,
    open_browser: bool,
}
fn options() -> Result<Options, Box<dyn std::error::Error + Send + Sync>> {
    let mut result = Options {
        root: None,
        bind: "127.0.0.1:8080".parse()?,
        open_lan: false,
        dev: false,
        parallel: None,
        no_tray: false,
        open_browser: false,
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
            Some("--no-tray") => result.no_tray = true,
            Some("--open") => result.open_browser = true,
            Some("--dev") => result.dev = true,
            Some("--help") => {
                println!(
                    "splitshare [--root DIRECTORY] [--bind IPv4:PORT] [--open-lan] [--parallel-uploads 1-32 | --serial-uploads] [--dev] [--no-tray] [--open]\nLAN binding uses token links by default. --open-lan disables the token requirement. Use only trusted local/private networks."
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

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
    let settings = splitshare_platform::load_or_create(&splitshare_platform::config_path()?)?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    tracing::info!(address = %listener.local_addr()?, upload_limit = settings.effective_upload_limit()?, "Foundation server started; file sharing is not yet available");
    let shutdown = CancellationToken::new();
    let server = splitshare_server::serve(listener, shutdown.clone());
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

//! Native menu adapter. Host policy stays in the composition root.
use std::{
    path::Path,
    time::{Duration, Instant},
};
use tao::{
    event::Event,
    event_loop::{ControlFlow, EventLoopBuilder},
    platform::run_return::EventLoopExtRunReturn,
};
use tokio::sync::{mpsc, watch};
use tray_icon::{
    Icon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Open,
    OpenFolder,
    CopyLink,
    Toggle,
    Settings,
    Quit,
}
#[derive(Clone, Debug, Default)]
pub struct Status {
    pub listening: bool,
    pub configured: bool,
    pub busy: bool,
    pub exiting: bool,
    pub message: Option<String>,
}

#[derive(Default)]
pub struct DesktopActions {
    clipboard: Option<arboard::Clipboard>,
}
impl DesktopActions {
    pub fn copy(&mut self, text: String) -> Result<(), &'static str> {
        if self.clipboard.is_none() {
            self.clipboard = Some(arboard::Clipboard::new().map_err(|_| "Clipboard unavailable")?);
        }
        self.clipboard
            .as_mut()
            .unwrap()
            .set_text(text)
            .map_err(|_| "Unable to copy share link")
    }
    pub async fn open(target: impl Into<std::ffi::OsString>) -> Result<(), &'static str> {
        let target = target.into();
        let launch = tokio::task::spawn_blocking(move || open::that(target));
        match tokio::time::timeout(Duration::from_secs(3), launch).await {
            Ok(Ok(Ok(()))) => Ok(()),
            Ok(_) => Err("Unable to open the host application"),
            Err(_) => Err("Host application launch could not be confirmed"),
        }
    }
    pub async fn folder(path: &Path) -> Result<(), &'static str> {
        Self::open(path.as_os_str().to_owned()).await
    }
}
fn icon() -> Result<Icon, Box<dyn std::error::Error>> {
    let decoder = png::Decoder::new(std::io::Cursor::new(include_bytes!(
        "../../../docs/assets/logo.png"
    )));
    let mut reader = decoder.read_info()?;
    let mut rgba = vec![0; reader.output_buffer_size().ok_or("Invalid icon size")?];
    let info = reader.next_frame(&mut rgba)?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return Err("Expected RGBA branding icon".into());
    }
    rgba.truncate(info.buffer_size());
    Ok(Icon::from_rgba(rgba, info.width, info.height)?)
}

/// Runs on the OS main thread, returning on shutdown or tray/display failure.
/// The caller keeps its Tokio server alive if this adapter cannot initialize.
pub fn run(
    commands: mpsc::UnboundedSender<Command>,
    status: watch::Receiver<Status>,
) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    gtk::init().map_err(|_| "Native display unavailable; continuing without tray")?;
    let mut event_loop = EventLoopBuilder::<()>::new().build();
    #[cfg(target_os = "macos")]
    {
        use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
        event_loop.set_activation_policy(ActivationPolicy::Accessory);
    }
    // Wake GTK/Cocoa/Win32 explicitly when the async host changes. A native loop
    // with no windows must not rely on redraw events to observe shutdown.
    let proxy = event_loop.create_proxy();
    let awake = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let awake_worker = awake.clone();
    let wake = std::thread::spawn(move || {
        while awake_worker.load(std::sync::atomic::Ordering::Relaxed) {
            if proxy.send_event(()).is_err() {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    });
    let menu = Menu::new();
    let title = MenuItem::new("SplitShare", false, None);
    let state = MenuItem::new("Starting…", false, None);
    let open = MenuItem::new("Open SplitShare", true, None);
    let folder = MenuItem::new("Open shared folder", false, None);
    let copy = MenuItem::new("Copy share link", false, None);
    let toggle = MenuItem::new("Stop sharing", true, None);
    let settings = MenuItem::new("Settings", true, None);
    let quit = MenuItem::new("Quit", true, None);
    menu.append_items(&[
        &title,
        &PredefinedMenuItem::separator(),
        &state,
        &open,
        &folder,
        &copy,
        &toggle,
        &PredefinedMenuItem::separator(),
        &settings,
        &quit,
    ])?;
    let mut tray = None;
    let mut failure = None;
    let mut last = None;
    event_loop.run_return(|event, _, control| {
        *control = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(100));
        if let Event::NewEvents(tao::event::StartCause::Init) = event {
            match icon().and_then(|icon| {
                Ok(TrayIconBuilder::new()
                    .with_menu(Box::new(menu.clone()))
                    .with_icon(icon)
                    .with_tooltip("SplitShare")
                    .build()?)
            }) {
                Ok(value) => {
                    tray = Some(value);
                    tracing::info!("Native tray ready");
                }
                Err(_) => {
                    failure = Some("Native tray initialization failed");
                    *control = ControlFlow::Exit;
                    return;
                }
            }
        }
        let current = status.borrow().clone();
        if current.exiting || status.has_changed().is_err() {
            *control = ControlFlow::Exit;
            return;
        }
        let text = current.message.clone().unwrap_or_else(|| {
            if current.busy {
                "Stopping…"
            } else if current.listening && current.configured {
                "Sharing"
            } else if current.listening {
                "No shared folder"
            } else {
                "Stopped"
            }
            .into()
        });
        if last.as_ref() != Some(&text) {
            state.set_text(&text);
            last = Some(text);
        }
        toggle.set_text(if current.listening {
            "Stop sharing"
        } else {
            "Start sharing"
        });
        toggle.set_enabled(!current.busy);
        open.set_enabled(current.listening);
        settings.set_enabled(current.listening);
        copy.set_enabled(current.listening && current.configured && !current.busy);
        folder.set_enabled(current.configured);
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            let command = if event.id == open.id() {
                Command::Open
            } else if event.id == folder.id() {
                Command::OpenFolder
            } else if event.id == copy.id() {
                Command::CopyLink
            } else if event.id == toggle.id() {
                Command::Toggle
            } else if event.id == settings.id() {
                Command::Settings
            } else if event.id == quit.id() {
                Command::Quit
            } else {
                continue;
            };
            if commands.send(command).is_err() {
                *control = ControlFlow::Exit;
            }
        }
    });
    awake.store(false, std::sync::atomic::Ordering::Relaxed);
    let _ = wake.join();
    drop(tray);
    if let Some(error) = failure {
        Err(error.into())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn embedded_branding_is_a_valid_native_icon() {
        assert!(super::icon().is_ok());
    }
}

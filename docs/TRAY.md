# Desktop System Tray

## Principle

The tray is a native control surface, not a custom application window.

It follows operating-system menu conventions.

## Recommended menu

```text
SplitShare
──────────────
● Sharing / Stopped

Open SplitShare
Open shared folder
Copy share link

Start sharing / Stop sharing
──────────────
Settings
Quit
```

Optional entries may expose connected/active counts only if native menu APIs support the state cleanly.

Do not build a bespoke tray dashboard.

## Behavior

### Open SplitShare

Open the active local browser URL.

### Open shared folder

Reveal/open the selected root on the host OS.

### Copy share link

Copy the active tokenized join URL.

### Start / Stop

Controls server sharing state without exiting the process.

### Settings

Open the local WebUI directly to settings using a local client route/state.

### Quit

Initiate the same graceful shutdown used by terminal signals.

Wait for owned runtime tasks within a bounded timeout and ensure partial uploads are not published as completed files.

## Failure

Tray initialization failure must be logged and must not prevent headless/browser operation when the HTTP server can otherwise run.

Provide a `--no-tray` path for terminal/headless use.

## Phase 07 implementation checkpoint

The OS main thread owns a `tray-icon` native menu and a Tao event loop with no
window or webview. Tokio owns the listener and transfer tasks in the same process.
A typed channel carries menu commands to the composition root; status updates
return through a watch channel. A small wakeup thread keeps windowless native
loops responsive to host shutdown. No remote API can send these commands.

The menu implements Open SplitShare, Open shared folder, Copy share link,
Start/Stop sharing, Settings and Quit. It renders real listening/configured state,
disables unavailable actions and displays safe action errors. The embedded brand
PNG supplies the native tray icon. macOS uses accessory activation policy.

- Launch attempts the tray by default. `--no-tray` skips display initialization.
- `--open` opens the local browser once after listener startup. Otherwise opening
  the browser is an explicit tray action. There is no browser spawn on restart.
- Settings opens the local URL with `#settings`; Vue consumes that fragment only
  after the server confirms host-local access. Remote authorization is unchanged.
- Copy reads the current token/policy at click time, selecting the first ranked
  non-loopback candidate, or loopback when no other candidate is bound. Open LAN
  copies its base URL. Use the browser QR selector to choose another interface.
- Clipboard ownership stays alive for the process lifetime. Browser/folder opening
  uses the OS opener with one application-owned URL or host-selected root argument;
  no browser-controlled command or shell string is accepted. Opener exit failures
  are reported; a launch taking longer than three seconds is reported as unconfirmed,
  without killing the external desktop application.
- Stop cancels uploads/downloads/SSE and drains HTTP within ten seconds, closing the
  listener while keeping the process and opened sandbox capability alive. Browsers
  show disconnection until Start. Open/Settings/Copy are disabled while stopped.
- Start rediscovers interfaces, rebinds the same address/port (including an initially
  OS-assigned port), preserves the latest drained settings and creates fresh token
  sessions. Old links and cookies cannot survive restart. If binding fails, the
  host remains stopped and can retry after the port is released.
- Listener address/root are startup options, not editable WebUI settings. Current
  permission/concurrency changes apply live and need no listener restart.
- Quit and OS signals use the same cancellation/drain path. A drain timeout is
  fatal rather than starting a second listener over unfinished work.
- Display/menu initialization errors and Rust panics in the tray adapter fall back
  to the running server. Dynamic loader failure before `main` cannot be recovered;
  Linux desktop libraries must be installed even for this build's `--no-tray` mode.

`tray-icon` is pinned to 0.24.2: the planned 0.25.1 libappindicator feature requires
`dirs ^7`, unavailable from the configured registry during this implementation.
Tao 0.35.3, `open` and `arboard` provide native event-loop/opener/clipboard adapters;
none introduces the Tauri runtime. Linux uses GTK3/AppIndicator and an X11 clipboard
backend; Wayland-only clipboard behavior remains a device-validation gate.

Linux's real desktop menu is exercised by `scripts/smoke-tray-linux.py` through the
spawned process's DBus menu. Windows/macOS native smoke remains unverified; the
phase is not complete until that acceptance gate passes.

## Native release smoke record

| Platform | Native menu/lifecycle | Opener/clipboard | Evidence |
|---|---|---|---|
| Linux (GTK3, Cinnamon/X11) | PASS locally | Manual checks pending | `scripts/smoke-tray-linux.py`, actual DBus-exported menu and unfinished HTTP upload |
| Windows | NOT RUN | NOT RUN | Workspace/all-target cross-compile only; needs native host |
| macOS | NOT RUN | NOT RUN | Workspace/all-target cross-compile only; needs native host |

On each native release host, build the embedded app, start it with a temporary
`--root` and an unused `--bind` port, and record OS/desktop and binary revision.
Check the branded tray icon and ordinary OS menu; Open must launch the local page,
Settings must open its dialog, Open shared folder must open only the selected root,
and Copy must yield the current join URL (repeat after token rotation). Stop must
close the listener; Start must reuse its port with new links. Hold an upload body
open, select Quit and verify timely process exit, no completed partial file and no
hidden upload residue. Also check `--no-tray` and unavailable-display fallback where
applicable. Do not record a platform PASS from cross-compilation or emulation.

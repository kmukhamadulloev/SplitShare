# Host setup and sharing

## Start the current application

Build the embedded UI and native executable together:

```bash
bash scripts/build-release.sh
./target/release/splitshare --open
```

Exit an older running instance before starting the rebuilt executable. Opening
`prototype/splitshare-ui/index.html` only shows the design prototype, not the app.
The real app is served by Rust at `http://127.0.0.1:8080` on a fresh installation.
The tray's **Open SplitShare** action opens the current address if the port changed.

A fresh installation has no selected folder and listens only on this computer.
Previously saved host setup is restored on later launches. If an unconfigured
launch cannot bind its default/saved port because it is occupied, setup temporarily
uses a free loopback port and explains this in the wizard. Explicit `--bind` values
never silently change. The temporary port is not saved until setup completes.

1. The host browser automatically opens the mandatory **Set up SplitShare** wizard
   when no usable configured folder exists. Native desktop launches also open the
   browser automatically in this state; `--no-tray` remains headless unless `--open`
   is supplied. There is no Skip/Close action, Escape or backdrop dismissal.
2. **Folder:** choose a folder in the native picker. This stages an opened sandbox
   in native memory only; it is not published or persisted yet. Cancellation keeps
   the wizard open. Quitting remains available through the native tray/process.
3. **Connection:** choose This computer only or Local network (LAN / VPN), and a
   port (fresh default `8080`). Advanced exposes individual available interfaces.
   An explicit CLI bind or saved listener supplies the current port instead.
4. **Access:** choose Private link / QR (default) or explicit Open LAN, then Download
   only (default) or Upload and download. Rename/delete remain disabled for remote
   clients under these presets. Detailed controls stay in normal Settings.
5. **Start sharing** applies folder/network/access together. The new listener must
   start and configuration must save before setup is published as complete. Failures
   retain the previous listener and staged folder so values can be corrected/retried.
6. Success opens the sharing QR dialog. A changed address automatically navigates
   to the requested host URL, where the current server state is loaded. The native
   tray always opens the current actual address if browser navigation is interrupted.

Existing valid saved roots (and explicit valid `--root` configurations) bypass the
wizard. Missing saved roots require setup again. A staged folder is deliberately
not persisted: quitting before completion requires choosing it again. Remote
visitors receive only a setup-in-progress status and cannot configure the host or
use file/transfer/event APIs before completion.

After setup, **Settings → Network → Save changes** applies port/interface changes
without restarting the native process. Port validation happens on apply, not each
keystroke. Occupied ports preserve the active listener and entered values. Successful
network changes disconnect sessions/transfers and move the host browser to the new
address. Generate a fresh QR/link for remote devices afterward.

`127.0.0.1` works only on the host. The QR dialog explains this and provides a
**Network settings** button. With no shared folder, the mandatory wizard owns setup; normal QR/settings
controls are unavailable until completion. Discovery/render errors are visible,
and **Refresh QR** retries discovery and generation.

## Terminal / headless use

```bash
./target/release/splitshare --no-tray --root "/path/to/your/SplitShare" --bind 0.0.0.0:8080
```

Open `http://127.0.0.1:8080` on the host, then use its QR dialog for remote links.
The folder must already exist. `--root` and `--bind` override saved setup for that
launch; applying a GUI setup change saves the effective folder/listener. Do not
use `--open-lan` merely to enable network listening: it also removes the token
requirement. `--dev` remains loopback-only and does not inherit a saved LAN bind.

Native selection requires a desktop session. Linux uses the XDG desktop portal
through `rfd` with Tokio; cancellation or an unavailable picker leaves the existing
share unchanged and shows a message. Headless hosts can select the root with
`--root`. Windows/macOS native picker verification remains user-owned.

## Implementation and boundaries

The application `HostControl` exposes a bounded command channel and path-free
setup snapshots. Host-only HTTP adapters enqueue commands; the composition root
owns native picker invocation, sandbox construction, listener draining/rebinding,
session invalidation and persistence. A 202 response means accepted, not applied;
the UI polls setup state and never reports a requested change as completed merely
because it was queued. Only one setup operation can be pending.

The native dialog returns a path directly to Rust. A newly selected root is opened
through the existing Storage sandbox. Network-only changes retain the already
opened capability. Separate ports are reserved before disrupting the old listener;
overlapping binds require stopping/rebinding. Bind or persistence failures restore the previous
share/configuration; a drain/rollback failure is fatal rather than continuing with
uncertain workers. The browser observes the old control endpoint for success/failure. When a changed
origin stops responding, it navigates to the requested address; this navigation is
not a success assertion. The destination reloads actual status. The tray provides
recovery if navigation cannot complete. No cross-origin API/CORS exception is added.

`host.json`, adjacent to the existing application-data `config.json`, stores the
version-1 native root/listener configuration. Writes use private temporary files,
fsync and atomic replacement. This native-only file is never served by HTTP;
setup DTOs contain only a folder-selected flag, interface choices, listener address,
state and safe messages. A missing saved folder starts an unconfigured host with
a working setup screen instead of preventing startup.

All setup endpoints require socket-derived host locality, allowed Host/Origin and
mutation headers. Remote sessions cannot trigger the native picker or alter the
listener. Forwarding headers and frontend booleans do not grant access.


The presence of a usable persisted root is the completion record; there is no
separate frontend flag that can unlock sharing. During first-run selection/apply,
HostControl keeps `setup_required` true and the server gates file/transfer/SSE
requests. Folder and listener are saved to `host.json` only after listener startup;
access settings save to `config.json` first. These are individual atomic writes,
not a cross-file transaction: interruption before the final root save leaves setup
incomplete. Normal failures roll back both settings and listener; rollback failure
is fatal rather than continuing with uncertain configuration.

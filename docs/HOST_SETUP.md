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
Previously saved host setup is restored on later launches.

1. On the host, click **Set up sharing**, or the settings icon → **general**.
2. Click **Choose shared folder**. In the native dialog, select the existing folder
   you want to share (for example, your `SplitShare` folder). The browser never
   receives the native filesystem path. Selection applies immediately and is saved.
3. In **network**, select **All interfaces (LAN / VPN)**, choose a free port
   (normally `8080`), and click **Apply network settings**. **Save changes** also
   applies edited network fields. Token protection remains enabled by default.
4. If the address/port changed, use **Open updated address** after reconnection is
   attempted, or reopen the browser from the tray. Network/root changes disconnect
   current sessions and cancel unfinished transfers; generate a fresh share link.
5. Click the QR icon. Select the reachable LAN/VPN address for the receiving
   device. Scan the QR code or copy that link. Both devices must be on a reachable
   trusted local/private network; host firewall policy still applies.

`127.0.0.1` works only on the host. The QR dialog explains this and provides a
**Network settings** button. With no shared folder, QR opens folder setup instead
of displaying a disabled or blank control. Discovery/render errors are visible,
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
overlapping binds require stopping/rebinding. Bind failures restore the previous
share/configuration; a drain/rollback failure is fatal rather than continuing with
uncertain workers. The browser reports an unconfirmed connection change if it
cannot reach the control endpoint, and offers the requested address.

`host.json`, adjacent to the existing application-data `config.json`, stores the
version-1 native root/listener configuration. Writes use private temporary files,
fsync and atomic replacement. This native-only file is never served by HTTP;
setup DTOs contain only a folder-selected flag, interface choices, listener address,
state and safe messages. A missing saved folder starts an unconfigured host with
a working setup screen instead of preventing startup.

All setup endpoints require socket-derived host locality, allowed Host/Origin and
mutation headers. Remote sessions cannot trigger the native picker or alter the
listener. Forwarding headers and frontend booleans do not grant access.

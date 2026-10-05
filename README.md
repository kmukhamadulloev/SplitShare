# SplitShare

SplitShare is a local-first, cross-platform folder sharing application.

Current version: **0.1.0** — development release checkpoint. See
[release notes](RELEASE.md) for implemented features, validation and limitations.

One device runs SplitShare and exposes a selected folder over the local network. Other devices do **not** need SplitShare installed: they open the host address in a browser, browse the shared virtual filesystem, download files, upload files, create folders, rename items, paste clipboard content, and use the interface from desktop or mobile.

**No cloud · No accounts · No external server · No mandatory internet · No Tauri · No Electron**

## Product model

```text
Host PC
┌─────────────────────────────────────┐
│ SplitShare native Rust process      │
│                                     │
│ Rust Core                           │
│ ├─ filesystem sandbox               │
│ ├─ sessions / permissions           │
│ ├─ transfer manager                 │
│ ├─ network interface selection      │
│ └─ host configuration               │
│                                     │
│ Axum HTTP                           │
│ ├─ REST API                         │
│ ├─ SSE                              │
│ ├─ upload/download streaming        │
│ ├─ HTTP Range                       │
│ └─ embedded Vue production UI       │
│                                     │
│ Native system tray                  │
└──────────────────┬──────────────────┘
                   │ LAN / hotspot
           ┌───────┴────────┐
           │                │
      Browser           Browser
      phone             laptop
```

The desktop application owns the server and native system-tray lifecycle. The browser UI is the remote file manager.

## Main capabilities

- Share one explicitly selected root folder.
- Default port `8080`, configurable by the host.
- Connect using URL or QR code.
- Responsive browser UI with List and Grid views.
- Custom desktop context menu and mobile action sheet.
- Upload and download files without installing a client.
- Multi-upload queue with host-controlled parallelism.
- Drag and drop.
- Clipboard paste:
  - files -> upload;
  - images -> upload after preview/confirmation;
  - text -> draft, then save as a file.
- Folder creation, rename and delete when permitted.
- File preview for safe browser-compatible formats.
- HTTP Range download/stream support.
- Virtual paths only; real host filesystem paths never leave the server.
- Token-protected share links by default, with optional open-LAN mode.
- Native system tray on Windows, Linux and macOS.
- Production Vue assets embedded into the Rust desktop binary.

## Monorepo

```text
SplitShare/
├── app/
│   └── splitshare/            # native desktop composition root
├── crates/
│   ├── splitshare-core/       # domain types and application rules
│   ├── splitshare-storage/    # sandboxed virtual filesystem
│   ├── splitshare-network/    # interfaces, URLs, LAN selection
│   ├── splitshare-server/     # Axum, API, SSE, embedded WebUI
│   └── splitshare-platform/   # tray + OS-specific adapters
├── web/                       # Vue 3 production WebUI
├── prototype/
│   └── splitshare-ui/         # visual source of truth
├── docs/
├── goals/
├── skills/
├── scripts/
└── .github/workflows/
```

Backend and frontend live in the same repository and release together.

## Mandatory stack

### Backend

- Rust
- Tokio
- Axum
- Tower / Tower HTTP where useful
- Serde
- tracing
- native filesystem I/O with streaming
- native tray integration

### Frontend

- Vue 3
- TypeScript
- Vite
- Tailwind CSS
- Pinia
- `@lucide/vue`
- same-origin relative `/api/...` calls

### Realtime

SSE for server-to-browser state changes. Commands remain ordinary HTTP requests.

### Persistence

SplitShare v1.0 does **not** require a database.

- durable host configuration -> application-data config file;
- active sessions/transfers -> in memory;
- shared files -> selected filesystem root.

A database may be added only by an explicit later goal that requires persistent history or another durable domain model.

## Use SplitShare

Run the rebuilt native app with `./target/release/splitshare --open`. First launch
opens mandatory setup: choose a folder in the native dialog, select the network
interface and port (default **8080**), then choose access policy and start sharing.
Sharing stays disabled until setup completes. Open the QR dialog and select a
reachable LAN address for the other device. Folder/listener choices persist;
subsequent port changes apply without restarting the native process.
See [host setup and troubleshooting](docs/HOST_SETUP.md) for complete steps and
terminal/headless commands.

To build a verified Linux archive candidate, run `python3 scripts/package-linux.py`.
The archive and SHA-256 checksum are written under `release/`. See
[packaging prerequisites and limits](docs/PACKAGING.md).

## Development

Phase 03 supports real HTTP browsing, file mutations, streamed downloads/Range,
SSE and the initial file-manager UI. Phase 04 adds streamed uploads, an enforced
parallel queue, cancellation/retry and clipboard workflows. Phase 05 adds token
sessions, remote permissions, persisted host settings and local QR sharing. Phase 06
adds the responsive production file manager, typed icons, selection actions,
accessible menus/dialogs and reconnect recovery. Phase 07 adds a native tray and
Start/Stop/Quit lifecycle (Windows/macOS native smoke is user-owned follow-up).
Phase 08 adds transport deadlines, bounded connections, truthful uncertain-upload
recovery and measured directory paging. See [running and access policy](docs/HTTP_BROWSER.md).
See [foundation decisions](docs/FOUNDATION.md). Rust 1.98+ and Node 24+ are required for development.

```bash
npm ci --prefix web
./scripts/dev.sh --root /path/to/share --parallel-uploads 2
```

Vite opens at http://127.0.0.1:5173; Rust serves embedded assets at
http://127.0.0.1:8080. Ctrl+C stops development processes.

For a standalone binary: `./scripts/build-release.sh`, then run
`./target/release/splitshare --root /path/to/share`. Node is not required at runtime.
For LAN sharing, add `--bind 0.0.0.0:8080`, then open **Share with QR** on the host
and select a reachable address. Token links are the default; `--open-lan` explicitly
allows reachable clients without a token. Host settings control remote permissions
and upload limits. See [session and QR policy](docs/SESSIONS.md).

Frontend:

```bash
cd web
npm install
npm run dev
```

Repository checks:

```bash
./scripts/check.sh
```

The production release build must first build `web/`, then embed `web/dist/` into the Rust binary.

## Source of truth

Implementation agents must read:

1. `AGENTS.md`
2. `GOAL.md`
3. the active file under `goals/`
4. relevant subsystem documentation under `docs/`
5. `prototype/splitshare-ui/index.html` for visual/interaction behavior

## Current scope

The current release program targets the **desktop host + browser client** product for Windows, Linux and macOS.

A future mobile host may reuse the Rust core as a native library and use a thin platform shell, but mobile-host implementation is intentionally outside the current v1.0 phases.

## License

MIT.

Desktop startup attempts a native tray. Use `--no-tray` for terminal/headless use
and `--open` to open the browser at startup. Linux builds need GTK3/AppIndicator
packages; see [tray behavior](docs/TRAY.md) and [packaging prerequisites](docs/PACKAGING.md).

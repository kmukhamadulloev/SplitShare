# SplitShare

SplitShare is a local-first, cross-platform folder sharing application.

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

## Development

Phase 03 supports real HTTP browsing, file mutations, streamed downloads/Range,
SSE and the initial file-manager UI. Phase 04 adds streamed uploads, an enforced
parallel queue, cancellation/retry and clipboard workflows. See [running and access policy](docs/HTTP_BROWSER.md).
See [foundation decisions](docs/FOUNDATION.md). Rust 1.98+ and Node 24+ are required for development.

```bash
npm ci --prefix web
./scripts/dev.sh --root /path/to/share --parallel-uploads 2
```

Vite opens at http://127.0.0.1:5173; Rust serves embedded assets at
http://127.0.0.1:8080. Ctrl+C stops development processes.

For a standalone binary: `./scripts/build-release.sh`, then run
`./target/release/splitshare --root /path/to/share`. Node is not required at runtime.
LAN access requires explicit `--bind 0.0.0.0:8080 --open-lan`; token sessions are Phase 05.

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

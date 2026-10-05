# Architecture

## System overview

```text
Desktop Browser / Phone / Tablet
              │
       HTTP / SSE / Range
              ▼
┌──────────────────────────────────────┐
│ SplitShare — Rust                   │
│ ├── Axum HTTP server                │
│ ├── REST API                        │
│ ├── SSE                             │
│ ├── embedded Vue production UI      │
│ ├── SessionManager                  │
│ ├── TransferManager                 │
│ ├── Storage service                 │
│ ├── Network service                 │
│ └── native desktop tray             │
└──────────────────────────────────────┘
              │
              ▼
      selected share root
```

## Workspace ownership

### `splitshare-core`

Owns domain types and application rules:

- `ShareMode`;
- `PermissionSet`;
- `ShareSession`;
- `ClientSession`;
- `FileEntry`;
- `VirtualPath`;
- `TransferId`;
- `TransferState`;
- `TransferProgress`;
- `HostSettings`;
- typed domain errors.

Must not depend on Axum, Vue or native tray code.

### `splitshare-application`

Owns FileService orchestration, bounded blocking filesystem jobs and mutation event
publication. Depends on core/storage/Tokio; never on Axum. Server calls this layer.

### `splitshare-storage`

Owns:

- virtual path parsing;
- root sandbox enforcement;
- directory listing;
- streaming reads;
- temporary upload creation;
- atomic publish;
- mkdir;
- rename;
- delete;
- conflict detection;
- metadata lookup;
- symlink/reparse-point rejection.

This crate is the main filesystem security boundary.

### `splitshare-network`

Owns:

- interface enumeration;
- loopback/private/link-local classification;
- virtual/VPN adapter metadata where detectable;
- candidate share URLs;
- interface ranking;
- bind compatibility;
- host-address classification.

Do not hide valid interfaces from the host merely because they are VPN interfaces. The UI may rank them lower than physical LAN/hotspot addresses.

### `splitshare-server`

Owns:

- Axum router;
- request/response mapping;
- SSE;
- share-session cookie handling;
- embedded frontend serving;
- streaming uploads;
- Range download/preview responses;
- API error envelopes;
- request limits/timeouts.

It calls application services. It does not perform ad-hoc filesystem access.

### `splitshare-platform`

Owns desktop-only adapters:

- native system tray;
- open browser;
- reveal shared folder;
- application data/config paths;
- platform startup details if an approved goal adds start-with-system;
- platform-specific icon packaging helpers where useful.

No custom desktop UI framework is allowed.

### `app/splitshare`

Composition root:

- tracing;
- config load/migration;
- storage root construction;
- managers/services;
- server listener;
- tray;
- graceful shutdown;
- browser opening policy.

## Frontend

Production stack:

- Vue 3;
- TypeScript;
- Vite;
- Tailwind CSS;
- Pinia;
- `@lucide/vue`.

Suggested structure:

```text
web/src/
├── app/
│   ├── api/
│   ├── events/
│   └── stores/
├── components/
│   ├── ui/
│   ├── files/
│   ├── transfers/
│   └── modals/
├── composables/
├── views/
├── types/
└── assets/
```

## Production asset embedding

Development uses Vite.

Release flow:

```text
npm run build
    ↓
web/dist/
    ↓
Rust compile/package step
    ↓
embedded static assets
    ↓
single SplitShare desktop runtime
```

Node.js is not a release runtime dependency.

SPA fallback may serve `index.html` for client routes, but must never intercept `/api/*` or `/j/*`.

## Realtime

Use SSE.

```text
GET /api/v1/events
```

Events include:

- filesystem changed;
- transfer created/updated/completed/failed;
- permissions/settings changed where client-visible;
- server/listener state changed if the connection remains valid.

Commands remain ordinary HTTP requests.

## Host and remote-client separation

The same WebUI may be opened on the host and remotely.

The backend determines whether a request is host-local using the actual socket peer address plus current local interface set. Frontend-provided locality flags are informational only.

Host-only settings APIs reject remote clients.

## No database in the current implementation

Configuration persistence should use a versioned application-data config file.

Sessions and active transfers are in-memory runtime state.

If persistent history becomes a future requirement, add it through an explicit migration-bearing persistence goal rather than preemptively introducing SQLite.

## Implemented foundation

See [Phase 01 decisions](FOUNDATION.md) for historical dependency and configuration decisions. Current subsystem documents
and the implementation take precedence over that initial checkpoint.

## Implemented storage boundary

Phase 02 adds validated `VirtualPath`, `FileEntry`, `StorageError` and
`ConflictPolicy` in core without HTTP or filesystem dependencies. Storage owns
native capabilities, filesystem resolution, readers and temporary upload lifetimes.
Read handles are seekable; upload chunks are bounded to 64 KiB. Phase 03 exposes it through FileService and typed HTTP routes. See [STORAGE.md](STORAGE.md) for exact behavior and limits.

## HTTP/browser milestone

See [HTTP_BROWSER.md](HTTP_BROWSER.md) for implemented routes, security boundary,
streaming, SSE, local classification and browser state.

Phase 04 adds the application TransferManager and raw streaming HTTP uploads,
transfer snapshots/cancellation and SSE progress. See [TRANSFERS.md](TRANSFERS.md)
and [API.md](API.md) for the implemented contract and startup host policy.

Phase 05 adds application SessionManager and SettingsStore, core PermissionSet,
host-only HTTP adapters, native atomic settings persistence and typed IPv4 network
candidates. Policy is independent of Axum; HTTP maps routes to capabilities and
passes socket-derived locality. Vue renders bundled QR images and host settings.
See [SESSIONS.md](SESSIONS.md) for policy and stream invalidation boundaries.

Phase 07 adds native desktop adapters in `splitshare-platform::desktop`. The
composition root's Host controller owns listener start/stop, preserved sandbox
capabilities, settings retention and shutdown. A typed in-process command channel
keeps desktop controls separate from HTTP authorization. See [TRAY.md](TRAY.md).


Host setup uses application `HostControl` snapshots and a bounded command channel.
HTTP never receives or returns native root paths. The composition root opens the
native picker through the platform adapter, reconfigures listeners/sandboxes and
persists native-only startup settings. See [HOST_SETUP.md](HOST_SETUP.md).

Host diagnostics use an application-owned bounded history/control service. The
native composition root installs the tracing capture/reload adapter and shares
one service across listener restarts. Capture includes only approved messages and
structured fields. Axum provides host-only snapshot/config/clear routes and SSE
notifications using the existing stream limit. Vue consumes this service through
Settings and a log modal; it never reads native log files or shell output.

Initial setup stages a native sandbox in Host until an application `InitialSetup`
command supplies validated listener and sharing policy. `HostControl.setup_required`
keeps file/transfer access gated while the listener starts and configuration is
persisted. Only the final ready snapshot enables sharing. The same native process
owns rebind and rollback. A default-port collision on an unconfigured launch may
use an unsaved temporary loopback listener so the mandatory wizard remains reachable;
explicit binds and already configured hosts do not silently change ports.

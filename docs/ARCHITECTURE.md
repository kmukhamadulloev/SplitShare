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

## No database in v1

Configuration persistence should use a versioned application-data config file.

Sessions and active transfers are in-memory runtime state.

If persistent history becomes a future requirement, add it through an explicit migration-bearing persistence goal rather than preemptively introducing SQLite.

## Implemented foundation

See [Phase 01 decisions](FOUNDATION.md) for dependencies, configuration, paths,
asset serving and lifecycle ownership. Later sections above describe target architecture.

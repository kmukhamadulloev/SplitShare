# SplitShare Agent Protocol

This file is authoritative for implementation agents.

## Product identity

SplitShare is a local-first folder sharing application.

A host runs one native Rust process. Remote users connect through a normal browser over LAN, Ethernet, VPN, or a local hotspot.

It is NOT:

- a cloud file hosting service;
- a Dropbox/Google Drive replacement;
- an internet-facing public file server;
- a multi-tenant SaaS;
- an account system;
- a synchronization daemon;
- a WebRTC peer-to-peer product;
- a Tauri/Electron application.

## Mandatory architecture

Use:

- Rust as the only runtime backend;
- Tokio;
- Axum;
- Vue 3 + TypeScript;
- Vite;
- Tailwind CSS;
- Pinia;
- `@lucide/vue` for application icons;
- SSE for server-to-browser realtime events;
- embedded production frontend assets served by Rust;
- a native desktop system tray owned by Rust;
- a sandboxed virtual filesystem rooted at one host-selected folder;
- HTTP streaming and Range support for downloads/previews.

The same Vue build must work on localhost and LAN. Use relative `/api/...` URLs.

## Native host rule

Desktop SplitShare is one native Rust process.

The system tray is a thin lifecycle/control adapter. It is not a second frontend and must not become a custom dashboard.

Do not introduce Tauri or Electron to implement tray behavior.

## Forbidden unless an explicitly approved goal requires it

Do NOT introduce:

- Tauri;
- Electron;
- Node.js backend;
- PHP/Laravel backend;
- nginx or Caddy as a runtime dependency;
- Redis;
- PostgreSQL/MySQL;
- cloud storage;
- external SaaS dependency;
- user accounts;
- OAuth;
- public tunnels;
- router auto-configuration;
- WebRTC;
- mandatory Docker runtime;
- arbitrary shell command execution;
- arbitrary filesystem access outside the selected share root.

Node.js is allowed only for frontend development/build tooling.

## Storage boundary

The shared folder is a security sandbox.

Browser clients must use virtual paths relative to `/`.

Never serialize:

- absolute filesystem paths;
- application-data paths;
- OS usernames;
- unrelated directory structure.

Path traversal, symlink escape and absolute-path access are release-blocking security bugs.

Symlinks are denied by default in the current implementation.

## Transfer rule

Never buffer whole files in memory.

Upload:

```text
HTTP body stream
  -> bounded chunks
  -> temporary file inside destination filesystem
  -> validation / conflict policy
  -> atomic publish
```

Download:

```text
file
  -> bounded read stream
  -> HTTP
```

Support valid Range requests.

Partial uploads must never appear as completed files.

## Multi-upload rule

Parallel upload is host policy, not merely frontend behavior.

The frontend may schedule several requests concurrently, but the backend must enforce the configured limit as well.

When disabled, effective concurrency is one.

## Security model

Default share access uses an opaque cryptographically random link token.

Preferred join flow:

```text
GET /j/<token>
  -> validate token
  -> issue same-origin HttpOnly session cookie
  -> redirect /
```

Optional Open LAN mode may be enabled by the host.

No public-internet security claim is permitted. Documentation must state that SplitShare is designed for trusted local/private networks.

## Host-only operations

Host configuration is not automatically writable by remote share clients.

Host-only actions must be authorized using the connection's local-host classification and application policy. Never trust a frontend boolean that says a browser is local.

## UI rule

`prototype/splitshare-ui/index.html` is the visual and interaction source of truth.

Preserve:

- full-screen file-manager layout;
- dark navy palette;
- SplitShare purple/indigo/cyan branding;
- List and Grid modes;
- compact toolbar;
- icon-only secondary controls where designed;
- desktop custom context menu;
- mobile action sheet;
- responsive behavior;
- aggregate transfer progress;
- modal blur/fade behavior;
- QR/settings/create/delete/queue flows.

Do not redesign it into a generic SaaS dashboard or marketing page.

## Required separation

Never put filesystem/process policy directly in HTTP route handlers.

```text
HTTP
  ↓
Application/Core service
  ↓
TransferManager / SessionManager / Storage service
  ↓
Adapter
  ↓
OS filesystem / network / tray
```

Core domain code must not depend on Axum or Vue.

## No fake production implementation

The following do NOT count as completed implementation:

- hardcoded API success payloads;
- fake transfer progress;
- fake connected-client counts;
- static arrays pretending to be server state;
- TODO route handlers presented as finished;
- swallowed errors;
- UI buttons that look functional but do nothing when the active goal requires behavior.

Mocks/fakes are allowed in tests and fixtures only.

## Autonomy

Do not ask for confirmation for routine engineering choices already constrained by repository documentation.

If several valid approaches exist, choose the simplest implementation consistent with:

1. security boundary;
2. architecture;
3. current goal;
4. visual source of truth.

Stop only when a decision materially changes product scope, network/security model, mandatory stack, or compatibility.

## Work loop

Before coding:

1. Read `GOAL.md`.
2. Read the current user-assigned scope and any active goal referenced by `GOAL.md`.
   Archived phases are historical context, not work assignments.
3. Read `docs/ARCHITECTURE.md`.
4. Read subsystem docs relevant to the task.
5. Inspect existing code.
6. Inspect `ISSUES.md`.
7. Identify acceptance criteria.

After coding:

1. format;
2. lint;
3. run relevant Rust unit tests;
4. run Rust integration tests;
5. run frontend type/build tests;
6. run browser E2E where applicable;
7. run security regression tests where applicable;
8. update docs;
9. update `ISSUES.md`;
10. commit the completed work according to the Git rule below;
11. report each acceptance criterion individually.

## Git rule

After completing a feature, phase, fix, or documentation change, automatically
create a local Git commit once all applicable validation passes. No additional
confirmation is required.

Include all changes belonging to that completed work: implementation, tests,
documentation, configuration and lockfiles. Review the diff and stage explicit
paths; preserve unrelated or unfinished workspace changes. Never commit secrets,
generated caches or build artifacts.

Use a clear commit message describing the completed change. Do not present work
with failing mandatory checks as complete. Report the commit hash, validation
results and any remaining uncommitted changes in the completion report.

Push only when explicitly requested by the user.

## Definition of Done

A feature is complete only when applicable items exist:

- real implementation;
- frontend integration;
- API contract;
- errors;
- logging;
- tests;
- security validation;
- documentation;
- passing acceptance criteria.

"Compiles" is not Definition of Done.

## Completion report

Always report:

- Implemented
- Exact tests executed and results
- Acceptance criteria: PASS / FAIL / NOT APPLICABLE
- Remaining real issues/blockers

Never claim completion with known failing mandatory criteria.

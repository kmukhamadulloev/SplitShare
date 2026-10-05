# Phase 05 — Sessions, Permissions and QR

> Archived historical plan. Superseded as a work assignment; see [archive status](README.md).

## Objective

Implement private-network access control and share capabilities.

## Deliverables

- cryptographic share token;
- `/j/<token>` join flow;
- HttpOnly session cookie;
- token rotation;
- optional Open LAN mode;
- capability model:
  - browse
  - download
  - upload
  - create directory
  - rename
  - delete
- host-only settings authorization;
- network address candidates;
- QR generation in browser using local code/library;
- share URL selection.

## Acceptance

- invalid token cannot join;
- rotated token invalidates old joins according to documented session policy;
- remote browser cannot modify host settings;
- permission-disabled actions are rejected by backend even if manually requested;
- QR requires no external service;
- correct LAN URL can be selected when multiple adapters exist.

## Phase 05 local acceptance — 2026-09-30

| Criterion | Result | Evidence |
|---|---|---|
| Invalid token cannot join | PASS | Application and HTTP invalid-token tests; 401, no cookie issued |
| Rotation invalidates old joins under documented policy | PASS | Old links/cookies rejected; active upload/download/SSE revoked; restart smoke; policy in SESSIONS.md |
| Remote browser cannot modify host settings | PASS | Router tests with remote socket peers, valid cookies and forged locality headers; all host endpoints return 403 |
| Disabled actions rejected by backend | PASS | Independent checks for all six capabilities, GET/HEAD coverage, Open LAN enforcement |
| QR uses no external service | PASS | Bundled local renderer; desktop/mobile tests decode canvas pixels and assert no external requests |
| Correct LAN URL selectable with multiple adapters | PASS | Multi-adapter network fixtures verify ranking/bind compatibility; browser selects discovered address options and verifies link/QR |

Exact validation:

- `bash scripts/check.sh`: PASS — formatting, Clippy with warnings denied, 47 Rust
  tests, frontend aggregation tests, typecheck and production build.
- `cargo build --locked -p splitshare`: PASS — embedded debug server for browser tests.
- `npm run test:e2e --prefix web`: PASS — eight desktop/mobile Chromium tests.
- `bash scripts/build-release.sh`: PASS.
- `python3 scripts/smoke-foundation.py`: PASS — standalone assets, safe unconfigured
  routes, configuration creation, shutdown and log redaction.
- `python3 scripts/smoke-sessions.py`: PASS — persistence across restart, new token,
  old join rejection, cookie redirect and no secrets in logs/config.
- `python3 scripts/test-upload-memory.py`: PASS — 8 MiB / 256 MiB uploads at
  7.70 / 8.70 MiB peak server RSS in the final local run.
- `cargo check --locked --workspace --all-targets --target x86_64-pc-windows-msvc`:
  PASS (compile check, not native runtime).
- `cargo check --locked --workspace --all-targets --target x86_64-apple-darwin`:
  PASS (compile check, not native runtime).
- OpenAPI YAML/local reference validation and `git diff --check`: PASS.

Remote authorization tests inject socket peers; browser tests run against the real
native server on this machine. Native Windows/macOS runtime, physical-device QR
scanning, LAN/VPN reachability and Firefox/Safari validation remain outstanding.
Local phase acceptance does not claim completed v1 release acceptance.

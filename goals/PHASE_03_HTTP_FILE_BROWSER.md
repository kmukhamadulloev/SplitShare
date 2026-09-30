# Phase 03 — HTTP and File Browsing

## Objective

Expose the real storage sandbox through a typed Axum API and embedded browser client foundation.

## Deliverables

- `/api/v1/status`;
- list directory;
- create directory;
- rename;
- delete;
- download;
- HTTP Range;
- typed errors;
- SSE infrastructure;
- embedded Vue SPA fallback;
- host-local classification;
- initial browser store/API layer.

## Acceptance

- remote browser lists real temporary test files;
- file paths are virtual;
- Range behavior is standards-correct for supported cases;
- malformed Range returns 416;
- routes remain thin;
- integration tests use real Axum + temporary filesystem.


## Implementation evidence — 2026-09-30

| Acceptance | Result |
|---|---|
| Browser client lists real temporary files over HTTP | PASS: Chromium desktop/mobile against the Rust process, no mocked APIs |
| File paths are virtual | PASS: HTTP traversal/link/redaction tests and domain DTOs |
| Range correct for supported cases | PASS: bounded/open/suffix/clamped/empty/HEAD/If-Range tests |
| Malformed Range returns 416 | PASS: invalid syntax, unsatisfiable, overflow and multi-range cases |
| Routes remain thin | PASS: filesystem work and mutation events owned by FileService; HTTP owns only transport mapping/streaming |
| Integration tests use real Axum + temporary filesystem | PASS: real TCP/reqwest tests and Playwright server fixtures |

Validation: `bash scripts/check.sh` (29 Rust tests), `npm run test:e2e --prefix web`
(2 desktop/mobile tests), full-workspace Windows/macOS cross-target checks,
release build/smoke and a real Vite proxy GET/mutation check all pass on Linux.
Exact commands are recorded in `docs/ACCEPTANCE.md`.

Native Windows/macOS runtime and separate-device LAN verification remain pending.
Phase 03 runs local-only by default; the host explicitly opts into Open LAN.
Token sessions and granular permissions remain Phase 05. External native edits
require Refresh until a filesystem watcher exists. See `docs/HTTP_BROWSER.md`.

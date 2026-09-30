# Acceptance Matrix

This file accumulates verified release evidence.

## Phase 01

- [x] Workspace structure created and documented.
- [x] Rust workspace checks pass on Linux.
- [x] Frontend install/typecheck/build.
- [x] CI matrix present (native results pending).
- [x] Prototype and branding assets inspected; PNG equality checked.
- [x] Active-goal/agent workflow verified.

## Storage — Phase 02

- [x] Domain DTOs/errors do not expose absolute paths (serialization regression).
- [x] Traversal, encoding, separators, device names and serde bypass rejected.
- [x] Linux symlink escape, root boundaries and concurrent link swaps rejected.
- [x] Upload partial files hidden; Drop/cancel/failed-length cleanup verified.
- [x] Ask/reject/replace/auto-rename policies and concurrent publication tested.
- [x] Real temporary filesystem list/metadata/mkdir/rename/delete/read tested.
- [x] 16 MiB upload/read using 64 KiB chunks verified.
- [x] Native platform link tests exist; Windows/macOS code and tests cross-compile.
- [ ] Native Windows/macOS runtime link/reparse and atomic rename verification.
- [ ] Network filesystem semantics verified.

### Evidence — 2026-09-30, Linux / Rust 1.98

- `bash scripts/check.sh`: PASS — frontend typecheck/build, `cargo fmt --check`,
  `cargo clippy --locked --workspace --all-targets -- -D warnings`,
  `cargo test --locked --workspace` (19 tests: 5 unit, 14 integration).
- `cargo check --locked -p splitshare-storage --tests --target x86_64-pc-windows-msvc`: PASS.
- `cargo check --locked -p splitshare-storage --tests --target x86_64-apple-darwin`: PASS.
- `cargo clippy --locked -p splitshare-storage --tests --target x86_64-pc-windows-msvc -- -D warnings`: PASS.
- `cargo clippy --locked -p splitshare-storage --tests --target x86_64-apple-darwin -- -D warnings`: PASS.
- `bash scripts/build-release.sh`: PASS.
- `python3 scripts/smoke-foundation.py`: PASS — standalone assets, config, route isolation and SIGTERM.
- Browser feature E2E: NOT APPLICABLE (storage library; HTTP/UI integration is later).
- Git checkpoint: unavailable — workspace has no `.git` repository.

## HTTP — Phase 03

- [x] Browser clients list real temporary files over HTTP (desktop/mobile Chromium).
- [x] Virtual paths and absolute-path redaction remain enforced.
- [x] Create/rename/delete work through real application/storage services.
- [x] Downloads stream; 8 MiB HTTP test consumes chunks without full buffering.
- [x] Supported Range cases and malformed 416 responses verified.
- [x] HEAD/empty-file/If-Range fallback verified.
- [x] Thin HTTP routes; blocking filesystem work belongs to FileService.
- [x] Real Axum TCP integration tests with temporary filesystems.
- [x] SSE mutation events, connection limit and shutdown tested.
- [x] Host/Origin/body-limit and peer-forgery regressions pass.
- [ ] Upload HTTP streaming (Phase 04).
- [ ] Native Windows/macOS runtime and separate-device LAN validation.

### Phase 03 exact checks — 2026-09-30

- `bash scripts/check.sh`: PASS (`npm run typecheck --prefix web`,
  `npm run build --prefix web`, `cargo fmt --check`,
  `cargo clippy --locked --workspace --all-targets -- -D warnings`,
  `cargo test --locked --workspace`: 29 tests, 7 unit + 22 integration).
- `npm run test:e2e --prefix web`: PASS, 2 workflows (desktop and Pixel mobile Chromium);
  real navigation, search, List/Grid, download bytes, create/rename/delete and SSE refresh.
- `cargo check --locked --workspace --all-targets --target x86_64-pc-windows-msvc`: PASS.
- `cargo check --locked --workspace --all-targets --target x86_64-apple-darwin`: PASS.
- `bash scripts/build-release.sh`: PASS.
- `python3 scripts/smoke-foundation.py`: PASS with updated unconfigured-file behavior.
- Real Vite proxy check: PASS, GET status and JSON mkdir via port 5173 with Origin
  `http://127.0.0.1:5173`, Rust `--dev`, isolated temporary root/config.
- OpenAPI YAML parsed and all six implemented route paths checked.
- `git diff --check`: PASS. Existing Phase 02 working changes were preserved.

## Security/session

- [ ] Token join.
- [ ] Token rotation.
- [ ] Open LAN optional mode.
- [ ] Host settings remote denial.
- [ ] Token not leaked in logs.

## UI

- [ ] Full-screen layout.
- [ ] List/Grid.
- [ ] Search.
- [ ] Desktop context menu.
- [ ] Mobile action sheet.
- [ ] Paste.
- [ ] Create/rename/delete.
- [ ] QR.
- [ ] Settings.
- [ ] aggregate transfer progress.
- [ ] queue.
- [ ] reduced-motion behavior.

## Native

- [ ] Windows tray.
- [ ] Linux tray.
- [ ] macOS tray.
- [ ] graceful quit.
- [ ] open browser/folder.
- [ ] release icon.

## Performance

- [ ] Large upload bounded memory.
- [ ] Large download bounded memory.
- [ ] configured concurrency enforced.
- [ ] many-file directory remains usable.

## Packaging

- [ ] Windows archive startup.
- [ ] Linux archive startup.
- [ ] macOS package startup.
- [ ] embedded frontend works without Node.

## Phase 04 local acceptance — 2026-09-30

| Criterion | Result | Evidence |
|---|---|---|
| Two files run with limit 2 | PASS | Application and real HTTP concurrency tests |
| Third file queues | PASS | Delayed body streams retain two permits and queue the third |
| Parallel disabled forces 1 | PASS | Application test with configured maximum 8 and parallel disabled |
| Modified client cannot exceed limit | PASS | Direct concurrent HTTP requests, independent of browser scheduler |
| Cancel/failure cleans partial files | PASS | Cancel, disconnect, shutdown and length-failure tests; browser cancel/retry |
| Aggregate byte progress is correct | PASS | Weighted, unknown-size, zero-byte and cancelled-row aggregate tests |
| Large uploads avoid proportional memory growth | PASS | Linux release process: 8 MiB / 256 MiB uploads, peak RSS 6.89 / 9.10 MiB |

Validation: `bash scripts/check.sh` passed formatting, Clippy with warnings denied,
36 Rust tests, frontend typecheck, aggregate unit tests and production build.
`npm run test:e2e --prefix web` passed six Chromium desktop/mobile tests against
real embedded Rust serving, including uploads, clipboard drafts/previews, conflicts,
active cancellation and restart-from-zero retry. `bash scripts/build-release.sh`,
`python3 scripts/smoke-foundation.py` and `python3 scripts/test-upload-memory.py`
passed. Workspace/all-target cross-checks passed for x86_64-pc-windows-msvc and
x86_64-apple-darwin. Native Windows/macOS runtime, separate-device LAN and native
clipboard permissions across browsers remain unverified; this is local acceptance,
not v1 release acceptance.

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

## Phase 06 local acceptance — 2026-09-30

| Criterion | Result | Evidence |
|---|---|---|
| No prototype-only fake controls remain | PASS | Real API-backed file, upload, clipboard, QR/settings and selection workflows; unsupported preview/per-file sharing controls omitted |
| Desktop and mobile E2E flows pass | PASS | 14 tests, seven workflows each on desktop and Pixel mobile Chromium against embedded Rust serving |
| All icon-only actions have accessible names | PASS | Named buttons/links, Axe audits of list/grid/menus/create/settings, keyboard navigation and visible focus |
| Refresh/reconnect converges to real server state | PASS | Initial API failure recovers through Refresh; offline browser reconnect discovers a real server mutation through SSE resync |
| UI does not expose real paths | PASS | Virtual-path API use, displayed-path checks, existing HTTP redaction and storage sandbox regressions |

Exact validation:

- `bash scripts/check.sh`: PASS — `npm run typecheck --prefix web`,
  `npm run test:unit --prefix web` (two test files: aggregation and file types),
  `npm run build --prefix web`, `cargo fmt --check`,
  `cargo clippy --locked --workspace --all-targets -- -D warnings`, and
  `cargo test --locked --workspace` (47 Rust tests, including security regressions).
- `cargo build --locked -p splitshare`: PASS.
- `npm run test:e2e --prefix web`: PASS — 14 tests, including Axe checks,
  nested conflict focus restoration, Keep existing, real downloads/deletions,
  reduced-motion content/backdrops and connection recovery.
- `bash scripts/build-release.sh`: PASS — production assets embedded in Rust.
- `python3 scripts/smoke-foundation.py`: PASS.
- `python3 scripts/smoke-sessions.py`: PASS.
- `git diff --check`: PASS.
- Desktop listing and mobile grid/action-sheet/settings screenshots inspected.

No HTTP contracts changed; existing mutation, download, transfer, settings and SSE
endpoints support this phase. Native tray is Phase 07. Native Windows/macOS runtime,
physical mobile devices, Firefox/Safari, manual screen-reader checks and release
stress/packaging gates remain pending in ISSUES.md. This is local phase acceptance.

## Phase 07 implementation checkpoint — 2026-10-01

| Criterion | Result | Evidence / remaining gate |
|---|---|---|
| Tray is native and follows platform conventions | PASS locally | Actual GTK/AppIndicator menu; Windows/macOS code cross-compiles, native convention checks pending |
| No custom tray dashboard | PASS | Native menu only; no window/webview; settings use local Vue UI |
| Quit cleans/cancels active uploads safely | PASS locally | Real Linux native Quit during unfinished upload; composition-root Stop cancellation and cleanup tests |
| Tray failure does not corrupt server operation | PASS locally | Deliberately unavailable display leaves HTTP usable and exits gracefully on SIGTERM |
| Native smoke tests recorded for all release platforms | FAIL — pending | Linux recorded; native Windows/macOS execution unavailable here; phase remains open |

All commands below passed for this checkpoint:

- `bash scripts/check.sh`: formatting, Clippy with warnings denied, 51 Rust tests,
  frontend typecheck, two frontend unit test files and production build.
- `cargo build --locked -p splitshare` and `bash scripts/build-release.sh`.
- `npm run test:e2e --prefix web`: 16 desktop/mobile Chromium tests.
- `/usr/bin/python3 scripts/smoke-tray-linux.py`: native Linux menu, Stop/Start,
  fresh links, stable port and native Quit/active-upload cleanup.
- `python3 scripts/smoke-tray-fallback.py`: failed-display headless recovery.
- `python3 scripts/smoke-foundation.py` and `python3 scripts/smoke-sessions.py`.
- `cargo check --locked --workspace --all-targets --target x86_64-pc-windows-msvc
  --target-dir /tmp/splitshare-tray-windows-check`: compile only.
- `cargo check --locked --workspace --all-targets --target x86_64-apple-darwin
  --target-dir /tmp/splitshare-tray-macos-check`: compile only.

No HTTP contract changed. Settings deep-link entry remains guarded by actual
server-reported host locality; all backend host authorization remains enforced.
The native platform smoke matrix and remaining manual OS-action checks are in
`docs/TRAY.md`. Phase 08 is not started.

Checkpoint upload memory regression: `python3 scripts/test-upload-memory.py` PASS,
8 MiB / 256 MiB uploads at 15.61 / 15.85 MiB peak RSS before the final opener-only
error-reporting adjustment.

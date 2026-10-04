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
- [x] Upload HTTP streaming (implemented in Phase 04).
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

- [x] Token join.
- [x] Token rotation.
- [x] Open LAN optional mode.
- [x] Host settings remote denial.
- [x] Token not leaked in logs.

## UI

- [x] Full-screen layout.
- [x] List/Grid.
- [x] Search.
- [x] Desktop context menu.
- [x] Mobile action sheet.
- [x] Paste.
- [x] Create/rename/delete.
- [x] QR.
- [x] Settings.
- [x] aggregate transfer progress.
- [x] queue.
- [x] reduced-motion behavior.

## Native

- [ ] Windows tray.
- [x] Linux tray.
- [ ] macOS tray.
- [x] Linux graceful quit; native Windows/macOS verification remains user-owned.
- [ ] open browser/folder.
- [ ] release icon.

## Performance

- [x] Large upload bounded memory.
- [x] Large download bounded memory.
- [x] configured concurrency enforced.
- [x] many-file directory remains usable.

## Packaging

- [ ] Windows archive startup.
- [ ] Linux archive startup.
- [ ] macOS package startup.
- [x] embedded frontend works without Node.

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
| Native smoke tests recorded for all release platforms | FAIL — pending | Linux recorded; native Windows/macOS execution is user-owned manual follow-up per 2026-10-01 instruction |

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
`docs/TRAY.md`. The user subsequently authorized Phase 08 and took ownership of native Windows/macOS checks.

Checkpoint upload memory regression: `python3 scripts/test-upload-memory.py` PASS,
8 MiB / 256 MiB uploads at 15.61 / 15.85 MiB peak RSS before the final opener-only
error-reporting adjustment.

## Phase 08 local acceptance — 2026-10-01

| Criterion | Result | Evidence |
|---|---|---|
| No known root escape | PASS locally | Full Linux traversal, encoded-path, symlink race/root-boundary regressions; native Windows/macOS execution remains user-owned |
| No known absolute-path leak | PASS | DTO/HTTP redaction tests, safe transport errors, token/log/config smoke |
| Bounded memory for large files | PASS | Generated 8 MiB / 10 MiB / 1 GiB upload and download; peak RSS 14.34 / 14.55 / 14.88 MiB |
| Bounded concurrency under hostile clients | PASS | Real 128-connection saturation, backend worker limits, bounded history/download/SSE, header/body/queue/idle deadlines |
| Interrupted transfers remain truthful | PASS | Disconnect/cancel/shutdown cleanup, timeout snapshots, lost-response unconfirmed state and reconciliation across browser engines |
| Mandatory browser/native matrix passes or release blockers are recorded | PASS with recorded follow-ups | 38 browser cases pass, two CDP-only cases N/A outside Chromium; Linux tray/fallback pass; native Windows/macOS user-owned, Safari/iOS and physical-device gates in ISSUES.md |

Exact checks:

- `bash scripts/check.sh`: PASS — frontend typecheck, two unit test files, Vite
  production build, Rust format, Clippy with warnings denied and all 57 Rust tests.
- After the final TCP latency adjustment: `cargo fmt --check`,
  `cargo clippy --locked --workspace --all-targets -- -D warnings` and
  `cargo test --locked --workspace`: PASS again (57 tests).
- `cargo build --locked -p splitshare`: PASS.
- `SPLITSHARE_BROWSER_MATRIX=1 LD_LIBRARY_PATH=/tmp/splitshare-playwright-deps/root/usr/lib/x86_64-linux-gnu npm run test:e2e --prefix web`:
  PASS, 38 tests; two explicitly skipped CDP throttling variants. Chromium desktop
  and Pixel viewport, Firefox desktop and Linux WebKit desktop all exercised.
- The same matrix with `-- --grep 'browse, mutate|lost upload response'` rechecks
  eight HTTP/download/SSE/reconciliation workflows after the final TCP adjustment:
  PASS, all eight.
- `bash scripts/build-release.sh`: PASS.
- `python3 scripts/smoke-foundation.py`: PASS — embedded standalone startup,
  API isolation, safe configuration, log redaction and SIGTERM.
- `python3 scripts/smoke-sessions.py`: PASS — persisted settings/restart, token
  regeneration, cookie redirects and secret redaction.
- `python3 scripts/smoke-tray-fallback.py`: PASS — unavailable-display recovery.
- `/usr/bin/python3 scripts/smoke-tray-linux.py`: PASS — native menu, Stop/Start,
  fresh link and Quit during an upload with cleanup.
- `python3 scripts/benchmark-reliability.py`: PASS — deterministic generated
  transfer bytes, bounded RSS, 100 small file round trips and 10,000-entry listing.
- OpenAPI YAML/local references and `git diff --check`: PASS.

Measured Linux results (local temporary filesystem, not network throughput claims):

| Workload | Result |
|---|---|
| 1 GiB upload / download | 0.739 s / 0.576 s; peak server RSS 14.88 MiB |
| 100 small-file uploads plus downloads | 0.187 s, down from 4.288 s before TCP_NODELAY |
| 10,000-entry API directory listing | 40 ms; 1,160,028 response bytes; server RSS 25.12 MiB |
| Chromium desktop / mobile directory render | 416 / 467 ms with 100 rows; previously about 7.8 s with 10,000 rows |
| Chromium desktop / mobile full-directory search | 50 / 44 ms; previously about 1.4 s |
| Firefox / WebKit directory render | 575 / 676 ms; search 72 / 151 ms |

Pagination is client-side because measured cost was DOM rendering, not listing.
Directory metadata still scales with entry count; larger directories need further
measurement. TCP_NODELAY avoids delayed-ACK stalls on small responses and SSE.

Local WebKit needed Ubuntu `libavif16`, `libgav1-1` and `libyuv0`; packages were
extracted to a temporary directory and their three libraries supplied to the
Playwright browser cache, without system installation. CI uses Playwright's normal
`--with-deps` installation. Test fixtures/build outputs are not committed.

Phase 08 local implementation is complete. Abrupt-kill partial scavenging,
physical-device/LAN stress, native clipboard/openers, native Safari/iOS and
user-owned Windows/macOS checks remain recorded. This is not v1 release acceptance;
Phase 09 packaging is next and has not been started.


## Host setup correction — 2026-10-01

The user's report exposed a usability gap in prior completion claims: folder and
listener setup were CLI-only, and fresh launches had no actionable QR setup flow.
Phase 08's earlier tests exercised a preconfigured root. This correction tests an
unconfigured launch explicitly; it does not pretend the earlier UI was complete.

| Criterion | Result | Evidence |
|---|---|---|
| Discoverable first-launch host setup | PASS | Fresh-process browser test opens Set up sharing; no-root QR opens General setup |
| Native folder selection, activation and persistence | PASS on Linux | Real portal dialog cancel/select smoke; actual file listing; private host.json; Rust restart/sandbox replacement tests |
| Real interface/port controls with recovery | PASS | Same-port loopback/LAN rebinding, changed port, occupied-port preservation, saved restart settings; browser Apply and Save changes |
| QR generation or actionable setup/error state | PASS | QR pixels decode to current links across engines; no-root guidance, loopback settings shortcut, failed discovery and retry |
| Host-only authorization and sandbox retained | PASS | Remote read/write/picker denied despite forged headers, arbitrary root payload rejected, full traversal/redaction suite |
| Documentation, validation and local commit | PASS | HOST_SETUP.md, API/OpenAPI and lifecycle docs aligned; exact checks below; completed fix committed locally |

Exact validation:

- `bash scripts/check.sh`: PASS — formatting, Clippy with warnings denied, all
  62 Rust tests, two frontend unit files, frontend typechecking and production build.
- `cargo test --locked -p splitshare-server --test access`: PASS, seven tests,
  rerun after strict rejection of nonempty folder-selection bodies.
- `cargo fmt --check` and `cargo clippy --locked --workspace --all-targets -- -D warnings`: PASS.
- `cargo build --locked -p splitshare`: PASS.
- `SPLITSHARE_BROWSER_MATRIX=1 LD_LIBRARY_PATH=/tmp/splitshare-playwright-deps/root/usr/lib/x86_64-linux-gnu npm run test:e2e --prefix web`:
  PASS, 46 cases; two explicitly skipped CDP-only variants. Chromium desktop/mobile,
  Firefox desktop and Linux WebKit desktop cover the corrected setup/QR flow.
- `bash scripts/build-release.sh` and `cargo build --locked --release -p splitshare`: PASS.
- `python3 scripts/smoke-foundation.py`, `python3 scripts/smoke-sessions.py`, and
  `python3 scripts/smoke-tray-fallback.py`: PASS.
- `SPLITSHARE_XDOTOOL=/tmp/splitshare-picker-tools/root/usr/bin/xdotool python3 scripts/smoke-host-setup-linux.py`:
  PASS — actual Linux/X11 native picker cancel/select, live sandbox and persistence.
- `/usr/bin/python3 scripts/smoke-tray-linux.py`: PASS — native lifecycle regression.
- OpenAPI YAML/local references, Python smoke syntax and `git diff --check`: PASS.

Native Windows/macOS picker/runtime validation remains user-owned. Other release
gates (physical-device QR/LAN, native Safari/iOS, crash-partial scavenging and
packaging) remain in ISSUES.md; this correction does not claim v1 release readiness.

## Clipboard image correction — 2026-10-01

| Criterion | Result |
|---|---|
| Image paste into permission fallback dialog, including file-item-only clipboard | PASS |
| Images take priority over separate text items in either order | PASS |
| Preview confirmation and exact saved image bytes | PASS |
| Existing file/text paste, upload and conflict flows | PASS |
| Documentation and local Git checkpoint | PASS |

Exact validation:

- `bash scripts/check.sh`: PASS (62 Rust tests, frontend unit tests, typecheck,
  production UI build, formatting and Clippy).
- `cargo build --locked -p splitshare`: PASS; rebuilt the executable before the
  final browser run. The initial run against stale embedded assets failed the new
  regressions and was superseded by the rebuilt run.
- `SPLITSHARE_BROWSER_MATRIX=1 LD_LIBRARY_PATH=/tmp/splitshare-playwright-deps/root/usr/lib/x86_64-linux-gnu npm run test:e2e --prefix web -- clipboard.spec.ts browser.spec.ts`:
  PASS, 22 passed and two existing non-Chromium CDP-only skips.
- `bash scripts/build-release.sh`: PASS, release binary includes corrected UI.
- `git diff --check`: PASS.

Clipboard format and permission failures use deterministic browser fixtures.
Native OS clipboard permissions and physical-device checks remain manual;
Windows/macOS validation remains user-owned. No backend/API contract changed.

## Paste button HTTP LAN correction — 2026-10-01

The user clarified that Ctrl+V works but the Paste button opens a text form on an
HTTP LAN address. The prior fix covered event handling, but left this misleading
classification intact: a missing/failed Async Clipboard API read opened an empty
text draft. The corrected button uses a neutral undetected state with a specific
HTTP/unsupported/permission/read-failure explanation and retry. HTTP LAN button
reads remain subject to browser secure-context restrictions; use localhost on the
host or keyboard paste on a LAN client. No browser-policy bypass was added.

Acceptance criteria:

- PASS: HTTP LAN blocked reads show no text editor or Save file action.
- PASS: Permission failures can retry and switch to an image preview.
- PASS: Real Chromium Clipboard API image writes/readback followed by clicking Paste
  produce a preview and exact uploaded clipboard bytes.
- PASS: Existing keyboard image/file/text paste and upload regressions remain passing.

The LAN test uses an actual local interface with desktop/system proxy bypass.
Initial proxy-routed test attempts returned a proxy 502 and were superseded by
direct-interface runs. Real clipboard permission grants are Chromium-only;
Firefox/WebKit use explicit skips for that test and still exercise actual HTTP
LAN restrictions plus permission fixtures. Native desktop/browser permission
prompts remain manual validation.

Validation (all PASS):

- `bash scripts/check.sh`: 62 Rust tests, frontend unit tests, typecheck/build,
  formatting and Clippy.
- `cargo build --locked -p splitshare`: rebuilt embedded UI for browser tests.
- `SPLITSHARE_BROWSER_MATRIX=1 LD_LIBRARY_PATH=/tmp/splitshare-playwright-deps/root/usr/lib/x86_64-linux-gnu npm run test:e2e --prefix web -- clipboard.spec.ts browser.spec.ts`:
  32 passed, four explicit skips (two CDP variants and two real-clipboard
  permission-grant variants).
- `bash scripts/build-release.sh`: rebuilt release executable.
- `git diff --check`: clean. Documentation and local Git checkpoint completed.

## Adaptive mobile upload — 2026-10-01

The user approved Upload as the primary mobile action, with photo/file selection,
a native long-press paste target and image confirmation. Implemented in the
existing Vue manager and upload queue; HTTP/API contracts and Rust policy are
unchanged. No HTTPS setup or synthetic keyboard shortcut is required for selection.

Acceptance criteria:

- PASS: Narrow and coarse-pointer devices offer Photos & videos, Browse files and Paste
  from clipboard in a bottom sheet, including wide touch tablets.
- PASS: Media/file actions invoke real browser file choosers with the appropriate accept
  filters; cancelling a chooser leaves the previous dialog available.
- PASS: Received images offer a preview, editable filename and explicit Upload. Multiple
  images can be confirmed/cancelled independently; downloaded bytes match input.
- PASS: An unsupported image preview retains the original uploadable file.
- PASS: Clipboard fallback has an editable Paste area with touch/desktop instructions,
  photo/file alternatives, and no ineffective Retry when the API is unavailable.
- PASS: Desktop direct picker, drag/drop, keyboard paste, existing file manager and host
  configuration regressions remain covered; modal accessibility remains covered.

Browser fixtures exercise the actual textarea and browser file-chooser events,
not physical mobile OS menus/photo libraries. Physical iPadOS/Android long-press
image paste and native media-provider validation remain explicitly pending in
ISSUES.md. Generated screenshots were inspected locally and are not committed.

Exact validation:

- `bash scripts/check.sh`: PASS — 62 Rust unit/integration/security tests, both
  frontend unit files, typecheck, build, formatting and Clippy.
- `npm run typecheck --prefix web`, `npm run test:unit --prefix web`,
  `npm run build --prefix web` and `cargo build --locked -p splitshare`: PASS
  after the final unsupported-retry UI adjustment.
- `SPLITSHARE_BROWSER_MATRIX=1 LD_LIBRARY_PATH=/tmp/splitshare-playwright-deps/root/usr/lib/x86_64-linux-gnu npm run test:e2e --prefix web`:
  all 42 final Chromium desktop/mobile cases passed before an external SIGTERM
  interrupted the runner during Firefox.
- `SPLITSHARE_BROWSER_MATRIX=1 LD_LIBRARY_PATH=/tmp/splitshare-playwright-deps/root/usr/lib/x86_64-linux-gnu npm run test:e2e --prefix web -- --project firefox --project webkit`:
  PASS — 38 passed, four explicit CDP/clipboard-permission-grant skips. Combined
  final coverage: 80 passed, four skipped.
- The earlier bottom-sheet-position assertion ran during its opening animation;
  it now waits for the final position. Final chooser/confirmation tests pass on
  all four configurations.
- `bash scripts/build-release.sh`: PASS with final embedded frontend.
- `git diff --check`: PASS. Documentation and local Git checkpoint completed.

## Unified paste workflow — 2026-10-03

This refinement supersedes the preceding adaptive upload-sheet and automatic
clipboard-read behavior.

- PASS: Paste opens one neutral editable modal without accessing the Clipboard API,
  including on an actual HTTP LAN origin. No permission/retry controls appear.
- PASS: Pasted image data takes precedence over accompanying text and changes the
  same modal to preview, editable filename and explicit Upload; saved bytes match.
- PASS: Pasted text becomes an editable draft with filename/extension and Save file;
  JSON receives a .json proposal. Cancel clears the draft without saving.
- PASS: Real Chromium Ctrl+V imports an image without application clipboard reads;
  cross-engine fixtures retain file/image/text paste and ordinary editing coverage.
- PASS: Upload opens the native multiple-file chooser directly at phone and tablet
  widths. Duplicate mobile sheets, media pickers and photo/file alternatives are
  removed. Sequential image confirmation and unsupported-preview retention pass.
- PASS: Empty/HTML-only payloads do not fabricate files, execute HTML or fetch image
  URLs. Accessibility, focus restoration, queue/conflict and host setup regressions pass.
- NOT APPLICABLE: New backend API/security policy; this refinement uses existing
  upload contracts. The existing Rust integration/security regressions pass.

Exact validation:

- `bash scripts/check.sh`: PASS — frontend typecheck, two frontend unit test files,
  production frontend build, `cargo fmt --check`, Clippy with warnings denied and
  all 62 Rust unit/integration/security tests.
- `cargo build --locked -p splitshare`: PASS; browser tests use the updated embedded UI.
- `npm run test:e2e --prefix web -- --project desktop --project mobile`: PASS — 42 tests.
- `SPLITSHARE_BROWSER_MATRIX=1 LD_LIBRARY_PATH=/tmp/splitshare-playwright-deps/root/usr/lib/x86_64-linux-gnu npm run test:e2e --prefix web -- --project firefox --project webkit`:
  PASS — 38 tests; four explicit skips for Chromium-only CDP cancellation and real
  clipboard-write permission automation. Combined browser result: 80 passed, four skipped.
- `bash scripts/build-release.sh`: PASS; release binary embeds the new interface.
- `git diff --check`: PASS. Mobile and Firefox desktop screenshots inspected.

Physical iPadOS/Android native long-press image paste remains a manual device check
in ISSUES.md. Automated paste payloads and native chooser events do not validate
OS menus or guarantee that every source app supplies image bytes. Windows/macOS
native validation remains user-owned. No known failing automated criteria remain.

## Remaining interface cleanup — 2026-10-03

- PASS: Network settings has one Save changes action; the duplicate Apply network
  settings button and repeated candidate-address list are removed.
- PASS: Invalid ports disable saving. LAN/loopback changes persist across restart
  through the remaining save action in all four browser configurations.
- PASS: QR discovery failure hides empty address/link controls. Refresh recovery,
  QR generation and the loopback Network settings shortcut remain functional.
- PASS: Empty notice rows are omitted; settings, permissions, token rotation,
  accessibility, selection/deletion and reconnection regressions pass.
- NOT APPLICABLE: Backend/API changes; existing policy and contracts are retained.

Exact validation:

- `bash scripts/check.sh`: PASS — frontend typecheck, two unit test files, build,
  Rust formatting, Clippy and 62 Rust unit/integration/security tests.
- `cargo build --locked -p splitshare`: PASS, with updated embedded frontend.
- `SPLITSHARE_BROWSER_MATRIX=1 LD_LIBRARY_PATH=/tmp/splitshare-playwright-deps/root/usr/lib/x86_64-linux-gnu npm run test:e2e --prefix web -- host-setup.spec.ts sharing.spec.ts production-ui.spec.ts`:
  PASS — 32 tests across desktop/mobile Chromium, Firefox and WebKit; no skips.
- `bash scripts/build-release.sh`: PASS.
- `git diff --check`: PASS.

No new blockers. Physical mobile/OS checks previously recorded in ISSUES.md remain
manual. Required delete/conflict confirmations, transfer recovery and file-action
menus are retained because they serve distinct operations.

## Folder-first browsing — 2026-10-03

- PASS: Folders precede files in List and Grid while preserving the existing name
  order within each group.
- PASS: Grouping precedes search and pagination; mixed directory fixtures verify
  the first page and the file-only second page, plus refresh behavior.
- PASS: Existing 10,000-file browsing, selection and mutation regressions pass.
- NOT APPLICABLE: Backend/API/security-policy changes; grouping is frontend display
  behavior applied whenever a directory listing loads.

Exact validation:

- `bash scripts/check.sh`: PASS — frontend typecheck, two unit test files, build,
  Rust formatting, Clippy and 62 Rust unit/integration/security tests.
- `cargo build --locked -p splitshare`: PASS.
- `SPLITSHARE_BROWSER_MATRIX=1 LD_LIBRARY_PATH=/tmp/splitshare-playwright-deps/root/usr/lib/x86_64-linux-gnu npm run test:e2e --prefix web -- directory.spec.ts browser.spec.ts`:
  PASS — 18 passed across desktop/mobile Chromium, Firefox and WebKit; two existing
  Chromium-only cancellation tests skipped in Firefox/WebKit.
- `bash scripts/build-release.sh`: PASS, with updated embedded frontend.
- `git diff --check`: PASS.

No new blockers; existing physical-device/native OS release checks remain pending.

## Upload keep-awake — 2026-10-03

- PASS: Pending uploads expose an opt-in Keep screen awake action in the transfer
  bar, with no additional modal or automatic playback.
- PASS: Native API requests use a user tap and only report active after acquisition.
  Rejection exposes a video retry; native lifecycle tests use deterministic mocks.
- PASS: Bundled MP4/WebM fallback plays silently and inline, including on an actual
  insecure HTTP LAN origin, with no external media dependency. Its status explicitly
  does not promise sleep prevention. Media metadata confirms no audio track.
- PASS: Manual stop and batch completion pause playback/release locks; page hiding
  releases the native lock, and returning requires a new tap. Denied playback does
  not claim success. Component teardown and non-pending queues also stop the helper.
- PASS: Existing upload, cancellation, conflict, accessibility and UI regressions pass.
- NOT APPLICABLE: New API/backend timeout/security policy; all existing limits remain.

Exact validation:

- `bash scripts/check.sh`: PASS — typecheck, two frontend unit files, frontend build,
  Rust formatting, Clippy and all 62 Rust unit/integration/security tests.
- `npm run build --prefix web` and `cargo build --locked -p splitshare`: PASS after
  final capability-detection/error-logging adjustment.
- `SPLITSHARE_BROWSER_MATRIX=1 LD_LIBRARY_PATH=/tmp/splitshare-playwright-deps/root/usr/lib/x86_64-linux-gnu npm run test:e2e --prefix web -- keep-awake.spec.ts browser.spec.ts production-ui.spec.ts`:
  37 passed, two existing Chromium-only cancellation skips, one Firefox test assertion
  failure: Firefox plays inline but does not expose the playsInline JS property.
  The assertion now checks the actual playsinline attribute. All 26 unrelated
  upload/UI regression cases passed on the final production code.
- `SPLITSHARE_BROWSER_MATRIX=1 LD_LIBRARY_PATH=/tmp/splitshare-playwright-deps/root/usr/lib/x86_64-linux-gnu npm run test:e2e --prefix web -- keep-awake.spec.ts`:
  PASS — all 16 final keep-awake cases, including added actual HTTP LAN checks.
  Combined final coverage: 42 passed, two explicitly skipped; no unresolved failure.
- `bash scripts/build-release.sh`: PASS with final embedded interface/media.
- `git diff --check`: PASS. Desktop transfer-bar screenshot inspected.

Actual automatic-lock prevention on physical Android/iPadOS remains unverified and
tracked in ISSUES.md. Headless playback/native mocks cannot establish OS sleep
behavior. The fallback cannot guarantee uploads while manually locked or backgrounded.

## Detailed upload logs — 2026-10-03

- PASS: Accepted/started/completed uploads correlate through a validated transfer ID,
  expected byte count, conflict policy and concurrency limit.
- PASS: Failure logs identify receive/write/prepare/publish/queue/worker stage,
  successfully written bytes, timings and safe typed causes. WARN-only output
  retains transfer ID and expected size without relying on filtered INFO spans.
- PASS: Cancellation and dropped handlers are distinct INFO events. Progress is
  throttled to at most once per 30 seconds of writes at DEBUG level; publication
  also has a DEBUG event. Existing failure codes/deadlines remain unchanged.
- PASS: Log capture and real HTTP checks verify absence of paths, keys and raw error
  text, plus interrupted-upload cleanup and conflict handling.
- NOT APPLICABLE: Browser E2E changes; no frontend, API response or upload policy
  change. Existing HTTP/security integration tests cover the affected lifecycle.

Exact validation:

- `bash scripts/check.sh`: PASS — 63 Rust unit/integration/security tests, both
  frontend unit files, frontend typecheck/build, formatting and Clippy.
- `cargo build --locked -p splitshare`: PASS.
- `bash scripts/build-release.sh`: PASS.
- `python3 scripts/test-upload-logging.py`: PASS — isolated release hosts verify
  successful upload, interrupted body, conflict, cleanup and privacy under both
  WARN-only and DEBUG filters.
- `git diff --check`: PASS.

The logging regression runs in a dedicated integration executable with a global
subscriber, avoiding scoped-subscriber callsite filtering interference from parallel
tests. Runtime WARN-only smoke exposed missing INFO-span context; failure entries
now carry critical fields directly, and both filters pass. No failing checks remain.
Storage Io and HTTP UnexpectedEof remain safe categories, not proof of a specific
OS/network root cause. Existing physical-device release checks remain pending.

## Host logging settings and live viewer — 2026-10-03

- PASS: Settings → Logging changes runtime verbosity without modifying sharing
  settings or revoking remote sessions.
- PASS: Open logs shows the newest 500 in-memory diagnostic entries with live SSE
  updates, search, severity filtering, refresh, clear and recoverable errors.
- PASS: Snapshot/configuration/clear/SSE routes enforce host-local access, mutation
  guards, no-store responses and the shared stream limit.
- PASS: Capture excludes paths, credentials, raw errors and unapproved messages;
  real upload lifecycle fields remain visible and correlated by transfer ID.
- PASS: Modal keyboard navigation, focus restoration and automated accessibility
  checks; desktop/mobile layouts visually inspected.

Validation:

- `bash scripts/check.sh`: PASS — 66 Rust unit/integration/security tests, both
  frontend unit files, typecheck/build, formatting and Clippy.
- `npm run build --prefix web`: PASS for the final frontend.
- `cargo build --locked -p splitshare`: PASS with final embedded frontend assets.
- `bash scripts/build-release.sh`: PASS with final embedded frontend assets.
- `python3 scripts/test-upload-logging.py`: PASS — release HTTP success,
  interrupted bodies, conflicts, cleanup and privacy under WARN and DEBUG filters.

History and selected verbosity last only for the current process. Restart restores
startup configuration and clears history. Persistent archives are outside this
implementation. Existing physical-device and native-platform checks remain manual.

- `SPLITSHARE_BROWSER_MATRIX=1 LD_LIBRARY_PATH=/tmp/splitshare-playwright-deps/root/usr/lib/x86_64-linux-gnu npm run test:e2e --prefix web -- logs.spec.ts sharing.spec.ts host-setup.spec.ts production-ui.spec.ts`:
  PASS — 40 tests across desktop/mobile Chromium, Firefox and WebKit.
- `git diff --check`: PASS.

## Stable Settings frame — 2026-10-04

- PASS: Desktop Settings stays 900 × 640 px across all six sections, capped to the
  available viewport. Mobile retains its full-screen frame.
- PASS: Content scrolls inside the frame; heading and Save/Cancel stay visible,
  including 1024 × 500 and 390 × 600 viewports.
- PASS: Existing sharing and nested log dialogs retain their behavior.
- NOT APPLICABLE: New API/backend/security behavior; this is a layout-only fix.

Validation:
- `bash scripts/check.sh`: PASS (66 Rust tests, frontend unit tests, typecheck,
  build, Rust formatting and Clippy).
- `cargo build --locked -p splitshare`: PASS.
- `SPLITSHARE_BROWSER_MATRIX=1 LD_LIBRARY_PATH=/tmp/splitshare-playwright-deps/root/usr/lib/x86_64-linux-gnu npm run test:e2e --prefix web -- settings-size-check.spec.ts sharing.spec.ts logs.spec.ts`:
  all 16 existing sharing/log tests passed. The temporary dimension probe initially
  sampled the opening animation; rerunning that probe with reduced motion passed
  all four browser configurations. Probe checked every section at normal, short
  desktop and narrow mobile sizes; temporary probe removed after verification.
- `git diff --check`: PASS.

Existing physical-device/native-platform checks remain manual; no new blockers.
- `bash scripts/build-release.sh`: PASS with updated embedded frontend.

## File previews — 2026-10-04

- PASS: Desktop filename double-click, mobile tap, keyboard Enter and Open context
  action launch the shared preview modal; Escape closes and restores focus.
- PASS: Images render with fit/zoom and previous/next navigation. Native video/audio
  players load and play real MP4/WAV fixtures; closing removes players and sources.
- PASS: UTF-8 text is read-only, selectable and optionally wrapped; previews fetch
  at most 256 KiB. Empty/truncated/binary text and unsupported files have clear states.
- PASS: HTML renders literally, without script execution. Inline allowlisted MIME,
  nosniff, no-store and sandbox CSP retain the existing filesystem security boundary.
- PASS: GET/HEAD require download permission and a valid session; traversal/symlink
  escape fails. Media uses the existing bounded Range stream and cancellation rules.
- PASS: Modal accessibility checks and desktop/mobile layout inspection.

Exact validation:
- `bash scripts/check.sh`: PASS — 68 Rust unit/integration/security tests, both
  frontend unit files, typecheck/build, formatting and Clippy.
- `npm run build --prefix web` and `cargo build --locked -p splitshare`: PASS for
  final frontend and embedded debug executable.
- `SPLITSHARE_BROWSER_MATRIX=1 LD_LIBRARY_PATH=/tmp/splitshare-playwright-deps/root/usr/lib/x86_64-linux-gnu npm run test:e2e --prefix web -- preview.spec.ts production-ui.spec.ts`:
  PASS — 24 tests across desktop/mobile Chromium, Firefox and WebKit.
- `LD_LIBRARY_PATH=/tmp/splitshare-playwright-deps/root/usr/lib/x86_64-linux-gnu npm run test:e2e --prefix web -- preview.spec.ts`:
  PASS — four additional desktop/mobile runs capturing preview screenshots.
- `bash scripts/build-release.sh`: PASS.
- `git diff --check`: PASS.

The earlier combined run also exercised browser.spec.ts: 10 passed and two existing
Firefox/WebKit upload-cancellation skips. New preview fixture uploads initially
lacked required transfer headers; menu assertions still expected Download first.
Both test issues were corrected before the final passing preview/production runs.

Browser media support depends on codecs. Physical iPadOS/Android media controls,
fullscreen and very large image memory use remain device checks. No transcoding,
editing or active document rendering is introduced. No failing mandatory checks remain.

## Preview usability refinement — 2026-10-04

- PASS: Compact filename/type/size header with persistent Download/Close; larger
  centered viewing area and full-screen mobile presentation.
- PASS: Grouped image navigation with position counter and tested Fit reset.
- PASS: Text wrapping and tested font-size adjustment; dedicated audio presentation
  with existing native playback controls. Clear loading/error/retry states.
- PASS: Preview accessibility, focus restoration, image/text rendering and real
  audio/video playback/close cleanup across Chromium desktop/mobile, Firefox, WebKit.
- NOT APPLICABLE: API/backend policy changes; permissions and streaming are unchanged.

Exact validation:
- `bash scripts/check.sh`: PASS — 68 Rust tests, two frontend unit files,
  typecheck/build, formatting and Clippy.
- `cargo build --locked -p splitshare`: PASS.
- `SPLITSHARE_BROWSER_MATRIX=1 LD_LIBRARY_PATH=/tmp/splitshare-playwright-deps/root/usr/lib/x86_64-linux-gnu npm run test:e2e --prefix web -- preview.spec.ts`:
  PASS — eight tests. Desktop text/mobile image screenshots visually inspected.
- `bash scripts/build-release.sh`: PASS.
- `git diff --check`: PASS.

Physical-device media/codec/accessibility checks remain manual. No new blockers.

# Testing

## Mandatory layers

### Rust unit tests

Cover:

- virtual path parser;
- permissions;
- share modes;
- conflict policy;
- session token validation;
- transfer state transitions;
- network address ranking.

### Storage integration tests

Use temporary directories.

Cover:

- list;
- read;
- upload publish;
- cancel cleanup;
- rename;
- mkdir;
- delete;
- traversal;
- encoded traversal;
- absolute paths;
- symlink escape;
- conflict outcomes.

### HTTP integration tests

Run real Axum router/server with temporary root.

Cover:

- status;
- file listing;
- permissions;
- upload;
- Range download;
- `416`;
- token join/cookie;
- remote host-settings denial;
- SSE event delivery.

### Browser tests

Use Playwright in production-like built frontend where possible.

Desktop:

- List/Grid switch;
- search;
- context menu;
- modals;
- drag/drop;
- multi-upload;
- queue;
- create/rename/delete;
- QR/settings;
- file conflict;
- reconnect.

Mobile viewport:

- toolbar;
- icon-only actions;
- action sheet;
- full-screen settings;
- upload queue;
- responsive list/grid.

### Native tests

On each release OS:

- tray appears;
- menu actions work;
- open folder;
- open browser;
- stop/start;
- graceful quit;
- application icon.

## Large-file tests

Do not commit giant fixtures.

Generate deterministic temporary files.

Minimum scenarios:

- 1 B;
- 1 KiB;
- 10 MiB;
- 1 GiB sparse/generated where platform allows;
- many small files;
- parallel uploads.

Assert memory does not scale with total file size.

## Security regression suite

Mandatory before v1:

- `../`;
- `%2e%2e`;
- mixed separators;
- double encoding where routing stack decodes;
- Windows drive/UNC attempts;
- symlink/reparse point;
- illegal filenames;
- token brute/invalid cases;
- malformed Range;
- oversized JSON;
- concurrency over configured limit.

## Repository check script

`./scripts/check.sh` should eventually run:

```text
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
frontend typecheck
frontend build
optional Playwright suite
```

The script must fail on a failed mandatory step.

## Foundation checks available now

Install dependencies with `npm ci --prefix web`, then `bash scripts/check.sh`.
The script builds frontend assets before Rust because they are compile-time inputs.
`bash scripts/build-release.sh` builds the standalone binary.
On Linux, `python3 scripts/smoke-foundation.py` runs a copied release binary in a
fresh temporary directory with isolated config, checks assets and unavailable
API/join routes, and verifies SIGTERM exit and token-log redaction. Port 8080 must
be available. Later-phase test lists above remain planned.

## Phase 02 storage checks

`cargo test -p splitshare-storage -p splitshare-core` covers path syntax and serde
validation, temporary filesystem operations, conflict publication races across
independent Storage instances, root boundaries, safe serialization, cleanup,
length checks, empty/unknown-size uploads, and a 16 MiB file in 64 KiB chunks.
Unix tests exercise final symlinks, sockets and repeated concurrent symlink swaps.
Windows tests explicitly exercise directory and file reparse links and require
Developer Mode or elevated symlink privileges; missing privileges fail the tests,
not silently skip them. The native CI matrix must supply those privileges.

Cross-target type/lint checks (not substitutes for native runtime tests):

```bash
cargo clippy --locked -p splitshare-storage --tests --target x86_64-pc-windows-msvc -- -D warnings
cargo clippy --locked -p splitshare-storage --tests --target x86_64-apple-darwin -- -D warnings
```

Browser E2E remains inapplicable to the Phase 02 library-only scope.


## Phase 03 checks

`bash scripts/check.sh` runs the full Rust suite including real HTTP listeners and
filesystem fixtures, Range/HEAD, malformed requests, safe errors, Host/Origin checks,
peer forgery, SSE emission/limits/shutdown and an 8 MiB streamed HTTP download.

Browser setup and checks:

```bash
npm ci --prefix web
npm run build --prefix web
cargo build --locked -p splitshare
npx --prefix web playwright install chromium
npm run test:e2e --prefix web
```

Playwright runs desktop Chromium and a mobile Pixel viewport against the embedded
production Vue build. Its Node helper is test-only; the server remains Rust.
CI installs Chromium prerequisites and runs this on Linux. Release smoke remains
`bash scripts/build-release.sh && python3 scripts/smoke-foundation.py` and now
verifies that unconfigured file endpoints return 503. Phase 05 also returns 503
for joins when no share is configured (the pre-session foundation used 404).

## Phase 04 checks

`bash scripts/check.sh` additionally runs frontend byte-aggregation unit tests and
application/HTTP transfer tests for backend limits, queued disconnects, cancellation
keys, cleanup, conflicts, streaming and bounded history. The browser suite now has
six tests: three workflows on desktop and mobile, including active cancellation
under network throttling followed by a full retry. Clipboard events are deterministic
fixtures; native permission prompts and Firefox/Safari still require manual testing.

After `bash scripts/build-release.sh`, run `python3 scripts/test-upload-memory.py`
on Linux. It samples real server RSS during generated 8 MiB and 256 MiB uploads;
port 43124 must be free. This does not replace the planned 1 GiB release stress test.

Cross-target compile checks:

```bash
cargo check --locked --workspace --all-targets --target x86_64-pc-windows-msvc
cargo check --locked --workspace --all-targets --target x86_64-apple-darwin
```

## Phase 05 checks

`bash scripts/check.sh` covers 47 Rust tests, frontend aggregation tests,
typechecking/build, formatting and Clippy. Session tests cover expiry, capacity,
rotation, permission independence, legacy configuration migration and atomic save
failure. HTTP adapter tests use injected socket peers for remote classification,
forged forwarding headers, host-only endpoints, mutation CSRF, GET/HEAD capability
checks and existing SSE/download/upload revocation.

`npm run test:e2e --prefix web` runs eight tests across desktop/mobile Chromium
against the native embedded server. Sharing tests select discovered address
options, decode rendered QR pixels with jsQR, assert no external requests, exercise
settings and rotation, and check real browser redirect/HttpOnly cookie behavior.
The fixture binds IPv4 with token mode and an isolated temporary share/config;
remote authorization is separately tested with synthetic socket peers, not claimed
as a physical-device test.

After a release build, Linux checks:

```bash
python3 scripts/smoke-foundation.py
python3 scripts/smoke-sessions.py
python3 scripts/test-upload-memory.py
```

The sessions smoke uses port 43125 and checks persisted permissions/concurrency
across native process restart, token regeneration, join redirect/cookie headers and
log/config redaction. Browser tests use port 43123. Native Windows/macOS runtime,
physical QR scanning, LAN/VPN reachability and Firefox/Safari remain manual gates.

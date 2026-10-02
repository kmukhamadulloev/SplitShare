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

## Phase 06 checks

Run `bash scripts/check.sh`, `cargo build --locked -p splitshare`, then
`npm run test:e2e --prefix web`. The frontend unit command includes byte aggregation
and metadata-only icon mapping/formatting tests. The 14 browser tests run seven
workflows each on desktop and Pixel mobile Chromium against the embedded build.
They cover previous transfer/sharing behavior plus selection, real downloads,
bulk deletion, keyboard menus, nested focus restoration, Keep existing, focus
containment, reduced-motion backdrops, horizontal overflow and safe displayed paths.
Axe checks WCAG 2 A/AA and 2.1 AA rules on list, grid, menus and create/settings
modals. This automated coverage does not replace manual assistive-technology checks.

Test-only request failures exercise initial recovery; actual browser offline/online
transitions exercise reconnect, with a real server mutation while disconnected.
Screenshots of list/grid, menus, transfers, QR and settings are written under the
ignored `web/test-results/` directory. The viewport test is mobile emulation, not
physical-device or Safari/Firefox evidence. Release build and foundation/session
smokes validate the same assets embedded in the standalone Rust executable.

## Phase 07 checkpoint checks

`bash scripts/check.sh` includes three composition-root lifecycle tests and an
embedded native-icon test (51 Rust tests total). Listener tests use actual HTTP:
Stop/rebind, occupied-port recovery, fresh tokens, settings retention, cancellation
and partial-file cleanup. `npm run test:e2e --prefix web` has 16 tests, including
host-only `#settings` entry on desktop/mobile. Browser fixtures pass `--no-tray`.

On a Linux desktop with GTK3/AppIndicator and the system Python Gio bindings:

```bash
cargo build --locked -p splitshare
/usr/bin/python3 scripts/smoke-tray-linux.py
```

This starts an isolated debug host on port 43126 and discovers only that process's
native menu through the session bus. It checks menu labels, Stop/Start with stable
port and new link, and native Quit during an unfinished upload with partial-file
cleanup. It does not click Open/Settings or overwrite the user's clipboard.

After `bash scripts/build-release.sh`:

```bash
python3 scripts/smoke-tray-fallback.py
python3 scripts/smoke-foundation.py
python3 scripts/smoke-sessions.py
python3 scripts/test-upload-memory.py
```

Fallback smoke uses port 43128 and deliberately unavailable display addresses,
then verifies HTTP and graceful signal exit. The ordinary smokes run `--no-tray`.
Native Windows/macOS menu, clipboard/opener and shutdown smoke must be run on those
hosts; cross-target compilation does not satisfy that acceptance requirement.

## Phase 08 reliability checks

`bash scripts/check.sh` now runs 57 Rust tests, including real-socket connection
exhaustion, oversized headers, incomplete-header and control-body deadlines, plus
paused-time queue/idle upload deadlines. Existing traversal/symlink races, absolute
path redaction, remote host denial, upload concurrency abuse, disconnect/shutdown
and SSE limit tests remain part of the full suite.

Build embedded assets and the debug binary before browser checks:

```bash
cargo build --locked -p splitshare
npx --prefix web playwright install --with-deps chromium firefox webkit
SPLITSHARE_BROWSER_MATRIX=1 npm run test:e2e --prefix web
```

The default browser command runs Chromium desktop/mobile (20 tests). The matrix
adds Firefox and WebKit desktop (40 cases total). The two non-Chromium variants of
the CDP-throttled active cancellation test are explicitly skipped; backend and
Chromium cancellation tests still run. WebKit on Linux is engine coverage, not
native Safari/iOS evidence. Clipboard tests inject deterministic paste events;
OS clipboard permissions remain manual. Firefox drops synthetic constructor
clipboard data, so fixtures define that property explicitly. WebKit route
interception omits File bytes; the lost-response fixture forwards its exact payload
to the real server before aborting the response. Normal uploads are independently
verified across engines.

Directory fixtures contain 10,000 files. Browser checks enforce 100 rendered rows,
full-snapshot search, cross-page selection and List/Grid paging. Printed local
load/search timings are measurements, not flaky wall-clock acceptance thresholds.
Other new cases verify unconfirmed upload responses reconcile to actual publication.
The existing real-mutation-after-download case catches Firefox SSE interruption;
links use the native `download` attribute to avoid navigation.

Release checks on Linux:

```bash
bash scripts/build-release.sh
python3 scripts/smoke-foundation.py
python3 scripts/smoke-sessions.py
python3 scripts/smoke-tray-fallback.py
python3 scripts/benchmark-reliability.py
/usr/bin/python3 scripts/smoke-tray-linux.py
```

The benchmark needs port 43129, Linux `/proc` RSS and roughly 1.1 GiB temporary
storage; `--large-mib`/`--files` allow smaller development runs but do not replace
the default 1 GiB/10,000-entry gate. CI runs the full browser engine matrix and this
benchmark on Linux. Physical-device LAN, native Safari/mobile, OS clipboard/openers
and Windows/macOS native validation remain separately recorded release follow-ups.


## Host setup correction

`bash scripts/check.sh` now includes 62 Rust tests. New coverage verifies native
setup remote denial (including forged forwarding headers), no browser-selected
filesystem paths, busy picker limits, real sandbox replacement, listener rebinding,
occupied-port preservation, startup persistence, missing-folder recovery and private
versioned configuration. The normal browser matrix has 48 cases: 46 pass and the
two existing non-Chromium CDP throttling cases are explicitly skipped.

`web/tests/host-setup.spec.ts` starts an additional isolated native process on port
43130 without a root. It checks the first-run setup action, QR-to-setup flow,
interface/port form, explicit Apply and Save changes, real LAN rebinding and restart
persistence. Another test injects a discovery failure and verifies QR retry plus
the loopback Network settings shortcut. Existing QR tests decode actual pixels.

For the real Linux/X11 native picker (requires a desktop portal and `xdotool`):

```bash
cargo build --locked -p splitshare
python3 scripts/smoke-host-setup-linux.py
```

The smoke starts an isolated host on port 43131, opens its actual portal folder
dialog, cancels once, then selects a temporary folder and verifies actual listing
and private persistence. It refuses to run with an existing SplitShare chooser
open and checks the target window before keyboard input. It briefly focuses the
native dialog; run it on a desktop test session. `SPLITSHARE_XDOTOOL` can specify
an existing test executable. This tool is not a production runtime dependency.
Windows/macOS picker tests remain user-owned; headless Linux cannot validate a
native picker and uses CLI root selection instead.

## Unified clipboard and upload regression

`web/tests/clipboard.spec.ts` verifies the neutral Paste modal without clipboard
API access, image file-item precedence and same-modal transition, exact uploaded
image bytes, editable text/JSON filename proposals, cancellation/reset, and safe
empty/HTML-only handling. It checks an actual HTTP LAN origin and modal accessibility.
Chromium also writes an image to its isolated clipboard and uses real Ctrl+V;
Firefox/WebKit skip this permission-grant-dependent case. Native physical-device
clipboard behavior remains manual.

`web/tests/mobile-upload.spec.ts` verifies direct native file-chooser events at
phone and tablet widths, unrestricted multiple selection, sequential image
confirmation/cancellation, exact uploaded bytes, touch paste guidance and original
file retention when an image format cannot be previewed.

Rebuild the embedded frontend before browser tests:

```bash
bash scripts/check.sh
cargo build --locked -p splitshare
npm run test:e2e --prefix web -- --project desktop --project mobile
SPLITSHARE_BROWSER_MATRIX=1 npm run test:e2e --prefix web -- --project firefox --project webkit
```

Browser fixtures do not automate native OS photo libraries or long-press menus.
On physical iPadOS Safari and Android: select multiple images through Upload,
cancel one and confirm another, then copy a screenshot/image and long-press the
Paste area. Confirm the detected image and filename. Repeat with text. If the OS
provides no image bytes, cancel and use Upload. Check portrait and landscape.
Generated `test-results/simple-paste-*.png` screenshots are ignored local artifacts.

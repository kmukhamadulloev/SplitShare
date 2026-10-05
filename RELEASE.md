# Release History

## 0.1.0 — 2026-10-05

Current development release checkpoint for the native desktop host and browser
client. Rust workspace crates, frontend package and lockfiles use **0.1.0**.
A locally verified Linux x86_64 archive candidate is available through the build
scripts. No public release or Git tag has been published. The future route to
1.0.0 is undefined; the original phase program is archived.

### Host setup and lifecycle

- One native Rust process serves the embedded Vue UI and owns the system tray;
  end users do not need Node.js, Electron, Tauri, a database or cloud services.
- Mandatory first-run setup guides the host through native folder selection,
  network interface/port and access policy. It cannot be skipped or dismissed.
  Sharing stays disabled until setup succeeds; remote clients see a waiting state.
- The chosen folder is staged privately during setup. Valid saved configuration
  bypasses onboarding on restart; a missing saved folder requires recovery.
- Default port is 8080. Settings can change interface/port without restarting the
  native process. The host browser follows the new address; the tray can reopen it.
  Failed changes preserve or restore the previous listener and configuration.
- An occupied default port on an unconfigured launch can use a temporary loopback
  port to make setup reachable. Explicit binds and configured hosts do not silently
  move to another port.
- Native tray controls provide browser/folder opening, Settings, Start/Stop sharing
  and graceful Quit. CLI root/bind options support terminal and headless use.

### Files, transfers and sharing

- A sandbox rooted at one host-selected folder exposes virtual paths only.
  Traversal, absolute paths and symlink escapes are rejected; native paths stay
  out of browser API responses.
- Browse, create folders, rename and delete according to host permissions.
- Uploads stream through bounded chunks into hidden temporary files, then publish
  atomically according to conflict policy. Downloads and previews stream with
  HTTP Range support; whole files are not buffered in memory.
- Multi-file queues support host-enforced concurrency, progress, cancellation,
  retry and conflict handling. Disabling parallel uploads enforces one at a time.
- Transport deadlines, connection limits and uncertain-upload recovery improve
  failure handling. Graceful shutdown/disconnect cleanup protects partial uploads.
- Opaque private links exchange tokens for same-origin HttpOnly sessions. The host
  can rotate/revoke access, configure remote permissions or opt into Open LAN mode.
- Locally generated QR codes use a selected reachable address, with useful
  readiness/error states. Host settings and native dialogs are protected by actual
  connection classification and mutation origin/header checks.
- SSE updates file, transfer and session state and supports browser reconnects.

### Browser interface and clipboard

- Responsive dark file manager with List/Grid modes, search, selection, compact
  toolbar, desktop context menus, mobile action sheets and paged directory rows.
- Folders appear before files in both views, including search and pagination.
- Ctrl+V remains supported. Paste opens one editable modal for native paste,
  including mobile long-press paste: images/files take precedence over accompanying
  text; text becomes a draft with editable filename, extension and content.
- Upload opens the native file picker; drag and drop is supported. Duplicate
  upload/paste dialogs and redundant network controls have been removed.
- Settings use a stable desktop frame, scrollable content and persistent actions,
  with a viewport-sized mobile layout.
- Optional keep-awake during uploads uses native wake lock where available and a
  bundled silent-video fallback on HTTP. Playback does not guarantee that a device
  will prevent sleep or continue uploading after locking/backgrounding.

### File previews and diagnostics

- Open supported images, video, audio and bounded plain text using existing
  download authorization. A conservative MIME allowlist, nosniff and sandbox
  headers protect the preview endpoint; HTML-like text renders as text.
- Image fit/zoom and previous/next controls, dedicated audio presentation, native
  video controls, adjustable text size, retry/download actions and mobile layouts.
- Text preview is limited to the first 256 KiB. Closing a preview cancels reads and
  unloads media. Unsupported types/codecs retain a download path.
- Upload diagnostics correlate transfer IDs with stages, byte counts, timings and
  typed failures; debug progress is throttled and sensitive details are excluded.
- Host Settings provide process-local logging verbosity and a live, bounded
  500-entry log viewer with clear/realtime updates. Logging-only changes do not
  revoke remote sessions. Persistent log archives are not included.

### Build, packaging and verification

- `python3 scripts/package-linux.py` builds production frontend assets and the
  locked Rust release executable, verifies an extracted archive, and writes
  `release/splitshare-0.1.0-linux-x86_64.tar.gz` with a SHA-256 sidecar.
- The archive includes the executable, branding PNG, startup instructions and
  project license. Linux still needs GTK3/AppIndicator runtime libraries;
  native folder selection requires an available desktop portal/backend.
- Recorded local checks cover frontend type/unit/build, Rust formatting, Clippy,
  unit/integration/security regressions and browser flows across Chromium,
  Firefox and WebKit, including mobile viewport emulation.
- Reliability evidence includes a 1 GiB transfer benchmark and measured
  10,000-entry directory behavior. Archive smoke verifies embedded assets, real
  upload/download bytes, Range and graceful exit with an empty runtime PATH.
- Exact commands, dates and limitations are recorded in
  [acceptance evidence](docs/ACCEPTANCE.md). These are accumulated checkpoint
  results, not a claim that every device/platform was retested for these notes.

### Known limitations and remaining acceptance

- Designed for trusted local/private networks, not public internet hosting.
- Windows/macOS native runtime acceptance remains user-owned. Wayland-only native
  dialogs/openers, physical iPadOS/Android paste, wake prevention, codecs, QR and
  separate-device LAN/VPN flows still require the checks in [ISSUES.md](ISSUES.md).
- Linux archive smoke uses installed system libraries; clean-distribution ABI
  compatibility, third-party notices and distribution/signing decisions remain
  release work. The archive is not a universally portable or published package.
- External native filesystem edits require Refresh; they do not yet emit SSE.
  Network interface/authority discovery uses a startup snapshot.
- Abrupt process termination can leave hidden upload partials consuming disk;
  automatic scavenging and resumable uploads across restart are not implemented.
- Directory metadata snapshots still scale with entry count; larger server-side
  pagination is not implemented. IPv6 advertisement is deferred.
- No mobile host application, cloud relay, accounts, multi-root sharing,
  generated multi-selection ZIP download or persistent transfer history.

## Future versions

No next release scope or route to 1.0.0 is defined. [ROADMAP.md](ROADMAP.md)
records this planning status. Historical phases are archived and no longer assign
work. The limitations above describe 0.1.0; they are not a promised release plan.

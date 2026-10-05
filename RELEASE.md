# Release History

## 0.1.0 — 2026-10-05

### Added

- Native Rust host with an embedded Vue browser interface.
- System tray controls for opening the browser, shared folder and Settings,
  starting/stopping sharing, and quitting the app.
- Mandatory first-run setup for the shared folder, network interface, port and
  access policy, with persistent configuration and default port 8080.
- Live interface and port changes with automatic host-browser navigation.
- Private share links, HttpOnly sessions, QR codes, access revocation, Open LAN
  mode and host-controlled permissions.
- Sandboxed file browsing, folder creation, rename and delete.
- Streaming uploads/downloads, HTTP Range support, atomic upload publication and
  conflict handling.
- Multi-file upload queues with backend-enforced concurrency, aggregate progress,
  cancellation and retry.
- Responsive List/Grid views, search, selection, pagination, desktop context menus
  and mobile action sheets.
- File-picker uploads, drag and drop, keyboard paste and a Paste modal for images,
  files and text drafts with editable filenames, extensions and content.
- Image, video, audio and plain-text previews with download actions.
- Realtime SSE updates and browser reconnection handling.
- Optional upload keep-awake with native wake lock and a silent-video fallback.
- Detailed upload diagnostics, runtime logging controls and a live host-only
  viewer for the latest 500 log entries.
- CLI configuration for terminal/headless operation and graceful shutdown cleanup.
- Linux x86_64 archive generation with embedded assets, branding, startup
  instructions, project license, SHA-256 checksum and archive smoke verification.

### Changed

- Unified clipboard imports into one adaptive Paste modal.
- Removed duplicate upload dialogs, redundant network controls and empty QR fields.
- Added image fit/zoom and previous/next controls, adjustable text size, dedicated
  audio presentation and full-screen mobile preview layouts.
- Improved transfer failure handling with connection deadlines and recovery for
  uncertain upload outcomes.

### Fixed

- Linux CI smoke test expectations for mandatory first-run setup, with isolated
  test ports and clearer assertion failures.

- Windows rename adapter now uses the native handle-relative API for upload
  publication and file moves, with NTSTATUS-to-I/O error conversion.

- Clipboard images being interpreted as text when mixed clipboard data is present.
- Paste-button behavior on HTTP LAN addresses by using native paste events.
- Missing folder/network setup controls and unhelpful QR readiness states.
- Files appearing before folders in List/Grid views, search and pagination.
- Settings changing size between tabs; the dialog now keeps a stable frame with
  scrollable content and persistent actions.
- Sharing becoming available before first-run setup completes.
- Failed network changes interrupting the previous configuration, with listener
  rollback and preservation of entered settings.
- Occupied default ports preventing unconfigured hosts from reaching setup, using
  a temporary local port when no explicit bind was supplied.

# Release History

## 0.1.0 — 2026-10-05

Initial development version of SplitShare: a native desktop host for sharing one
folder with browser clients on trusted local/private networks.

### Added

- Native Rust host with an embedded Vue browser interface and system tray controls
  for opening the browser/shared folder, Settings, Start/Stop sharing and Quit.
- Mandatory first-run setup for the shared folder, network interface, port and
  access policy. Settings persist across restarts; sharing starts only after setup
  succeeds. Default port is 8080.
- Live interface/port changes with automatic host-browser navigation and rollback
  on failure. Unconfigured hosts can use a temporary local port when the default
  port is occupied.
- Private share links, same-origin HttpOnly sessions, locally generated QR codes,
  link rotation/revocation, optional Open LAN mode and host-controlled permissions.
- Sandboxed file browsing, folder creation, rename and delete. Browser clients use
  virtual paths; traversal, absolute-path access and symlink escapes are rejected.
- Streaming uploads/downloads, HTTP Range support, atomic upload publication,
  conflict handling and backend-enforced upload concurrency.
- Multi-file transfer queues with aggregate progress, cancellation, retry,
  connection deadlines and recovery for uncertain upload outcomes.
- Responsive List/Grid file manager with search, selection, folder-first sorting,
  paged rows, desktop context menus and mobile action sheets.
- File-picker uploads, drag and drop, Ctrl+V and a unified Paste modal. Pasted
  images/files take priority over text; text drafts have editable filenames,
  extensions and content.
- Image, video, audio and plain-text previews, with image fit/zoom/navigation,
  media controls, text-size adjustment and download fallback. Text previews are
  limited to the first 256 KiB and render content as plain text.
- Settings with stable desktop dimensions, scrollable content and mobile layouts.
- Realtime SSE updates and browser reconnection handling.
- Optional upload keep-awake using native wake lock or a bundled silent-video
  fallback on HTTP.
- Detailed privacy-safe upload diagnostics, adjustable runtime logging verbosity
  and a live host-only viewer for the latest 500 log entries.
- CLI configuration for terminal/headless operation and graceful shutdown cleanup.
- Linux x86_64 archive tooling with embedded frontend assets, branding, startup
  instructions, project license, SHA-256 checksum and extracted-archive smoke test.
  Node.js is needed only for frontend development/builds, not at runtime.

### Known limitations

- Windows/macOS native verification remains user-owned. Physical mobile clipboard,
  media, QR and keep-awake behavior still require device testing; video playback
  does not guarantee that uploads continue when a device locks or backgrounds.
- Linux requires native GTK/AppIndicator libraries and a desktop portal for folder
  selection. Clean-distribution compatibility and third-party notices remain open;
  the local archive is a development candidate, not a published release.
- External filesystem edits require Refresh. Abrupt process termination can leave
  hidden upload partials; uploads do not resume across process restarts.
- Network discovery uses a startup snapshot; IPv6 advertisement is deferred.
  Directory metadata memory use grows with entry count.

See [known issues](ISSUES.md) for the complete limitations and
[acceptance evidence](docs/ACCEPTANCE.md) for recorded validation results.

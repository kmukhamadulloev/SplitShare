# Historical foundation decisions

This document records the initial foundation checkpoint. Its dependency choices,
phase references and runnable-scope section describe that time, not current 0.1.0
behavior or future assignments. For current behavior see [architecture](ARCHITECTURE.md),
[host setup](HOST_SETUP.md), [tray implementation](TRAY.md) and
[release notes](../RELEASE.md).

Rust MSRV is 1.98 (edition 2024), matching the installed and CI toolchain.
Cargo.lock and web/package-lock.json lock reproducible application builds.

## Dependencies

- `axum`: `0.8.9`
- `directories`: `6.0.0`
- `mime_guess`: `2.0.5`
- `rust-embed`: `8.12.0`
- `serde`: `1.0.229`
- `serde_json`: `1.0.151`
- `tempfile`: `3.27.0`
- `thiserror`: `2.0.21`
- `tokio`: `1.53.1`
- `tokio-util`: `0.7.19`
- `tower`: `0.5.3`
- `tracing`: `0.1.44`
- `tracing-subscriber`: `0.3.23`

Tokio owns async execution; Axum owns HTTP mapping. Core depends only on Serde
and thiserror. Platform owns configuration I/O. The composition root owns startup,
tracing, OS signals and the CancellationToken passed to the HTTP adapter. Shutdown
stops accepting requests and allows up to ten seconds to drain before exit.
Request URLs are not logged, to avoid future join-token disclosure.

## Configuration

Versioned JSON envelope: `{"version":1,"settings":{...}}`. Missing configuration
is created with validated defaults using a same-directory temporary file and
no-clobber publication. Existing malformed, oversized (>64 KiB), unknown-field,
or unsupported-version data fails startup without replacement. There is no legacy
migration yet; a future version must introduce an explicit migration. Settings
updates are deferred to the host-settings phase. No share root is selected or
exposed in Phase 01.

`directories::BaseDirs::data_local_dir()/SplitShare/config.json` resolves to:

- Linux: `$XDG_DATA_HOME/SplitShare/config.json`, default `~/.local/share/SplitShare/config.json`.
- Windows: `%LOCALAPPDATA%\SplitShare\config.json`.
- macOS: `~/Library/Application Support/SplitShare/config.json`.

These are native-only paths; no HTTP endpoint returns configuration.

## Assets and frontend

Build Vue before compiling Rust. rust-embed 8.12 with `debug-embed` embeds
`web/dist` in both debug and release builds. The build script tracks dist changes
and fails clearly when it is absent. Asset lookup never accesses the host filesystem.
GET/HEAD serve assets; extensionless client routes receive index.html. `/api`,
`/api/*`, `/j`, `/j/*` return safe JSON 404s, never the SPA. Missing assets return 404.

The existing Vue/Pinia/Tailwind/Vite project and relative API/join development
proxy are retained. TypeScript 5.9.3 is pinned because the scaffold's TypeScript 7
broke vue-tsc's compiler entry point. Production file-manager work is Phase 06;
prototype fixture rows and progress are not copied into production.
The canonical and web logo PNGs match (549 × 587 RGBA). Prototype source reviewed:
List/Grid, compact controls, context menu/mobile sheet, QR/settings/create/delete/
queue overlays, transfer summary and backdrop transitions.

## Native tray decision

Use standalone `tray-icon` 0.25.1 in Phase 07. It supports Windows, macOS and Linux;
it is not the Tauri runtime. Native event-loop and Linux GTK/AppIndicator
prerequisites belong to that phase. Do not add unused native dependencies before
tray implementation. Source: https://docs.rs/tray-icon/0.25.1/tray_icon/.
Embedding behavior: https://docs.rs/rust-embed/8.12.0/rust_embed/.

## Current runnable scope

Only `127.0.0.1:8080` is bound. No file/status/session/settings APIs are implemented.
Settings model future policy, not an active transfer/session service. File sharing,
LAN binding and token issuance await their ordered phases. SplitShare is designed
for trusted local/private networks, not public-internet hosting.

CI runs checks and a release build on Linux, Windows and macOS. Native Windows/macOS
results remain unverified until those runners execute; local evidence is Linux only.

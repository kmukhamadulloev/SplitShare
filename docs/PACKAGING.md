# Packaging

## Principle

Release artifacts contain a native SplitShare host executable/application with the production Vue UI embedded.

Node.js is never required by end users.

## Targets

Initial release matrix:

- Windows x86_64
- Linux x86_64
- macOS Apple Silicon
- macOS x86_64 if maintained by CI/release resources

Additional targets require explicit support/acceptance.

## Windows

Release should include:

- `splitshare.exe`;
- embedded application icon;
- no mandatory installer for the first archive release;
- optional installer only after archive startup is verified.

## Linux

Release archive includes:

- `splitshare`;
- application icon/assets as needed by desktop integration;
- optional desktop file only when installation script/package owns it cleanly.

Do not require Docker.

## macOS

Package as a standard `.app` bundle when native tray/menu identity requires it.

Signing/notarization remains a release gate when distribution policy requires it.

## Frontend embedding

Packaging fails if `web/dist/` is absent or stale relative to the requested release process.

## Release archive check

For each produced artifact:

1. unpack into clean temporary directory;
2. launch;
3. confirm listener;
4. open local WebUI;
5. share temporary directory;
6. upload and download;
7. quit gracefully;
8. ensure no dependency on repo/node_modules.

## Native tray prerequisites (Phase 07)

Linux builds require GTK3 and Ayatana AppIndicator development packages. On the
Ubuntu CI image: `libgtk-3-dev libayatana-appindicator3-dev`. Release hosts need the
corresponding GTK3/AppIndicator runtime libraries and a desktop tray/status-notifier
implementation. `--no-tray` avoids display initialization, but this desktop binary
still links GTK libraries. No Node or webview runtime is added.

The native menu embeds the branding PNG. Windows executable resources and macOS
bundle icons/identity remain packaging work for the release artifact phase.

## Linux archive checkpoint

Run `python3 scripts/package-linux.py` from a checkout with frontend build tools,
Rust and the native development libraries installed. It rebuilds the production
frontend and release executable, takes the development version from Cargo metadata,
and stages the executable, branding PNG, project license and startup instructions.
It verifies the staged archive before replacing the candidate under `release/`:
`splitshare-<version>-linux-x86_64.tar.gz` plus a SHA-256 sidecar.

The smoke test extracts into a temporary directory, launches with an empty PATH
and isolated configuration, retrieves embedded assets, uploads/downloads real bytes,
checks Range responses and exits via SIGTERM. Run it independently with
`python3 scripts/smoke-linux-archive.py release/<archive>.tar.gz`.
This uses the current machine's native libraries; it is not a clean-distribution,
tray, second-device LAN or QR-scanning acceptance claim.

The current Cargo development version is retained rather than relabeling this
candidate as v1.0. No tagging or publishing occurs. Third-party license/notice
collection and distribution compatibility must be completed before public release.

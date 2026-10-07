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

## Native tray prerequisites

Linux builds require GTK3 and Ayatana AppIndicator development packages. On the
Ubuntu CI image: `libgtk-3-dev libayatana-appindicator3-dev`. Release hosts need the
corresponding GTK3/AppIndicator runtime libraries and a desktop tray/status-notifier
implementation. `--no-tray` avoids display initialization, but this desktop binary
still links GTK libraries. No Node or webview runtime is added.

The native menu embeds the branding PNG. Windows executable resources and macOS
bundle icons/identity are not completed by the Linux archive tooling.

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
candidate as a later release. No tagging or publishing occurs. Third-party license/notice
collection and distribution compatibility must be completed before public release.

## GitHub builds and releases

The Release workflow mirrors Fetch's tag-based archive flow. Run it manually
from Actions to build downloadable artifacts without publishing a release. Push
a version tag such as `v0.1.0` to build and publish **SplitShare v0.1.0** with the
matching Added/Changed/Fixed content from `RELEASE.md`. Missing/empty/duplicate
notes and mismatched Rust/frontend/project versions fail before packaging.

The workflow first runs the reusable CI suite, then builds on native runners:

| Runner | Target | Archive |
|---|---|---|
| Ubuntu 24.04 | Linux x86_64 | `.tar.gz` |
| Windows Server 2022 | Windows x86_64 | `.zip` |
| macOS 14 | Apple Silicon aarch64 | `.tar.gz` with `SplitShare.app` |
| macOS 15 Intel | macOS x86_64 | `.tar.gz` with `SplitShare.app` |

Archive names are `splitshare-v<version>-<platform>-<architecture>`, each with a
SHA-256 sidecar. Artifacts are retained for 14 days on workflow runs; tagged release
assets persist on GitHub Releases. Only the publish job receives contents-write
permission. No personal access token is required: it uses GitHub's workflow token.
All native packaging jobs must pass before publishing; checksum verification is
repeated on the collected assets. Re-running publication for an existing release
fails rather than silently overwriting its assets or notes.

Each package includes the executable/application, logo, startup guide, version
notes, project license, third-party notice inventory and available dependency
license files. Version-pinned upstream supplements live in `docs/third-party/`
for crates omitting those files; `SOURCES.json` records their provenance. Collection
is not a legal audit of dependency terms.

`python scripts/package-release.py --target <triple>` packages a previously built
`target/<triple>/release/` executable and tests its extracted archive. Build the
frontend immediately before compiling Rust, as the workflow does. macOS packages
include Info.plist and ICNS branding and are ad-hoc signed; Developer ID signing
and notarization are not configured. Windows packages are unsigned.

Archive smoke verifies embedded assets, status, real upload/download bytes and
Range responses on every runner. POSIX hosts exit gracefully via SIGTERM; Windows
smoke terminates the test process after requests complete, so it does not verify
graceful tray Quit. Windows/macOS smoke runs only in disposable GitHub runner
accounts; Linux uses isolated XDG configuration. Native dialogs, QR scans and
tray interaction remain separate manual coverage.

To release from a checked and committed checkout, push the branch and then the
matching tag. Workflow setup does not itself push commits or create tags. For
example, after reviewing 0.1.0: `git tag v0.1.0` and `git push origin v0.1.0`.
See [archive startup instructions](RELEASE_ARCHIVES.md).

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

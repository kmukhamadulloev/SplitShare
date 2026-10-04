# SplitShare Linux archive candidate

Extract the archive and run `./splitshare`. The host opens first-run setup to
choose a shared folder, network interface, port (default 8080) and access policy.
Use the native tray to reopen the browser or quit. Only share on trusted local or
private networks. This is not a public internet file server.

The Vue UI is embedded; Node.js, a webview and Docker are not runtime dependencies.
The desktop executable needs GTK3 and Ayatana AppIndicator runtime libraries and
a desktop tray implementation. On Debian/Ubuntu these are typically supplied by
`libgtk-3-0` (or its distribution replacement) and `libayatana-appindicator3-1`.
The folder picker requires a working XDG desktop portal/backend.

For a headless host: `./splitshare --no-tray --root /path/to/share --bind 127.0.0.1:8080`.
Open http://localhost:8080 on the host. Change the interface in Settings to allow
LAN clients, then share the private QR/link. GTK libraries are still required for
this desktop binary even with `--no-tray`.

This archive is a local development candidate, not a v1.0 release. It targets the
build machine's Linux ABI; compatibility with older distributions is not proven.
Native device/tray acceptance and the third-party license/notice audit remain
release gates. The included LICENSE covers SplitShare itself.

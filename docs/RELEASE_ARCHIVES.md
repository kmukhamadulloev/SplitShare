# Run SplitShare

Extract the archive for your platform:

- Windows x64: run `splitshare.exe`.
- Linux x64: run `./splitshare`.
- macOS: open `SplitShare.app` (choose Apple Silicon or Intel to match your Mac).

First launch opens setup for the shared folder, interface, port (default 8080)
and access policy. Share the QR/link with browsers on your trusted local/private
network. The tray menu reopens the browser or quits the host. Do not expose this
application as a public internet file server.

The browser interface is embedded. Node.js and Docker are not runtime requirements.
Linux needs GTK3 and Ayatana AppIndicator runtime libraries, a desktop tray, and an
XDG portal/backend for native folder selection. The Linux build targets Ubuntu
24.04; older distributions may need newer system libraries.

For terminal use, run the executable with `--no-tray --root /path/to/share --bind
127.0.0.1:8080`. On macOS the executable is inside
`SplitShare.app/Contents/MacOS/splitshare`.

Windows packages are unsigned. macOS bundles are ad-hoc signed, not Developer ID
signed or notarized; OS security prompts may apply. Only run archives from a
source you trust. The adjacent `.sha256` file can verify downloaded bytes.

`RELEASE.md` contains changes for this version. `LICENSE`, `THIRD_PARTY_NOTICES.md`
and `licenses/` contain licensing information.

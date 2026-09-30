# HTTP and browser foundation (Phases 03–05)

## Running

Build with `npm ci --prefix web`, then `bash scripts/build-release.sh`.

```bash
# Local-only browsing of an explicitly selected root
./target/release/splitshare --root /path/to/share

# Explicit Open LAN: reachable private-network clients can browse and mutate
./target/release/splitshare --root /path/to/share --bind 0.0.0.0:8080 --open-lan

# Development: Rust + Vite, with the same-origin API proxy
./scripts/dev.sh --root /path/to/share
```

No folder is selected automatically. Without `--root`, status reports `sharing:
false` and storage endpoints return 503 `SHARE_NOT_CONFIGURED`. IPv4 listeners are
supported; IPv6 advertisement remains a later validation item. Binding to a LAN
address now supports token links by default. Explicit `--open-lan` disables the
cookie requirement but retains remote capabilities. Status reports `token_link`
or `open_lan`. See [SESSIONS.md](SESSIONS.md) for Phase 05 policy and QR selection.
SplitShare is for trusted local/private networks, not public internet serving.

## Ownership

`splitshare-application::FileService` coordinates storage, a 16-permit worker gate
and a bounded broadcast channel. Blocking filesystem calls run on blocking workers.
Mutation events are emitted inside the worker after success, even if the initiating
HTTP client has disconnected. Core remains independent of HTTP and native I/O.

`splitshare-server` handles typed extraction, error/status mapping, origin/peer
checks, Range parsing and HTTP streaming. No handler constructs native paths or
performs ad-hoc native filesystem lookup. `ReadHandle::into_file` transfers an
already sandboxed file handle to the HTTP adapter for Tokio streaming.

`splitshare-network` enumerates native interfaces with if-addrs. Socket peers are
classified using loopback and the interface snapshot obtained at startup, including
IPv4-mapped loopback. Forwarded/X-Forwarded-For and browser flags are never trusted.
Interface candidates are ranked and selectable; hotplug refresh remains future work.

## Request boundary

Host headers must match localhost or the enumerated local IPv4 authorities and
actual bound port. Origin, when present, must match that authority. JSON mutations
require `X-SplitShare-Request: 1`; no cross-origin CORS permission is provided.
This blocks browser simple-request CSRF and arbitrary-host DNS rebinding. Errors
return `{error:{code,message,details:null}}` without raw extraction or OS messages.

Only `--dev` permits `http://127.0.0.1:5173` as an extra Origin; it requires loopback
binding. Vite uses `changeOrigin: true` for the backend Host header. There is no
production proxy trust or forwarding-header authorization.

Requests use virtual paths. JSON bodies are limited to 16 KiB, downloads to 32
active streams, and SSE to 32 connections. Worker jobs are limited to 16 concurrent
blocking operations. Header parsing uses Axum/Hyper's defaults. Broader per-client
quotas and request deadlines belong to reliability work.

## Downloads

File bodies stream through at most 64 KiB application chunks. No complete file is
buffered. Responses use attachment disposition with RFC 5987 UTF-8 filename encoding,
`application/octet-stream`, nosniff, no-store, Content-Length and Accept-Ranges.
Preview MIME allowlists are not implemented.

Supported byte ranges: bounded (`bytes=2-4`), open-ended (`bytes=2-`) and suffix
(`bytes=-3`). End offsets clamp to EOF. Unsatisfiable, malformed, duplicate or
multiple ranges return 416 plus `Content-Range: bytes */size`. The supported range
count is one. Empty files return 200/zero length without Range, and 416 with Range.
HEAD ignores Range and returns full-file headers without a body. ETag/Last-Modified
validators are not yet issued, so If-Range requests conservatively receive the full
representation. Midstream read failures terminate the body; they cannot become a
JSON error after headers are sent. Shutdown cancels streams.

Range reference: [RFC 9110](https://www.rfc-editor.org/rfc/rfc9110.html#section-14).

## SSE and browser state

SSE sends `filesystem.resync` on connect/reconnect and broadcast lag. The Pinia
store reloads a snapshot. Successful mutations send `filesystem.changed` with
virtual parent `paths`. The channel retains 128 events; keepalive is 15 seconds.
No replay IDs are advertised. This is event infrastructure for application changes,
not yet a native filesystem watcher; external host edits require Refresh.

The embedded Vue browser implements real listing, breadcrumbs, search, List/Grid,
attachment download, create/rename and confirmed deletion, desktop context menu
and mobile action sheet. Dialogs use native focus trapping, Escape, focus restoration,
blur/fade and reduced-motion support. It does not display pretend transfer, QR,
settings or paste controls. Those belong to later phases. Nonempty directory deletion
still returns a typed conflict rather than recursively removing contents.

## Verification

See `docs/ACCEPTANCE.md`. Rust tests launch real Axum listeners with temporary
filesystems and real HTTP clients. Peer-forgery tests additionally inject socket
metadata into the real router. Playwright launches the actual Rust binary against
a temporary share and runs Chromium desktop/mobile workflows without mocked APIs.
Cross-target compilation does not replace Windows/macOS runtime tests or testing
from a separate physical LAN device.

Phase 04 adds the application TransferManager and raw streaming HTTP uploads,
transfer snapshots/cancellation and SSE progress. See [TRANSFERS.md](TRANSFERS.md)
and [API.md](API.md) for the implemented contract and startup host policy.

# Sessions, permissions and QR — Phase 05

SplitShare is designed for trusted local/private networks. It does not claim
public-internet security. HTTP is currently unencrypted; share links are bearer
credentials, and access to the local machine grants host control.

## Join and lifetime

At process startup, the application SessionManager generates a 256-bit share token
from the OS random source (`getrandom`). Each successful `GET /j/{token}` creates
an independent random 256-bit session ID and returns 303 to `/`, with:

```text
Set-Cookie: splitshare_session=<opaque ID>; HttpOnly; SameSite=Strict; Path=/; Max-Age=43200
```

No Domain attribute is set. Secure is omitted because the native listener serves
HTTP, not HTTPS. Cookies are host-scoped: joining a different selected IP requires
joining its link. Session validity is checked against an in-memory monotonic
12-hour expiry, not client cookie claims. Cookies do not identify user accounts.
A repeat join replaces the session identified by the request cookie. The table
holds at most 256 live sessions, removes expired entries on join, and rejects
capacity exhaustion with 429. Restart generates a new token and loses all sessions.

Invalid links return 401 without issuing a cookie. Unconfigured shares return 503.
Responses, including errors and redirects, use `Cache-Control: no-store`,
`Referrer-Policy: no-referrer` and `X-Content-Type-Options: nosniff`. Tokens/session
IDs are never written to logs or the configuration file. Only host network metadata
includes join URLs; public status and SSE do not disclose them.

`POST /api/v1/host/share-token/rotate` invalidates the previous join token and all
existing remote sessions. `POST /api/v1/session/leave` revokes the caller's session
and expires its cookie; it also works with an expired/missing cookie. Both return
204 and require `X-SplitShare-Request: 1`.

## Capabilities and host authorization

Remote token-mode clients need a valid cookie for API access. Open LAN removes
that requirement but retains the same capability checks. The application enforces
six independent booleans:

| Capability | Operations |
|---|---|
| browse | Directory listing (GET and HEAD), filesystem SSE payloads |
| download | Download/Range/HEAD |
| upload | Upload submission, transfer snapshots (GET and HEAD), transfer SSE payloads |
| create_directory | Create folder |
| rename | Rename/move |
| delete | Delete file/empty folder |

Defaults enable all except delete. Host browsers have all capabilities regardless
of remote settings. Default permissions are also applied when loading legacy
version-1 configuration without a permissions field. Disabled remote operations
return 403 `PERMISSION_DENIED`; absent/expired sessions return 401
`SESSION_REQUIRED`. Transfer cancellation still requires its independent secret
key and a valid session (unless Open LAN); upload permission is not required to
stop an already-started transfer. Transfer history is share-wide for clients with
upload permission, not a per-user history.

Host settings, network candidates and rotation endpoints require the actual socket
peer to be loopback or an address in the native local-interface snapshot. Forwarding
headers and frontend booleans never authorize them. Remote access returns 403
`HOST_CLIENT_REQUIRED` even with a valid share cookie. Only direct native serving
is supported; do not put a forwarding proxy in front of host authorization.
Existing Host/Origin allowlists, same-origin policy, mutation marker and 16 KiB JSON
limit apply. Frontend controls reflect permissions, but the server is authoritative.

## Changes during work

Successful settings changes invalidate remote stream grants. Remote uploads that
have not crossed the atomic publishing boundary are cancelled and cleaned up;
downloads stop reading and SSE emits `session.permissions_changed` then closes.
SSE reconnects under the new policy, and the UI refreshes status, removes directory
entries when browse is disabled and stops scheduling uploads when upload is denied.
Rotation, leaving and expiry also invalidate the corresponding stream grants.
Already delivered bytes cannot be recalled. Metadata operations admitted before a
change may finish, and uploads already publishing may finish atomically.

Permission-only changes preserve cookies. Changing access mode revokes sessions;
returning to token mode requires joining again. Rotation in Open LAN does not
prevent unauthenticated reconnects; enable token mode to require a link.

## Host configuration

`GET /api/v1/host/settings` returns HostSettings. `PUT` accepts a complete settings
object and returns the saved state. The application validates it and invokes the
native configuration adapter on a blocking worker. The adapter atomically replaces
the versioned config via a temporary file. If saving fails, active settings and
upload limits remain unchanged. Configuration never contains a share root or token.

Parallel upload limits are 1–32; disabling parallel uploads forces one. A changed
effective limit is accepted only when all transfers and worker permits are idle;
otherwise 409 `TRANSFERS_ACTIVE` leaves settings untouched. Permission/access-mode
updates with an unchanged effective limit are allowed during uploads. CLI upload
flags and `--open-lan` override startup config for the current process; a later
successful settings save persists the values shown in the host dialog.

## Network selection and local QR

The default listener remains `127.0.0.1:8080`. To enable reachable token sharing:

```bash
./target/release/splitshare --root /path/to/share --bind 0.0.0.0:8080
```

Open the host UI, choose **Share with QR**, then choose an address reachable by the
other device. `GET /api/v1/host/network` lists IPv4 candidates compatible with the
listener, with interface, address, kind and complete URL. Token mode URLs use
`/j/<token>`; Open LAN URLs use `/`. Private addresses rank first, followed by VPN,
link-local, other, virtual and loopback candidates. Adapter classification is a
name/address heuristic, not a reachability guarantee. VPN/virtual adapters are not
hidden. IPv6 is intentionally not advertised.

The browser lets the host select a candidate and renders its exact URL with the
bundled [qrcode library](https://github.com/soldair/node-qrcode). No external request
is made to generate QR images. Copy-link falls back to selectable text when clipboard
permission is unavailable. Loopback choices explicitly explain their host-only
reachability. Native interface hotplug still requires restarting the process.

## Evidence and limits

Application tests cover token generation, expiry, session bounds, rotation/leave,
capabilities, persistence failure and idle-only limit changes. Router tests inject
socket peers to verify remote/host policy, forged headers, all six independent
capabilities, cookies, CSRF and revocation of SSE/download/upload streams. These
synthetic-peer tests supplement actual HTTP/browser tests; they do not claim a
separate-device LAN test.

Desktop/mobile Chromium tests exercise settings, candidate selection, cookie
redirect/HttpOnly behavior, token rotation, and locally decode the rendered QR
pixels using jsQR. The Linux standalone smoke checks settings across restart,
restart invalidation and absence of secrets from logs/config. Native Windows/macOS
runtime, physical-device QR scanning and LAN/VPN reachability still require testing.

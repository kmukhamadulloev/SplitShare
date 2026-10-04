# HTTP API

Phases 03–08 implement file operations, streaming transfers, SSE, token sessions,
remote permissions, host settings and network candidates. Preview remains planned.
Exact schemas are in [openapi.yaml](openapi.yaml). See [SESSIONS.md](SESSIONS.md)
for authorization, expiry, revocation and persistence policy.

Implemented mutation bodies:

- Create: `{ "parent": "/", "name": "New folder" }` → 201 `{ "path": "/New folder" }`.
- Rename: `{ "from": "/old", "to": "/new" }` → 204; never overwrites.
- Delete: `{ "path": "/item" }` → 204; nonempty directories return 409.

All mutations require `X-SplitShare-Request: 1`. File metadata mutations use JSON;
uploads use a raw file body and cancellation has no body. Invalid/oversized bodies
return safe typed errors. Unknown endpoints return JSON 404 after authorization, never SPA HTML.

Base:

```text
/api/v1
```

All normal errors:

```json
{
  "error": {
    "code": "PATH_NOT_FOUND",
    "message": "The requested item no longer exists.",
    "details": null
  }
}
```

Never expose internal paths or raw stack traces.

## Status

```text
GET /api/v1/status
```

Returns:

- version;
- share running state;
- virtual root label;
- requesting client locality;
- current effective share mode;
- upload concurrency policy.

## Files

```text
GET    /api/v1/files?path=/
POST   /api/v1/directories
POST   /api/v1/files/rename
DELETE /api/v1/files
```

Requests and responses use virtual paths only.

## Upload

```text
POST /api/v1/uploads?path=/destination
```

One raw file per request with `Content-Type: application/octet-stream`.
Required headers: `X-SplitShare-Request: 1`, `X-Transfer-ID` and `X-Transfer-Key`.
Generate independent cryptographically random 128-bit ID/key values as 32 hex
characters. Keys authorize cancellation and never appear in snapshots or SSE.
Optional `policy` query: `ask` (default), `reject`, `replace`, `auto_rename`.
Ask/reject conflicts return 409; the browser asks before retrying with another policy.
Optional Content-Length supplies the expected byte count. Success is 201 with a
completed Transfer only after atomic publication. JSON's 16 KiB limit does not
apply to uploads. Capacity exhaustion returns 429; body failure returns a safe error.

`GET /api/v1/transfers` returns at most 128 transfer snapshots.
`DELETE /api/v1/transfers/{id}` requires the mutation header and transfer key;
202 requests cancellation, 403 rejects a wrong key, 404 means missing, and 409
means publication has begun or the transfer is already terminal. Observe the
terminal snapshot before assuming cleanup has completed.

See [TRANSFERS.md](TRANSFERS.md) for states, limits and host policy.

The frontend handles multi-selection as a queue; server concurrency policy remains authoritative.

## Download / preview

```text
GET /api/v1/files/download?path=/file.ext
GET /api/v1/files/preview?path=/file.ext
```

Download supports Range.

Planned preview will allow only browser-safe types and may use the same Range-backed file response with stricter content policy.

No transcoding.

## Clipboard text

No separate persistence domain is required.

The frontend converts approved draft text into a file creation/upload request.

## Sessions

```text
GET /j/{token}
POST /api/v1/session/leave
```

Host-only token rotation (204; revokes old joins and remote sessions):

```text
POST /api/v1/host/share-token/rotate
```

## Settings — host only

```text
GET /api/v1/host/settings
PUT /api/v1/host/settings
GET /api/v1/host/network
```

Remote clients receive 403 `HOST_CLIENT_REQUIRED`, including for GET. PUT accepts
all HostSettings fields: share_mode (`token_link`/`open_lan`), permissions (six
booleans), parallel_uploads_enabled and max_parallel_uploads (1–32). Returns the
persisted settings, or 409 while transfers prevent a concurrency change.

Network response: `{ "candidates": [{ "interface": "eth0", "address": "192.168.1.20",
"kind": "private", "url": "http://192.168.1.20:8080/j/<opaque-token>" }] }`.
Only compatible IPv4 candidates are returned. Select a candidate; do not assume
the first interface is reachable by the intended client.

A valid join returns 303, same-origin HttpOnly/SameSite=Strict cookie, and Location
`/`. Invalid token returns 401; sessions expire after 12 hours or rotation/restart.
Leave is a marked POST returning 204 and an expired cookie. Remote API requests
require the cookie in token mode and appropriate capabilities in either mode.

## Realtime

```text
GET /api/v1/events
```

SSE event names:

- `filesystem.changed`
- `transfer.created`
- `transfer.updated`
- `filesystem.resync`
- `transfer.resync`
- `session.permissions_changed`

Resync events require fetching the corresponding snapshot. Transfer created/updated
payloads are Transfer objects. Session change events contain `{}`; refresh status.
Remote streams close on policy changes and reconnect under current authorization.

Event payloads are typed JSON.

## Host locality

`status` may expose `local_client: true|false`, but authorization is computed server-side for every host-only operation.

## Transport limits — Phase 08

The HTTP/1 listener accepts at most 128 simultaneous connections. Excess sockets
are closed before request tasks are allocated. Request headers must arrive within
10 seconds, contain at most 64 fields, and fit the 16 KiB application metadata
budget (request line plus decoded headers). Hyper also has a 32 KiB parser buffer
setting. Parser rejection may close the socket or return a plain HTTP error;
parsed oversized metadata returns 431 `HEADER_LIMIT`.

Authorized control/mutation bodies are limited to 16 KiB and a 10-second total
read deadline: 413 `CONTROL_BODY_LIMIT` or 408 `CONTROL_BODY_TIMEOUT`. These limits
do not buffer or cap raw upload bodies. Upload queue waits expire after 60 seconds
with 408 `UPLOAD_QUEUE_TIMEOUT`; input idle periods expire after 60 seconds with
408 `UPLOAD_IDLE_TIMEOUT`. Both produce failed transfer snapshots without publication.
A lost response alone does not establish whether publication succeeded; reconcile
`/transfers` and the destination before explicitly retrying.

## Native host setup

`GET /api/v1/host/setup` returns `{bind_ip, port, folder_selected, interfaces,
state, message, local_url}`. Interfaces contain `{address,label}`. No native paths
are included. State is `ready`, `selecting`, `applying`, `cancelled` or `failed`.

`POST /api/v1/host/folder` (no body) requests the native host folder dialog.
`PUT /api/v1/host/setup` accepts only `{ "bind_ip": "0.0.0.0", "port": 8080 }`;
choose an advertised interface and a port from 1–65535. Both require the mutation
header and return 202 plus the current setup snapshot after enqueueing. Poll GET
to establish the actual outcome. A changed port may require opening the updated
host address. Queue/busy or unavailable interface/port validation returns 409
`HOST_SETUP_REJECTED`; malformed network JSON returns 400 `INVALID_NETWORK_SETTINGS`.
A nonempty folder-selection body returns 400 `INVALID_REQUEST`.
Servers without the native controller return 503 `HOST_CONTROL_UNAVAILABLE`.
Remote peers always receive 403, including reads and folder-dialog requests.

Binding/selection failures appear as a safe `failed` snapshot, not a false success.
Cancel/unavailable picker produces `cancelled` with explanatory text. Applying a
change cancels active transfers, revokes sessions and persists native startup
configuration. See [HOST_SETUP.md](HOST_SETUP.md) for lifecycle and recovery details.

## Host logging

All `/api/v1/host/logs` routes require actual host-local socket classification,
including GET/HEAD and SSE. Remote clients receive 403 even with a valid session
or spoofed forwarding headers. Responses are no-store. Mutations require the
existing same-origin checks and `X-SplitShare-Request: 1`.

- `GET /api/v1/host/logs`: oldest-first array of at most 500 entries. Each entry is
  `{id,timestamp_ms,level,target,message,fields}`. IDs are monotonic within a run;
  `fields` is a bounded map of approved diagnostic strings. No paths/keys/cookies.
- `DELETE /api/v1/host/logs`: clears in-memory history; 204. IDs are not reset.
- `GET /api/v1/host/logs/config`: `{level,available,capacity:500}`. `level:null`
  means the startup environment/default filter remains active.
- `PUT /api/v1/host/logs/config`: `{ "level":"warn"|"info"|"debug" }`. Reloads the
  running console/history filter and returns config. Changes are process-local,
  survive listener restart, and reset on process restart. No session revocation.
  Invalid JSON/levels/unknown fields return 400 INVALID_REQUEST; unavailable runtime
  control returns 503 LOGGING_UNAVAILABLE. The existing control-body limits apply.
- `GET /api/v1/host/logs/events`: SSE `logs.changed` notifications with `{}` data.
  Sent immediately on subscribe, history/config changes and receiver lag. Clients
  refetch the bounded snapshot, also after reconnect. Shares the existing 32-stream
  limit (429 EVENT_LIMIT) and exits on listener shutdown. It works before a folder
  is selected and never publishes logs through remote filesystem SSE.

## File previews

`GET/HEAD /api/v1/files/preview?path=/virtual/file` requires Download capability,
including the token session rules. It uses the download stream/range implementation
and shared 32-stream limit. HEAD returns full length and MIME without a body;
GET supports a single Range and the existing 416/If-Range rules.

Approved extensions receive an allowlisted MIME, inline Content-Disposition,
no-store, nosniff and `Content-Security-Policy: sandbox; default-src 'none'`.
Unsupported extensions return 415 PREVIEW_UNAVAILABLE. Text/source/HTML/XML are
served only as plain UTF-8 text. The browser reads at most 256 KiB for its text view;
the endpoint itself remains a normal bounded stream. See [PREVIEWS.md](PREVIEWS.md).


## Mandatory initial setup

Host setup snapshots include `setup_required`. `folder_selected` may describe a
staged native sandbox while `setup_required:true`; it does not mean files are shared.
`POST /api/v1/host/folder` stages selection in memory during initial setup and
continues to apply immediately for an already configured host.

`POST /api/v1/host/setup/complete` accepts
`{network:{bind_ip,port},settings:HostSettings}` with the existing strict settings
schema. It is host-only, same-origin and mutation-marker protected. A native folder
must already be staged; interface/port and settings must validate. Returns 202 plus
a setup snapshot, never an immediate success claim. Poll `/host/setup` until ready
with `setup_required:false`, or a failed/cancelled state. Invalid JSON returns 400;
invalid state/network or a pending operation returns 409 HOST_SETUP_REJECTED.
`PUT /host/setup` cannot bypass initial setup and returns 409 until completion.

While setup is required, GET/HEAD `/status` may be read without a join session and
reports `sharing:false`, generic root label and no file permissions. Host-only
routes retain socket authorization. File/transfer/filesystem-SSE APIs return 503
SETUP_REQUIRED for both local and remote clients, including the short interval
between listener startup and final persistence. Normal session rules resume after
completion. No browser paths, native root values or persistent configuration paths
are exposed. See [HOST_SETUP.md](HOST_SETUP.md) for restart and rollback behavior.

Normal `PUT /host/settings` returns 409 HOST_SETUP_REQUIRED during initial setup
or an applying lifecycle change, preventing conflicting policy writes.

# HTTP API

Phases 03–05 implement file operations, streaming transfers, SSE, token sessions,
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

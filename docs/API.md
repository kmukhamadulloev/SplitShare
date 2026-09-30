# HTTP API

Phases 03–04 implement status, directory listing/create/rename/delete, streamed
downloads with Range, uploads, transfer snapshots/cancellation and SSE. Previews,
sessions and host settings endpoints below remain planned. The exact implemented schemas and response statuses are in
[openapi.yaml](openapi.yaml). Startup/access policy: [HTTP_BROWSER.md](HTTP_BROWSER.md).

Implemented mutation bodies:

- Create: `{ "parent": "/", "name": "New folder" }` → 201 `{ "path": "/New folder" }`.
- Rename: `{ "from": "/old", "to": "/new" }` → 204; never overwrites.
- Delete: `{ "path": "/item" }` → 204; nonempty directories return 409.

All mutations require `X-SplitShare-Request: 1`. File metadata mutations use JSON;
uploads use a raw file body and cancellation has no body. Invalid/oversized bodies
return safe typed errors. Unknown endpoints return JSON 404, never SPA HTML.

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

Preview exists only for allowlisted browser-safe types and may use the same Range-backed file response with stricter content policy.

No transcoding.

## Clipboard text

No separate persistence domain is required.

The frontend converts approved draft text into a file creation/upload request.

## Sessions

```text
GET /j/{token}
POST /api/v1/session/leave
```

Host-only session management may later include:

```text
POST /api/v1/host/share-token/rotate
```

## Settings — host only

```text
GET /api/v1/host/settings
PUT /api/v1/host/settings
GET /api/v1/host/network
```

Remote clients receive `HOST_CLIENT_REQUIRED`.

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

Resync events require fetching the corresponding snapshot. Transfer created/updated
payloads are Transfer objects. Session events remain planned.

Event payloads are typed JSON.

## Host locality

`status` may expose `local_client: true|false`, but authorization is computed server-side for every host-only operation.

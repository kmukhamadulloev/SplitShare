# HTTP API

Phase 01 status: all feature endpoints below are planned. Requests currently
return a safe `NOT_FOUND` JSON envelope with HTTP 404. Only frontend assets are served.

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

One file per request.

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
- `transfer.removed`
- `session.permissions_changed`

Event payloads are typed JSON.

## Host locality

`status` may expose `local_client: true|false`, but authorization is computed server-side for every host-only operation.

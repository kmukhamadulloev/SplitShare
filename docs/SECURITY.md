# Security

## Threat model

SplitShare is intended for private/local networks but must still assume another LAN client can be malicious.

Protect against:

- path traversal;
- symlink/reparse-point escape;
- oversized metadata/control requests;
- malformed Range headers;
- filename injection;
- unauthorized mutation;
- token guessing;
- host settings mutation by remote clients;
- accidental absolute-path leakage;
- resource exhaustion through uncontrolled parallel uploads.

## Session modes

### Token link — default

Generate at least 128 bits of cryptographically secure randomness.

Join:

```text
GET /j/<token>
```

On success issue an HttpOnly same-origin cookie and redirect to `/`.

Tokens are never logged in full.

Token rotation invalidates prior join links.

### Open LAN — optional

No token requirement, but normal permission capability checks remain active.

The UI must clearly show that any reachable allowed client may connect.

### Manual approval

Not required for initial release unless the active phase explicitly implements the complete approval flow. Do not expose a non-functional setting.

## Cookies

Recommended:

- HttpOnly;
- SameSite=Strict;
- Path=/;
- Secure only when HTTPS is actually active.

## Mutation requests

Require same-origin session plus normal capability authorization.

Do not rely on hidden buttons as authorization.

## Filesystem

See `STORAGE.md`.

Traversal/symlink escapes are release blockers.

## Error responses

Return stable codes and user-safe messages.

Do not expose:

- stack traces;
- absolute paths;
- raw OS errors containing private paths;
- token values.

Retain detailed diagnostics in host logs.

## Resource limits

Define bounded limits for:

- request headers;
- JSON bodies;
- filename length;
- text-draft body;
- simultaneous uploads;
- SSE connections per source/client if needed.

Do not impose an arbitrary file-size ceiling unless required by platform constraints; streaming should be the normal path.

## Security acceptance

Mandatory before v1:

- traversal fuzz/regression tests;
- encoded traversal cases;
- Windows path edge cases;
- symlink/reparse-point tests on native runners;
- token entropy and invalidation tests;
- remote host-settings denial;
- concurrency-limit bypass attempt;
- malformed Range tests;
- absolute-path redaction tests.

## Implemented Phase 05 policy

See [SESSIONS.md](SESSIONS.md) for 256-bit token/cookie generation, 12-hour expiry,
256-session capacity, rotation invalidation, default remote permissions and
socket-derived host authorization. Token mode is active for remote API access;
Open LAN retains capability enforcement. Local host control is intentionally full.

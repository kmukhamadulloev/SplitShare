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

## Phase 08 resource hardening

Transport policy is specified in [API.md](API.md#transport-limits--phase-08).
Connection identity is attached directly from the accepted socket, never forwarded
headers. There are 128 HTTP connection permits, 32 download permits, 32 SSE permits,
128 transfer records and 1–32 upload workers (one when parallel mode is disabled).
Slow metadata requests and queued/idle uploads have deadlines. Shutdown drains
HTTP connections for at most five seconds; the host retains its outer drain limit.
These bounds limit allocation, not availability against every malicious LAN peer:
there is no per-IP fairness quota and a client can occupy slots until its deadline.

Raw parser errors, request URLs, cookies, transfer keys and filesystem paths are
not logged by the transport. Upload warnings report stable failure codes and safe
recovery suggestions. Existing token/config redaction, path/symlink race tests,
remote host authorization and hostile-client concurrency regressions remain
mandatory. This does not make SplitShare a public-internet server.

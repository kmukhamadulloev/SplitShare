# Phase 02 — Storage Sandbox

> Archived historical plan. Superseded as a work assignment; see [archive status](README.md).

## Objective

Implement the secure virtual filesystem root.

## Deliverables

- `VirtualPath`;
- native storage adapter;
- directory listing;
- metadata;
- mkdir;
- rename;
- delete;
- streamed read handle;
- temporary upload writer/publisher;
- conflict policy;
- symlink/reparse rejection;
- absolute-path redaction.

## Security cases

Must test traversal, encoding, separators, Windows paths, symlinks/reparse points and root deletion/rename boundaries.

## Acceptance

- clients cannot escape root;
- absolute host paths never appear in API/domain-facing serialization;
- interrupted upload cannot publish a completed destination;
- operations are tested on temporary filesystems;
- platform-specific link tests exist where necessary.


## Implementation evidence — 2026-09-30

| Acceptance | Result |
|---|---|
| Clients cannot escape root | PASS on Linux: traversal, links and concurrent link-swap regressions |
| Absolute host paths absent from serialization | PASS: domain metadata/error serialization regression |
| Interrupted uploads cannot publish completed destinations | PASS: drop/cancel, poisoned writer, exact-length and namespace tests |
| Operations tested on temporary filesystems | PASS: 11 storage integration tests on Linux |
| Platform-specific link tests exist | PASS: Unix symlink/socket/race tests and Windows reparse tests; cross-target compilation passes |

`bash scripts/check.sh` passes all 19 workspace tests, frontend checks, format and
Clippy. Windows/macOS storage cross-target Clippy also passes. Native runtime
validation on those operating systems remains pending CI; this is not a
cross-platform release sign-off. Exact commands are in `docs/ACCEPTANCE.md`.

Storage deletion currently covers files and empty directories. API integration,
recursive deletion behavior, transfer manager, SSE and permission enforcement are
not claimed as implemented by this library milestone. See `docs/STORAGE.md`.

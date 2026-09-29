# Phase 02 — Storage Sandbox

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

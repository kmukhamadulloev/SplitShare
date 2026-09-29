# Phase 05 — Sessions, Permissions and QR

## Objective

Implement private-network access control and share capabilities.

## Deliverables

- cryptographic share token;
- `/j/<token>` join flow;
- HttpOnly session cookie;
- token rotation;
- optional Open LAN mode;
- capability model:
  - browse
  - download
  - upload
  - create directory
  - rename
  - delete
- host-only settings authorization;
- network address candidates;
- QR generation in browser using local code/library;
- share URL selection.

## Acceptance

- invalid token cannot join;
- rotated token invalidates old joins according to documented session policy;
- remote browser cannot modify host settings;
- permission-disabled actions are rejected by backend even if manually requested;
- QR requires no external service;
- correct LAN URL can be selected when multiple adapters exist.

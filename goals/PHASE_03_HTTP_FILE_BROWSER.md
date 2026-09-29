# Phase 03 — HTTP and File Browsing

## Objective

Expose the real storage sandbox through a typed Axum API and embedded browser client foundation.

## Deliverables

- `/api/v1/status`;
- list directory;
- create directory;
- rename;
- delete;
- download;
- HTTP Range;
- typed errors;
- SSE infrastructure;
- embedded Vue SPA fallback;
- host-local classification;
- initial browser store/API layer.

## Acceptance

- remote browser lists real temporary test files;
- file paths are virtual;
- Range behavior is standards-correct for supported cases;
- malformed Range returns 416;
- routes remain thin;
- integration tests use real Axum + temporary filesystem.

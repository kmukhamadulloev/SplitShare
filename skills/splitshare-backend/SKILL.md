---
name: splitshare-backend
description: Use when implementing Rust backend behavior: Axum API, storage sandbox, sessions, transfers, SSE, network policy, configuration, or graceful shutdown.
---

# SplitShare Backend Skill

## Before work

Read:

- `docs/ARCHITECTURE.md`
- `docs/STORAGE.md`
- `docs/API.md`
- `docs/TRANSFERS.md`
- `docs/SECURITY.md`
- active goal

## Non-negotiable

- stream files;
- never expose absolute paths;
- deny root escape;
- deny symlink/reparse escape;
- backend enforces upload concurrency;
- permissions enforced server-side;
- host-only settings enforced server-side;
- no arbitrary shell execution;
- no fake progress.

## API style

Use typed requests/responses and stable error codes.

Internal OS errors may be logged with context but user responses must be safe.

## Long-running work

Managers/services own lifecycle. HTTP handlers should initiate or observe work, not own hidden background tasks directly.

## Tests

Every filesystem/security bug fix must add a regression test.

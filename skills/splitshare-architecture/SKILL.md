---
name: splitshare-architecture
description: Use when changing SplitShare crate boundaries, data flow, domain contracts, application services, or native/frontend ownership.
---

# SplitShare Architecture Skill

Read `AGENTS.md`, `GOAL.md` and `docs/ARCHITECTURE.md` first.

## Rules

- Domain rules belong in `splitshare-core`.
- Filesystem safety belongs in `splitshare-storage`.
- Interface/address policy belongs in `splitshare-network`.
- HTTP mapping belongs in `splitshare-server`.
- tray/OS operations belong in `splitshare-platform`.
- wiring belongs in `app/splitshare`.
- browser behavior belongs in `web`.

Dependency direction must not make core depend on HTTP, Vue or tray.

Routes call application services. They do not implement filesystem policy inline.

Do not add a database, desktop webview framework, cloud service, or second backend runtime without an approved goal.

When a change crosses boundaries, document the ownership decision in `docs/ARCHITECTURE.md`.

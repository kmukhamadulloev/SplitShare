# Phase 01 — Foundation

> Archived historical plan. Superseded as a work assignment; see [archive status](README.md).

## Objective

Create the real monorepo foundation and architecture boundaries before feature work.

## Deliverables

- finalize Cargo workspace;
- add actual shared dependencies with deliberate versions;
- establish config model and application-data paths;
- establish typed core errors/domain contracts;
- establish tracing;
- wire minimal Axum startup without fake feature routes;
- establish Vue production project;
- establish development proxy;
- establish frontend embedding mechanism;
- establish CI/check scripts;
- establish native shutdown token/lifecycle boundary;
- validate logo/prototype assets.

## Required design decisions

Document:

- Rust minimum supported version;
- dependency choices;
- config format/versioning;
- embedded asset strategy;
- tray library;
- graceful shutdown ownership;
- application-data paths per OS.

## Acceptance

- `cargo fmt --check` passes;
- `cargo clippy --workspace --all-targets -- -D warnings` passes;
- `cargo test --workspace` passes;
- frontend typecheck/build passes;
- release-mode Rust build can embed the built frontend;
- no fake file APIs are exposed;
- docs reflect actual chosen dependencies.

## Implementation evidence — 2026-09-30

Foundation implemented and locally verified on Linux with Rust/Cargo 1.98.

| Acceptance criterion | Result |
|---|---|
| `cargo fmt --check` | PASS |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | PASS |
| `cargo test --locked --workspace` | PASS: 3 unit + 3 HTTP/lifecycle integration tests |
| Frontend typecheck/build | PASS |
| Release embeds built frontend | PASS: build and standalone-binary smoke test |
| No fake file APIs | PASS: API/join isolation regression tests |
| Dependency decisions documented | PASS: `docs/FOUNDATION.md` |

Additional validation: `bash -n scripts/*.sh`; `python3 scripts/smoke-foundation.py`;
canonical/web/prototype PNG equality. Browser interaction E2E and sandbox/transfer
security suites are not applicable until those features exist. Windows/macOS CI
is configured but not executed locally. Phase 02 is the next implementation scope;
no storage or sharing feature is claimed by this foundation milestone.

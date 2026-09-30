# Phase 04 — Transfers and Clipboard

## Objective

Implement real uploads, bounded concurrency and clipboard workflows.

## Deliverables

- streaming single-file upload endpoint;
- TransferManager;
- backend semaphore;
- host parallel-upload settings;
- truthful progress/SSE;
- cancellation;
- aggregate progress model;
- browser queue;
- drag/drop;
- file paste;
- image paste preview;
- text draft and save;
- conflict modal/policy;
- retry from beginning.

## Acceptance

- two files can upload concurrently when limit=2;
- third file queues;
- disabling parallel uploads forces limit=1;
- modified client cannot exceed backend limit;
- partial files are cleaned on cancel/failure;
- aggregate byte progress is correct;
- large files do not cause proportional memory growth.

## Phase 04 local acceptance — 2026-09-30

| Criterion | Result | Evidence |
|---|---|---|
| Two files run with limit 2 | PASS | Application and real HTTP concurrency tests |
| Third file queues | PASS | Delayed body streams retain two permits and queue the third |
| Parallel disabled forces 1 | PASS | Application test with configured maximum 8 and parallel disabled |
| Modified client cannot exceed limit | PASS | Direct concurrent HTTP requests, independent of browser scheduler |
| Cancel/failure cleans partial files | PASS | Cancel, disconnect, shutdown and length-failure tests; browser cancel/retry |
| Aggregate byte progress is correct | PASS | Weighted, unknown-size, zero-byte and cancelled-row aggregate tests |
| Large uploads avoid proportional memory growth | PASS | Linux release process: 8 MiB / 256 MiB uploads, peak RSS 6.89 / 9.10 MiB |

Validation: `bash scripts/check.sh` passed formatting, Clippy with warnings denied,
36 Rust tests, frontend typecheck, aggregate unit tests and production build.
`npm run test:e2e --prefix web` passed six Chromium desktop/mobile tests against
real embedded Rust serving, including uploads, clipboard drafts/previews, conflicts,
active cancellation and restart-from-zero retry. `bash scripts/build-release.sh`,
`python3 scripts/smoke-foundation.py` and `python3 scripts/test-upload-memory.py`
passed. Workspace/all-target cross-checks passed for x86_64-pc-windows-msvc and
x86_64-apple-darwin. Native Windows/macOS runtime, separate-device LAN and native
clipboard permissions across browsers remain unverified; this is local acceptance,
not v1 release acceptance.

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

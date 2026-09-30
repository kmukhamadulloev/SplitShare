# Current Goal — Phase 04 Transfers and Clipboard

The active implementation goal is:

`goals/PHASE_04_TRANSFERS_CLIPBOARD.md`

## Product release program

SplitShare v1.0 is delivered through ordered phases:

1. Foundation
2. Storage sandbox
3. HTTP + file browsing
4. Transfers + clipboard
5. Sessions + permissions + QR
6. Production WebUI
7. Desktop tray + host lifecycle
8. Reliability + security + performance
9. Packaging + v1.0 acceptance

Do not skip phases by fabricating later behavior. A phase may prepare interfaces needed by later work, but it must leave the repository internally consistent and tested.

## Scope boundary

Current release scope:

- Windows host
- Linux host
- macOS host
- browser clients on desktop/mobile
- local/private networks

Not in current v1.0 implementation:

- Android host app
- iOS host app
- public relay
- cloud service
- account system
- internet tunnel

Future mobile host work may reuse the Rust core through a native library, but current architecture must not be distorted to implement it prematurely.

## Latest checkpoint

Phase 04 implementation and local acceptance passed: streamed uploads, backend
concurrency/progress/cancellation, browser queue and clipboard workflows.
See the active phase for individual criteria and evidence. Phase 05 is next and
has not started. Native Windows/macOS runtime validation remains pending.

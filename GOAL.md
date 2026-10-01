# Current Goal — Phase 08 Reliability, Security and Performance

The active implementation goal is:

`goals/PHASE_08_RELIABILITY_SECURITY_PERFORMANCE.md`

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

Phase 08 local implementation and acceptance are complete; see
[acceptance evidence](docs/ACCEPTANCE.md#phase-08-local-acceptance--2026-10-01).
Delivered bounded transport/transfer resources, truthful interruption recovery,
security regressions, measured 1 GiB and directory performance, browser engine
coverage and CI checks. Phase 09 packaging is next; it has not been started.

Native Windows/macOS validation is user-owned manual follow-up and does not block
this phase, as authorized on 2026-10-01. Unexecuted native checks remain recorded,
not marked PASS. Remaining release follow-ups are tracked in ISSUES.md.

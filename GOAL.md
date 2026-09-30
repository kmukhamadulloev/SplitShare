# Current Goal — Phase 07 Desktop Tray and Host Lifecycle

The active implementation goal is:

`goals/PHASE_07_DESKTOP_TRAY_LIFECYCLE.md`

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

Phase 07 native tray/lifecycle implementation checkpoint is delivered, with local
Linux menu and failure-recovery validation. Phase 07 remains active: its mandatory
native Windows/macOS smoke gate is not yet satisfied. See `docs/TRAY.md` and
`docs/ACCEPTANCE.md` for evidence and remaining native OS-action checks.
Phase 08 has not started.

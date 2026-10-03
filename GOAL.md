# Current Goal — Host setup correction

The active implementation goal is:

`goals/HOST_SETUP_CORRECTION.md`

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

The user's first-run report exposed missing folder/network setup controls that
previous Phase 08 tests did not cover. The host setup correction is implemented and locally verified before
Phase 09 packaging: native folder selection, persistent interface/port controls,
honest QR readiness and first-run guidance now have integration/browser coverage.
See docs/HOST_SETUP.md and the correction section of docs/ACCEPTANCE.md.

Phase 08 backend hardening remains delivered. Windows/macOS native checks remain
user-owned; no unexecuted checks are marked PASS.

Clipboard refinement: Paste now always opens one editable modal. A native paste
turns it into image confirmation or a text draft with filename/extension editing.
Ctrl+V remains supported. Upload opens the native file picker on all devices;
duplicate upload sheets, picker alternatives and automatic clipboard reads are
removed. Physical iPad/Android paste-menu behavior remains a device acceptance
check. See docs/ACCEPTANCE.md for validation.

Interface cleanup: network settings now use one Save changes action, without a
duplicate apply button or address summary. QR hides unavailable fields and empty
notices while preserving error recovery.

File browsing now groups folders before files in both views, before search and
pagination, preserving name order within each group.

Upload keep-awake is opt-in through the transfer bar: native wake lock where
available, bundled silent-video fallback on HTTP. Actual sleep prevention on
physical phones/tablets remains device-dependent and must be manually verified.

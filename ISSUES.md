# Known Issues / Open Decisions

This file tracks real unresolved engineering work. Remove items only when verified.

## Foundation

- Decisions and resolved dependency choices: [FOUNDATION.md](docs/FOUNDATION.md).
- [ ] Run configured native Windows/macOS CI checks; only Linux is locally verified.
- [ ] Complete Phase 07 native Windows/macOS tray smoke; Linux native menu/lifecycle smoke is implemented.
- [ ] Manually validate browser/folder openers and clipboard on each desktop, including Wayland-only Linux.
- Tray dependency is pinned to 0.24.2 because 0.25.1 requires unavailable `dirs ^7` with libappindicator; see `docs/TRAY.md`.

## Storage

- [ ] Validate the handle-relative Windows rename/replace implementation on native NTFS; implementation cross-compiles but has not run locally.
- [ ] Confirm filesystem behavior when the shared folder resides on a network filesystem.
- Unicode policy resolved: preserve bytes; native filesystem decides case/normalization conflicts (see `docs/STORAGE.md`).
- [ ] Execute native Windows/macOS link, reparse, and rename tests; Linux tests and Windows/macOS cross-target Clippy pass.

## Networking

- [ ] Validate interface ranking with Docker, WireGuard, Tailscale and common VM adapters.
- [ ] Define IPv6 advertisement behavior before enabling IPv6 by default.
- [ ] Confirm host-client classification on all supported bind modes.

## HTTP/browser foundation

- [ ] Native filesystem changes do not yet emit SSE; Refresh reloads them.
- [ ] Interface classification/allowed authorities use a startup snapshot; hotplug refresh remains future work.
- [ ] Validate separate-device LAN flows and native Windows/macOS Phase 03 runtime behavior.
- Phase 05 token sessions, granular remote permissions and host-only settings are implemented; see `docs/SESSIONS.md`.

## Browser

- [ ] Run manual screen-reader checks and native mobile touch/keyboard checks; Phase 06 automated Chromium accessibility and viewport checks are local coverage.

- [ ] Validate Clipboard API behavior on Chromium, Firefox and Safari.
- [ ] Validate multi-file browser download UX; no ZIP generation is planned for v1.
- [ ] Define preview allowlist by MIME and browser capability.

## Packaging

- [ ] Windows signing strategy is not defined.
- [ ] macOS signing/notarization strategy is not defined.
- [ ] Linux packaging formats beyond release archive are not defined.

## Transfers

- [ ] Validate native Windows/macOS upload publication and cancellation at runtime.
- [ ] Add crash-recovery scavenging for hidden partial files after abrupt process death.
- [ ] Run 1 GiB and multi-device transfer stress tests for release acceptance.
- Runtime upload limits now update through host-only settings when the transfer queue is idle.

## Sessions and QR

- [ ] Validate physical-device QR scanning and private LAN/VPN routing across supported hosts.
- [ ] Validate native Windows/macOS settings replacement and cookie/browser flows at runtime.
- [ ] Verify clipboard copy and join redirects in Firefox/Safari; Chromium desktop/mobile is locally covered.

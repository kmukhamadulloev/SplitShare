# Known Issues / Open Decisions

This file tracks real unresolved engineering work. Remove items only when verified.

## Foundation

- Decisions and resolved dependency choices: [FOUNDATION.md](docs/FOUNDATION.md).
- [ ] Run configured native Windows/macOS CI checks; only Linux is locally verified.
- [ ] Validate tray-icon 0.25.1 native event loops and Linux desktop prerequisites in Phase 07.

## Storage

- [ ] Confirm Windows rename/replace semantics for atomic upload publishing.
- [ ] Confirm filesystem behavior when the shared folder resides on a network filesystem.
- [ ] Define exact Unicode normalization policy for conflict detection.
- [ ] Validate symlink/reparse-point rejection on each desktop OS.

## Networking

- [ ] Validate interface ranking with Docker, WireGuard, Tailscale and common VM adapters.
- [ ] Define IPv6 advertisement behavior before enabling IPv6 by default.
- [ ] Confirm host-client classification on all supported bind modes.

## Browser

- [ ] Validate Clipboard API behavior on Chromium, Firefox and Safari.
- [ ] Validate multi-file browser download UX; no ZIP generation is planned for v1.
- [ ] Define preview allowlist by MIME and browser capability.

## Packaging

- [ ] Windows signing strategy is not defined.
- [ ] macOS signing/notarization strategy is not defined.
- [ ] Linux packaging formats beyond release archive are not defined.

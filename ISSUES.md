# Known Issues / Open Decisions

This file tracks real limitations, unresolved decisions and unverified checks in
0.1.0. It is not an assigned backlog or a roadmap to 1.0.0. Remove items only when
verified; schedule work only when its scope is chosen.

## Foundation

- Decisions and resolved dependency choices: [FOUNDATION.md](docs/FOUNDATION.md).
- [ ] Run configured native Windows/macOS CI checks; only Linux is locally verified.
- [ ] User-owned follow-up: complete Phase 07 native Windows/macOS tray smoke; Linux native menu/lifecycle smoke is implemented.
- [ ] Manually validate browser/folder openers and clipboard on each desktop, including Wayland-only Linux.
- Tray dependency is pinned to 0.24.2 because 0.25.1 requires unavailable `dirs ^7` with libappindicator; see `docs/TRAY.md`.

## Storage

- [ ] Rerun native Windows CI after the reported `transfer_logging` upload failure
  (`Storage(Io)`). The rename adapter now uses `NtSetInformationFile` and has a
  focused native regression for conflicts, replacement, Unicode and directory
  moves. Windows cross-target Clippy passes; native execution remains unverified.
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

- [ ] Verify the opt-in video keep-awake fallback on physical iPadOS/Android over
  HTTP, including automatic screen timeout, low-power mode and app switching.
  Playback success does not prove sleep prevention.

- Folder-first display is implemented for List/Grid, search and pagination.

- [ ] Verify the native file picker and long-press image Paste menu on physical
  iPadOS Safari and Android browsers, including screenshots and copied images.
  Browser emulation checks flow/bytes/accessibility, not OS menus or photo providers.
- Paste uses native paste events on both localhost and HTTP LAN addresses. It no
  longer reads the clipboard automatically or needs a Clipboard API permission.
  Image/file payloads take precedence over accompanying text.

- [ ] Run manual screen-reader checks and native mobile touch/keyboard checks; Phase 06 automated Chromium accessibility and viewport checks are local coverage.

- [ ] Validate native clipboard copy permissions and paste behavior on Chromium, Firefox and Safari; deterministic paste fixtures cover Chromium/Firefox/WebKit.
- [ ] Validate multi-file browser download UX; no ZIP generation is planned for v1.
- Preview MIME allowlist and safe text rendering are implemented; see docs/PREVIEWS.md.
- [ ] Verify media codecs, seeking/fullscreen and large-image memory use on physical
  iPadOS/Android devices. Unsupported codecs fall back to a message and Download.

## Packaging

- The local Linux archive is a development candidate; it has not been published.
  The desktop binary needs GTK3/AppIndicator runtime libraries even with `--no-tray`.

- Linux archive generation and isolated extracted-archive smoke are implemented.
- [ ] Audit and bundle required third-party notices before public distribution.
- [ ] Verify the Linux archive on clean supported distributions; local smoke uses
  installed GTK/AppIndicator libraries and does not prove portable ABI support.

- [ ] Windows signing strategy is not defined.
- [ ] macOS signing/notarization strategy is not defined.
- [ ] Linux packaging formats beyond release archive are not defined.

## Transfers

- Uploads do not resume across process restarts.

- Upload logs now identify transfer/stage/bytes/timings and safe error categories.
  Storage Io and HTTP body errors still do not expose exact OS/network root causes.

- [ ] Validate native Windows/macOS upload publication and cancellation at runtime.
- [ ] Run multi-device LAN/VPN transfer stress tests for release acceptance; the Linux 1 GiB upload/download benchmark is automated.
- Runtime upload limits now update through host-only settings when the transfer queue is idle.

## Sessions and QR

- [ ] Validate physical-device QR scanning and private LAN/VPN routing across supported hosts.
- [ ] Validate native Windows/macOS settings replacement and cookie/browser flows at runtime.
- [ ] Verify native clipboard copy and Safari/iOS join flows; automated redirects/cookies cover Chromium, Firefox and Linux WebKit.

## Reliability / release follow-ups

- [ ] Abrupt-kill partial-file scavenging remains unimplemented; hidden reserved
  partials are excluded from browser listings but can consume disk until removed
  by the host. Graceful shutdown/disconnect cleanup is tested.
- [ ] Directory metadata snapshots scale with entry count. The 10,000-entry case
  is measured and UI rows are paged; larger directory/server pagination work needs
  further measurements before claiming support at larger scales.
- [ ] Native Safari/iOS, physical mobile touch/accessibility, separate-device LAN
  and OS-action checks remain release gates; Linux browser-engine automation does
  not substitute for these checks. Windows/macOS native checks are user-owned.

## Host setup

- Logging level and live host log viewer are process-local; persistent log archives
  and persisted verbosity are outside this implementation.

- Duplicate network apply controls and empty QR fields are removed; one Save
  changes action retains network validation and persistence.

- First-run folder/network controls and QR setup/retry are implemented; see
  [HOST_SETUP.md](docs/HOST_SETUP.md) and the correction acceptance evidence.
- [ ] User-owned: native Windows/macOS folder picker validation. Linux/X11 portal
  cancel/select was exercised with a real native dialog; Wayland remains manual.
- Linux desktop selection needs an available XDG portal/backend. Headless hosts
  use `--root`; picker cancellation/unavailability leaves the existing share intact.

- Settings frame size is stable across tabs; long content scrolls inside the
  viewport-bounded dialog with persistent heading and save/cancel actions.

- Preview usability refined with full-screen mobile layout, compact image controls,
  fit reset, text-size adjustment and dedicated audio presentation. Physical-device
  media/codec and accessibility checks remain as listed above.


- Mandatory first-run setup now stages the folder privately, applies network/access
  together and gates file APIs until completion. Port changes move the host browser;
  tray Open remains recovery if navigation is interrupted. Native picker tests on
  Windows/macOS and Wayland remain user/device checks, not browser-fixture coverage.

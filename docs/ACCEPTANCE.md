# Acceptance Matrix

This file accumulates verified release evidence.

## Phase 01

- [ ] Workspace structure created and documented.
- [ ] Rust workspace checks.
- [ ] Frontend install/typecheck/build.
- [ ] CI skeleton present.
- [ ] prototype and branding assets present.
- [ ] active-goal/agent workflow verified.

## Storage

- [ ] Virtual root never leaks absolute path.
- [ ] Traversal rejected.
- [ ] Symlink/reparse escape rejected.
- [ ] Upload partial files never appear completed.
- [ ] Conflict policy tested.

## HTTP

- [ ] File list works.
- [ ] Upload streams.
- [ ] Download streams.
- [ ] Range 206/416 tested.
- [ ] API errors stable.

## Security/session

- [ ] Token join.
- [ ] Token rotation.
- [ ] Open LAN optional mode.
- [ ] Host settings remote denial.
- [ ] Token not leaked in logs.

## UI

- [ ] Full-screen layout.
- [ ] List/Grid.
- [ ] Search.
- [ ] Desktop context menu.
- [ ] Mobile action sheet.
- [ ] Paste.
- [ ] Create/rename/delete.
- [ ] QR.
- [ ] Settings.
- [ ] aggregate transfer progress.
- [ ] queue.
- [ ] reduced-motion behavior.

## Native

- [ ] Windows tray.
- [ ] Linux tray.
- [ ] macOS tray.
- [ ] graceful quit.
- [ ] open browser/folder.
- [ ] release icon.

## Performance

- [ ] Large upload bounded memory.
- [ ] Large download bounded memory.
- [ ] configured concurrency enforced.
- [ ] many-file directory remains usable.

## Packaging

- [ ] Windows archive startup.
- [ ] Linux archive startup.
- [ ] macOS package startup.
- [ ] embedded frontend works without Node.

# Phase 07 — Desktop Tray and Host Lifecycle

## Objective

Turn SplitShare into a polished native desktop host without adding a desktop webview framework.

## Deliverables

- native system tray on Windows/Linux/macOS;
- Open SplitShare;
- Open shared folder;
- Copy share link;
- Start/Stop sharing;
- Settings -> local browser UI;
- Quit;
- `--no-tray`;
- graceful shutdown;
- application icons;
- host browser-opening behavior;
- stable listener restart/change flow when settings require it.

## Acceptance

- tray is native and follows platform conventions;
- no custom tray dashboard;
- quit cleans/cancels active uploads safely;
- tray failure does not corrupt server operation;
- native smoke tests recorded for all release platforms.

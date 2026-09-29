---
name: splitshare-release
description: Use when building SplitShare release archives, embedding the Vue frontend, integrating native icons/tray, or validating Windows/Linux/macOS packages.
---

# SplitShare Release Skill

Read `docs/PACKAGING.md` and `docs/ACCEPTANCE.md`.

## Release invariant

End users receive a native desktop host with embedded WebUI.

Node/npm are build-time only.

## Before packaging

- clean frontend production build;
- Rust release build;
- correct version;
- icons included;
- no repo-local development paths;
- no token/config secrets;
- required notices included.

## Smoke test

On each supported artifact:

1. launch;
2. verify tray;
3. open WebUI;
4. select/share temp folder;
5. connect from another browser where possible;
6. upload;
7. download;
8. test QR/link;
9. graceful quit;
10. confirm no Node dependency.

Do not claim a native target passed unless that target was actually built/run or a clearly documented CI result proves it.

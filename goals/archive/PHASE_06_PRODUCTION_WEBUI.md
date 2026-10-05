# Phase 06 — Production WebUI

> Archived historical plan. Superseded as a work assignment; see [archive status](README.md).

## Objective

Replace scaffold UI with the approved full SplitShare file-manager experience.

## Source of truth

`prototype/splitshare-ui/index.html`

## Deliverables

- full-screen layout;
- SplitShare branding/palette/logo;
- List/Grid views;
- breadcrumbs;
- search;
- desktop context menu;
- mobile action sheet;
- responsive toolbar;
- icon-only compact actions;
- file type icon mapping;
- QR modal;
- settings with real backend integration;
- create/delete/rename/conflict modals;
- queue modal;
- animated 10px modal blur/fade;
- aggregate bottom transfer progress;
- accessibility/focus;
- reduced-motion behavior;
- loading/empty/error/disconnected states.

## Acceptance

- no prototype-only fake controls remain;
- desktop and mobile E2E flows pass;
- all icon-only actions have accessible names;
- refresh/reconnect converges to real server state;
- UI does not expose real paths.

## Completion

Local acceptance PASS on 2026-09-30. All five criteria have individual evidence in
[the acceptance matrix](../../docs/ACCEPTANCE.md#phase-06-local-acceptance--2026-09-30).
Native/device and cross-browser release gates remain tracked in `ISSUES.md`.

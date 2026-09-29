---
name: splitshare-frontend
description: Use when implementing SplitShare Vue 3, TypeScript, Tailwind, Pinia, file manager UX, modals, transfer queue, mobile behavior, or accessibility.
---

# SplitShare Frontend Skill

## Source of truth

`prototype/splitshare-ui/index.html`

Production implementation uses Vue components and bundled dependencies.

Do not ship CDN Tailwind or CDN Font Awesome from the prototype.

Use the documented palette and logo.

## UX rules

- full viewport;
- List + Grid;
- desktop right-click menu;
- mobile action sheet;
- search;
- breadcrumbs;
- Paste and New Folder in file toolbar;
- QR/Settings in top bar;
- compact icon-only actions where approved;
- aggregate bottom transfer progress;
- detailed queue modal;
- 10px animated backdrop blur;
- reduced-motion support.

## State

Pinia stores reflect backend snapshots/events.

Do not invent completed transfer state client-side.

Browser-local upload progress may be optimistic only until server confirmation.

## Accessibility

Icon-only buttons require accessible labels/tooltips.
Dialogs require focus management.
Keyboard users need equivalents for pointer-only interactions.

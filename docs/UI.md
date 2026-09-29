# UI Specification

## Source of truth

`prototype/splitshare-ui/index.html`

The prototype defines visual hierarchy and interaction intent. Production should use Vue components and bundled icon packages rather than CDN dependencies.

## Brand palette

```text
#6366F1  indigo
#A855F7  purple
#22D3EE  cyan
#E2E8F0  main text
#94A3B8  muted text
#0F172A  primary dark background
```

## Full-screen file manager

The application viewport fills available width and height.

Primary regions:

1. host identity / QR / settings;
2. breadcrumbs;
3. search + Paste + New folder + Upload + List/Grid;
4. file area;
5. aggregate transfer bar.

## File views

### List

Columns on desktop:

- selection;
- name/type;
- size;
- modified;
- action icon.

`Download` is an icon-only action.

### Grid

Cards use category/type icon, name, compact metadata and overflow action.

## Desktop context menu

Right-click on file/folder opens a custom menu positioned within viewport bounds.

Potential actions, capability-dependent:

- download;
- preview;
- rename;
- copy link if supported;
- delete.

Menu dismisses on:

- outside click;
- Escape;
- action completion;
- navigation.

## Mobile actions

No right-click dependency.

Use an action sheet with icon actions.

Top/toolbar actions stay compact. Paste and New Folder labels may collapse to icon-only at mobile width.

QR and Settings remain available in the main top bar and must not be duplicated in a second mobile toolbar.

## Modals

Required:

- QR;
- settings;
- create folder;
- delete confirmation;
- transfer queue;
- file conflict;
- text draft;
- image paste preview.

Backdrop:

- dark translucent overlay;
- animated blur around 10 px;
- backdrop begins before/with content fade;
- avoid heavy scale animation that makes dialogs feel slow.

Respect `prefers-reduced-motion`.

## Multi-upload

Bottom transfer area shows aggregate batch state:

- file count;
- active count;
- total transferred / total size;
- percentage when known;
- combined speed;
- ETA when meaningful.

Queue modal shows per-file state.

## File icons

Do not require one artwork asset per extension.

Mapping order:

1. special known extension;
2. MIME category;
3. generic file.

Categories:

- folder;
- image;
- video;
- audio;
- PDF;
- text;
- source code;
- archive;
- executable/binary;
- document;
- spreadsheet;
- presentation;
- database;
- font;
- disk image;
- torrent;
- certificate/key;
- config;
- unknown.

Never execute or inspect file content solely to choose an icon.

## Accessibility

- every icon-only action has accessible name;
- keyboard context-menu alternatives;
- visible focus states;
- focus trap in dialogs;
- focus restoration;
- Escape dismisses dismissible overlays;
- touch targets at least approximately 40–44 px;
- no action communicated only by color.

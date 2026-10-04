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

Folders appear before files in List and Grid views. Existing name order is
preserved within each group. Grouping happens before search and pagination and
is reapplied on navigation, refresh and realtime listing updates.

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

## Phase 06 implementation

The production Vue build preserves the prototype's full-screen navy file manager,
branding, compact controls and responsive List/Grid modes. QR and settings live in
the header; search, paste, upload, create, refresh and view controls occupy the
workspace toolbar. List rows include type, size and modification metadata. Icons
use filename metadata with a generic fallback; contents are never opened for icons.

Right-click, Shift+F10, the Context Menu key and each item's actions button open the
same permission-aware menu. At mobile widths this becomes a bottom action sheet.
Native modal dialogs make the background inert; shared dialog helpers wrap Tab,
restore focus through nested dialogs and fall back to search when an opener is
removed. Modal backdrops fade to 10px blur. Reduced motion disables both content
and backdrop animation. The transfer footer participates in layout rather than
covering files, and reports actual upload bytes and terminal outcomes.

Selection supports individual downloads and confirmed deletion. Download selected
opens individual links; it does not promise ZIP generation. Bulk deletion stops at
the first server error and retains only remaining targets for a deliberate retry.
Per-file share-link controls are omitted. File previews are now implemented as
described in PREVIEWS.md. Settings, QR,
file mutations, clipboard and queue controls use the existing real API contracts.
Keep existing resolves a conflict by cancelling that local upload attempt without
changing the destination file.

Connection status follows SSE, with browser offline events providing an immediate
disconnection hint. Reconnect uses bounded exponential backoff (500ms–10s), and
server resync events reload status and the current virtual directory. Browser
online events trigger a connection attempt, never a premature Connected state.
Request cancellation and revision checks prevent older listings from replacing a
newer navigation. Initial failure has a working Refresh path; session-required,
permission-disabled, empty, loading and failure states are explicit.

## Phase 08 measured directory behavior

A 10,000-entry local directory took about 7.8 seconds to render and 1.4 seconds to
search when every row was mounted. The backend listing took about 44 ms and
returned 1.16 MB, so this phase uses client-side pages of 100 entries rather than
virtual scrolling or an API pagination change. Search covers the full snapshot,
resets to page one, and works in List/Grid modes. Selection persists across pages;
Select all visible items affects the current page. Page count is clamped after
refresh/deletion. This bounds rendered rows, not directory metadata memory;
substantially larger directories still require measurement before release claims.

Lost upload responses show an explicit unconfirmed state until server reconciliation;
see [TRANSFERS.md](TRANSFERS.md#phase-08-interruption-policy).

Download anchors explicitly use `download`, including row/menu/selected-file links,
so Firefox does not treat the request as navigation and interrupt live SSE updates.


## Host setup correction

General settings now offers native **Choose shared folder**, and Network has real
interface/port inputs. Network changes apply through the single **Save changes**
action. Fresh launches expose **Set up sharing** in the empty state.
The QR icon routes unconfigured hosts to folder setup; loopback QR has a Network
settings shortcut. QR loading, generation errors and retries are explicit.
See [HOST_SETUP.md](HOST_SETUP.md) for the complete workflow and persistence rules.

## Upload and clipboard

Upload has a visible label and opens the native file picker directly on desktop,
phones and tablets. Paste opens one neutral editable modal with keyboard or
long-press guidance. Actual image content changes it to preview/filename/Upload;
actual text changes it to filename (including extension)/content/Save file.

The existing navy palette, modal backdrop and focus handling are retained.
Automatic clipboard reads, permission/retry controls, the mobile upload sheet and
duplicate photo/file alternatives are removed. Queue and conflict dialogs remain.
See TRANSFERS.md for detection, confirmation and browser limitations.

Settings uses one Save changes action for access, transfer and network fields.
The Network tab omits the duplicate address summary; QR owns share-address
selection. Empty address/link fields and empty notice rows are hidden while QR
discovery loads or fails; recovery controls remain available.

The transfer bar offers an optional Keep screen awake control while uploads are
pending. Its status distinguishes a native wake lock from best-effort video
playback and exposes failure/interruption inline, without another dialog.

## Host log viewer

Settings → Logging offers Warnings and errors, Standard and Detailed levels.
Save changes reloads the filter for the current run; Startup configuration denotes
RUST_LOG/default selection. Logging-only saves do not modify sharing settings or
revoke clients. Open logs opens a host-only, focus-trapped modal with timestamps,
severity, source, message and structured fields. Latest entries appear first;
search, severity filtering, Refresh and Clear logs operate on the bounded history.
Live SSE notifications coalesce snapshot refreshes; closing cancels network work
and restores focus. Errors and disconnected/reconnecting state remain visible.

Settings keeps a 900 × 640 px desktop frame, capped to the viewport with a 16 px
outer margin. Mobile settings fills the viewport. Switching sections does not
resize the frame; the body scrolls while the heading and action buttons stay visible.

## File preview modal

Files open through desktop double-click, touch tap, keyboard Enter or the Open
context action. A shared responsive modal provides native media players, image
zoom/navigation and bounded read-only text. Loading/errors/retry/download remain
explicit. Escape closes and stops playback. See [PREVIEWS.md](PREVIEWS.md).

Preview refinement: a fixed header groups filename/type/size with Download/Close.
The viewing area takes the remaining space; image navigation/zoom and text controls
are grouped below it. Audio has a compact dedicated desktop frame. Mobile previews
fill the viewport with safe-area-aware controls. Error/retry states are centered.

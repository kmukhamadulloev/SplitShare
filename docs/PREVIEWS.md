# File previews

Open files by double-clicking the filename on desktop, tapping it on touch devices,
pressing Enter on the filename/row, or choosing Open in the context menu/action
sheet. The shared modal supports Escape to close and restores focus to the opener.
Download remains available, including for unsupported files.

- Images: PNG, JPEG, GIF, WebP and AVIF. Fit view, zoom, previous/next buttons and
  Left/Right keys. Navigation follows the current filtered directory, across pages.
- Video: MP4/M4V, WebM and OGV, with native controls, seeking and fullscreen.
- Audio: MP3, M4A, WAV, OGG/OGA and FLAC, with native controls and seeking.
- Text: UTF-8 text/source/config extensions listed in core `preview.rs`, including
  TXT, Markdown, JSON, CSV, HTML and XML. Display is read-only plain text; no markup
  rendering or code execution. Wrapping is optional. The first 256 KiB are fetched
  with Range and decoded incrementally; longer files show a truncation notice.
  Invalid UTF-8 and NUL-containing binary data show an error. Empty files are explicit.

Extension support does not guarantee codec support. The browser decodes media;
SplitShare does not transcode. Failed decoding/network requests show an error with
Retry and Download. No autoplay. Closing removes media sources, stops playback,
clears text and aborts outstanding metadata/text requests.

## Security and architecture

Core owns the conservative extension-to-MIME policy. The HTTP adapter reuses the
existing sandboxed FileService read handle and bounded streaming implementation.
GET/HEAD `/api/v1/files/preview?path=...` requires Download capability and an active
session under token mode. The shared connection/stream limits, Range handling,
no-store, nosniff and session/shutdown cancellation apply to previews too.

Only approved MIME types are served inline. HTML/XML/source files are always
`text/plain; charset=utf-8`; SVG, PDF, office documents and unknown extensions return
415 PREVIEW_UNAVAILABLE. Preview responses also carry a restrictive sandbox CSP.
The application never embeds shared documents in an iframe or uses innerHTML.
File extensions are hints, not content validation; malformed media fails in the
browser. Download keeps its attachment/octet-stream behavior.

The server does not buffer complete files. The browser bounds text to 256 KiB;
image decoding still uses browser memory proportional to the image. Very large
images and physical-device codec/fullscreen behavior require device testing.

## Preview usability refinement

The header shows the filename (full name on hover), file type and size, with
Download and Close always available. Images have compact previous/next controls,
a position counter and a Fit reset beside zoom. Text has wrapping and 12–24 px
font-size controls. Audio uses a dedicated compact desktop player with a decorative
music icon; video retains native controls. Loading/error states occupy the viewing
area. Mobile uses the full viewport with safe-area padding and touch-sized controls.

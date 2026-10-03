# Transfer Engine

## Goals

Transfers must:

- support large files;
- keep memory bounded;
- expose truthful progress;
- allow cancellation;
- isolate partial files;
- support multiple clients;
- enforce host concurrency.

## Upload API model

One HTTP upload request carries one file.

The browser may select many files and create a local queue. It starts requests up to the host-advertised limit.

The backend also owns a semaphore/permit limit, so a modified client cannot bypass host policy.

## Parallel uploads

Settings:

- `parallel_uploads_enabled: bool`
- `max_parallel_uploads: integer`

Effective limit:

```text
enabled = false -> 1
enabled = true  -> max_parallel_uploads
```

Example:

```text
selected: A, B, C
limit: 2

A  running
B  running
C  queued
```

Aggregate UI progress:

```text
(sum transferred bytes across batch)
------------------------------------ × 100
(sum total bytes across batch)
```

Unknown-size streams must not fabricate a percentage.

## Download

Downloads are streamed directly from disk.

Support:

- `Content-Length` when known;
- `Content-Type`;
- safe `Content-Disposition`;
- bounded/open-ended/suffix `Range`;
- `206 Partial Content`;
- `416 Range Not Satisfiable`.

Never load a whole file into memory.

## Cancellation

Upload cancellation must stop further reads/writes and clean the partial file.

Server shutdown must cancel or safely terminate active transfers.

## Progress

Transfer progress includes:

- ID;
- direction;
- virtual destination/source;
- optional total bytes;
- transferred bytes;
- state;
- speed when measured;
- ETA only when meaningful;
- failure code/message;
- timestamps.

Use SSE for server-authoritative state.

Browser-local upload progress may update more frequently, but completed/failure state comes from the server.

## Retry

Retry restarts an upload from zero in v1.

Cross-restart resumable upload is post-v1 scope.

## Clipboard

### Files

Treat pasted files as normal uploads.

### Images

Paste event creates a preview and filename proposal before upload. File/image
representations take precedence over accompanying text. Paste events inspect both
file lists and file items. The Paste button opens a neutral editable dialog on
both localhost and HTTP LAN pages. Native keyboard or touch-menu paste supplies
the content; no automatic clipboard read is attempted. Ordinary text editing
remains unchanged. Images require Upload confirmation.

### Text

Text remains frontend draft state until the user saves.

Save as file uses the same storage safety/conflict policy as other writes.

Suggested extensions:

- `.txt`
- `.md`
- `.json`
- `.yaml`
- `.sh`

Format detection is a convenience only. Never execute pasted content.

## Implemented Phase 04 policy

Host configuration is loaded at startup. `--parallel-uploads N` selects 1–32
workers; `--serial-uploads` forces one. Overrides are process-local until the next host settings save. Phase 05 adds
persisted runtime limits; changing the effective limit requires idle transfers.

TransferManager lives in the application layer and does not depend on Axum.
A permit is acquired before consuming the request body and retained through
cleanup/publication. A bounded two-slot channel carries chunks of at most 64 KiB
to a blocking filesystem worker. Input idle timeout is 60 seconds. Temporary files
remain inside the destination filesystem and are never listed as completed files.

States: queued → uploading → publishing → completed, or failed/cancelled.
Progress counts bytes actually written; events are throttled to about 100 ms.
Publishing is an atomic cancellation boundary: cancellation after it starts returns
409. Retrying uses a fresh ID/key and starts at byte zero. Snapshots and SSE expose
only virtual paths. History is in-memory, bounded to 128 records; terminal records
are evicted first, and a full active queue rejects new work with 429.

The browser uses one shared SSE connection, resynchronizes snapshots after gaps,
and limits scheduling to the advertised policy (at most six browser requests).
Pasted images require preview confirmation; text is a local draft until Save and
is limited to 2 MiB. File paste and drag/drop use the same upload queue. Unknown
sizes have no invented percentage. Failed/cancelled rows do not claim publication.
Token-mode remote requests require a session; cancellation additionally requires
the separate transfer key. History is share-wide for upload-capable clients.
Session revocation cancels remote transfers before publication (see SESSIONS.md).

Linux memory regression: `python3 scripts/test-upload-memory.py` runs the release
binary with 8 MiB and 256 MiB generated uploads and checks RSS growth below 32 MiB.
Abrupt process death can leave hidden partial files; crash scavenging is future
reliability work. Graceful cancellation, disconnection and shutdown are covered.

## Phase 08 interruption policy

Queue permits have a 60-second wait deadline. Idle input also expires after 60
seconds; these failures have distinct `UPLOAD_QUEUE_TIMEOUT` and
`UPLOAD_IDLE_TIMEOUT` codes and never claim publication. Workers retain permits
until temporary-file cleanup completes.

The browser reconciles failed network responses with server snapshots. If status
cannot be established, the row is `unconfirmed` (browser-only, not a new server
Transfer state), with Check status and explicit Retry actions. A retry starts at
zero with conflict policy `ask`, preventing an uncertain retry from silently
replacing a published file. Available server state remains authoritative, including
an upload still running. Reconnect/`transfer.resync` refetches snapshots; missing
nonterminal history becomes unconfirmed rather than fabricated failure/success.

`python3 scripts/benchmark-reliability.py` exercises generated 8 MiB, 10 MiB and
1 GiB uploads/downloads, verifies every returned byte and checks Linux server RSS
growth below 32 MiB. It also transfers 100 small files and lists 10,000 entries.
Temporary fixtures are removed after the host exits. Local timings are not LAN or
physical-disk throughput guarantees. Abrupt process death can still leave hidden
partials; safe crash scavenging remains an explicitly recorded issue.

## Unified paste and upload controls

Paste always opens one modal with an editable Paste area. Desktop guidance uses
Ctrl+V / ⌘V; touch layouts offer touch-and-hold Paste. Receiving image bytes changes
that same modal into a preview with filename and Upload confirmation. Receiving
text changes it into an editable text draft and filename (including extension),
with Save file. JSON text receives a .json filename proposal. Cancel discards the
draft. Text is limited to 2 MiB of UTF-8 bytes.

This flow handles native paste events and does not call `navigator.clipboard.read()`.
It needs no automatic-read permission or HTTPS certificate for the Paste button.
The browser/OS still determines which image/file representations native paste
provides. Empty or unsupported payloads produce a local error, never a fabricated
text draft. HTML is not inserted or executed; image URLs are not fetched.
Keyboard paste outside an editable field remains available without clicking Paste.

Upload opens one unrestricted multiple-file picker on every device. The OS/browser
owns its providers. There is no intermediate upload sheet or duplicate picker in
the Paste dialog. Selected images use the image confirmation form, one at a time;
unsupported previews retain the original uploadable bytes. Other files enter the
normal streaming queue. Drag/drop retains its direct queue behavior. Queue/retry
and file-conflict dialogs remain necessary for recovery and overwrite protection.
No HTTP contract, backend policy or streaming behavior changes.

## Keeping the screen awake

During queued/running/publishing uploads, the transfer bar offers **Keep screen
awake**. It is opt-in and starts directly from a user tap. Secure contexts prefer
the native Screen Wake Lock API. HTTP clients use a tiny, silent, looping inline
video bundled with SplitShare; no external media requests or new runtime are used.
If a native request fails, another tap can try video playback.

Only an acquired native lock is labelled active. Video playback is labelled a
fallback that may not prevent device locking. Playback/permission failures remain
visible and do not affect upload correctness. Stop, a finished/failed/cancelled
batch, component teardown or page hiding releases the lock and pauses the video.
Returning to the page requires another tap; there is no automatic playback.
Manual locking, background suspension and low-power restrictions are not bypassed.
Uploads still have the existing idle timeout and retry-from-zero policy.

Physical iPadOS/Android HTTP testing is required to establish whether their
browsers actually suppress automatic sleep. Automated playback tests cannot prove
that. Prefer increasing the device timeout when the fallback is ineffective.

The first-party media assets in `web/public/media` are one-second 16×16 black
clips at two frames per second with no audio track (H.264 MP4 and VP9 WebM). They
were generated with FFmpeg, a development tool only:

```bash
ffmpeg -f lavfi -i color=c=black:s=16x16:r=2 -t 1 -an -c:v libx264 -pix_fmt yuv420p -movflags +faststart web/public/media/keep-awake.mp4
ffmpeg -f lavfi -i color=c=black:s=16x16:r=2 -t 1 -an -c:v libvpx-vp9 web/public/media/keep-awake.webm
```

## Upload diagnostics

Default INFO logs include accepted requests, worker start, completion and
cancellation. Each accepted upload has an `upload` span containing `transfer_id`,
`expected_bytes` (None if unknown), `conflict_policy` and `concurrency_limit`.
The ID correlates logs with transfer snapshots; it is not the cancellation key.
Warnings carry their transfer ID and expected size directly, so they remain
useful when `RUST_LOG=warn` disables the INFO span.

Failures report `stage` (prepare, receive, write, publish, queue or worker),
`bytes_written`, elapsed/queue-wait milliseconds, the existing failure code and
a typed cause. Receive errors include only their safe I/O error kind; idle and
queue deadlines are identified separately. Cancellation requests and dropped
handlers are logged at INFO, not as unexplained warnings. A dropped handler may
mean disconnection, session invalidation or shutdown; it does not prove screen
locking. Worker panics/cancellation are identified without panic payloads.

Enable additional progress/publication diagnostics with:

```bash
RUST_LOG=splitshare=info,splitshare_application::transfers=debug ./target/release/splitshare --open
```

Progress is logged at most once every 30 seconds when chunks are written; this
is not a periodic heartbeat during a stalled input. Times after worker start
exclude queue wait. `bytes_written` counts successfully acknowledged writes, not
bytes queued in transport memory. No per-chunk log flood is introduced.

Logs omit filenames, native/virtual paths, transfer keys, cookies, link tokens,
file contents and raw I/O error text. Storage errors retain their safe domain
category: `Storage(Io)` at `write` identifies disk-write failure but cannot alone
distinguish a full disk from every other OS error. HTTP body errors currently
map to `UnexpectedEof`; this identifies transport failure, not its mobile/Wi-Fi
root cause. No timeout, retry or upload API behavior is changed.

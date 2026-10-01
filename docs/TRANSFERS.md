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

Paste event creates a preview and filename proposal before upload.

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

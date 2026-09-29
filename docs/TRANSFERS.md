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

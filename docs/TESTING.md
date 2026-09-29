# Testing

## Mandatory layers

### Rust unit tests

Cover:

- virtual path parser;
- permissions;
- share modes;
- conflict policy;
- session token validation;
- transfer state transitions;
- network address ranking.

### Storage integration tests

Use temporary directories.

Cover:

- list;
- read;
- upload publish;
- cancel cleanup;
- rename;
- mkdir;
- delete;
- traversal;
- encoded traversal;
- absolute paths;
- symlink escape;
- conflict outcomes.

### HTTP integration tests

Run real Axum router/server with temporary root.

Cover:

- status;
- file listing;
- permissions;
- upload;
- Range download;
- `416`;
- token join/cookie;
- remote host-settings denial;
- SSE event delivery.

### Browser tests

Use Playwright in production-like built frontend where possible.

Desktop:

- List/Grid switch;
- search;
- context menu;
- modals;
- drag/drop;
- multi-upload;
- queue;
- create/rename/delete;
- QR/settings;
- file conflict;
- reconnect.

Mobile viewport:

- toolbar;
- icon-only actions;
- action sheet;
- full-screen settings;
- upload queue;
- responsive list/grid.

### Native tests

On each release OS:

- tray appears;
- menu actions work;
- open folder;
- open browser;
- stop/start;
- graceful quit;
- application icon.

## Large-file tests

Do not commit giant fixtures.

Generate deterministic temporary files.

Minimum scenarios:

- 1 B;
- 1 KiB;
- 10 MiB;
- 1 GiB sparse/generated where platform allows;
- many small files;
- parallel uploads.

Assert memory does not scale with total file size.

## Security regression suite

Mandatory before v1:

- `../`;
- `%2e%2e`;
- mixed separators;
- double encoding where routing stack decodes;
- Windows drive/UNC attempts;
- symlink/reparse point;
- illegal filenames;
- token brute/invalid cases;
- malformed Range;
- oversized JSON;
- concurrency over configured limit.

## Repository check script

`./scripts/check.sh` should eventually run:

```text
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
frontend typecheck
frontend build
optional Playwright suite
```

The script must fail on a failed mandatory step.

## Foundation checks available now

Install dependencies with `npm ci --prefix web`, then `bash scripts/check.sh`.
The script builds frontend assets before Rust because they are compile-time inputs.
`bash scripts/build-release.sh` builds the standalone binary.
On Linux, `python3 scripts/smoke-foundation.py` runs a copied release binary in a
fresh temporary directory with isolated config, checks assets and unavailable
API/join routes, and verifies SIGTERM exit and token-log redaction. Port 8080 must
be available. Later-phase test lists above remain planned.

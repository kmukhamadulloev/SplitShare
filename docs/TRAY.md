# Desktop System Tray

## Principle

The tray is a native control surface, not a custom application window.

It follows operating-system menu conventions.

## Recommended menu

```text
SplitShare
──────────────
● Sharing / Stopped

Open SplitShare
Open shared folder
Copy share link

Start sharing / Stop sharing
──────────────
Settings
Quit
```

Optional entries may expose connected/active counts only if native menu APIs support the state cleanly.

Do not build a bespoke tray dashboard.

## Behavior

### Open SplitShare

Open the active local browser URL.

### Open shared folder

Reveal/open the selected root on the host OS.

### Copy share link

Copy the active tokenized join URL.

### Start / Stop

Controls server sharing state without exiting the process.

### Settings

Open the local WebUI directly to settings using a local client route/state.

### Quit

Initiate the same graceful shutdown used by terminal signals.

Wait for owned runtime tasks within a bounded timeout and ensure partial uploads are not published as completed files.

## Failure

Tray initialization failure must be logged and must not prevent headless/browser operation when the HTTP server can otherwise run.

Provide a `--no-tray` path for terminal/headless use.

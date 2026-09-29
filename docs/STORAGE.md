# Storage and Virtual Filesystem

## Root model

The host selects one directory.

Example host path:

```text
/home/user/Documents/SplitShare
```

Browser clients see only:

```text
/
├── Design/
├── notes.md
└── archive.zip
```

No API response contains the absolute root.

## Virtual paths

Every client path is relative to virtual `/`.

Reject:

- absolute paths;
- `..` escape;
- encoded traversal;
- NUL bytes;
- invalid path separators for the current platform;
- device/special paths;
- Windows drive/UNC path injection.

Normalize carefully without changing legitimate Unicode filenames unexpectedly.

## Symlinks

v1 policy: do not expose/follow symlinks or equivalent reparse-point links from the shared tree.

They may be hidden or returned as unsupported entries, but must not be followed.

## Directory listing

Return typed entries containing only safe metadata:

- opaque/virtual path;
- display name;
- kind;
- size for files;
- modified timestamp where available;
- MIME/category hint;
- capabilities allowed for the current client.

Do not serialize owner/group, permissions, inode/device or absolute path unless a later approved goal requires it.

## Upload publication

Write uploads to an application-created temporary filename in the destination directory or same filesystem.

Example:

```text
.splitshare-upload-<uuid>.part
```

On success:

1. flush writes;
2. apply conflict policy;
3. atomically rename/publish;
4. emit filesystem event.

On failure/cancel:

- close handle;
- remove partial file;
- never emit completed-file state.

## Conflict policy

Host-configurable policy may be:

- ask client;
- auto-rename;
- replace;
- reject.

The API must make conflict outcomes explicit.

"Ask" means the first request returns a typed conflict and the browser presents a modal before resubmission with an explicit resolution.

## Delete

Delete is capability-controlled and requires a typed confirmation in the UI.

v1 delete is permanent filesystem deletion. Do not call it Trash unless an OS trash adapter is explicitly implemented.

## Hidden files

Recommended default: hidden/system entries are not shown.

If host-configurable exposure is added, it must not weaken the root sandbox.

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

## Phase 02 implemented adapter

`splitshare-core::VirtualPath` is a validated, serde-safe domain value.
`splitshare-storage::Storage` is the native adapter. `Storage::open` is the only
entry point accepting a host path. It opens a directory capability; subsequent
operations resolve validated components relative to directory handles. Each
component uses no-follow opening and metadata checks. The selected root itself
must not be a symlink/reparse point. Windows metadata rejects all reparse attributes.

Implemented operations: `list`, `metadata`, `mkdir`, no-clobber `rename`, `delete`,
seekable `read`, `begin_upload`, chunk writes, explicit cancel and publication.
`delete` removes files or **empty** directories; nonempty directories return
`DirectoryNotEmpty`. Recursive deletion has not been implemented. Root rename,
delete, upload and mkdir are prohibited. Operations are blocking adapter calls:
future async services must use blocking workers and bounded transfer queues, not
run filesystem calls on Tokio's request executor. HTTP integration is Phase 03;
transfer scheduling/concurrency enforcement is Phase 04.

### Path and listing policy

- `/` denotes virtual root. `/etc/passwd` means an item *inside* the selected root,
  never the native `/etc/passwd`. No client string is passed to ambient filesystem I/O.
- Require exactly one leading slash, no trailing slash except root, and no empty,
  `.` or `..` components. Reject backslashes, drive/ADS colons, percent signs
  (including encoded and double-encoded paths), controls, Windows device names,
  trailing dots/spaces and Windows-forbidden punctuation. Reject `~` to prevent
  DOS short-name aliases from bypassing the reserved temporary namespace.
- Limits: 4096 UTF-8 bytes per path, 255 bytes per component, 128 components.
- Preserve Unicode bytes; do not normalize or case-fold names. Conflict detection
  uses the native filesystem's case/normalization semantics. This allows composed
  and decomposed names on filesystems that distinguish them.
- `.splitshare-` prefixes are reserved, case-insensitively. They cannot be parsed
  or deserialized as client paths. Listings omit dotfiles, reserved/invalid names,
  non-UTF-8 names, links and special files; Windows hidden/system entries are omitted.
- DTOs serialize only virtual name/path, kind, optional size and modified time.
  MIME hints and authorization capabilities will be attached by application services.
  Storage errors expose stable enum codes and safe messages; OS error text and
  native paths are not included, including in storage logs.

### Streaming and publication

`ReadHandle` implements `Read` and `Seek` over an open regular file; it does not
load file contents. It reports the size observed at opening. Concurrent native
host edits can still change file contents/length; this is not a snapshot service.

An upload owns a randomly named, create-new temporary file in the destination
filesystem. `write_chunk` accepts at most 64 KiB and tracks written bytes. An
expected length, if supplied, must match exactly before publish. Failed or
oversized writes poison the upload. Flush and file sync occur before atomic
publication. No user file is visible before `publish` succeeds.

`Ask` and `Reject` both return `Conflict` without replacing data. `AutoRename`
tries `name (1)` through `name (1000)`; generated names must pass the same parser.
`Replace` only replaces regular files and never edits the existing inode in place.
No-clobber rename uses OS atomic primitives, not an existence check followed by
ordinary overwrite rename. Linux/macOS use rustix `renameat_with(NOREPLACE)`;
Windows uses handle-relative `NtSetInformationFile(FileRenameInformation)` with
replacement disabled. NTSTATUS failures are translated to Win32 I/O errors before
mapping to public storage errors; source/destination paths remain capability-relative.
Windows replacement also uses the handle-relative adapter. Unsupported filesystems
fail the operation; there is no unsafe check-then-rename fallback.

Explicit cancel and Drop close and remove partial files. Cleanup failures are
logged by error kind. Abrupt process termination/power loss can leave reserved
partial files on disk, but they remain hidden and cannot be published by restart.
Automatic stale-partial scavenging and power-loss durability of directory entries
are not implemented. A concurrent client cannot access temporary names. Destination
parents are re-resolved on publication; a swapped external symlink is rejected.

### Dependency and security rationale

Added cap-std/cap-fs-ext 4.0.3, rustix 1.1.5, getrandom 0.4.3 and Windows-only
windows-sys 0.61.2 (exact resolution in Cargo.lock). Native source/destination
names passed to platform rename adapters are always single validated components.
Internal upload names are generated by the adapter. The Windows unsafe FFI block
has a documented aligned-buffer and borrowed-handle safety boundary.

References: [capability confinement](https://github.com/bytecodealliance/cap-std),
[no-follow directory opening](https://docs.rs/cap-fs-ext/4.0.3/cap_fs_ext/trait.DirExt.html),
[Windows native rename API](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/nf-ntifs-ntsetinformationfile).

The sandbox protects against untrusted client paths and link traversal. It is not
an OS sandbox against another privileged local process: an administrator can move
opened directories, change mount points or create hard links. Hosts must select
and manage the shared tree accordingly. Network filesystem semantics and native
Windows/macOS runtime behavior remain release validation gates.

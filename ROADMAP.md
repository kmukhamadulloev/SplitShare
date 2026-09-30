# SplitShare Roadmap

## Release target

The first release target is **SplitShare 1.0.0** for desktop hosts with browser clients.

The project intentionally follows a release-quality path instead of an MVP path.

## Phase status

| Phase | Goal | Status |
|---|---|---|
| 01 | Foundation | Implemented; local acceptance PASS, native CI pending |
| 02 | Storage sandbox | Implemented; Linux acceptance PASS, native runtime CI pending |
| 03 | HTTP + file browsing | Implemented; Linux/browser acceptance PASS, native runtime CI pending |
| 04 | Transfers + clipboard | Implemented; Linux/browser acceptance PASS, native runtime CI pending |
| 05 | Sessions + permissions + QR | Implemented; Linux/browser acceptance PASS, native/device validation pending |
| 06 | Production WebUI | Planned |
| 07 | Desktop tray + lifecycle | Planned |
| 08 | Reliability + security + performance | Planned |
| 09 | Packaging + v1.0 acceptance | Planned |

## v1.0 definition

v1.0 is not reached until:

- a selected folder can be safely shared;
- remote browsers can browse it using virtual paths;
- upload/download are streamed;
- parallel uploads are bounded by host configuration;
- text/image/file paste behaves correctly;
- List/Grid UI and desktop/mobile interactions work;
- token join and optional open-LAN access work;
- QR is generated locally;
- Range responses work;
- host settings are protected from remote mutation;
- native system tray works on release targets;
- graceful shutdown does not leave partial transfers marked complete;
- traversal/symlink/filename tests pass;
- large-file memory usage remains bounded;
- production frontend is embedded;
- release archives start on supported targets;
- documentation and acceptance evidence are current.

## Post-v1 candidates

These are intentionally not approved v1 scope:

- mobile host shell backed by Rust native library;
- persistent transfer history database;
- mDNS discovery UI;
- resumable uploads across process restart;
- generated ZIP download for multi-selection;
- multiple simultaneous share roots;
- read-only public guest session profiles;
- optional end-to-end encrypted remote relay.

Each requires an explicit future goal before implementation.

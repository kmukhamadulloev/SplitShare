# Third-party notices

SplitShare's own code is licensed under the included MIT LICENSE. Dependencies
retain their respective licenses and copyright notices.

Release archives include `licenses/inventory.json` and license/notice files for
resolved Rust dependencies and installed production frontend dependencies. Build
dependencies may also be included. Exact dependency versions are recorded in the
repository's Cargo.lock and web/package-lock.json. Packaging fails if a collected
dependency has no license file available for review and inclusion.

The embedded UI uses Vue, Pinia, Tailwind CSS, Lucide icons and qrcode. The native
host uses Tokio, Axum, capability-based filesystem libraries and native tray/OS
adapters. See the bundled per-dependency files for their terms and attribution.

Linux GTK and Ayatana AppIndicator runtime libraries are supplied by the operating
system and are not copied into the archive. Their distribution licenses remain
applicable to those separately installed libraries.

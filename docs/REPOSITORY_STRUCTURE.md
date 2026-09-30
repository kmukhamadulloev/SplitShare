# Repository Structure

```text
SplitShare/
├── .github/workflows/ci.yml
├── app/
│   └── splitshare/
│       ├── Cargo.toml
│       └── src/main.rs
├── crates/
│   ├── splitshare-core/
│   ├── splitshare-storage/
│   ├── splitshare-network/
│   ├── splitshare-server/
│   └── splitshare-platform/
├── web/
│   ├── public/logo.png
│   ├── src/
│   ├── package.json
│   ├── tsconfig.json
│   └── vite.config.ts
├── prototype/
│   └── splitshare-ui/index.html
├── docs/
│   ├── assets/
│   ├── ACCEPTANCE.md
│   ├── API.md
│   ├── ARCHITECTURE.md
│   ├── BRANDING.md
│   ├── NETWORK.md
│   ├── PACKAGING.md
│   ├── SECURITY.md
│   ├── STORAGE.md
│   ├── TESTING.md
│   ├── TRANSFERS.md
│   ├── TRAY.md
│   ├── UI.md
│   └── openapi.yaml
├── goals/
│   ├── PHASE_01_FOUNDATION.md
│   ├── PHASE_02_STORAGE_SANDBOX.md
│   ├── PHASE_03_HTTP_FILE_BROWSER.md
│   ├── PHASE_04_TRANSFERS_CLIPBOARD.md
│   ├── PHASE_05_SESSIONS_PERMISSIONS_QR.md
│   ├── PHASE_06_PRODUCTION_WEBUI.md
│   ├── PHASE_07_DESKTOP_TRAY_LIFECYCLE.md
│   ├── PHASE_08_RELIABILITY_SECURITY_PERFORMANCE.md
│   └── PHASE_09_PACKAGING_V1.md
├── skills/
├── scripts/
├── AGENTS.md
├── CODEX_START.txt
├── GOAL.md
├── ROADMAP.md
├── ISSUES.md
├── RELEASE.md
├── README.md
├── Cargo.toml
└── project.json
```

Phase 03 adds `crates/splitshare-application` for transport-independent service
orchestration between HTTP and storage. `web/src/app` contains typed API and Pinia
state; `web/tests` runs browser workflows against the actual Rust process.

# Repository Structure

```text
SplitShare/
├── .github/workflows/ci.yml
├── app/
│   └── splitshare/
│       ├── Cargo.toml
│       └── src/main.rs
├── crates/
│   ├── splitshare-application/
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
├── goals/archive/            # superseded implementation plans
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

Phase 05 adds `splitshare-application/src/sessions.rs` for session/policy rules,
`splitshare-server/src/access.rs` for cookie and host configuration adapters,
`web/src/components/Sharing.vue` for settings/QR, and `docs/SESSIONS.md` for policy.
The native config adapter persists settings; token/session data remains in memory.

Current version is 0.1.0. `GOAL.md` records active work, if assigned; archived
plans do not prescribe future work. `project.json` has no active goal until one
is assigned.

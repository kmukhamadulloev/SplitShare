---
name: splitshare-security-testing
description: Use for SplitShare security review, filesystem attack cases, transfer robustness, API integration tests, Playwright E2E, or release acceptance evidence.
---

# SplitShare Security and Testing Skill

Treat LAN clients as untrusted.

Mandatory adversarial areas:

- traversal;
- percent encoding;
- Windows path syntax;
- symlink/reparse points;
- malformed Range;
- filename/control-character edge cases;
- settings authorization;
- token handling;
- upload concurrency bypass;
- cancellation cleanup;
- absolute-path redaction.

Prefer real temporary directories and real HTTP integration tests.

Browser E2E must cover desktop and mobile viewport behavior.

Never satisfy acceptance with a mocked production server when the goal requires end-to-end behavior.

Record exact commands/results in completion report and update `docs/ACCEPTANCE.md`.

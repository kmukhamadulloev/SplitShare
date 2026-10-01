# Host setup correction

The user reported missing network/folder controls and unusable QR sharing after
Phase 08. Correct this before packaging; prior phase evidence did not cover a
first launch without CLI configuration.

Acceptance:

- A first launch has a discoverable host setup action.
- A host can choose the sandbox root with a native folder picker, without sending
  filesystem paths through the browser. The selection persists across restart.
- Interface and port controls apply real listener changes and persist. Occupied
  ports leave the existing share usable; changes revoke old sessions.
- QR either renders the actual selected current URL or shows a useful setup/error
  state; a loopback-only listener links to LAN network configuration.
- Remote clients cannot read/write host setup or trigger native dialogs; mutation
  origin/header checks and storage sandbox regressions continue to pass.
- Documentation explains launch, folder selection, LAN binding and joining; checks
  pass and the completed fix is committed locally.

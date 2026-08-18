# Native Module Boundaries

Status: active
Owner: AI
Last updated: 2026-07-23

## Direction

```text
Tauri command registration
  -> command adapters in lib.rs
    -> workflow_controller pure policy and effect-port contract
    -> repository_service filesystem/Git port contracts
    -> review_receipt bounded current-worktree evidence
    -> git_handoff explicit local-only mutation boundary
    -> existing collaboration policy ports
      -> mdsync_transport and ha2ha_envelope adapters
```

- `command_contract.rs` freezes the public command-name list and proves the
  explicit Tauri registration matches it.
- `repository_service.rs` owns the Git read adapter and the filesystem port
  required by artifact planning and review. Existing versioned/no-follow
  persistence stays behind the command adapter until a workflow consumes the
  port; its semantics are not duplicated.
- `workflow_controller.rs` owns resolver selection/stop policy, declared
  effects/stops, and the repository/helper/runtime/persistence port shape. It
  contains no Tauri or WebView state and performs no effect.
- `review_receipt.rs` owns read-only, bounded, sanitized current-worktree
  evidence for the post-run receipt. It reuses the injected Git read port,
  attributes no authorship, and cannot stage, commit, reset, or push.
- `git_handoff.rs` owns the only local Git mutation surface: clean-index
  inspection, current receipt-path exclusions, repository/fingerprint/content
  bound one-use confirmation, filter-free selected-blob staging,
  hook-isolated local commit, and exact HEAD/path/message readback. It has no
  remote, reset, checkout, revert, delete, amend, merge, or collaboration
  operation.
- `collaboration.rs`, `mdsync_transport.rs`, and `ha2ha_envelope.rs` remain the
  only collaboration policy, native transport, and envelope authorities.
- `lib.rs` retains proved process, cancellation, storage, and compatibility
  implementations; upcoming workflows should depend on the focused ports and
  must not add WebView-owned authority.

## Compatibility Rules

- Command names and camelCase request/response fields cannot change as part of
  extraction.
- Repository reads/writes retain path containment, no-follow, version, lock,
  atomic replacement, and post-commit error semantics.
- Goal/controller extraction cannot change resolver order, one-use
  confirmation, effect ordering, timeout/cancellation, repository verification,
  or shared repair debt.
- Remote/local versions and collaboration write order remain owned by the
  existing Sprint 2 modules.

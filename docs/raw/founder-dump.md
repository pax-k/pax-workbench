# Founder Dump

Status: captured
Owner: founder
Confidence: medium
Last updated: 2026-07-21

This is the canonical raw-context index for deterministic preflight tooling. The
full discussion and design synthesis are preserved in
`docs/raw/product-discussion.md`.

## Product Idea

A local-first desktop engineering workbench built on Build Right skills.

## Target Customer

Founders and engineers who want to understand and control evidence-backed agent
work without living in a terminal.

## Observed Pain

Skill state, sprint/task progression, Markdown authority, commands, gates, and
evidence are difficult to understand when distributed across a repository and
terminal transcripts.

## Desired Capabilities

- Open and inspect an engineering repository.
- Install and inspect Build Right skills.
- Plan and execute bounded work through the skills.
- Edit Markdown and visualize its task/sprint projections.
- See semantic agent activity and workflow checkpoints.
- Continue through explicit, checkpointed goals.

## Explicit Non-Goals

- Multi-agent orchestration in the MVP.
- Cloud sync, marketplaces, issue trackers, and visual workflow builders.
- Hidden proprietary planning state.

## Claims

| Claim | Status | Evidence | Notes |
| --- | --- | --- | --- |
| Repository Markdown must remain authoritative | founder-claimed | docs/raw/product-discussion.md | Core product invariant |
| A workbench can make the coding loop understandable outside a terminal | founder-claimed | docs/raw/product-discussion.md | Requires user validation |
| Tauri UI is a suitable visual/scaffolding reference | founder-claimed | docs/raw/product-discussion.md | It is not the workflow engine |

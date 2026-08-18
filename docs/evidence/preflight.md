# Preflight Evidence

Status: active
Owner: AI
Confidence: high for repository state; medium for product assumptions
Last updated: 2026-07-22

## Repository Evidence

| Date | Source Type | Source | Claim Supported Or Challenged | Strength | Notes |
| --- | --- | --- | --- | --- | --- |
| 2026-07-21 | command | `git status --short --branch` | Repository has no commits and its initial files are untracked | strong | Fresh project state |
| 2026-07-21 | file | `skills-lock.json` | Four Build Right skills are installed from `pax-k/build-right` | strong | Tool provenance only |
| 2026-07-21 | command | `bun .../preflight-check.ts --mode all` | Initial decision was `ask-founder`; canonical artifacts were absent | strong | Run before founder thread import |
| 2026-07-21 | command | `bun --version` | Bun 1.3.14 is available | strong | Frontend/tooling baseline |
| 2026-07-21 | command | `node --version` | Node v24.14.0 is available | strong | Secondary JS runtime evidence |
| 2026-07-21 | command | `rustc --version`; `cargo --version` | Rust/Cargo are unavailable | strong | Native verification gate |
| 2026-07-22 | command | `rustc --version`; `cargo --version`; native verification suite | Rust/Cargo are available and the native gate passes | strong | Supersedes the original environment blocker without erasing its historical evidence |
| 2026-07-22 | compiled and signed native evidence | Tasks 004-011 and `output/native/` | Native compile/unit proof plus signed repository, helper, runtime, bounded-task, persistence, and confirmed-loop component evidence pass | strong | Task 004 is compile/test evidence; signed component trials begin with the subsequent native surfaces. Production distribution remains excluded. |
| 2026-07-22 | cohesive signed native trial | `docs/evidence/manual-trials.md`; `output/native/task-012-final-*.jpeg` | One post-dependency signed-app trial proves provenance, helpers, Task 920, restart recovery, a fresh Task 921 confirmation, `goalComplete`, and final `no-ready-task` | strong | Exact bundle, binary hash, source-input digest, disposable repository, commands, artifacts, and real/simulated/unproven boundaries are recorded. |

## Founder Evidence

- `docs/raw/product-discussion.md` records the founder prompt, recommended
  product shape, MVP boundary, visual direction, and repository decision.

## Unsupported Important Claims

- The workflow is materially easier to understand than terminal usage.
- The proposed UI model is materially easier to use than terminal-only operation.
- The first Codex JSON adapter contract is stable enough for long-lived or
  non-Codex provider use.

## Prototype Assumptions

| Claim | Source | Reversibility | Validation Required Before Product Truth |
| --- | --- | --- | --- |
| Three-pane workbench plus ribbon is the right control surface | docs/raw/product-discussion.md | easy | usability trial |
| Markdown headings and tables can support useful task projections | docs/raw/product-discussion.md | easy | parser fixtures and real-repo trial |
| Tauri 2 is the right native shell | docs/raw/product-discussion.md | moderate | technically validated for the local MVP; long-term product fit still needs usage evidence |

## Evidence To Gather Next

- Founder usability feedback against the completed signed local MVP.
- Production signing/notarization evidence only if distribution enters scope.
- Representative adapter evidence before claiming provider portability.

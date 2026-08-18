# Sprint 3: Founder-Facing Product Loop

Status: active
Purpose: turn the proved local execution kernel into a coherent founder-facing
workflow from empty repository through discovery, planning, one bounded task,
result review, continuity, and usability proof.

## Requirement Basis

- Founder instruction on 2026-07-22 to document and fix the complete product,
  UI/UX, and engineering audit.
- `docs/evidence/founder-workflow-ui-ux-audit.md`.
- `docs/evidence/sprint-3-post-ha2ha-reconciliation.md`.
- `docs/founder-facing-workflow.md`.
- Sprint 1 technical proof in `docs/evidence/manual-trials.md`.

## Sequencing Gate

Sprint 2 is terminal after Tasks 013-019 changed overlapping native and UI
surfaces. Tasks 031 and 022 installed deterministic authority-drift enforcement
and the unified local/shared product contracts. Task 020 completed the
behavior-preserving frontend extraction, and Task 021 installed focused native
repository/controller seams, Task 023 completed the safe local
artifact-plan/apply boundary, Task 024 completed signed guided
Discover/bootstrap, Task 025 completed the authenticated, previewed, confirmed,
repository-verified planning workflow, and Task 026 completed signed
goal-centered restart/reinspection, and Task 027 completed product-action versus
Developer Tools separation, and Task 028 completed the bounded post-run review
receipt, Task 028A completed the separate safe local Git handoff, and Task 029
completed grouped/searchable navigation plus the signed 900x700 responsive
shell. Task 030 completed the accessibility, keyboard, contrast, zoom,
preference-media, deterministic visual, signed-native, and VoiceOver gates.
The automated Task 032 rehearsal found and Task 033 repaired the missing
founder-facing skill setup, preflight, and founder-gate transitions. Task 032
remains planned because founder participation is required.

## Tasks

| ID | Title | Status | Depends On | Evidence |
| --- | --- | --- | --- | --- |
| 031 | Add docs and authority drift enforcement | complete | Sprint 2 complete | tasks/issues/031-add-docs-and-authority-drift-enforcement.md |
| 022 | Define unified local/shared workflow effects and typed repair contracts | complete | 031 | tasks/issues/022-define-guided-workflow-effects-and-typed-repair-contracts.md |
| 020 | Extract frontend project-session, workflow, and collaboration projections | complete | 022 | tasks/issues/020-extract-frontend-project-session-and-workflow-projections.md |
| 021 | Extract focused native repository and workflow controller modules | complete | 020 | tasks/issues/021-extract-native-repository-and-workflow-controller-modules.md |
| 023 | Implement safe new-project artifact plan and apply boundary | complete | 021-022 | tasks/issues/023-implement-safe-new-project-artifact-plan-and-apply-boundary.md |
| 024 | Build guided Discover and project bootstrap experience | complete | 023 | tasks/issues/024-build-guided-discover-and-project-bootstrap-experience.md |
| 025 | Build functional feature-planning experience | complete | 022, 024 | tasks/issues/025-build-functional-feature-planning-experience.md |
| 026 | Make the shell goal-centered and recovery-aware | complete | 024-025 | tasks/issues/026-make-shell-goal-centered-and-recovery-aware.md |
| 027 | Separate product workflows from developer diagnostics | complete | 022, 026 | tasks/issues/027-separate-product-workflows-from-developer-diagnostics.md |
| 028 | Add post-run diff and evidence review receipt | complete | 021, 027 | tasks/issues/028-add-post-run-diff-and-evidence-review-receipt.md |
| 028A | Add safe local Git handoff and commit boundary | complete | 028 | tasks/issues/028a-add-safe-local-git-handoff-and-commit-boundary.md |
| 029 | Rework navigation, information architecture, and responsive layout | complete | 026-028A | tasks/issues/029-rework-navigation-information-architecture-and-responsive-layout.md |
| 030 | Enforce accessibility and visual behavior | complete | 029 | tasks/issues/030-enforce-accessibility-and-visual-behavior.md |
| 033 | Add founder gate resolution to Discover | complete | 030, Task 032 automated rehearsal | tasks/issues/033-add-founder-gate-resolution-to-discover.md |
| 032 | Run founder usability trial and close the product loop | planned | 020-031, 028A, founder available | tasks/issues/032-run-founder-usability-trial-and-close-product-loop.md |

## Dependency Policy

- Execute in the reconciled order recorded above; task numbers preserve the
  original planning history and are not the execution order.
- Task 031 runs first so later Sprint 3 changes cannot silently drift authority.
- Task 022 composes existing local and Sprint 2 collaboration contracts; it
  must not introduce a second workflow or repair state machine.
- Execute one task at a time through `build-right-execution`.
- Promote a planned task only after all dependencies are complete and its
  contract matches current repository evidence.
- Every UI task must preserve keyboard semantics and readable text; Task 030
  enforces and completes the coverage rather than postponing accessibility.
- Local solo mode and Sprint 2 optional shared mode remain regression gates.
- Bootstrap and Plan never mirror their artifact/task changes into HA2HA;
  publishing one resolver-selected execution envelope remains a separate action.
- Task 032 requires founder participation and must not be selected as ordinary
  autonomous AI execution.

## Sprint Exit

Sprint 3 is complete only when:

- the signed app supports empty-repository bootstrap without a terminal;
- Discover and Plan perform complete previewed, confirmed, repository-verified
  workflows;
- the shell is goal-centered across open, resume, block, review, continue, and
  complete states;
- product actions are distinct from developer diagnostics;
- typed failure evidence selects truthful repair guidance;
- post-run changes, diff, checks, criteria, evidence, risks, and tracker state
  form one review receipt;
- an optional local commit can stage only reviewed paths through a separate
  preview/confirmation and never pushes or changes completion authority;
- navigation, readability, responsive behavior, keyboard access, semantics,
  and critical visual states pass their verification;
- frontend/native module boundaries and authority-doc drift checks are enforced;
- one cohesive signed-native founder trial records friction and every real,
  manual, simulated, and unproved boundary;
- Tasks 020-032 are terminal and all Sprint 1/Sprint 2 guarantees remain green.

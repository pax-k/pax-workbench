# Engineering Principles

This reference is the shared engineering baseline for Build Right-guided code,
docs, generated guidance, reviews, and future package work. It expands product
and architecture commitments into reviewable software and architecture
principles.

These principles are guidance unless backed by code, checks, tests, policy, or
contracts. Critical architecture, security, release, traceability, and provider
boundary rules must still be enforceable outside markdown.

## Operating Summary

Build systems that are easy to change without becoming easy to break:

- keep responsibilities clear
- keep dependencies explicit and acyclic
- depend on contracts before concrete implementations
- isolate side effects behind adapters
- make invalid states hard to represent
- test behavior and boundary contracts
- record enough evidence to debug and review the work later

## Requirements Fit

- Design against validated user needs, product outcomes, and current constraints.
- Complexity is justified when a real requirement demands it.
- Treat capabilities, abstractions, boundaries, and operational machinery without a traceable requirement as speculative complexity.
- Choose the smallest solution that fully satisfies current requirements while preserving required guarantees.
- When a design cannot be traced to product truth, evidence, or an explicit assumption, remove it, defer it, or return it to planning.
- Reconsider the solution when requirements or constraints change.

## Code Principles

### Clear Responsibility

- A module, class, function, package, or workflow step should have one primary reason to change.
- Split code by ownership and behavior, not by arbitrary layers or file-size targets.
- Prefer names that describe domain intent over names that describe implementation mechanics.
- Keep generated code, templates, adapters, and hand-written domain logic visibly separated.

### Readability Before Cleverness

- Prefer direct, boring code when it communicates the behavior.
- Avoid hidden control flow, surprising mutation, ambient globals, broad monkey-patching, and implicit I/O.
- Comments should explain non-obvious intent, constraints, or tradeoffs; they should not restate simple code.

### Explicit Dependencies

- Pass dependencies through function parameters, constructors, registries, or context objects.
- Avoid singleton-style coupling when a dependency can be injected.
- Keep imports pointed in the documented package direction.
- Do not introduce dependency cycles. Cycles make ownership, tests, extraction, and reasoning weaker.

### Small Public Surfaces

- Export the smallest stable API callers need.
- Keep package internals private unless another package has a real ownership need.
- Avoid leaking persistence models, provider SDK shapes, framework request objects, or generated artifact details across boundaries.

### Type And State Discipline

- Prefer precise domain types, discriminated unions, enums, schemas, and result objects over unstructured strings or broad `any`.
- Make invalid states unrepresentable where the language and local patterns allow it.
- Validate external input at trust boundaries.
- Convert external library, provider, or parser failures into local structured result shapes before crossing package boundaries.

### Failure Semantics

- Fail fast when an invariant is broken.
- Fail clearly when user input, environment state, provider capability, or validation is missing.
- Prefer typed or structured errors that can be routed, tested, traced, and repaired.
- Include enough context in errors to identify the failed boundary without leaking secrets.

### Side Effects

- Keep pure policy, state, parsing, routing, and validation logic separate from filesystem, shell, network, provider, VCS, deployment, and tracing effects.
- Make mutation opt-in and visible in API names, command flags, policy decisions, and traces.
- Design important operations to be idempotent when retry, repair, or resume behavior is plausible.

## Architecture Principles

### Dependency Direction

- Stable domain and contract packages should not depend on volatile provider, CLI, framework, plugin, or generated-artifact packages.
- Higher-level policy should depend on abstractions. Concrete adapters should depend on the contracts they implement.
- Package boundaries should be documented and checked when violations would be costly.

### Ports, Adapters, And Contracts

- Core behavior defines ports, contracts, registries, policies, states, effects, and trace shapes.
- Concrete implementations live behind adapters: agent runtimes, model providers, filesystems, git, CI, deployment, issue trackers, observability sinks, databases, APIs, and cloud services.
- Add new implementation variants by registering or injecting them instead of scattering conditionals through the kernel.
- Contracts should be testable with at least one real implementation or deterministic fake.

### Open For Extension, Closed For Breakage

- Prefer extension points that are typed, validated, observable, and policy-aware.
- Avoid premature abstractions. Add an abstraction when it removes real duplication, isolates volatility, or matches an existing extension pattern.
- New extension points should define ownership, lifecycle, validation, failure behavior, and trace evidence.

### Open Ecosystem Standards

- Treat public primitives as standards, not incidental internal types.
- Every public primitive should have an owning package, typed contract, serialized schema when relevant, valid and invalid examples, compatibility notes, and tests.
- Every extension point should have a local validation or conformance path before it becomes a public ecosystem promise.
- Every reusable package should be useful on its own and should document why a user would install only that package.
- Partial adoption is a design requirement: internal packages, contracts, validators, deployable apps, and hosted services should remain understandable and testable without requiring the whole platform to run at once.
- Registry artifacts should be portable, reviewable, and low-trust by default.
- Code or plugin extensions require explicit install, permission projection, policy approval, bounded execution, structured output validation, and trace evidence.
- Backward compatibility is part of product quality. Breaking public contract changes need maturity status, migration notes, validation evidence, and docs updates.
- Reference implementations should be minimal and readable so contributors can learn the contract without reverse-engineering production code.
- Markdown guidance is not a standard unless backed by schemas, tests, checks, policy, conformance suites, or examples.

### Cohesion And Coupling

- Keep behavior that changes together in the same module or package.
- Keep unrelated domains from sharing mutable state or low-level utility paths as a shortcut.
- Cross-cutting concerns such as logging, config, validation, and errors should have approved access patterns.

### Boundary Ownership

- Every package, app, workflow, generated artifact, table, schema, or public command should have a clear owner.
- Public contracts should be versioned or evolved compatibly when consumers exist.
- A breaking semantic change needs a task, validation evidence, and docs updates.

### Evolutionary Architecture

- Favor decisions that preserve future movement: replaceable providers, adapter-owned mechanics, thin generated artifacts, and explicit contracts.
- Defer irreversible decisions until the system has evidence.
- Record significant architectural choices in docs or ADRs so future agents know why the shape exists.

## Delivery Principles

### Testing

- Test behavior and contracts, not private implementation details.
- Add regression tests for meaningful bugs.
- Keep fast checks fast; use targeted integration, fixture, e2e, or dogfood tests for boundary behavior.
- Make tests deterministic by controlling time, randomness, network, filesystem roots, and provider behavior.

### Observability

- Important workflows should emit enough logs, traces, evidence, and structured results to reconstruct what happened.
- Trace state transitions, policy decisions, effect planning, provider execution, validation, failures, and repair recommendations.
- Observability is part of the design, not a cleanup task after failures become confusing.

### Security And Reliability

- Apply least privilege to users, tools, tokens, files, services, and provider capabilities.
- Treat every external input as untrusted.
- Add timeouts, bounded retries, and explicit stop conditions around external calls and long-running workflows.
- Keep secrets out of logs, traces, generated docs, and task evidence.
- Design migrations, repairs, and deployment steps with rollback, forward-fix, or staged rollout expectations.

### Documentation And Automation

- Keep docs close to the code, command, contract, or workflow they describe.
- Docs should identify authority: what is guidance, what is enforced, and which command proves it.
- Automate repeatable checks instead of relying on reviewer memory.
- Update navigation when adding new authority surfaces.

## Build Right Application

- Follow the target repo's root and local agent instructions for ownership and dependency direction.
- Follow the target repo's product truth, MVP scope, architecture, test, and execution-rule docs before changing product boundaries.
- Keep generated or agent-authored task files thin; reusable behavior belongs in packages and validated workflows.
- Treat markdown skills, instructions, and templates as guidance. Programmatic policy, checks, state, tests, contracts, and traces are authoritative.
- Prefer structured results with `ok`, `errors`, `warnings`, `evidence`, and typed data when matching local package patterns.
- Keep Build Right lifecycle skills as workflow guidance, and promote repeated engineering findings into target-repo enforcement surfaces when possible.

## Review Checklist

Use this checklist for implementation and review:

- Requirements fit: Which requirement or constraint justifies this design?
- Product fit: Which user and outcome does it serve?
- Tradeoff: What cost, guarantee, or simplicity does it sacrifice?
- Speculation: Is any complexity justified only by hypothetical future needs?
- Sufficiency: Would a smaller solution fully satisfy the same requirements?
- Responsibility: Does each changed module have a clear owner and reason to change?
- Boundary: Are package and provider boundaries preserved?
- Dependency direction: Are imports acyclic and pointed toward stable contracts?
- Contracts: Are new public shapes typed, documented, validated, and tested?
- Adapters: Are concrete provider, framework, or infrastructure details isolated?
- State: Are invalid or ambiguous states prevented or validated?
- Effects: Are filesystem, shell, network, deployment, VCS, and trace effects explicit?
- Failure: Are errors structured enough to route, debug, test, and repair?
- Tests: Is changed behavior protected at the right level?
- Observability: Can a future maintainer reconstruct decisions and failures?
- Docs: Did changed commands, contracts, ownership, generated artifacts, or workflows update navigation and guidance?

## Enforcement Surfaces

Principles become durable when they are enforced through:

- package-boundary tests
- import/dependency checks
- type checks
- schema validation
- unit, contract, integration, fixture, e2e, and dogfood tests
- policy gates and allowed-effect checks
- generated-guidance validation
- docs-drift checks
- trace and evidence requirements

When repeated review findings expose a gap, promote the gap into one of these
enforcement surfaces instead of only adding more prose.

## Update Triggers

Update this reference when Build Right changes its package model, provider
adapter model, generated artifact contract, validation strategy, security
baseline, observability contract, or definition of done.

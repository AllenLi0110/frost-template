# AI Collaboration Log

Use this log to preserve the review story: what was asked, what AI produced, where it was wrong, how it was corrected, and how the final behavior was verified.

### 2026-06-23 - Phase 0 Roadmap Framing Correction

Prompt summary:
- Asked whether the implementation roadmap looked too much like a fully solved up-front plan, instead of an iterative phase-by-phase AI-native workflow.

Important context provided:
- The implementation should proceed one phase at a time, with prompts refined after learning from each completed phase.

AI output summary:
- Updated the roadmap framing to describe it as a working hypothesis and living artifact.
- Updated the AI-native README to clarify that prompts should evolve after each phase.

Human corrections:
- The roadmap should not imply every technical detail is already known before implementation.

Verification:
- Command: not run
- Result: documentation-only update

Evidence links:
- Files: `docs/ai-native/01-implementation-roadmap.md`, `docs/ai-native/README.md`

Follow-up:
- Before starting Phase 1, review and refine `prompts/01-project-foundation.md` based on the current repository state.

### 2026-06-24 - Phase 0 Automation Scaffold

Prompt summary:
- Asked how to evolve the project from manual AI-native development toward a fully automated agent system.

Important context provided:
- The target workflow should not let an agent directly push to `main` or merge its own work.
- Automation should use event triggers, isolated branches, verification, PRs, CI, human review, and trace-based improvement.

AI output summary:
- Added loop engineering documentation for agent, verification, event-driven, and self-improvement loops.
- Added automation design documentation for issue-triggered agent work, branch strategy, PR gates, and trace requirements.
- Added a verification harness document and `scripts/verify-phase.sh`.
- Added GitHub issue and PR templates for future agent tasks.
- Added a CI skeleton that checks Phase 0 repository hygiene and frontend linting.
- Updated the AI-native README, roadmap, and operating rules to explain the automation-ready workflow.

Human corrections:
- The project should become automation-ready, but human approval should remain the merge gate.

Verification:
- Command: `./scripts/verify-phase.sh 0`
- Result: passed
- Command: `npm --prefix frontend run lint`
- Result: passed

Evidence links:
- Files: `docs/ai-native/03-loop-engineering.md`, `docs/ai-native/04-automation-design.md`, `docs/ai-native/05-verification-harness.md`, `docs/ai-native/templates/agent-task.md`, `docs/ai-native/templates/agent-run-report.md`, `.github/ISSUE_TEMPLATE/agent-task.yml`, `.github/pull_request_template.md`, `.github/workflows/ci.yml`, `scripts/verify-phase.sh`

Follow-up:
- Use a branch and PR for Phase 1 instead of continuing to make feature work directly on `main`.

### 2026-06-24 - Phase 1 Project Foundation

Prompt summary:
- Started Phase 1 from the `agent/phase-1-foundation` branch to make all runtime services boot and expose health contracts.

Important context provided:
- Phase 1 should only establish the foundation stack: coordinator, TSS nodes, PostgreSQL, frontend, env config, initial migrations, and tests.
- FROST DKG, wallet derivation, signing, and one-click shortcuts remain out of scope.

AI output summary:
- Added a project foundation BDD feature and foundation contract document.
- Converted coordinator and tss-node from hello-world binaries into testable axum services.
- Added config loading from environment variables with defaults and validation.
- Added coordinator `/health` and `/health/nodes`.
- Added tss-node `/health`.
- Added Docker Compose for PostgreSQL 18, coordinator, node-a, node-b, and frontend.
- Added sanitized `.env.example`.
- Added initial schema migration for `coordinator`, `node_a`, and `node_b`.
- Expanded `scripts/verify-phase.sh 1` into an executable Phase 1 verification harness.

Human corrections:
- None during this phase.

Verification:
- Command: `docker compose run --rm --no-deps coordinator sh -c "rustup component add rustfmt >/dev/null && cargo fmt --all"`
- Result: passed
- Command: `docker compose run --rm --no-deps coordinator cargo test --workspace`
- Result: passed
- Command: `npm --prefix frontend run lint`
- Result: passed
- Command: `docker compose config`
- Result: passed
- Command: `docker compose up -d --force-recreate`
- Result: passed
- Command: compose-network smoke test for coordinator `/health`, node-a `/health`, node-b `/health`, and coordinator `/health/nodes`
- Result: all endpoints returned HTTP 200 and both nodes were reachable
- Command: `./scripts/verify-phase.sh 1`
- Result: passed after rerunning outside the sandbox because Docker daemon access is required

Evidence links:
- Files: `features/project-foundation.feature`, `docs/contracts/foundation.md`, `backend/coordinator/src/lib.rs`, `backend/tss-node/src/lib.rs`, `docker-compose.yml`, `.env.example`, `backend/migrations/0001_create_foundation_schemas.sql`, `scripts/verify-phase.sh`, `docs/ai-native/logs/phase-1-agent-run-report.md`

Follow-up:
- Phase 2 should build on the Phase 1 axum routers, config structs, Docker Compose topology, and migration layout.

## Entry Template

### YYYY-MM-DD - Phase Name

Prompt summary:
- 

Important context provided:
- 

AI output summary:
- 

Human corrections:
- 

Verification:
- Command:
- Result:

Evidence links:
- Files:
- Screenshots:

Follow-up:
- 

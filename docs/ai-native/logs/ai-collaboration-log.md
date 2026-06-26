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

### 2026-06-24 - Phase 2 DKG State Machine

Prompt summary:
- Started Phase 2 from the `agent/phase-2-dkg-state-machine` branch to make DKG observable, persistent, manually triggerable, and visible in the frontend.

Important context provided:
- The original Phase 2 prompt focused on backend state transitions, but the project still showed the default Next.js starter page after Phase 1.
- The Phase 2 scope was updated to include the first reviewer-facing DKG control surface and to record that this was a dynamic adjustment.
- Placeholder crypto is acceptable only if isolated behind a node-side crypto service boundary.

AI output summary:
- Added Phase 2 DKG BDD scenarios for frontend visibility, out-of-order rejection, idempotent replay, and restart persistence.
- Added a DKG state machine contract covering coordinator API, node internal API, database tables, status values, errors, and frontend requirements.
- Added coordinator DKG session and node-step persistence with migration `0002_create_dkg_tables.sql`.
- Added coordinator public DKG APIs for create/read active session and per-node/per-round triggering.
- Added transition validation for Round 2 and Round 3 prerequisites.
- Added idempotent completed-step replay using stored public payloads.
- Added TSS node internal DKG endpoints backed by `DkgCryptoService` and `PlaceholderDkgCryptoService`.
- Replaced the default Next.js starter page with a DKG control surface and a Next.js coordinator proxy route.
- Updated Phase 2 verification to run tests, lint/build, compose health checks, DKG smoke tests, restart persistence, and frontend load checks.

Human corrections:
- Phase 2 needed to include a visible frontend control surface because API-only progress made the project hard to inspect.

Verification:
- Command: `docker compose run --rm --no-deps coordinator cargo test --workspace`
- Result: passed
- Command: `npm --prefix frontend run lint`
- Result: passed
- Command: `npm --prefix frontend run build`
- Result: passed outside the sandbox after Turbopack process creation was blocked inside the sandbox
- Command: `./scripts/verify-phase.sh 2`
- Result: passed
- Command: desktop and mobile headless browser screenshots
- Result: DKG control surface rendered; mobile layout metrics reported `innerWidth=390`, `scrollWidth=390`, and no overflowing elements

Evidence links:
- Files: `features/dkg-flow.feature`, `docs/contracts/dkg-state-machine.md`, `backend/migrations/0002_create_dkg_tables.sql`, `backend/coordinator/src/lib.rs`, `backend/tss-node/src/lib.rs`, `frontend/app/page.tsx`, `frontend/app/api/coordinator/[...path]/route.ts`, `scripts/verify-phase.sh`, `docs/ai-native/logs/phase-2-agent-run-report.md`
- Screenshots: `/tmp/frost-template-phase2-dkg-ui.png`, `/tmp/frost-template-phase2-dkg-mobile-fixed2.png`

Follow-up:
- Phase 3 should replace the placeholder DKG implementation behind `DkgCryptoService` with real `frost-ed25519` behavior while preserving the coordinator API contract and frontend workflow.

### 2026-06-24 - Phase 3 FROST DKG Crypto Integration

Prompt summary:
- Started Phase 3 from the `agent/phase-3-frost-dkg-crypto` branch to replace placeholder DKG behavior with real FROST Ed25519 DKG.

Important context provided:
- Coordinator public APIs and frontend workflow from Phase 2 should remain stable.
- Root shares, nonce secrets, and long-lived key packages must stay out of Coordinator and Frontend.
- Cargo resolves `frost-ed25519` to `2.2.0` even though the manifest declares `2.1.0`.

AI output summary:
- Added Phase 3 BDD scenarios for real FROST DKG and node-local private material boundaries.
- Added a FROST DKG crypto contract describing internal package maps, response payloads, and forbidden coordinator fields.
- Added node-local DKG state tables in `node_a` and `node_b`.
- Replaced `PlaceholderDkgCryptoService` with `FrostDkgCryptoService`.
- Added node-local encryption for Round 1 secret package, Round 2 secret package, and final key package.
- Updated Coordinator to route peer Round 1 and Round 2 packages from stored step payloads.
- Updated Coordinator completion logic to store the real matching master public key from Round 3 payloads.
- Redacted Round 2 routing packages from frontend-facing coordinator responses while keeping them available for internal Round 3 routing.
- Added Phase 3 verification harness for real DKG, node-local private persistence, coordinator forbidden-field checks, and restart persistence.

Human corrections:
- None during this phase.

Verification:
- Command: `docker compose run --rm --no-deps coordinator cargo test --workspace`
- Result: passed
- Command: `npm --prefix frontend run lint`
- Result: passed
- Command: `npm --prefix frontend run build`
- Result: passed
- Command: `./scripts/verify-phase.sh 3`
- Result: passed
- Note: `cargo fmt --check` could not run because the Docker Rust image does not include `rustfmt`.

Evidence links:
- Files: `features/dkg-flow.feature`, `docs/contracts/frost-dkg-crypto.md`, `backend/migrations/0003_create_node_dkg_state.sql`, `backend/coordinator/src/lib.rs`, `backend/tss-node/src/lib.rs`, `docker-compose.yml`, `.env.example`, `scripts/verify-phase.sh`, `docs/ai-native/logs/phase-3-agent-run-report.md`

Follow-up:
- Phase 4 should derive Solana wallet addresses from the completed public DKG context while keeping private child share derivation node-local.

### 2026-06-24 - Phase 4 Wallet Derivation

Prompt summary:
- Started Phase 4 from the `agent/phase-4-wallet-derivation` branch to derive Solana wallet addresses from completed DKG public material.

Important context provided:
- Wallet creation must wait for completed DKG.
- Coordinator may use public derivation material, but must not store private child shares.
- Node A and Node B should not be required for public address derivation.
- Balance lookup should fail gracefully when Solana RPC is unavailable.

AI output summary:
- Added wallet derivation BDD scenarios and a wallet derivation contract.
- Added `coordinator.dkg_sessions.public_derivation_context` and `coordinator.wallets`.
- Added Coordinator wallet APIs for create, list, and balance refresh.
- Implemented deterministic `hd-wallet` Edwards non-hardened public derivation.
- Added Solana JSON-RPC balance lookup with `AVAILABLE` / `UNAVAILABLE` status.
- Sanitized Solana RPC error messages before storing or returning them.
- Added frontend wallet panel for wallet creation, address display, balance refresh, and sender selection.
- Added Phase 4 verification harness.

Human corrections:
- None during this phase.

Verification:
- Command: `docker compose run --rm --no-deps coordinator cargo test --workspace`
- Result: passed
- Command: `npm --prefix frontend run lint`
- Result: passed
- Command: `npm --prefix frontend run build`
- Result: passed
- Command: `./scripts/verify-phase.sh 4`
- Result: passed
- Note: frontend dev CSS was refreshed by restarting the frontend container after visual inspection showed a stale stylesheet.

Evidence links:
- Files: `features/wallet-derivation.feature`, `docs/contracts/wallet-derivation.md`, `backend/migrations/0004_create_wallet_tables.sql`, `backend/coordinator/src/lib.rs`, `frontend/app/page.tsx`, `frontend/app/globals.css`, `scripts/verify-phase.sh`, `docs/ai-native/logs/phase-4-agent-run-report.md`

Follow-up:
- Phase 5 should add transfer preparation and threshold signing while keeping child private material node-local.

### 2026-06-24 - CI And Versioning Foundation

Prompt summary:
- After Phase 5, the user noticed that pull requests were not automatically enforcing full frontend/backend CI and asked to add proper CI/CD and version management.

Important context provided:
- The project already had local phase verification scripts.
- The user wanted GitHub PRs to run tests before merge and wanted release/version checkpoints.
- Earlier phase history should be preserved instead of rewritten.

AI output summary:
- Expanded GitHub Actions CI to run repository hygiene, backend tests, frontend lint/build, and integration verification.
- Added release metadata through `VERSION`, `CHANGELOG.md`, frontend package version, and backend workspace version.
- Added release process documentation and a draft GitHub release workflow for matching `v*` tags.
- Updated the PR template, roadmap, automation design, verification harness, and decision log.

Human corrections:
- None yet.

Verification:
- Command: `node scripts/verify-release-metadata.mjs`
- Result: passed
- Command: `node scripts/extract-release-notes.mjs v0.1.1`
- Result: passed
- Command: `bash -n scripts/verify-phase.sh`
- Result: passed
- Command: `./scripts/verify-phase.sh 0`
- Result: passed
- Command: `npm --prefix frontend run lint`
- Result: passed
- Command: `npm --prefix frontend run build`
- Result: passed
- Command: `docker compose run --rm --no-deps coordinator cargo test --workspace`
- Result: passed
- Command: `./scripts/verify-phase.sh 5`
- Result: passed
- Note: GitHub CI will run after the branch is pushed and the PR is opened.

Evidence links:
- Files: `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `VERSION`, `CHANGELOG.md`, `docs/release-process.md`, `scripts/verify-release-metadata.mjs`, `scripts/extract-release-notes.mjs`

Follow-up:
- Configure GitHub branch protection for `main` so required checks must pass before merge.

### 2026-06-25 - CI Readiness Fix

Prompt summary:
- The user reported that PR #6 CI did not pass after the branch protection work.

Important context provided:
- GitHub Actions had produced the required checks, but the integration job failed.

AI output summary:
- Inspected the GitHub Actions failure and found the integration harness hit `ECONNREFUSED` while waiting for Coordinator on a fresh runner.
- Increased the Phase 5 initial health polling window from 60 seconds to 240 seconds.
- Renamed the CI check from `Phase 5 integration verification` to `Integration verification` so Phase 6 and later phases can keep using the same required check.

Human corrections:
- None.

Verification:
- Command: `bash -n scripts/verify-phase.sh`
- Result: passed
- Command: `./scripts/verify-phase.sh 0`
- Result: passed
- Command: `git diff --check`
- Result: passed
- Command: `./scripts/verify-phase.sh 5`
- Result: passed

Evidence links:
- Files: `.github/workflows/ci.yml`, `scripts/verify-phase.sh`, `docs/release-process.md`, `docs/ai-native/04-automation-design.md`

Follow-up:
- Update the GitHub ruleset required check from `Phase 5 integration verification` to `Integration verification` after the new CI run appears.

### 2026-06-25 - Phase 6 Broadcast And Confirmation

Prompt summary:
- The user asked to start Phase 6 after merging the CI/versioning foundation.

Important context provided:
- Phase 6 must aggregate FROST signature shares, broadcast a Solana Devnet transfer, and confirm it.
- Phase 5 signed a transfer intent, so Phase 6 needed to avoid broadcasting a signature that did not match the transaction signer.

AI output summary:
- Changed Round 2 signing to use the exact serialized Solana transfer message.
- Implemented node-local child-share signing for the selected wallet index.
- Added Coordinator broadcast and confirmation endpoints.
- Added transaction signature and Explorer URL display in the frontend.
- Added a CI-safe Phase 6 integration harness using mock Solana RPC.

Human corrections:
- None yet.

Verification:
- Command: `docker compose run --rm --no-deps coordinator cargo test --workspace`
- Result: passed
- Command: `npm --prefix frontend run lint`
- Result: passed
- Command: `npm --prefix frontend run build`
- Result: passed
- Command: `./scripts/verify-phase.sh 6`
- Result: passed

Evidence links:
- Files: `backend/coordinator/src/lib.rs`, `backend/tss-node/src/lib.rs`, `frontend/app/page.tsx`, `frontend/app/globals.css`, `docs/contracts/signing-state-machine.md`, `scripts/verify-phase.sh`, `docs/ai-native/logs/phase-6-agent-run-report.md`

Follow-up:
- Record manual Devnet verification after funding a derived wallet and broadcasting a real transfer.

### 2026-06-25 - Phase 7 Reviewer Experience And Hardening

Prompt summary:
- The user asked to start Phase 7 after finishing the Solana broadcast and confirmation flow.

Important context provided:
- The reviewer needs to understand what the project does from the UI, not just see buttons.
- The user had already tested Devnet funding and observed a finalized Explorer transaction during Phase 6.
- Earlier troubleshooting showed that mock RPC verification must not contaminate the normal Devnet demo stack.

AI output summary:
- Reworked README into a reviewer-first handoff guide with setup, health checks, local verification, manual acceptance, API reference, AI evidence, CI/versioning, troubleshooting, and out-of-scope notes.
- Added BDD coverage for the reviewer experience.
- Added a reviewer-experience contract.
- Extended Phase 7 verification to check README handoff content, AI evidence files, backend tests, frontend lint, and frontend build.

Human corrections:
- Treat Devnet SOL as test money only.
- Clearly explain where to copy derived wallet addresses and how to fund them.
- Keep manual Devnet transfer success as a human acceptance step, not a CI dependency.

Verification:
- Command: `bash -n scripts/verify-phase.sh`
- Result: passed
- Command: `node scripts/verify-release-metadata.mjs`
- Result: passed
- Command: `./scripts/verify-phase.sh 7`
- Result: passed

Evidence links:
- Files: `README.md`, `features/reviewer-experience.feature`, `docs/contracts/reviewer-experience.md`, `docs/ai-native/logs/phase-7-agent-run-report.md`, `docs/ai-native/05-verification-harness.md`, `scripts/verify-phase.sh`

Follow-up:
- Run the manual acceptance checklist on the PR branch before merge if reviewer screenshots or fresh Devnet evidence are requested.

### 2026-06-26 - Phase 8 Crypto Dashboard UX

Prompt summary:
- The user asked how to make the app feel more like a crypto workflow, then started Phase 8.

Important context provided:
- The existing app works but reads like a protocol control panel.
- The user wants a reviewer-friendly crypto operations dashboard without changing the underlying challenge requirements.
- After the first pass, the user clarified that the style should feel more like a mobile one-page flow with animated process steps.
- The user later said the current UI/UX was still unsatisfactory and asked for a larger crypto/mobile redesign.
- The user then clarified the target further: reference OKX/exchange-style clarity and exchange registration/login-style step transitions, keep everything in one screen, avoid page scrolling, and keep the result simple.

AI output summary:
- Added Phase 8 prompt, BDD scenarios, and crypto dashboard UX contract.
- Planned a frontend-only reframing into an MPC wallet dashboard.
- Preserved all manual DKG and signing controls as non-negotiable challenge behavior.
- Added active/completed/queued workflow states, active-step animation, and reduced-motion handling for a mobile-friendly one-page demo flow.
- Reworked the visual system toward a mobile wallet app: dark app header, horizontal workflow rail, swipeable summary chips, persistent vault watch, and more compact operation cards.
- Reworked the UI again into a single-screen exchange-style terminal: clickable scene stepper, active scene panel, compact side/bottom wallet status, and no page-level scroll in the normal demo path.
- Refined the mobile layout again after the user pointed out cramped DKG round cards and crowded metrics: DKG controls became compact rows, and summary state became a primary status card with secondary metric chips.
- Refined workflow selection semantics after the user clarified that `Now` should not imply selected-scene highlighting, and added a copy-vault-address button to the side action panel.
- Refined the Transfer Intent form after the user noted the form had awkward blank space and an oversized `Create Ticket` action.
- Added manual `Next Step` gates after Vault Funding and Threshold Signing so demo recording does not jump scenes before the user is ready.
- Replaced light table rows with dark wallet-style cards for vaults, transfer tickets, and broadcast receipt controls.
- Added a dashboard screenshot to the top of README for immediate reviewer context.

Human corrections:
- Requested a mobile-friendly one-page style with animated workflow steps before committing and opening the draft PR.
- Rejected the mobile wallet/dashboard feel and requested a cleaner exchange-style single-screen flow with animated scene transitions.
- Noted that the stage-to-scene flow was correct, but the DKG control cards and summary metrics were visually cramped and needed better presentation.
- Clarified that the current `Now` stage may stay labeled, but only the scene being viewed should receive the highlighted border.
- Clarified that the ticket form should not use a large landing-page-style CTA inside the compact terminal layout.

Verification:
- Command: `bash -n scripts/verify-phase.sh`
- Result: passed
- Command: `npm --prefix frontend run lint`
- Result: passed
- Command: `npm --prefix frontend run build`
- Result: passed
- Command: `./scripts/verify-phase.sh 8`
- Result: passed
- Check: `390x844` browser viewport
- Result: passed; no horizontal overflow, single-screen terminal present, workflow scene switching present, reduced-motion CSS present

Evidence links:
- Files: `features/crypto-dashboard-ux.feature`, `docs/contracts/crypto-dashboard-ux.md`, `docs/ai-native/prompts/08-crypto-dashboard-ux.md`, `frontend/app/page.tsx`, `frontend/app/globals.css`

Follow-up:
- Capture a short demo video after the Phase 8 UI is reviewed.

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

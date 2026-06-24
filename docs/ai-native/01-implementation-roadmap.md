# Adaptive Implementation Roadmap

This roadmap is a working hypothesis, not a fixed master plan. It exists so each AI session has a clear next step, but the details should evolve as we learn from implementation, tests, library behavior, and reviewer constraints.

Only the current phase should be treated as actionable. Later phases are intentionally written as rough placeholders so we can explain the intended direction without pretending every implementation detail is already solved.

After each phase:

1. Record what was learned in `logs/ai-collaboration-log.md`.
2. Record important architecture choices in `logs/decision-log.md`.
3. Update the next phase prompt before asking an agent to continue.
4. Keep scope small enough that one agent session can finish and verify it.

## Phase 0: AI-Native Bootstrap

Goal: Create BDD scenarios, agent context, prompts, logs, and automation-ready guardrails for future agent work.

Outputs:
- `features/*.feature`
- `docs/ai-native/*`
- `.github/ISSUE_TEMPLATE/agent-task.yml`
- `.github/pull_request_template.md`
- `.github/workflows/ci.yml`
- `scripts/verify-phase.sh`

Definition of done:
- Future agents can understand the project without reading the whole assignment from scratch.
- The collaboration process has a place to record prompts and corrections.
- Agent tasks have a branch/PR workflow with verification and human approval gates.
- Phase 0 verification passes with `./scripts/verify-phase.sh 0`.

## Phase 1: Project Foundation

Goal: Make all services boot with real HTTP servers and a shared database.

Scope:
- Coordinator axum server with `/health`.
- TSS node axum server with `/health`.
- Environment config.
- Docker Compose for PostgreSQL, coordinator, node A, node B, frontend.
- Initial SQL migrations.
- Minimal backend tests.

Definition of done:
- `docker compose up` starts every service.
- Coordinator can reach node A and node B health endpoints.
- Backend tests cover config loading and health routes.

Suggested prompt:
- `prompts/01-project-foundation.md`

## Phase 2: DKG State Machine

Goal: Implement the observable DKG workflow before deep crypto integration.

Scope:
- DKG session API.
- Node step status.
- Coordinator state transitions.
- TSS node internal DKG endpoints.
- Persistence and idempotency.
- Tests for invalid transitions.
- Frontend DKG control surface and coordinator proxy.

Phase 2 scope update:
- The initial Phase 2 outline focused on backend state transitions. After Phase 1, the project still showed the default Next.js starter page, so Phase 2 now also includes the first visible protocol control surface. This keeps the demo reviewer-facing instead of API-only.

Definition of done:
- Frontend and API clients can independently trigger Node A/B Round 1, 2, and 3.
- Coordinator rejects out-of-order rounds.
- Completed DKG status persists after restart.

Suggested prompt:
- `prompts/02-dkg-state-machine.md`

## Phase 3: FROST DKG Crypto Integration

Goal: Replace placeholder DKG behavior with `frost-ed25519`.

Scope:
- Crypto adapter boundary.
- Node local encrypted key material persistence.
- Public DKG package routing through coordinator.
- Master public key output.

Phase 3 scope clarification:
- The public coordinator API remains the Phase 2 state-machine API.
- Coordinator sends internal peer package maps to TSS nodes for Round 2 and Round 3.
- Node schemas persist encrypted secret packages and key packages; the coordinator schema stores only step payloads and the master public key.

Definition of done:
- 2-of-2 DKG produces a master public key.
- Coordinator never stores private root shares.
- Unit tests prove private state stays node-local.

Suggested prompt:
- `prompts/03-frost-dkg-crypto.md`

## Phase 4: Wallet Derivation

Goal: Derive multiple Solana addresses from the completed root material.

Scope:
- Wallet API.
- Public derivation context.
- Sequential wallet indexes.
- Balance lookup through Solana Devnet RPC.
- Wallet list UI.

Definition of done:
- Clicking Create Wallet creates index 0, then 1, then 2.
- Node-to-node communication is not required for address derivation.
- Wallets persist after restart.

Suggested prompt:
- `prompts/04-wallet-derivation.md`

## Phase 5: Signing Request State Machine

Goal: Implement step-by-step signing request orchestration before broadcast.

Scope:
- Transfer intent API.
- Pending request list.
- Signing Round 1 commitments.
- Signing Round 2 signature shares.
- Node step status.
- Nonce single-use protection.

Definition of done:
- Users can create multiple signing requests and select one.
- Node A/B signing rounds are independently triggerable.
- Round 2 cannot run before both commitments exist.
- Nonces cannot be reused.

Suggested prompt:
- `prompts/05-signing-state-machine.md`

## Phase 5.5: CI And Versioning Foundation

Goal: Turn the local verification harness into a GitHub PR gate and introduce release version checkpoints before Phase 6.

Why this phase exists:
- By Phase 5, the project had meaningful backend, frontend, Docker, and protocol verification, but GitHub PRs were not yet protected by complete CI.
- Adding this now preserves the existing phase history while improving the engineering workflow for all remaining work.

Scope:
- Expand GitHub Actions CI to run repository hygiene, backend tests, frontend lint/build, and integration verification.
- Add version metadata with `VERSION`, `CHANGELOG.md`, package versions, and release docs.
- Add a release workflow that creates a draft GitHub release from a matching `v*` tag.
- Update PR and automation docs so CI passed status is required before merge.

Definition of done:
- `./scripts/verify-phase.sh 0` verifies release metadata.
- GitHub CI runs automatically on pull requests.
- `main` can be protected with required status checks.
- Future phases use version/changelog impact as part of PR review.

## Phase 6: Aggregation, Solana Broadcast, Confirmation

Goal: Aggregate FROST signature shares, build Solana transfer transactions, broadcast to Devnet, and confirm.

Scope:
- Fresh recent blockhash handling.
- Child-wallet share signing for the selected `wallet_index`.
- Signature share aggregation and final signature verification.
- Solana transfer transaction construction.
- Broadcast and confirmation polling.
- Explorer link.
- Mock RPC integration verification that does not require CI funds.

Definition of done:
- A funded derived wallet can send Devnet SOL.
- UI shows Broadcasted, Confirmed, or Failed.
- Confirmed means Solana returned confirmed status.
- Private root and child shares remain node-local.

Suggested prompt:
- `prompts/06-broadcast-confirmation.md`

## Phase 7: Reviewer Experience And Hardening

Goal: Make the assignment easy to run, inspect, and evaluate.

Scope:
- README run guide.
- `.env.example`.
- Docker Compose polish.
- AI collaboration documentation.
- Security review.
- End-to-end acceptance checklist.

Definition of done:
- Reviewer can run one command and follow README acceptance steps.
- AI workflow documentation explains prompts, corrections, decisions, and test evidence.

Suggested prompt:
- `prompts/07-reviewer-hardening.md`

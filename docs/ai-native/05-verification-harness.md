# Verification Harness

The verification harness defines the checks that prove a phase is complete.

Commands may evolve as the implementation grows. When a command changes, update both this document and `scripts/verify-phase.sh`.

## General Checks

Run for documentation-only changes:

```bash
! grep -RInE "/Users/[[:alnum:]_.-]+/|([A-Z0-9_]*(SECRET|PRIVATE_KEY|API_KEY)[A-Z0-9_]*[[:space:]]*[:=])" docs features .github scripts
git diff --check
node scripts/verify-release-metadata.mjs
```

## Phase 0: AI-Native Bootstrap

Purpose:
- Confirm the project has safe AI-native instructions, prompts, logs, and automation scaffolding.

Checks:

```bash
! grep -RInE "/Users/[[:alnum:]_.-]+/|([A-Z0-9_]*(SECRET|PRIVATE_KEY|API_KEY)[A-Z0-9_]*[[:space:]]*[:=])" docs features .github scripts
git diff --check
test -f docs/ai-native/00-agent-context.md
test -f docs/ai-native/01-implementation-roadmap.md
test -f docs/ai-native/03-loop-engineering.md
test -f docs/ai-native/04-automation-design.md
test -f VERSION
test -f CHANGELOG.md
test -f docs/release-process.md
test -f scripts/verify-phase.sh
node scripts/verify-release-metadata.mjs
```

## Phase 1: Project Foundation

Purpose:
- Confirm the runtime stack can boot and basic service contracts work.

Expected checks after implementation:

```bash
./scripts/verify-phase.sh 1
```

The script verifies:

- Docker Compose config.
- Backend workspace tests.
- Frontend lint.
- Docker Compose startup.
- Coordinator `/health`.
- Coordinator `/health/nodes`.
- Node A `/health`.
- Node B `/health`.

## Phase 2: DKG State Machine

Purpose:
- Confirm DKG is observable, step-by-step, idempotent, and persisted.

Expected checks after implementation:

```bash
./scripts/verify-phase.sh 2
```

The script verifies:

- Sensitive-pattern scan and whitespace checks.
- Docker Compose config.
- Backend workspace tests.
- Frontend lint.
- Frontend production build.
- Docker Compose startup.
- Coordinator, Node A, Node B, and node registry health.
- Node internal API ports are not published to the host.
- DKG session creation.
- Concurrent DKG session creation returns the same active session instead of a database error.
- Round 2 is rejected before both Round 1 steps complete.
- Round 3 is rejected before both Round 2 steps complete.
- Duplicate in-flight round triggers return either the completed replay or HTTP `409`, not a second unsafe state transition.
- Re-triggering a completed step returns the stored result.
- Completed session survives coordinator restart.
- Frontend can load the active session.

Note:
- Phase 2 verification truncates `coordinator.dkg_sessions` in the local Docker Compose database before the DKG smoke test so the workflow starts from a deterministic empty active session.

## Phase 3: FROST DKG Crypto Integration

Purpose:
- Replace placeholder DKG with real FROST DKG while preserving private-material boundaries.

Expected checks:

```bash
./scripts/verify-phase.sh 3
```

The script verifies:

- Sensitive-pattern scan and whitespace checks.
- Docker Compose config.
- Backend workspace tests.
- Frontend lint.
- Frontend production build.
- Docker Compose startup.
- 2-of-2 DKG produces a master public key.
- Node A and Node B persist their own private material.
- Coordinator stores only public metadata.
- Coordinator API responses do not contain root shares.
- Completed FROST DKG session survives coordinator and node restart.

## Phase 4: Wallet Derivation

Purpose:
- Confirm wallet derivation is deterministic, persisted, and does not require private material in Coordinator.

Expected checks:

```bash
./scripts/verify-phase.sh 4
```

The script verifies:

- Completed DKG gating before wallet creation.
- Sequential wallet indexes.
- Public Solana address derivation.
- Balance refresh status handling.
- Restart persistence.
- Frontend wallet rendering.

## Phase 5: Signing Request State Machine

Purpose:
- Confirm transfer intents and signing rounds can be orchestrated without exposing node-local secrets.

Expected checks:

```bash
./scripts/verify-phase.sh 5
```

The script verifies:

- Backend workspace tests.
- Frontend lint and production build.
- Full Docker Compose startup.
- DKG and wallet setup prerequisites.
- Signing request creation and listing.
- Round 1 commitment replay behavior.
- Round 2 gating and nonce single-use protection.
- Failure propagation from node step to parent signing request.
- Frontend signing panel rendering.

## CI And Versioning Foundation

Purpose:
- Confirm GitHub PR checks and release metadata are wired before later phases continue.

Expected checks:

```bash
./scripts/verify-phase.sh 0
```

The script verifies:

- Existing Phase 0 AI-native files.
- CI and release workflow files.
- Root `VERSION`.
- `CHANGELOG.md` contains a dated entry for the current version.
- Frontend package metadata matches `VERSION`.
- Backend workspace package metadata matches `VERSION`.
- `docs/release-process.md` exists.

## Later Phases

## Phase 6: Aggregation, Broadcast, Confirmation

Purpose:
- Confirm FROST child-wallet signature shares aggregate into a Solana transfer transaction and advance through broadcast and confirmation states.

Expected checks:

```bash
./scripts/verify-phase.sh 6
```

The script verifies:

- Backend workspace tests, including child-wallet signature verification.
- Frontend lint and production build.
- Docker Compose startup in an isolated project with `SOLANA_RPC_URL=mock://phase6`.
- Isolated host ports `13000`, `18080`, and `15432`, so mock verification does not overwrite the normal Devnet demo stack.
- DKG, wallet creation, signing rounds, aggregate/broadcast, and confirmation.
- Broadcast is rejected before `READY_TO_AGGREGATE`.
- Broadcast stores transaction signature and Explorer URL.
- Confirmation advances only after the mock RPC reports confirmed.
- Frontend broadcast controls render.

Manual Devnet verification is still required with a funded derived wallet because CI must not depend on a live wallet balance or external airdrop availability.

## Phase 7: Reviewer Experience And Hardening

Purpose:
- Confirm the repository is ready for reviewer execution, explanation, and AI-collaboration inspection.

Expected checks:

```bash
./scripts/verify-phase.sh 7
```

The script verifies:

- Sensitive-pattern scan and whitespace checks.
- Docker Compose config.
- Backend workspace tests.
- Frontend lint and production build.
- Reviewer README contains setup, manual acceptance, Devnet funding, troubleshooting, AI evidence, CI/versioning, and out-of-scope guidance.
- `.env.example` exists as an empty variable template and documents `SOLANA_RPC_URL`.
- Reviewer BDD scenario, reviewer contract, collaboration log, decision log, and Phase 7 run report exist.

Manual Devnet verification remains a human acceptance step because it requires a funded Devnet wallet and live RPC/faucet availability.

## Phase 8: Crypto Dashboard UX

Purpose:
- Confirm the reviewer-facing frontend reads as an MPC Solana wallet dashboard while preserving protocol controls.

Expected checks:

```bash
./scripts/verify-phase.sh 8
```

The script verifies:

- Phase 7 reviewer hardening checks still pass.
- Frontend lint and production build.
- Phase 8 BDD scenario, UX contract, prompt, collaboration log, decision log, and run report exist.
- The frontend source contains the required dashboard labels: `MPC Wallet Dashboard`, `Key Ceremony`, `Derived Vaults`, `Transfer Tickets`, `Threshold Signing`, `Transaction Receipt`, `Solana Devnet`, and `2-of-2 MPC`.
- The frontend source contains the vault watch panel and terminal scene layout.
- The CSS contains active workflow-step styling, single-screen terminal layout styling, horizontal mobile rail styling for narrow screens, and a reduced-motion fallback.

Phase 8 is frontend UX only. Manual Devnet verification remains the same as Phase 7.

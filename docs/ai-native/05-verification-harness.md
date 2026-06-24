# Verification Harness

The verification harness defines the checks that prove a phase is complete.

Commands may evolve as the implementation grows. When a command changes, update both this document and `scripts/verify-phase.sh`.

## General Checks

Run for documentation-only changes:

```bash
! grep -RInE "/Users/[[:alnum:]_.-]+/|([A-Z0-9_]*(SECRET|PRIVATE_KEY|API_KEY)[A-Z0-9_]*[[:space:]]*[:=])" docs features .github scripts
git diff --check
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
test -f scripts/verify-phase.sh
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
docker compose run --rm --no-deps coordinator cargo test --workspace
npm --prefix frontend run lint
npm --prefix frontend run build
docker compose config
docker compose up -d --force-recreate
```

Smoke checks should verify:

- DKG session creation.
- Round 2 is rejected before both Round 1 steps complete.
- Round 3 is rejected before both Round 2 steps complete.
- Re-triggering a completed step returns the stored result.
- Completed session survives coordinator restart.
- Frontend can load the active session.

## Phase 3: FROST DKG Crypto Integration

Purpose:
- Replace placeholder DKG with real FROST DKG while preserving private-material boundaries.

Expected checks:

```bash
docker compose run --rm --no-deps coordinator cargo test --workspace
```

Additional checks should prove:

- 2-of-2 DKG produces a master public key.
- Node A and Node B persist their own private material.
- Coordinator stores only public metadata.
- Coordinator API responses do not contain root shares.

## Later Phases

Later phases must add checks for:

- Deterministic wallet derivation.
- Signing request state transitions.
- Nonce single-use protection.
- Signature aggregation.
- Solana Devnet broadcast and confirmation.
- Frontend workflow coverage.

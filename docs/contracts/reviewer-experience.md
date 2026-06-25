# Reviewer Experience Contract

Phase 7 does not add product protocol scope. It turns the completed local demo into a reviewer-ready handoff.

## Reviewer Goals

A reviewer should be able to:

- Start the full stack with `docker compose up -d`.
- Open the frontend at `http://localhost:3000`.
- Complete DKG using explicit Node A and Node B round controls.
- Create derived wallet addresses and know exactly where to copy them.
- Fund a derived wallet with Devnet SOL through a faucet or Solana CLI.
- Create a signing request, run both signing rounds, broadcast, confirm, and open the Solana Explorer transaction.
- Inspect AI collaboration evidence, decisions, prompts, and verification reports without reading the entire codebase first.

## Required Handoff Files

| File | Requirement |
|---|---|
| `README.md` | Copy-pasteable setup, verification, manual acceptance, API examples, troubleshooting, CI/versioning, and out-of-scope notes. |
| `.env.example` | Empty variable template only. No secrets, local absolute paths, or private RPC keys. |
| `features/reviewer-experience.feature` | BDD scenarios for reviewer startup, wallet funding, manual acceptance, and AI evidence inspection. |
| `docs/ai-native/logs/ai-collaboration-log.md` | Phase 7 entry summarizing prompt, corrections, output, verification, and follow-up. |
| `docs/ai-native/logs/decision-log.md` | Reviewer handoff decision with tradeoffs and consequences. |
| `docs/ai-native/logs/phase-7-agent-run-report.md` | Concrete file changes and verification results for this phase. |
| `docs/ai-native/05-verification-harness.md` | Explanation of what `./scripts/verify-phase.sh 7` proves. |

## Acceptance Boundaries

- Devnet SOL is test money only; README must not describe it as real mainnet value.
- Manual Devnet transfer success must not be claimed unless a funded derived wallet was actually used.
- CI must not require live faucet access, a funded wallet, or a real Devnet balance.
- Mock Solana RPC verification must be isolated from the normal Devnet demo stack.
- The browser must call only the Coordinator, never TSS node internal endpoints.
- Private root shares, child shares, nonce secrets, and key packages must stay out of README examples and Coordinator-facing API responses.

## Verification

`./scripts/verify-phase.sh 7` must check:

- Sensitive pattern scan and whitespace checks.
- Docker Compose configuration validity.
- Backend Rust workspace tests.
- Frontend lint and production build.
- Reviewer README sections and key operational phrases.
- Empty `.env.example` safety.
- BDD, contract, collaboration log, decision log, and Phase 7 report presence.

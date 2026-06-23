# Decision Log

Record meaningful architecture decisions here. Keep entries short and concrete.

## Entry Template

### YYYY-MM-DD - Decision Title

Decision:
- 

Context:
- 

Options considered:
- 

Reasoning:
- 

Consequences:
- 

Verification:
- 

## Initial Decisions

### 2026-06-23 - Use Coordinator As Protocol Orchestrator

Decision:
- The frontend calls only the coordinator. The coordinator calls TSS nodes through internal APIs.

Context:
- The assignment requires the frontend to drive protocol steps while private shares stay inside nodes.

Reasoning:
- This keeps browser code simple, centralizes state transitions, and prevents direct exposure of node internals.

Consequences:
- Coordinator must implement strict state machine and idempotency controls.
- Node APIs should be treated as internal-only.

### 2026-06-23 - Keep Private Crypto Material Node-Local

Decision:
- Root shares, child shares, and nonce secrets are stored and used only by TSS nodes.

Context:
- A TSS demo is only meaningful if the coordinator cannot reconstruct private signing material.

Reasoning:
- Coordinator may route public packages and aggregate signature shares, but it must never own private key material.

Consequences:
- Node storage needs encrypted-at-rest key material.
- Coordinator tests should assert private material does not appear in coordinator persistence or API responses.

### 2026-06-24 - Require Human Approval For Agent PRs

Decision:
- Agents may prepare branches and pull requests, but they must not directly merge their own work or push feature work straight to `main`.

Context:
- The project is moving toward an automation-ready agent workflow with CI and verification harnesses.

Options considered:
- Let an agent push directly to `main` after local checks pass.
- Let an agent open PRs and require CI plus human review before merge.

Reasoning:
- This project includes cryptographic boundaries, secret handling, and git history hygiene. Automation should increase speed without removing accountability.

Consequences:
- Phase 1 and later feature work should happen on agent branches.
- Every agent PR needs verification evidence and a run report.
- Humans remain responsible for merge decisions and skipped checks.

Verification:
- `.github/pull_request_template.md` and `docs/ai-native/04-automation-design.md` encode the approval gate.

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

### 2026-06-24 - Isolate Phase 2 DKG Placeholder Behind Node Crypto Service

Decision:
- Phase 2 uses `PlaceholderDkgCryptoService` inside each TSS node instead of placing placeholder DKG behavior in the coordinator.

Context:
- Phase 2 needs a working state machine before real `frost-ed25519` integration, but the private-material boundary must already be represented correctly.

Options considered:
- Let the coordinator synthesize DKG round payloads directly.
- Let each TSS node expose internal DKG endpoints and keep placeholder behavior behind a node-local service trait.

Reasoning:
- The second option preserves the production-intended ownership model: coordinator orchestrates state, nodes own crypto behavior.

Consequences:
- Phase 3 can replace the placeholder service with real FROST logic without changing the public coordinator API or frontend control flow.
- Tests can already assert that node responses expose public payloads only.

Verification:
- `cargo test --workspace` covers node placeholder response boundaries and coordinator state transitions.

### 2026-06-24 - Add Frontend Control Surface To Phase 2

Decision:
- Phase 2 includes a Next.js DKG control surface and coordinator proxy, not only backend APIs.

Context:
- After Phase 1, the project booted successfully but still showed the default Next.js starter page, making the demo hard to inspect.

Options considered:
- Leave frontend work for a later phase.
- Add the first visible DKG workflow now while keeping wallet derivation and signing out of scope.

Reasoning:
- The assignment is reviewer-facing and requires manual protocol operation from the frontend. Showing the DKG state machine early creates a real demo feedback loop.

Consequences:
- Phase 2 verification now includes frontend lint/build, frontend load checks, and browser screenshots.
- Later phases can extend the same control surface instead of replacing a starter page later.

Verification:
- `./scripts/verify-phase.sh 2` verifies frontend load, and headless browser checks confirmed the DKG UI renders on desktop/mobile.

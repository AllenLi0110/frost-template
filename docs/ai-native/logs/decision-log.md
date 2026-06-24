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

### 2026-06-24 - Persist FROST DKG Private Material Only In Node Schemas

Decision:
- TSS nodes persist encrypted Round 1 secret packages, Round 2 secret packages, and final key packages in `node_a.node_dkg_state` and `node_b.node_dkg_state`; Coordinator persists only protocol step payloads and the final master public key.

Context:
- Phase 3 replaces placeholder DKG with real `frost-ed25519` while preserving the Phase 2 coordinator state machine and frontend workflow.

Options considered:
- Store all DKG material in the coordinator schema for simpler orchestration.
- Store only public/routed payloads in Coordinator and keep secret/key packages encrypted in node schemas.

Reasoning:
- The second option matches the TSS ownership boundary: Coordinator can orchestrate and route, but cannot reconstruct long-lived signing material.

Consequences:
- TSS nodes now require a `NODE_SEALING_KEY`.
- Coordinator must build Round 2 and Round 3 internal requests from previously completed public payloads.
- Coordinator must redact Round 2 routing packages from frontend-facing responses.
- Verification must inspect both coordinator and node schemas.

Verification:
- `./scripts/verify-phase.sh 3` verifies real DKG completion, encrypted node-local private material, absence of forbidden private fields in coordinator payloads, and restart persistence.

### 2026-06-24 - Keep Coordinator Public DKG API Stable Across Phase 3

Decision:
- Phase 3 changes only the coordinator-to-node internal request body and stored payload contents; the public coordinator endpoints from Phase 2 remain unchanged.

Context:
- The frontend already drives DKG manually through the Phase 2 state machine.

Options considered:
- Change public APIs to expose FROST-specific package exchange details.
- Keep public APIs stable and hide package routing behind Coordinator.

Reasoning:
- The visible product requirement is manual step control, not raw crypto package management. Keeping the public API stable protects the frontend and future agents from unnecessary churn.

Consequences:
- Coordinator owns the mapping from completed step payloads to node internal request maps.
- Frontend continues to display session status, node round status, action results, and master public key without direct node calls.

Verification:
- Coordinator request-builder unit tests cover Round 2 and Round 3 peer package maps.

### 2026-06-24 - Derive Wallet Addresses From Public DKG Context In Coordinator

Decision:
- Phase 4 derives Solana wallet addresses in the Coordinator from `master_public_key_base58` plus a public `hd-wallet-edwards-v1` derivation context.

Context:
- The assignment requires wallet creation after DKG completion, but private root shares and private child shares must remain outside Coordinator and Frontend.

Options considered:
- Ask Node A and Node B to derive every public address.
- Let Coordinator derive child public keys using only public material and reserve private child share derivation for the signing phase.

Reasoning:
- Ed25519 public derivation through `hd-wallet` only needs the master public key, chain code, and non-hardened index. This keeps wallet creation fast and keeps the TSS node boundary intact.

Consequences:
- `coordinator.dkg_sessions` stores public derivation context.
- `coordinator.wallets` stores only wallet index, public key, Solana address, and balance cache.
- Transfer signing still requires a later node-local child-share/signing phase.

Verification:
- `./scripts/verify-phase.sh 4` verifies DKG gating, sequential wallet indexes, restart persistence, balance lookup status handling, and frontend rendering.

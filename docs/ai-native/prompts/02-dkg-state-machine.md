# Prompt: Phase 2 DKG State Machine

```text
You are working in the repository root.

Read first:
- docs/ai-native/00-agent-context.md
- docs/ai-native/01-implementation-roadmap.md
- docs/contracts/foundation.md
- docs/contracts/dkg-state-machine.md
- features/dkg-flow.feature
- External system design reference: 15.design-frost-template.md
- backend/Cargo.toml

Goal:
Implement the observable DKG state machine, APIs, persistence, and first visible DKG control surface, with placeholder crypto behavior only where necessary to validate state transitions.

Scope:
- Coordinator public DKG APIs.
- Node internal DKG endpoints.
- PostgreSQL tables for DKG sessions and node steps.
- Idempotent node round triggering.
- Transition validation.
- Backend tests for valid and invalid DKG flows.
- Frontend DKG control surface that can create/read a session and manually trigger Node A/B rounds 1-3 through the coordinator.
- Next.js API proxy from frontend to coordinator so the browser never calls TSS nodes directly.

Important:
- Build on the Phase 1 axum routers, config structs, Docker Compose stack, and migration layout.
- If placeholder crypto is used in this phase, isolate it behind a CryptoService trait so Phase 3 can replace it with frost-ed25519.
- Do not return or persist private root shares in coordinator tables.
- Do not add a "Run All" button or equivalent shortcut.
- Do not leave the frontend as the default Next.js starter page.

Required tests:
- Cannot trigger Round 2 before both nodes finish Round 1.
- Cannot trigger Round 3 before both nodes finish Round 2.
- Re-triggering a completed step returns the existing completed result.
- Completed DKG session persists after restart or repository reload.
- TSS node placeholder DKG responses do not expose root shares or nonce secrets.

Definition of done:
- API clients can manually complete Node A/B Round 1, Round 2, and Round 3.
- The frontend shows the DKG state machine and independent node/round controls.
- DKG status becomes COMPLETED.
- Backend tests, frontend lint/build, compose smoke tests, and frontend load check pass.
- Collaboration log is updated.
```

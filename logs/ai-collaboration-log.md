# AI Collaboration Log

## Phase 5: Signing Request State Machine

- Started Phase 5 from the latest `main` after Phase 4 was merged.
- Added signing BDD scenarios for unknown wallets, Round 1 replay, Round 2 nonce reuse rejection, and multiple pending requests.
- Scoped Phase 5 to manual orchestration up to `READY_TO_AGGREGATE`; aggregation, Solana transaction construction, broadcast, and confirmation remain Phase 6.
- Implemented Coordinator signing request APIs, TSS node internal signing Round 1/2 APIs, node-local encrypted nonce persistence, frontend signing controls, and Phase 5 verification harness.
- Verified with `./scripts/verify-phase.sh 5`.
- Review fix: signing failures now mark the parent request `FAILED`, failed requests cannot continue, and node signing requests load the key package for the request wallet's `dkg_session_id`.

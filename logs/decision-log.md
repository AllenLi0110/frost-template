# Decision Log

## Phase 5: Signing State Machine Scope

- Coordinator stores transfer intents, public nonce commitments, public signature shares, and node step status.
- TSS nodes store encrypted signing nonce state in node-local schemas and mark nonces consumed after Round 2.
- Phase 5 signs a canonical transfer-intent message with the completed FROST root key package to prove the signing state machine and nonce safety. Derived child-share signing and final Solana transaction aggregation remain Phase 6 work.
- Phase 6 must not broadcast a derived wallet transfer until the aggregated signature is bound to the exact Solana transaction signer public key.
- Signing node requests carry the wallet's `dkg_session_id` so nodes load the key package for the request's source DKG session rather than whichever completed session is newest.

# Prompt: Phase 3 FROST DKG Crypto Integration

```text
You are working in the repository root.

Read first:
- docs/ai-native/00-agent-context.md
- docs/ai-native/01-implementation-roadmap.md
- docs/contracts/dkg-state-machine.md
- features/dkg-flow.feature
- External system design reference: 15.design-frost-template.md
- backend/Cargo.toml

Goal:
Replace placeholder DKG behavior with real frost-ed25519 DKG integration.

Scope:
- Implement CryptoService DKG adapter using frost-ed25519 2.1.0.
- Replace the Phase 2 PlaceholderDkgCryptoService behavior without changing the coordinator state machine contract.
- Persist node-local encrypted root share and key package.
- Persist coordinator-visible public DKG packages and master public key.
- Add tests proving coordinator persistence does not contain root shares.

Do not:
- Implement signing yet.
- Implement Solana broadcast yet.
- Expose node private state through public APIs.

Required tests:
- 2-of-2 DKG produces a master public key.
- Node A and Node B each persist their own private material.
- Coordinator stores only public metadata.
- Re-running a completed DKG round is idempotent.

Definition of done:
- A completed DKG session has a Base58 master public key.
- Private root shares never appear in coordinator API responses or coordinator tables.
- Tests pass.
- Collaboration log records any crypto API issues and corrections.
```

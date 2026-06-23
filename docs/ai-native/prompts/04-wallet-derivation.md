# Prompt: Phase 4 Wallet Derivation

```text
You are working in the repository root.

Read first:
- docs/ai-native/00-agent-context.md
- docs/ai-native/01-implementation-roadmap.md
- features/wallet-derivation.feature
- External system design reference: 15.design-frost-template.md

Goal:
Implement wallet derivation from completed DKG root material.

Scope:
- Coordinator wallet APIs.
- Sequential wallet index allocation.
- Public derivation context handling.
- hd-wallet Edwards non-hardened public derivation.
- Solana address Base58 formatting.
- Balance lookup through SOLANA_RPC_URL.
- Frontend wallet list and Create Wallet control.

Do not:
- Require Node A and Node B to communicate when deriving a public address.
- Store private child shares in the coordinator.
- Implement transfer signing yet.

Required tests:
- Cannot create wallet before DKG is completed.
- Wallet indexes are sequential and never reused after restart.
- Derivation is deterministic for the same master public key, public derivation context, and wallet index.
- Balance lookup handles RPC failure gracefully.

Definition of done:
- Create Wallet produces index 0, then 1, then 2.
- Wallet list survives restart.
- Frontend displays wallet index, address, and balance.
- Tests pass.
- Collaboration log is updated.
```


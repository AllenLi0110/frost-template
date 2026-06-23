# Prompt: Phase 6 Broadcast And Confirmation

```text
You are working in the repository root.

Read first:
- docs/ai-native/00-agent-context.md
- docs/ai-native/01-implementation-roadmap.md
- features/signing-transfer.feature
- External system design reference: 15.design-frost-template.md

Goal:
Aggregate FROST signature shares, construct a Solana Devnet transfer, broadcast it, and update confirmation status.

Scope:
- Fetch fresh recent blockhash before signing or aggregation as appropriate.
- Verify signature shares.
- Aggregate final Ed25519 signature.
- Construct Solana transfer transaction.
- Broadcast through SOLANA_RPC_URL.
- Poll or refresh confirmation status.
- Store transaction signature and Explorer URL.
- Frontend aggregate/broadcast button and status display.

Do not:
- Mark a request CONFIRMED unless Solana returns confirmed status.
- Hide the signing flow behind a single action.
- Ignore expired blockhash handling.

Required tests:
- Aggregate requires both signature shares.
- Broadcast stores transaction signature and Explorer URL.
- Confirmation transition only happens from Solana confirmed response.
- RPC failures produce FAILED or retryable error state with clear message.

Definition of done:
- A funded derived wallet can send Devnet SOL.
- UI displays BROADCASTED, CONFIRMED, or FAILED.
- Tests pass.
- Collaboration log records the manual Devnet verification result.
```


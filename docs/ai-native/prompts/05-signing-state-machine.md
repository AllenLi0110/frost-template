# Prompt: Phase 5 Signing Request State Machine

```text
You are working in the repository root.

Read first:
- docs/ai-native/00-agent-context.md
- docs/ai-native/01-implementation-roadmap.md
- features/signing-transfer.feature
- External system design reference: 15.design-frost-template.md

Goal:
Implement the step-by-step signing request workflow up to READY_TO_AGGREGATE.

Scope:
- Create transfer intent API.
- List pending and historical signing requests.
- Internal node Signing Round 1 endpoint for commitments.
- Internal node Signing Round 2 endpoint for signature shares.
- Nonce state persistence and single-use protection.
- Frontend pending request list and selected request controls.

Do not:
- Broadcast to Solana yet.
- Add a one-click Sign & Send button.
- Reuse nonce state after Round 2.

Required tests:
- Cannot create signing request for unknown wallet index.
- Round 2 cannot run before both Round 1 commitments exist.
- Re-triggering Round 1 is idempotent.
- Round 2 consumes nonce once and rejects reuse.
- Multiple pending requests can be distinguished.

Definition of done:
- Users can create a request and manually trigger Node A/B Signing Round 1 and Round 2.
- Request reaches READY_TO_AGGREGATE.
- Tests pass.
- Collaboration log is updated.
```


# Prompt: Phase 8 Crypto Dashboard UX

```text
You are working in the repository root.

Read first:
- README.md
- docs/contracts/dkg-state-machine.md
- docs/contracts/wallet-derivation.md
- docs/contracts/signing-state-machine.md
- docs/contracts/reviewer-experience.md
- docs/contracts/crypto-dashboard-ux.md
- frontend/app/page.tsx
- frontend/app/globals.css

Goal:
Rework the frontend into an institutional crypto MPC wallet dashboard while preserving all existing protocol behavior.

Scope:
- Rename visible UI sections into crypto-friendly terms:
  - DKG -> Key Ceremony
  - Wallet Derivation -> Derived Vaults
  - Signing Requests -> Transfer Tickets
- Add a five-step workflow:
  Key Ceremony, Vault Funding, Transfer Intent, Threshold Signing, Broadcast.
- Make the workflow mobile-friendly:
  - one-page top-to-bottom flow
  - active/completed/queued step states
  - lightweight active-step animation
  - reduced-motion fallback
- Keep all Node A / Node B manual controls visible and independently clickable.
- Keep browser-to-coordinator-only boundary clear.
- Keep Devnet/test-money wording clear.
- Improve reviewer demo readability without changing backend APIs.
- Update verification so the crypto dashboard labels are checked automatically.

Do not:
- Change coordinator APIs.
- Hide DKG or signing rounds behind a single one-click flow.
- Change FROST crypto behavior.
- Add mainnet behavior.
- Add unrelated product scope.

Definition of done:
- Existing Phase 7 verification still passes.
- Phase 8 verification passes.
- Frontend lint and build pass.
- Reviewer can still complete the same manual acceptance checklist.
- UI reads like a crypto MPC wallet dashboard instead of a raw protocol test panel.
- Mobile viewport presents the flow clearly enough for a short demo recording.
```

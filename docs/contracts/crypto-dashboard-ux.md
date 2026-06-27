# Crypto Dashboard UX Contract

Phase 8 changes the reviewer-facing presentation only. It must not change coordinator APIs, TSS node behavior, FROST cryptography, database schema, or Solana transaction semantics.

## Product Frame

The frontend should read as an institutional MPC wallet dashboard:

- Product: `FROST MPC Wallet`
- Primary surface: `MPC Wallet Dashboard`
- Network: `Solana Devnet`
- Threshold: `2-of-2 MPC`
- Setup flow: `Key Ceremony`
- Wallets: `Derived Vaults`
- Transfers: `Transfer Tickets`
- Signature process: `Threshold Signing`
- Final artifact: `Transaction Receipt`

## Required Workflow

The first screen must expose a five-step workflow:

```text
Key Ceremony -> Vault Funding -> Transfer Intent -> Threshold Signing -> Broadcast
```

The workflow is a visual orientation layer. It must:

- Show the current step, completed steps, and queued steps.
- Use a single-screen scene layout: the reviewer changes scenes by clicking the workflow stepper instead of scrolling through stacked sections.
- Use lightweight step animation for the active protocol stage.
- Keep the workflow and compact status summaries available as app-like rails on narrow screens without creating page-level scroll.
- Respect reduced-motion settings.
- Not become an automated one-click protocol runner.

## Manual Protocol Controls

The UI must preserve:

- Node A and Node B DKG Round 1, 2, and 3 controls.
- Node A and Node B signing Round 1 and 2 controls.
- Replay behavior for completed DKG and signing Round 1 steps.
- Consumed state for completed signing Round 2 steps.
- Existing broadcast and confirmation buttons.

## Boundary Messaging

The reviewer-facing experience must keep the custody boundary clear without forcing a large persistent explainer panel into the demo flow:

- The app shell must keep `Solana Devnet`, `2-of-2 MPC`, and test-SOL context visible.
- The UI copy must continue to state that private root shares stay sealed inside TSS nodes.
- README and contracts must document that the browser calls Coordinator only and Coordinator stores public protocol state only.
- TSS nodes keep private root shares, child shares, and nonce secrets node-local.
- Devnet SOL is test money only.

## Visual Direction

Use a restrained crypto exchange operations style inspired by modern exchange onboarding and wallet flows:

- Dense but readable operational layout.
- Mobile-first single-screen flow suitable for a demo recording.
- Animated scene transitions for the current protocol stage.
- App-like summary chips instead of dense dashboard tables on narrow screens.
- Clear network, threshold, and security badges.
- Status colors for confirmed, pending, failed, and idle states.
- A persistent vault watch surface should show derived vault addresses, balance status, and SOL balances during signing and broadcast scenes.
- The Vault Watch refresh control must refresh every visible vault through the balance lookup API, not only reload cached wallet rows.
- Broadcast and confirmation refresh actions must refresh Vault Watch balances after the signing request updates so sender and recipient funds reflect the latest Solana Devnet state.
- No landing-page hero, no decorative gradients, and no decorative orbs.
- No card nesting beyond actual repeated items and tool panels.
- Do not copy an exchange brand, logo, proprietary layout, or exact wording.

## Verification

`./scripts/verify-phase.sh 8` must verify:

- Full Phase 6 mock Solana integration still passes.
- Phase 7 documentation checks still pass.
- Frontend lint and build pass.
- The production HTML or CSS contains the Phase 8 dashboard labels.
- CSS contains active workflow step styling, single-screen terminal layout styling, vault watch styling, and reduced-motion handling.
- BDD, contract, prompt, collaboration log, decision log, and Phase 8 run report exist.

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
- Use a mobile-friendly one-page layout that reads top-to-bottom.
- Use lightweight step animation for the active protocol stage.
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

The UI must clearly state:

- Browser calls Coordinator only.
- Coordinator stores public protocol state only.
- TSS nodes keep private root shares, child shares, and nonce secrets node-local.
- Devnet SOL is test money only.

## Visual Direction

Use a restrained crypto operations dashboard style:

- Dense but readable operational layout.
- Mobile-first one-page flow suitable for a demo recording.
- Animated status stepper for the current protocol stage.
- Clear network, threshold, and security badges.
- Status colors for confirmed, pending, failed, and idle states.
- No landing-page hero, no decorative gradients, and no decorative orbs.
- No card nesting beyond actual repeated items and tool panels.

## Verification

`./scripts/verify-phase.sh 8` must verify:

- Phase 7 checks still pass.
- Frontend lint and build pass.
- The production HTML or CSS contains the Phase 8 dashboard labels.
- CSS contains active workflow step styling and reduced-motion handling.
- BDD, contract, prompt, collaboration log, decision log, and Phase 8 run report exist.

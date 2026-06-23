# Agent Context

## Project Summary

Build a minimal 2-of-2 TSS Solana wallet demo using FROST Ed25519 and non-hardened Edwards derivation. The reviewer must be able to manually operate DKG, wallet derivation, signing, aggregation, broadcast, and confirmation from the frontend.

## Existing Skeleton

- `frontend/`: Next.js 16, React 19, TypeScript.
- `backend/`: Rust workspace with two crates:
  - `coordinator`
  - `tss-node`
- Required tooling is managed by `mise.toml`:
  - Node.js `24.14.0`
  - Rust `1.94.0`

## Required Stack

- Backend: Rust `1.94.0`, axum `0.8.8`, snafu `0.8.7`, sqlx `0.8.6`, PostgreSQL `18`.
- Frontend: Next.js `16`, React `19`, TypeScript `5.9`.
- Solana: `solana-client` `3.1.8`, `solana-sdk` `3.0.0`.
- Crypto: `frost-ed25519` `2.1.0`, `hd-wallet` `0.6.1`.

## Non-Negotiable Product Requirements

1. DKG must be step-by-step. No single "Run All" button.
2. Signing must be step-by-step. No single "Sign & Send" button.
3. Node A and Node B must be independently triggerable for each protocol round.
4. Root shares and nonce secrets must never leave TSS nodes.
5. Derived wallets and transaction records must survive service restarts.
6. The system must use Solana Devnet by default: `https://api.devnet.solana.com`.
7. Docker Compose must start frontend, coordinator, node A, node B, and PostgreSQL.

## Architecture Boundaries

Frontend:
- Displays protocol state.
- Calls coordinator APIs.
- Does not call TSS nodes directly.
- Does not hold private crypto material.

Coordinator:
- Owns public APIs.
- Owns protocol state machine.
- Persists public protocol state.
- Routes protocol packages between nodes.
- Derives public wallet addresses.
- Aggregates signature shares.
- Broadcasts Solana transactions.

TSS Node:
- Owns private root share.
- Owns nonce secrets.
- Computes DKG and signing rounds.
- Derives child shares in memory for a wallet index.
- Returns only public packages, commitments, and signature shares.

PostgreSQL:
- Single instance.
- Can use separate logical schemas for coordinator, node A, and node B.
- Coordinator schema must not store private node shares.

## Existing Design Reference

Read the system design before implementing:

- `15.design-frost-template.md` (external system design reference)

## Implementation Rule

For every feature, first update `features/*.feature`, then define API or data contracts, then write tests, then implement.


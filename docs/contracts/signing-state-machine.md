# Signing State Machine Contract

Phase 5 adds a manual signing request workflow up to `READY_TO_AGGREGATE`.
Phase 6 extends the same workflow with FROST aggregation, Solana Devnet broadcast, and confirmation refresh.

## Boundary

Frontend:
- Calls only the Coordinator through the Next.js API proxy.
- Creates transfer intents.
- Lists signing requests.
- Manually triggers Node A and Node B Signing Round 1 and Round 2.
- Manually triggers aggregate/broadcast after both signature shares exist.
- Manually refreshes confirmation after broadcast.
- Must not provide a one-click sign-and-send action that hides signing rounds.

Coordinator:
- Owns public signing APIs and state transitions.
- Stores transfer intent metadata, public node step payloads, public nonce commitments, signature shares, transaction signature, and Explorer URL.
- Fetches a fresh recent blockhash before Round 2 signing.
- Builds the exact Solana transfer message that TSS nodes sign.
- Aggregates FROST signature shares and broadcasts the signed transaction.
- Marks a request `CONFIRMED` only after Solana reports `confirmed` or `finalized`.
- Does not store nonce secrets, root shares, child shares, or node key packages.

TSS Node:
- Owns node-local key material and nonce state.
- Generates a single-use FROST signing nonce in Round 1.
- Uses and consumes that nonce in Round 2.
- Derives child signing shares in memory from the node-local root share and `wallet_index`.
- Returns only public nonce commitments, child verifying material, and signature share payloads.

## Crypto Scope

Round 1 and Round 2 use the completed FROST root key package as node-local input, derive the selected child wallet share in memory, and produce real FROST signing commitments and signature shares over the serialized Solana transfer message.

Coordinator never receives child private shares. It receives public child verifying shares and the child verifying key so it can aggregate and verify the final Ed25519 signature against the transaction signer.

## Public Coordinator APIs

### `POST /api/signing-requests`

Request:

```json
{
  "wallet_index": 0,
  "recipient_address_base58": "11111111111111111111111111111111",
  "amount_lamports": 1000
}
```

Rules:
- `wallet_index` must already exist in `coordinator.wallets`.
- `recipient_address_base58` must decode to 32 bytes.
- `amount_lamports` must be positive.
- A new request always starts with four node steps: Node A/B Round 1 and Node A/B Round 2.

Response:

```json
{
  "request_id": "00000000-0000-0000-0000-000000000000",
  "dkg_session_id": "00000000-0000-0000-0000-000000000000",
  "wallet_index": 0,
  "sender_address_base58": "9xQeWvG816bUx9EPfY...",
  "recipient_address_base58": "11111111111111111111111111111111",
  "amount_lamports": 1000,
  "status": "PENDING",
  "message_hash_hex": null,
  "recent_blockhash": null,
  "transaction_signature": null,
  "explorer_url": null,
  "error_message": null,
  "created_at": "2026-06-24 12:00:00+00",
  "updated_at": "2026-06-24 12:00:00+00",
  "node_steps": [
    { "node_id": "node-a", "round": 1, "status": "NOT_STARTED" },
    { "node_id": "node-a", "round": 2, "status": "NOT_STARTED" },
    { "node_id": "node-b", "round": 1, "status": "NOT_STARTED" },
    { "node_id": "node-b", "round": 2, "status": "NOT_STARTED" }
  ]
}
```

### `GET /api/signing-requests`

Lists signing requests in reverse creation order. `?status=pending` returns non-terminal requests.

Response:

```json
{
  "requests": [
    {
      "request_id": "00000000-0000-0000-0000-000000000000",
      "dkg_session_id": "00000000-0000-0000-0000-000000000000",
      "wallet_index": 0,
      "sender_address_base58": "9xQeWvG816bUx9EPfY...",
      "recipient_address_base58": "11111111111111111111111111111111",
      "amount_lamports": 1000,
      "status": "PENDING",
      "message_hash_hex": null,
      "recent_blockhash": null,
      "transaction_signature": null,
      "explorer_url": null,
      "error_message": null,
      "created_at": "2026-06-24 12:00:00+00",
      "updated_at": "2026-06-24 12:00:00+00",
      "node_steps": []
    }
  ]
}
```

### `GET /api/signing-requests/{request_id}`

Returns one signing request with node step statuses.

### `POST /api/signing-requests/{request_id}/nodes/{node_id}/rounds/{round}`

Rules:
- `node_id` must be `node-a` or `node-b`.
- `round` must be `1` or `2`.
- Round 1 is idempotent and returns the stored commitment when already completed.
- Round 2 requires both Round 1 commitments.
- Round 2 is not replayable. A completed Round 2 step rejects follow-up calls because its nonce is consumed.

Round 1 response:

```json
{
  "request_id": "00000000-0000-0000-0000-000000000000",
  "node_id": "node-a",
  "round": 1,
  "status": "COMPLETED",
  "signing_status": "COMMITMENTS_IN_PROGRESS",
  "public_payload": {
    "kind": "frost-signing-round1",
    "package_format": "frost-ed25519-2.2.0-hex",
    "node_id": "node-a",
    "round": 1,
    "commitment_scope": "child-wallet-solana-transfer",
    "child_verifying_share_hex": "...",
    "child_verifying_key_hex": "...",
    "commitments_hex": "..."
  }
}
```

Round 2 response:

```json
{
  "request_id": "00000000-0000-0000-0000-000000000000",
  "node_id": "node-a",
  "round": 2,
  "status": "COMPLETED",
  "signing_status": "SHARES_IN_PROGRESS",
  "public_payload": {
    "kind": "frost-signing-round2",
    "package_format": "frost-ed25519-2.2.0-hex",
    "node_id": "node-a",
    "round": 2,
    "signature_scope": "child-wallet-solana-transfer",
    "message_hash_hex": "...",
    "child_verifying_share_hex": "...",
    "child_verifying_key_hex": "...",
    "signature_share_hex": "..."
  }
}
```

### `POST /api/signing-requests/{request_id}/broadcast`

Rules:
- Request status must be `READY_TO_AGGREGATE`.
- Both Round 2 signature shares must exist.
- Stored message payload must use `frost-template-solana-transfer-message-v1`.
- Aggregated signature must verify against the child wallet signer.
- RPC failures mark the request `FAILED` with a public error message.
- Expired blockhash errors tell the user to create a new signing request.

Response uses the normal signing request shape with:

```json
{
  "status": "BROADCASTED",
  "transaction_signature": "...",
  "explorer_url": "https://explorer.solana.com/tx/...?cluster=devnet"
}
```

### `POST /api/signing-requests/{request_id}/confirm`

Rules:
- Request status must be `BROADCASTED`.
- `CONFIRMED` is set only when Solana returns `confirmed` or `finalized`.
- A Solana transaction error marks the request `FAILED`.
- An unconfirmed transaction remains `BROADCASTED`.

Response uses the normal signing request shape.

## Internal TSS Node APIs

- `POST /internal/signing/{request_id}/round1`
- `POST /internal/signing/{request_id}/round2`

Round 2 request includes all public Round 1 commitments:

```json
{
  "dkg_session_id": "00000000-0000-0000-0000-000000000000",
  "wallet_index": 0,
  "sender_address_base58": "9xQeWvG816bUx9EPfY...",
  "recipient_address_base58": "11111111111111111111111111111111",
  "amount_lamports": 1000,
  "message_payload": {
    "format": "frost-template-solana-transfer-message-v1",
    "signature_scope": "child-wallet-solana-transfer",
    "recent_blockhash": "...",
    "transaction_message_hex": "..."
  },
  "message_hash_hex": "...",
  "signing_commitments": {
    "node-a": "...",
    "node-b": "..."
  }
}
```

## State Values

Signing request status:

```text
PENDING
COMMITMENTS_IN_PROGRESS
COMMITMENTS_READY
SHARES_IN_PROGRESS
READY_TO_AGGREGATE
BROADCASTED
CONFIRMED
FAILED
EXPIRED
```

Node step status:

```text
NOT_STARTED
RUNNING
COMPLETED
FAILED
```

## Storage Boundary

Coordinator tables must not include these field names in public payloads:

- `root_share`
- `private_share`
- `nonce_secret`
- `secret_key`
- `key_package_ciphertext`
- `signing_nonces_ciphertext`

Node tables may store encrypted nonce state, but never plaintext nonce material.

## Verification Notes

- CI may use `SOLANA_RPC_URL=mock://phase6` for deterministic broadcast and confirmation checks without requiring Devnet funds.
- Manual Devnet verification still requires funding the derived sender wallet before broadcast.

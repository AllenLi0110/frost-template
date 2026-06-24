# Signing State Machine Contract

Phase 5 adds a manual signing request workflow up to `READY_TO_AGGREGATE`. It does not aggregate signatures, build a Solana transaction, broadcast, or confirm a transfer.

## Boundary

Frontend:
- Calls only the Coordinator through the Next.js API proxy.
- Creates transfer intents.
- Lists signing requests.
- Manually triggers Node A and Node B Signing Round 1 and Round 2.
- Must not provide a one-click sign-and-send action.

Coordinator:
- Owns public signing APIs and state transitions.
- Stores transfer intent metadata, public node step payloads, public nonce commitments, and signature shares.
- Does not store nonce secrets, root shares, child shares, or node key packages.

TSS Node:
- Owns node-local key material and nonce state.
- Generates a single-use FROST signing nonce in Round 1.
- Uses and consumes that nonce in Round 2.
- Returns only public nonce commitments and signature share payloads.

## Phase 5 Crypto Scope

Round 1 and Round 2 use the completed FROST root key package to produce real FROST signing commitments and signature shares over a canonical transfer-intent message.

Derived child-share signing and Solana transaction aggregation remain Phase 6 work. Phase 5 still binds every request to `wallet_index`, sender address, recipient address, amount, and message hash so the orchestration boundary is ready for the Phase 6 replacement.

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
  "wallet_index": 0,
  "sender_address_base58": "9xQeWvG816bUx9EPfY...",
  "recipient_address_base58": "11111111111111111111111111111111",
  "amount_lamports": 1000,
  "status": "PENDING",
  "message_hash_hex": null,
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
      "wallet_index": 0,
      "sender_address_base58": "9xQeWvG816bUx9EPfY...",
      "recipient_address_base58": "11111111111111111111111111111111",
      "amount_lamports": 1000,
      "status": "PENDING",
      "message_hash_hex": null,
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
    "message_hash_hex": "...",
    "signature_share_hex": "..."
  }
}
```

## Internal TSS Node APIs

- `POST /internal/signing/{request_id}/round1`
- `POST /internal/signing/{request_id}/round2`

Round 2 request includes all public Round 1 commitments:

```json
{
  "wallet_index": 0,
  "sender_address_base58": "9xQeWvG816bUx9EPfY...",
  "recipient_address_base58": "11111111111111111111111111111111",
  "amount_lamports": 1000,
  "message_payload": {
    "format": "frost-template-transfer-intent-v1",
    "canonical_message": "..."
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

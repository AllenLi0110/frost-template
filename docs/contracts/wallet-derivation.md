# Wallet Derivation Contract

Phase 4 derives public Solana wallet addresses from the completed FROST DKG master public key. The coordinator never stores private root shares or private child shares.

## Public Boundary

The browser calls the Coordinator only.

The Coordinator can derive wallet public keys because the DKG master public key and derivation context are public. Node A and Node B are not called for address derivation in Phase 4.

Private child share derivation and threshold signing are out of scope until the transfer-signing phase.

## Database

`coordinator.dkg_sessions` gains:

| Column | Type | Notes |
|---|---|---|
| `public_derivation_context` | `jsonb` | Public derivation metadata for the completed DKG session. |

`coordinator.wallets` stores public wallet metadata:

| Column | Type | Notes |
|---|---|---|
| `wallet_index` | `integer primary key` | Sequential non-hardened child index. |
| `dkg_session_id` | `uuid` | Completed DKG session used for derivation. |
| `derivation_path` | `text` | Public path, currently `m/{wallet_index}`. |
| `public_key_base58` | `text` | Derived Ed25519 public key. |
| `address_base58` | `text unique` | Solana address encoded from the derived public key bytes. |
| `balance_lamports` | `bigint null` | Last known balance. |
| `balance_status` | `text` | `UNKNOWN`, `AVAILABLE`, or `UNAVAILABLE`. |
| `balance_error_message` | `text null` | Last Solana RPC error summary. |
| `balance_checked_at` | `timestamptz null` | Last balance lookup time. |
| `created_at` | `timestamptz` | Wallet creation time. |

## Derivation

The coordinator builds an `hd-wallet` Edwards extended public key from:

- `master_public_key_base58`
- `public_derivation_context.chain_code_base58`

For wallet index `n`, the coordinator derives a non-hardened child public key at path `m/n`.

The derived public key bytes are Base58-encoded as both `public_key_base58` and `address_base58`, because a Solana address is the Base58 representation of the account public key bytes.

## APIs

### `POST /api/wallets`

Creates the next sequential wallet.

Requirements:

- An active DKG session must exist.
- The active DKG session must be `COMPLETED`.
- The session must have `master_public_key_base58`.
- `wallet_index` is allocated inside a database transaction and is never reused.

Response `200`:

```json
{
  "wallet_index": 0,
  "dkg_session_id": "00000000-0000-0000-0000-000000000000",
  "derivation_path": "m/0",
  "public_key_base58": "9xQeWvG816bUx9EPfY...",
  "address_base58": "9xQeWvG816bUx9EPfY...",
  "balance_lamports": null,
  "balance_status": "UNKNOWN",
  "balance_error_message": null,
  "balance_checked_at": null,
  "created_at": "2026-06-24 12:00:00+00"
}
```

Errors:

| Status | Condition |
|---|---|
| `409` | DKG session is missing, incomplete, or missing a master public key. |
| `500` | Derivation or database failure. |

### `GET /api/wallets`

Lists derived wallets in ascending `wallet_index` order.

Response `200`:

```json
{
  "wallets": [
    {
      "wallet_index": 0,
      "dkg_session_id": "00000000-0000-0000-0000-000000000000",
      "derivation_path": "m/0",
      "public_key_base58": "9xQeWvG816bUx9EPfY...",
      "address_base58": "9xQeWvG816bUx9EPfY...",
      "balance_lamports": 0,
      "balance_status": "AVAILABLE",
      "balance_error_message": null,
      "balance_checked_at": "2026-06-24 12:01:00+00",
      "created_at": "2026-06-24 12:00:00+00"
    }
  ]
}
```

### `GET /api/wallets/{wallet_index}/balance`

Refreshes one wallet balance using `SOLANA_RPC_URL`.

Response `200` when RPC succeeds:

```json
{
  "wallet_index": 0,
  "address_base58": "9xQeWvG816bUx9EPfY...",
  "balance_lamports": 0,
  "balance_status": "AVAILABLE",
  "balance_error_message": null
}
```

Response `200` when RPC fails gracefully:

```json
{
  "wallet_index": 0,
  "address_base58": "9xQeWvG816bUx9EPfY...",
  "balance_lamports": null,
  "balance_status": "UNAVAILABLE",
  "balance_error_message": "Solana RPC request failed"
}
```

Errors:

| Status | Condition |
|---|---|
| `404` | Wallet index does not exist. |

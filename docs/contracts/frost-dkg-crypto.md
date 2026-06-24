# Phase 3 FROST DKG Crypto Contract

Phase 3 replaces the Phase 2 placeholder crypto adapter with real `frost-ed25519` DKG. The public coordinator API and visible step-by-step workflow stay unchanged.

## Version

The workspace dependency is declared as `frost-ed25519 = "2.1.0"`, and Cargo currently resolves it to `2.2.0`. Phase 3 payloads therefore use `frost-ed25519-2.2.0` serialization bytes encoded as lowercase hex.

## Internal TSS Node Requests

Coordinator calls remain internal only:

- `POST /internal/dkg/{session_id}/round1`
- `POST /internal/dkg/{session_id}/round2`
- `POST /internal/dkg/{session_id}/round3`

Request body:

```json
{
  "peer_round1_packages": {
    "node-b": "hex-encoded-frost-round1-package"
  },
  "peer_round2_packages": {
    "node-b": "hex-encoded-frost-round2-package-addressed-to-this-node"
  }
}
```

Rules:
- Round 1 ignores both maps.
- Round 2 requires the peer Round 1 package.
- Round 3 requires the peer Round 1 package and the peer Round 2 package addressed to the current node.
- Coordinator builds these maps only from previously completed node step payloads.

## Internal TSS Node Responses

Round 1 public payload:

```json
{
  "kind": "frost-dkg-round1",
  "package_format": "frost-ed25519-2.2.0-hex",
  "node_id": "node-a",
  "round": 1,
  "public_package_hex": "..."
}
```

Round 2 routed payload:

```json
{
  "kind": "frost-dkg-round2",
  "package_format": "frost-ed25519-2.2.0-hex",
  "node_id": "node-a",
  "round": 2,
  "round2_packages": {
    "node-b": "hex-encoded-package-for-node-b"
  }
}
```

Coordinator stores the full Round 2 routed payload so it can build Round 3 node requests. Public coordinator trigger responses must redact `round2_packages` before returning data to the frontend.

Round 3 public payload:

```json
{
  "kind": "frost-dkg-round3",
  "package_format": "frost-ed25519-2.2.0-hex",
  "node_id": "node-a",
  "round": 3,
  "public_key_package_hex": "...",
  "master_public_key_base58": "..."
}
```

## Storage Boundary

Coordinator schema:
- Stores DKG session state.
- Stores completed node step public payloads needed for routing and auditability.
- Stores `master_public_key_base58` after both Round 3 steps complete.
- Must not store long-lived key packages, root shares, nonce secrets, or secret keys.

Node schemas:
- `node_a.node_dkg_state` stores Node A DKG secret packages and key package.
- `node_b.node_dkg_state` stores Node B DKG secret packages and key package.
- Secret packages and key packages are encrypted at rest with a node-local sealing key.

## Forbidden Coordinator Fields

Coordinator API responses and coordinator tables must not contain these field names:

- `root_share`
- `private_share`
- `nonce_secret`
- `secret_key`
- `key_package_ciphertext`
- `round1_secret_package_ciphertext`
- `round2_secret_package_ciphertext`

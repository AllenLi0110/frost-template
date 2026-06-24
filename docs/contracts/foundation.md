# Phase 1 Foundation Contracts

This contract defines only the bootstrap behavior required before DKG, wallet derivation, or signing are implemented.

## Runtime Services

1. `frontend`: Next.js UI shell.
2. `coordinator`: Rust axum public API service.
3. `tss-node` with `NODE_ID=node-a`: internal TSS node service.
4. `tss-node` with `NODE_ID=node-b`: internal TSS node service.
5. `postgres`: single PostgreSQL 18 instance.

## Coordinator Environment

| Variable | Required | Default | Purpose |
|---|---:|---|---|
| `COORDINATOR_HOST` | No | `0.0.0.0` | Bind host. |
| `COORDINATOR_PORT` | No | `8080` | Bind port. |
| `DATABASE_URL` | Yes | None | PostgreSQL connection string. |
| `SOLANA_RPC_URL` | No | `https://api.devnet.solana.com` | Solana Devnet RPC endpoint. |
| `NODE_A_URL` | Yes | None | Internal URL for TSS Node A. |
| `NODE_B_URL` | Yes | None | Internal URL for TSS Node B. |

## TSS Node Environment

| Variable | Required | Default | Purpose |
|---|---:|---|---|
| `NODE_ID` | Yes | None | Stable node id, such as `node-a` or `node-b`. |
| `TSS_NODE_HOST` | No | `0.0.0.0` | Bind host. |
| `TSS_NODE_PORT` | No | `8081` | Bind port. |
| `DATABASE_URL` | Yes | None | PostgreSQL connection string. |
| `COORDINATOR_URL` | No | `http://coordinator:8080` | Coordinator URL for future internal callbacks. |

## Health API

### Coordinator

`GET /health`

Response:

```json
{
  "service": "coordinator",
  "status": "ok",
  "database_configured": true,
  "solana_rpc_url": "https://api.devnet.solana.com",
  "node_a_url": "http://node-a:8081",
  "node_b_url": "http://node-b:8081"
}
```

`GET /health/nodes`

Response:

```json
{
  "nodes": [
    {
      "node_id": "node-a",
      "url": "http://node-a:8081",
      "reachable": true
    },
    {
      "node_id": "node-b",
      "url": "http://node-b:8081",
      "reachable": true
    }
  ]
}
```

### TSS Node

`GET /health`

Response:

```json
{
  "service": "tss-node",
  "node_id": "node-a",
  "status": "ok",
  "database_configured": true,
  "coordinator_url": "http://coordinator:8080"
}
```

## Initial Database Layout

Phase 1 creates only logical schemas:

1. `coordinator`
2. `node_a`
3. `node_b`

Protocol tables are intentionally deferred to later phases.

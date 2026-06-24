# Phase 2 DKG State Machine Contracts

Phase 2 makes DKG observable and persistent. It intentionally uses placeholder crypto responses from TSS nodes, but the placeholder behavior must live behind a node-side crypto service boundary so Phase 3 can replace it with `frost-ed25519`.

## Runtime Boundary

Frontend:
- Calls only the coordinator through the Next.js API proxy.
- Displays the DKG session, node round status, master public key, and last action result.
- Provides independent controls for Node A and Node B rounds 1, 2, and 3.
- Must not provide a "Run All" shortcut.

Coordinator:
- Owns public DKG APIs.
- Persists DKG sessions and node round steps.
- Validates round transition order.
- Calls TSS node internal APIs.
- Stores only public payloads and master public key metadata.

TSS Node:
- Owns placeholder DKG crypto behavior in Phase 2.
- Returns only public payloads.
- Must not return root shares, nonce secrets, or private key material.
- Must stay on the Docker internal network; node internal API ports are not published to the host.

## Public Coordinator API

### Create Or Read Active DKG Session

`POST /api/dkg/sessions`

Request:

```json
{
  "threshold": 2,
  "participants": ["node-a", "node-b"]
}
```

Response:

```json
{
  "session_id": "00000000-0000-0000-0000-000000000000",
  "status": "NOT_STARTED",
  "master_public_key_base58": null,
  "node_steps": [
    { "node_id": "node-a", "round": 1, "status": "NOT_STARTED" },
    { "node_id": "node-a", "round": 2, "status": "NOT_STARTED" },
    { "node_id": "node-a", "round": 3, "status": "NOT_STARTED" },
    { "node_id": "node-b", "round": 1, "status": "NOT_STARTED" },
    { "node_id": "node-b", "round": 2, "status": "NOT_STARTED" },
    { "node_id": "node-b", "round": 3, "status": "NOT_STARTED" }
  ]
}
```

Rules:
- Only `threshold: 2` is accepted in this demo.
- Participants must be exactly `node-a` and `node-b`.
- If an active session already exists, the coordinator returns it instead of creating a second active session.

### Read Active DKG Session

`GET /api/dkg/sessions/active`

Response shape is the same as `POST /api/dkg/sessions`.

If no active session exists, the coordinator returns HTTP `404`.

### Trigger A Node Round

`POST /api/dkg/sessions/{session_id}/nodes/{node_id}/rounds/{round}`

Response:

```json
{
  "session_id": "00000000-0000-0000-0000-000000000000",
  "node_id": "node-a",
  "round": 1,
  "status": "COMPLETED",
  "dkg_status": "ROUND_1_IN_PROGRESS",
  "public_payload": {
    "kind": "phase-2-placeholder-dkg-round",
    "node_id": "node-a",
    "round": 1
  }
}
```

Rules:
- `node_id` must be `node-a` or `node-b`.
- `round` must be `1`, `2`, or `3`.
- Round 2 requires both node Round 1 steps to be completed.
- Round 3 requires both node Round 2 steps to be completed.
- Re-triggering a completed step returns the stored public payload and does not call crypto again.
- Re-triggering a step while another request is already running returns HTTP `409`.
- When both Round 3 steps are completed, session status becomes `COMPLETED`.

## Internal TSS Node API

These endpoints are called by the coordinator only:

- `POST /internal/dkg/{session_id}/round1`
- `POST /internal/dkg/{session_id}/round2`
- `POST /internal/dkg/{session_id}/round3`

Response:

```json
{
  "session_id": "00000000-0000-0000-0000-000000000000",
  "node_id": "node-a",
  "round": 1,
  "status": "COMPLETED",
  "public_payload": {
    "kind": "phase-2-placeholder-dkg-round",
    "node_id": "node-a",
    "round": 1
  }
}
```

Forbidden fields:
- `root_share`
- `private_share`
- `nonce_secret`
- `secret_key`

## Database Tables

Coordinator schema:

```sql
coordinator.dkg_sessions
- id uuid primary key
- threshold int not null
- participant_count int not null
- status text not null
- master_public_key_base58 text null
- active boolean not null
- created_at timestamptz not null
- updated_at timestamptz not null

coordinator.dkg_node_steps
- session_id uuid not null references coordinator.dkg_sessions(id)
- node_id text not null
- round int not null
- status text not null
- public_payload jsonb null
- error_message text null
- completed_at timestamptz null
- updated_at timestamptz not null
- primary key (session_id, node_id, round)
```

Coordinator tables must not contain private root shares, nonce secrets, or node-local key packages.

## State Values

DKG session status:

```text
NOT_STARTED
ROUND_1_IN_PROGRESS
ROUND_1_COMPLETE
ROUND_2_IN_PROGRESS
ROUND_2_COMPLETE
ROUND_3_IN_PROGRESS
COMPLETED
FAILED
```

Node step status:

```text
NOT_STARTED
RUNNING
COMPLETED
FAILED
```

## Error Responses

Error body:

```json
{
  "error": "round 2 requires both round 1 steps to be completed"
}
```

Expected status codes:

| Status | Case |
|---:|---|
| 400 | Invalid threshold, participant list, node id, or round. |
| 404 | Active session or requested session does not exist. |
| 409 | Round transition is out of order, or the same node round is already running. |
| 502 | TSS node call fails or returns an invalid response. |
| 503 | DKG API is used without a database pool. |
| 500 | Database failure. |

## Frontend Control Surface

The first viewport should show the actual DKG control workflow, not a marketing landing page.

Required elements:
- Product identity: `FROST Template`.
- Current session status.
- Master public key placeholder or completed value.
- A 2 x 3 control grid for Node A/B and Round 1/2/3.
- Per-step status badge.
- Per-step run or replay button.
- Latest action response area.

Forbidden elements:
- A single action that completes all DKG steps.
- Direct browser calls to TSS node URLs.

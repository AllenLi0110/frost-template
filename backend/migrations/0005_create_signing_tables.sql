CREATE TABLE IF NOT EXISTS coordinator.signing_requests (
    id UUID PRIMARY KEY,
    wallet_index INTEGER NOT NULL REFERENCES coordinator.wallets(wallet_index),
    sender_address_base58 TEXT NOT NULL,
    recipient_address_base58 TEXT NOT NULL,
    amount_lamports BIGINT NOT NULL CHECK (amount_lamports > 0),
    status TEXT NOT NULL,
    message_payload JSONB,
    message_hash_hex TEXT,
    recent_blockhash TEXT,
    transaction_signature TEXT,
    explorer_url TEXT,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS signing_requests_wallet_index_idx
ON coordinator.signing_requests (wallet_index);

CREATE INDEX IF NOT EXISTS signing_requests_status_created_at_idx
ON coordinator.signing_requests (status, created_at DESC);

CREATE TABLE IF NOT EXISTS coordinator.signing_node_steps (
    request_id UUID NOT NULL REFERENCES coordinator.signing_requests(id) ON DELETE CASCADE,
    node_id TEXT NOT NULL,
    round INTEGER NOT NULL,
    status TEXT NOT NULL,
    public_payload JSONB,
    error_message TEXT,
    completed_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (request_id, node_id, round)
);

CREATE TABLE IF NOT EXISTS node_a.node_signing_states (
    request_id UUID PRIMARY KEY,
    node_id TEXT NOT NULL,
    wallet_index INTEGER NOT NULL,
    status TEXT NOT NULL,
    commitment_payload JSONB,
    signing_nonces_ciphertext TEXT,
    message_hash_hex TEXT,
    signature_share_hex TEXT,
    round2_consumed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS node_b.node_signing_states (
    request_id UUID PRIMARY KEY,
    node_id TEXT NOT NULL,
    wallet_index INTEGER NOT NULL,
    status TEXT NOT NULL,
    commitment_payload JSONB,
    signing_nonces_ciphertext TEXT,
    message_hash_hex TEXT,
    signature_share_hex TEXT,
    round2_consumed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO coordinator.schema_migrations (version)
VALUES ('0005_create_signing_tables')
ON CONFLICT (version) DO NOTHING;

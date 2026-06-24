ALTER TABLE coordinator.dkg_sessions
ADD COLUMN IF NOT EXISTS public_derivation_context JSONB;

CREATE TABLE IF NOT EXISTS coordinator.wallets (
    wallet_index INTEGER PRIMARY KEY,
    dkg_session_id UUID NOT NULL REFERENCES coordinator.dkg_sessions(id),
    derivation_path TEXT NOT NULL,
    public_key_base58 TEXT NOT NULL,
    address_base58 TEXT NOT NULL UNIQUE,
    balance_lamports BIGINT,
    balance_status TEXT NOT NULL DEFAULT 'UNKNOWN',
    balance_error_message TEXT,
    balance_checked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS wallets_dkg_session_id_idx
ON coordinator.wallets (dkg_session_id);

INSERT INTO coordinator.schema_migrations (version)
VALUES ('0004_create_wallet_tables')
ON CONFLICT (version) DO NOTHING;

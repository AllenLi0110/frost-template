CREATE SCHEMA IF NOT EXISTS coordinator;

CREATE TABLE IF NOT EXISTS coordinator.dkg_sessions (
    id UUID PRIMARY KEY,
    threshold INTEGER NOT NULL,
    participant_count INTEGER NOT NULL,
    status TEXT NOT NULL,
    master_public_key_base58 TEXT,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS dkg_sessions_single_active_idx
ON coordinator.dkg_sessions (active)
WHERE active = TRUE;

CREATE TABLE IF NOT EXISTS coordinator.dkg_node_steps (
    session_id UUID NOT NULL REFERENCES coordinator.dkg_sessions(id) ON DELETE CASCADE,
    node_id TEXT NOT NULL,
    round INTEGER NOT NULL,
    status TEXT NOT NULL,
    public_payload JSONB,
    error_message TEXT,
    completed_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (session_id, node_id, round)
);

INSERT INTO coordinator.schema_migrations (version)
VALUES ('0002_create_dkg_tables')
ON CONFLICT (version) DO NOTHING;

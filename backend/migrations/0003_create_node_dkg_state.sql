CREATE TABLE IF NOT EXISTS node_a.node_dkg_state (
    session_id UUID PRIMARY KEY,
    node_id TEXT NOT NULL,
    status TEXT NOT NULL,
    round1_secret_package_ciphertext TEXT,
    round1_public_package_hex TEXT,
    round2_secret_package_ciphertext TEXT,
    round2_public_packages JSONB,
    key_package_ciphertext TEXT,
    public_key_package_hex TEXT,
    master_public_key_base58 TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS node_b.node_dkg_state (
    session_id UUID PRIMARY KEY,
    node_id TEXT NOT NULL,
    status TEXT NOT NULL,
    round1_secret_package_ciphertext TEXT,
    round1_public_package_hex TEXT,
    round2_secret_package_ciphertext TEXT,
    round2_public_packages JSONB,
    key_package_ciphertext TEXT,
    public_key_package_hex TEXT,
    master_public_key_base58 TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO coordinator.schema_migrations (version)
VALUES ('0003_create_node_dkg_state')
ON CONFLICT (version) DO NOTHING;

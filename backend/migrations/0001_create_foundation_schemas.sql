CREATE SCHEMA IF NOT EXISTS coordinator;
CREATE SCHEMA IF NOT EXISTS node_a;
CREATE SCHEMA IF NOT EXISTS node_b;

CREATE TABLE IF NOT EXISTS coordinator.schema_migrations (
    version TEXT PRIMARY KEY,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO coordinator.schema_migrations (version)
VALUES ('0001_create_foundation_schemas')
ON CONFLICT (version) DO NOTHING;

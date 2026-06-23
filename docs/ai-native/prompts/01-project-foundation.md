# Prompt: Phase 1 Project Foundation

```text
You are working in the repository root.

Read first:
- Workspace or project AGENTS.md, if available
- README.md
- ASSIGNMENT_en.md
- docs/ai-native/00-agent-context.md
- docs/ai-native/01-implementation-roadmap.md
- features/dkg-flow.feature
- backend/Cargo.toml
- backend/coordinator/Cargo.toml
- backend/tss-node/Cargo.toml
- frontend/package.json
- frontend/tsconfig.json

Goal:
Implement the project foundation so all runtime services can boot and talk to each other.

Scope:
- Coordinator axum server with `/health`.
- TSS node axum server with `/health`.
- Config loading from env vars.
- Docker Compose with PostgreSQL 18, coordinator, node-a, node-b, frontend.
- `.env.example`.
- Initial SQL migration layout.
- Tests for health routes and config parsing.

Do not:
- Implement FROST crypto yet.
- Implement wallet derivation yet.
- Implement signing yet.
- Add one-click DKG or signing shortcuts.

Required methodology:
1. Update or confirm BDD scenarios.
2. Define config and service contracts.
3. Write failing tests.
4. Implement.
5. Run tests.
6. Add a short entry to docs/ai-native/logs/ai-collaboration-log.md.

Definition of done:
- `docker compose up` can start postgres, coordinator, node-a, node-b, and frontend.
- Coordinator can reach node health endpoints by configured URLs.
- Tests pass.
```


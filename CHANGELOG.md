# Changelog

All notable project changes are tracked here. Versions follow numeric SemVer patch increments.

## [0.1.1] - 2026-06-24

### Added

- AI-native phase workflow from repository bootstrap through Phase 5 signing orchestration.
- Docker Compose runtime for PostgreSQL, Coordinator, TSS node A, TSS node B, and Next.js frontend.
- Observable DKG state machine with real `frost-ed25519` DKG integration.
- Public wallet derivation from completed DKG public material.
- Signing request state machine through nonce commitment and signature-share collection.
- PR CI foundation for repository hygiene, backend tests, frontend lint/build, and integration verification.
- Version metadata foundation using `VERSION`, package metadata, changelog entries, and draft GitHub releases.

### Security

- Private DKG material and signing nonce secrets remain node-local.
- Coordinator and Frontend store and display public protocol state only.

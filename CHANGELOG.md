# Changelog

All notable project changes are tracked here. Versions follow numeric SemVer patch increments.

## [0.1.4] - 2026-06-27

### Fixed

- Refreshed each vault balance from Solana Devnet after transaction broadcast and confirmation updates.

## [0.1.3] - 2026-06-26

### Changed

- Added a README demo GIF for the MPC wallet dashboard.
- Refined the demo dashboard by removing ambiguous summary counters from the side panel.
- Refreshed Vault Watch balances automatically after broadcast and confirmation updates.

## [0.1.2] - 2026-06-25

### Added

- Phase 6 child-wallet FROST signing over serialized Solana transfer messages.
- Coordinator signature aggregation, Solana broadcast, confirmation refresh, and Explorer URL storage.
- Frontend aggregate/broadcast and confirmation controls.
- CI-safe Phase 6 integration verification with mock Solana RPC.

### Security

- Private root shares, child shares, and nonce secrets remain node-local.

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

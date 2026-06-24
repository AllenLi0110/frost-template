# Release Process

This project uses pull request CI gates for every change and SemVer tags for release checkpoints.

## Pull Request Gate

Every pull request must pass these GitHub Actions jobs before merge:

- `Repository hygiene and release metadata`
- `Backend Rust tests`
- `Frontend lint and build`
- `Integration verification`

Recommended GitHub branch rule for `main`:

1. Require a pull request before merging.
2. Require status checks to pass.
3. Select the four CI jobs listed above as required checks.
4. Block force pushes and branch deletion.
5. Require review from the project owner before merge.

## Version Policy

Use SemVer:

- Patch versions for every accepted project checkpoint, such as `0.1.1`, `0.1.2`, and `0.1.3`.
- Minor versions for larger capability milestones when the project owner explicitly chooses to advance the minor version.

The version must be a numeric SemVer value with no prerelease text, such as `0.1.1`. It must be updated in all release metadata locations:

- `VERSION`
- `frontend/package.json`
- `frontend/package-lock.json`
- `backend/Cargo.toml`
- `CHANGELOG.md`

Run this before opening a release PR:

```bash
./scripts/verify-phase.sh 0
```

## Release Flow

After a PR is merged to `main` and CI is green:

```bash
git checkout main
git pull origin main
git tag v$(cat VERSION)
git push origin v$(cat VERSION)
```

The `Release` GitHub Action verifies the tag matches `VERSION`, extracts the matching changelog section, and creates a draft GitHub release.

Human approval is still required before publishing the draft release.

# Agent Operating Rules

Use these rules for every implementation agent.

## Before Editing

Read:

1. `AGENTS.md` from the workspace root if available.
2. `README.md`
3. `ASSIGNMENT_en.md` or `ASSIGNMENT_zh.md`
4. `docs/ai-native/00-agent-context.md`
5. The prompt for the current phase.
6. Nearest `package.json`, `Cargo.toml`, `tsconfig.json`, and lint config for files being changed.

## Required Work Order

1. Update BDD feature file if behavior changes.
2. Define or update API/data contracts.
3. Add failing tests.
4. Implement production code.
5. Run tests.
6. Add a short note to `docs/ai-native/logs/ai-collaboration-log.md`.
7. Add or update an agent run report when working from an automated task.

## Boundaries

- Do not install dependencies at the workspace root.
- Do not hide DKG or signing behind a single button.
- Do not expose root shares, child shares, or nonce secrets through coordinator or frontend APIs.
- Do not skip tests for a feature.
- Do not introduce a different stack unless the decision is recorded in `logs/decision-log.md`.
- Do not push directly to `main` after the branch/PR automation workflow is active.
- Do not merge your own pull request.
- Do not accept failed verification without explicit human approval.

## Automation Rules

- Work on an isolated branch for Phase 1 and later.
- Use `docs/ai-native/templates/agent-task.md` to scope automated tasks.
- Use `docs/ai-native/templates/agent-run-report.md` to record trace evidence.
- Run `./scripts/verify-phase.sh <phase-number>` before opening a PR.
- Human review is required before merge.

## Completion Report Template

At the end of a phase, report:

1. Files changed.
2. Behavior implemented.
3. Tests added and command output summary.
4. Known limitations.
5. Next recommended phase.

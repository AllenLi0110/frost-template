# Agent Task Template

## Task

- Phase:
- Goal:
- Trigger:
- Branch:

## Read First

- Workspace or project AGENTS.md, if available
- `docs/ai-native/00-agent-context.md`
- `docs/ai-native/01-implementation-roadmap.md`
- Relevant phase prompt in `docs/ai-native/prompts/`
- Relevant feature file in `features/`
- Relevant contracts or specs

## Scope

- TBD

## Out Of Scope

- TBD

## Required Method

1. Update or confirm BDD scenarios.
2. Update or define contracts/specs.
3. Add or update tests.
4. Implement.
5. Run the phase verification command.
6. Update collaboration and decision logs.
7. Produce an agent run report.

## Verification

```bash
./scripts/verify-phase.sh <phase-number>
```

## Human Approval Required For

- Skipping any verification command.
- Changing security boundaries.
- Rewriting git history.
- Merging the PR.

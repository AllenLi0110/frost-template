# Automation Design

This document defines how the project can evolve from manual AI-native development to an automation-ready agent workflow.

## Automation Goal

The goal is not to let an agent directly push to `main` and merge its own work.

The goal is:

```text
event trigger
-> isolated agent branch
-> implementation
-> verification harness
-> pull request
-> CI
-> human review gate
-> merge
-> trace-based prompt/rule improvement
```

## Automation Levels

| Level | Name | Behavior |
|---:|---|---|
| 1 | Manual AI-native | Human starts each phase, agent implements, human reviews. |
| 2 | Scripted verification | Each phase has a repeatable verification command. |
| 3 | Issue-triggered agent | A GitHub issue or label starts an agent branch and PR. |
| 4 | CI/review repair | Failing CI or review comments can trigger a focused agent fix. |
| 5 | Self-improving harness | Run traces are analyzed to improve prompts, tests, and rules. |

This repository starts with Level 1 and Phase 0 prepares the files needed to move toward Levels 2 and 3.

## Event Sources

Supported human-triggered events:

- A user message such as `start phase 1`.
- A GitHub issue created from the agent task template.
- A PR review request asking for a specific fix.

Future automated events:

- CI failure on an agent PR.
- Review comment with an `agent:fix` label.
- Scheduled check for stale or blocked agent tasks.

## Branch And PR Strategy

Agents should not work directly on `main` after Phase 0.

Recommended branch format:

```text
agent/phase-<number>-<short-topic>
```

Examples:

```text
agent/phase-1-foundation
agent/phase-2-dkg-state-machine
agent/fix-ci-phase-2
```

Each automated run should end with either:

- A pull request that includes the agent run report.
- A blocked report explaining what human input is required.

## Human Approval Gates

Human approval is required before:

- Merging any PR.
- Rewriting git history.
- Adding or changing secrets.
- Accepting skipped tests.
- Changing cryptographic boundaries.
- Changing private-key or nonce handling.

## CI And Release Gates

Every pull request must run CI on GitHub before merge. The required checks are:

- `Repository hygiene and release metadata`
- `Backend Rust tests`
- `Frontend lint and build`
- `Phase 5 integration verification`

The repository owner should configure a GitHub branch rule for `main` that requires these checks and blocks direct pushes. Local verification is useful during development, but GitHub CI is the merge gate.

Version checkpoints are managed with `VERSION`, `CHANGELOG.md`, and Git tags. A release tag must match `v$(cat VERSION)`. Pushing a matching tag starts the release workflow and creates a draft GitHub release for human approval.

## Trace Requirements

Each agent run should record:

- Trigger.
- Scope.
- Files changed.
- Commands run.
- Test results.
- Failures and retries.
- Human corrections.
- Follow-up prompt or rule updates.

Use `docs/ai-native/templates/agent-run-report.md` as the standard report shape.

## Pull Request Requirements

Every agent PR should include:

- Phase or task id.
- Summary.
- Verification evidence.
- Risk notes.
- Screenshots when UI changes are included.
- Links to updated collaboration or decision log entries.

## Safety Rules

Agents must not:

- Commit `.env` or other secret files.
- Return or persist private root shares outside TSS nodes.
- Add one-click DKG or signing shortcuts.
- Push directly to `main` after the automation branch workflow is active.
- Merge their own PRs.
- Publish a release without a matching changelog entry and human approval.

## Recommended Phase 1 Automation

For Phase 1, create a branch such as:

```text
agent/phase-1-foundation
```

Then run:

```text
./scripts/verify-phase.sh 1
```

If verification passes, open a PR and wait for human review.

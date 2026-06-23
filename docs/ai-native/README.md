# AI-Native Implementation Playbook

This folder documents how the FROST assignment will be built with AI agents. The goal is not only to finish the demo, but to preserve enough reasoning, prompts, corrections, and verification evidence so the implementation process can be explained during review.

The roadmap and prompts are living artifacts. They should be revised after each phase based on what was learned, instead of being treated as a complete up-front design.

## How To Use This Folder

1. Start every new agent session with `00-agent-context.md`.
2. Pick exactly one phase from `01-implementation-roadmap.md`.
3. Read `03-loop-engineering.md` and `04-automation-design.md` when automation or agent handoff is involved.
4. Copy the matching prompt from `prompts/`.
5. Ask the agent to implement only that phase.
6. Run the matching verification command from `05-verification-harness.md`.
7. Record important prompts, wrong turns, fixes, and verification output in `logs/ai-collaboration-log.md`.
8. Record architecture decisions in `logs/decision-log.md`.

## Development Order

Follow the workspace methodology:

1. BDD: update or add `features/*.feature`.
2. SDD: define API contracts, data models, and service boundaries before implementation.
3. TDD: add failing tests before production code.
4. Implementation: make the smallest working slice pass.
5. Verification: run tests and record the result.

## Current Target Architecture

The demo has five runtime components:

1. Next.js frontend
2. Rust coordinator service
3. Rust TSS node A
4. Rust TSS node B
5. PostgreSQL

The frontend only calls the coordinator. The coordinator routes protocol steps and talks to Solana Devnet. TSS nodes own private crypto material and must never expose root shares, child shares, or nonce secrets.

## Recommended Agent Pattern

Use one agent per bounded phase. Each prompt should include:

1. Context files to read.
2. Exact scope.
3. Files the agent may edit.
4. Tests required.
5. Definition of done.
6. Evidence to add to the collaboration log.

## Automation-Ready Pattern

After Phase 0, agents should work through isolated branches and pull requests:

1. Create or select an agent task from `.github/ISSUE_TEMPLATE/agent-task.yml`.
2. Use `docs/ai-native/templates/agent-task.md` to define scope.
3. Work on an agent branch, not directly on `main`.
4. Run `./scripts/verify-phase.sh <phase-number>`.
5. Add an agent run report using `docs/ai-native/templates/agent-run-report.md`.
6. Open a pull request using `.github/pull_request_template.md`.
7. Wait for CI and human review before merge.

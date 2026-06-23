# Loop Engineering Model

This project treats AI-native development as a set of controlled feedback loops, not as one large prompt that writes the whole system.

## Loop Layers

### Agent Loop

The agent loop is the runtime behavior inside a single task:

1. Read project context and task instructions.
2. Inspect the current repository state.
3. Edit the smallest required set of files.
4. Run the required checks.
5. Use failures as feedback and retry until the task is complete or blocked.

In this repository, each phase prompt in `docs/ai-native/prompts/` is designed to start one agent loop.

### Verification Loop

The verification loop decides whether a task is actually done.

Each phase must define:

- Required behavior.
- Required contracts or specs.
- Required tests and smoke checks.
- A definition of done.

The agent may retry implementation work, but completion requires the verification commands for that phase to pass or an explicit human decision to accept a known gap.

### Event-Driven Loop

The event-driven loop decides when an agent should start.

Current events are human-triggered:

- `start phase 1`
- `start phase 2`
- A user correction such as "the reviewer cannot see the UI"

Automation-ready events can include:

- A GitHub issue labeled `agent:task`.
- A pull request review comment.
- A failing CI check.
- A scheduled workflow that checks unfinished tasks.

### Self-Improvement Loop

The self-improvement loop uses traces from previous agent runs to improve the system.

Inputs:

- Collaboration log entries.
- Decision log entries.
- Test failures.
- CI failures.
- Human corrections.

Outputs:

- Better phase prompts.
- More precise verification commands.
- New BDD scenarios.
- Updated operating rules.
- Smaller or better-scoped tasks.

## Feedback Mapping

Every important correction should answer four questions:

| Question | Meaning |
|---|---|
| Trigger | What started the loop? |
| Verification | How did we know the result was incomplete or correct? |
| Gap | What was missing or unsafe? |
| System update | Which prompt, rule, test, or document changed so the next loop improves? |

## Human Gate

This project can use agents to implement and verify work, but human approval remains required for:

- Force-pushing shared branches.
- Merging pull requests.
- Accepting security-sensitive changes.
- Accepting failed or skipped verification.
- Changing private-key, secret, or signing behavior.

Agent automation should speed up delivery without removing accountability.

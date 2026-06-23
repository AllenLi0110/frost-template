# Prompt: Bootstrap AI-Native Repo Files

Use this prompt when asking an agent to create or update the project planning foundation.

```text
You are working in the repository root.

Read:
- Workspace or project AGENTS.md, if available
- README.md
- ASSIGNMENT_en.md
- docs/ai-native/00-agent-context.md if it exists
- External system design reference: 15.design-frost-template.md

Goal:
Create the AI-native implementation foundation for this FROST assignment.

Scope:
- Add BDD feature files for DKG, wallet derivation, and signing transfer.
- Add docs explaining how agents should work phase by phase.
- Add collaboration and decision logs.
- Do not implement runtime code yet.

Constraints:
- Keep docs concise and actionable.
- Preserve the assignment requirement that DKG and signing are step-by-step.
- Record that root shares and nonce secrets must never leave TSS nodes.

Definition of done:
- Future agents can start from the docs without rereading the full assignment.
- Feature files describe the observable reviewer workflows.
- No application code is changed.
```


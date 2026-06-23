#!/usr/bin/env bash
set -euo pipefail

phase="${1:-}"

if [[ -z "$phase" ]]; then
  echo "Usage: ./scripts/verify-phase.sh <phase-number>"
  exit 2
fi

check_no_sensitive_patterns() {
  local paths=()

  [[ -d docs ]] && paths+=(docs)
  [[ -d features ]] && paths+=(features)
  [[ -d .github ]] && paths+=(.github)
  [[ -d scripts ]] && paths+=(scripts)

  if [[ "${#paths[@]}" -gt 0 ]]; then
    ! grep -RInE "/Users/[[:alnum:]_.-]+/|([A-Z0-9_]*(SECRET|PRIVATE_KEY|API_KEY)[A-Z0-9_]*[[:space:]]*[:=])" "${paths[@]}"
  fi
}

check_phase_zero_files() {
  test -f docs/ai-native/00-agent-context.md
  test -f docs/ai-native/01-implementation-roadmap.md
  test -f docs/ai-native/02-agent-operating-rules.md
  test -f docs/ai-native/03-loop-engineering.md
  test -f docs/ai-native/04-automation-design.md
  test -f docs/ai-native/05-verification-harness.md
  test -f docs/ai-native/templates/agent-task.md
  test -f docs/ai-native/templates/agent-run-report.md
  test -f .github/ISSUE_TEMPLATE/agent-task.yml
  test -f .github/pull_request_template.md
  test -f .github/workflows/ci.yml
}

case "$phase" in
  0)
    check_no_sensitive_patterns
    git diff --check
    check_phase_zero_files
    ;;
  1)
    echo "Phase 1 verification is available after the service foundation is implemented."
    echo "Expected checks: docker compose config, backend tests, frontend lint, compose smoke tests."
    ;;
  2)
    echo "Phase 2 verification is available after the DKG state machine is implemented."
    echo "Expected checks: backend tests, frontend lint/build, compose DKG smoke tests."
    ;;
  *)
    echo "No verification harness is defined for phase ${phase} yet."
    exit 2
    ;;
esac

echo "Phase ${phase} verification passed."

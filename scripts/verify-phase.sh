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

check_phase_one_stack() {
  docker compose config >/dev/null
  docker compose run --rm --no-deps coordinator cargo test --workspace
  npm --prefix frontend run lint
  docker compose up -d --force-recreate
  docker compose ps
  docker compose exec frontend node -e '
const urls = [
  "http://coordinator:8080/health",
  "http://node-a:8081/health",
  "http://node-b:8081/health",
  "http://coordinator:8080/health/nodes"
];

async function main() {
  for (const url of urls) {
    const response = await fetch(url);
    const body = await response.text();

    if (!response.ok) {
      throw new Error(`${url} returned ${response.status}: ${body}`);
    }

    if (url.endsWith("/health/nodes")) {
      const payload = JSON.parse(body);
      const unreachable = payload.nodes.filter((node) => !node.reachable);

      if (unreachable.length > 0) {
        throw new Error(`unreachable nodes: ${JSON.stringify(unreachable)}`);
      }
    }
  }
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
'
}

case "$phase" in
  0)
    check_no_sensitive_patterns
    git diff --check
    check_phase_zero_files
    ;;
  1)
    check_no_sensitive_patterns
    git diff --check
    check_phase_one_stack
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

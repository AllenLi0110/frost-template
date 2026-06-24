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

check_phase_two_stack() {
  docker compose config >/dev/null
  docker compose run --rm --no-deps coordinator cargo test --workspace
  npm --prefix frontend run lint
  npm --prefix frontend run build
  docker compose up -d --force-recreate
  docker compose ps

  if docker compose port node-a 8081 >/tmp/frost-node-a-port.txt 2>&1; then
    echo "node-a must not publish its internal API port to the host"
    cat /tmp/frost-node-a-port.txt
    exit 1
  fi

  if docker compose port node-b 8081 >/tmp/frost-node-b-port.txt 2>&1; then
    echo "node-b must not publish its internal API port to the host"
    cat /tmp/frost-node-b-port.txt
    exit 1
  fi

  docker compose exec -T frontend node -e '
async function fetchWithRetry(url, options = {}, attempts = 60) {
  let lastError;

  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    try {
      const response = await fetch(url, options);

      if (response.ok) {
        return response;
      }

      lastError = new Error(`${url} returned HTTP ${response.status}`);
    } catch (error) {
      lastError = error;
    }

    await new Promise((resolve) => setTimeout(resolve, 1000));
  }

  throw lastError;
}

async function main() {
  await fetchWithRetry("http://coordinator:8080/health");
  await fetchWithRetry("http://node-a:8081/health");
  await fetchWithRetry("http://node-b:8081/health");
  const nodes = await fetchWithRetry("http://coordinator:8080/health/nodes").then((response) => response.json());
  const unreachable = nodes.nodes.filter((node) => !node.reachable);

  if (unreachable.length > 0) {
    throw new Error(`unreachable nodes: ${JSON.stringify(unreachable)}`);
  }
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
'

  docker compose exec -T postgres psql -U "${POSTGRES_USER:-frost}" -d "${POSTGRES_DB:-frost}" -c "TRUNCATE coordinator.dkg_sessions CASCADE;"

  docker compose exec -T frontend node -e '
const baseUrl = "http://coordinator:8080";

async function request(path, options = {}) {
  const response = await fetch(`${baseUrl}${path}`, options);
  const text = await response.text();
  const body = text ? JSON.parse(text) : null;

  return { response, body };
}

async function expectOk(path, options = {}) {
  const { response, body } = await request(path, options);

  if (!response.ok) {
    throw new Error(`${path} returned HTTP ${response.status}: ${JSON.stringify(body)}`);
  }

  return body;
}

async function expectStatus(path, status, options = {}) {
  const { response, body } = await request(path, options);

  if (response.status !== status) {
    throw new Error(`${path} expected HTTP ${status}, got ${response.status}: ${JSON.stringify(body)}`);
  }

  return body;
}

async function trigger(sessionId, nodeId, round) {
  return expectOk(`/api/dkg/sessions/${sessionId}/nodes/${nodeId}/rounds/${round}`, {
    method: "POST"
  });
}

async function main() {
  const createRequest = {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      threshold: 2,
      participants: ["node-a", "node-b"]
    })
  };

  const [firstCreate, secondCreate] = await Promise.all([
    expectOk("/api/dkg/sessions", createRequest),
    expectOk("/api/dkg/sessions", createRequest)
  ]);

  if (firstCreate.session_id !== secondCreate.session_id) {
    throw new Error(`concurrent creates returned different sessions: ${JSON.stringify([firstCreate, secondCreate])}`);
  }

  const session = firstCreate;

  if (session.status !== "NOT_STARTED" || session.node_steps.length !== 6) {
    throw new Error(`unexpected initial DKG session: ${JSON.stringify(session)}`);
  }

  await expectStatus(`/api/dkg/sessions/${session.session_id}/nodes/node-a/rounds/2`, 409, {
    method: "POST"
  });

  const duplicateRoundOne = await Promise.all([
    request(`/api/dkg/sessions/${session.session_id}/nodes/node-a/rounds/1`, { method: "POST" }),
    request(`/api/dkg/sessions/${session.session_id}/nodes/node-a/rounds/1`, { method: "POST" })
  ]);
  const duplicateStatuses = duplicateRoundOne.map((item) => item.response.status);

  if (!duplicateStatuses.includes(200) || duplicateStatuses.some((status) => status !== 200 && status !== 409)) {
    throw new Error(`duplicate round trigger returned unexpected statuses: ${JSON.stringify(duplicateStatuses)}`);
  }

  await trigger(session.session_id, "node-b", 1);

  await expectStatus(`/api/dkg/sessions/${session.session_id}/nodes/node-a/rounds/3`, 409, {
    method: "POST"
  });

  await trigger(session.session_id, "node-a", 2);
  await trigger(session.session_id, "node-b", 2);
  await trigger(session.session_id, "node-a", 3);
  const completedRound = await trigger(session.session_id, "node-b", 3);

  if (completedRound.dkg_status !== "COMPLETED") {
    throw new Error(`DKG did not complete: ${JSON.stringify(completedRound)}`);
  }

  const replay = await trigger(session.session_id, "node-a", 1);

  if (replay.status !== "COMPLETED" || replay.public_payload?.kind !== "phase-2-placeholder-dkg-round") {
    throw new Error(`completed round replay did not return stored public payload: ${JSON.stringify(replay)}`);
  }

  const active = await expectOk("/api/dkg/sessions/active");

  if (active.status !== "COMPLETED" || !active.master_public_key_base58) {
    throw new Error(`active DKG session is not completed: ${JSON.stringify(active)}`);
  }
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
'

  docker compose restart coordinator

  docker compose exec -T frontend node -e '
async function fetchWithRetry(url, attempts = 60) {
  let lastError;

  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    try {
      const response = await fetch(url);
      const text = await response.text();
      const body = text ? JSON.parse(text) : null;

      if (response.ok) {
        return body;
      }

      lastError = new Error(`${url} returned HTTP ${response.status}: ${JSON.stringify(body)}`);
    } catch (error) {
      lastError = error;
    }

    await new Promise((resolve) => setTimeout(resolve, 1000));
  }

  throw lastError;
}

async function main() {
  const active = await fetchWithRetry("http://coordinator:8080/api/dkg/sessions/active");

  if (active.status !== "COMPLETED" || !active.master_public_key_base58) {
    throw new Error(`completed DKG session did not survive restart: ${JSON.stringify(active)}`);
  }
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
'

  docker compose exec -T frontend node -e '
async function main() {
  const response = await fetch("http://localhost:3000/");
  const html = await response.text();

  if (!response.ok) {
    throw new Error(`frontend returned HTTP ${response.status}`);
  }

  if (!html.includes("FROST Template") || !html.includes("DKG Control Surface")) {
    throw new Error("frontend did not render the DKG control surface");
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
    check_no_sensitive_patterns
    git diff --check
    check_phase_two_stack
    ;;
  *)
    echo "No verification harness is defined for phase ${phase} yet."
    exit 2
    ;;
esac

echo "Phase ${phase} verification passed."

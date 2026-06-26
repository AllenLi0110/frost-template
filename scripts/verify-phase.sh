#!/usr/bin/env bash
set -euo pipefail

phase="${1:-}"

if [[ -z "$phase" ]]; then
  echo "Usage: ./scripts/verify-phase.sh <phase-number>"
  exit 2
fi

phase_six_compose() {
  COMPOSE_PROJECT_NAME="${PHASE_SIX_COMPOSE_PROJECT:-frost-template-verify-phase6}" \
  POSTGRES_HOST_PORT="${PHASE_SIX_POSTGRES_HOST_PORT:-15432}" \
  COORDINATOR_HOST_PORT="${PHASE_SIX_COORDINATOR_HOST_PORT:-18080}" \
  FRONTEND_HOST_PORT="${PHASE_SIX_FRONTEND_HOST_PORT:-13000}" \
  SOLANA_RPC_URL=mock://phase6 \
  docker compose "$@"
}

cleanup_phase_six_stack() {
  phase_six_compose down -v --remove-orphans >/dev/null 2>&1 || true
}

check_no_sensitive_patterns() {
  local paths=()

  [[ -d docs ]] && paths+=(docs)
  [[ -d features ]] && paths+=(features)
  [[ -d .github ]] && paths+=(.github)
  [[ -d scripts ]] && paths+=(scripts)
  [[ -f README.md ]] && paths+=(README.md)
  [[ -f .env.example ]] && paths+=(.env.example)

  if [[ "${#paths[@]}" -gt 0 ]]; then
    ! grep -RInE "/Users/[[:alnum:]_.-]+/|([A-Z0-9_]*(SECRET|PRIVATE_KEY|API_KEY)[A-Z0-9_]*[[:space:]]*[:=])" "${paths[@]}"
  fi
}

check_phase_zero_files() {
  test -f VERSION
  test -f CHANGELOG.md
  test -f docs/release-process.md
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
  test -f .github/workflows/release.yml
  test -f scripts/verify-release-metadata.mjs
  test -f scripts/extract-release-notes.mjs
}

check_release_metadata() {
  node scripts/verify-release-metadata.mjs
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

check_phase_three_stack() {
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

  docker compose exec -T postgres psql -U "${POSTGRES_USER:-frost}" -d "${POSTGRES_DB:-frost}" -c "TRUNCATE coordinator.dkg_sessions CASCADE; TRUNCATE node_a.node_dkg_state; TRUNCATE node_b.node_dkg_state;"

  docker compose exec -T frontend node -e '
const baseUrl = "http://coordinator:8080";
const forbiddenFields = [
  "root_share",
  "private_share",
  "nonce_secret",
  "secret_key",
  "key_package_ciphertext",
  "round1_secret_package_ciphertext",
  "round2_secret_package_ciphertext"
];

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

async function trigger(sessionId, nodeId, round) {
  return expectOk(`/api/dkg/sessions/${sessionId}/nodes/${nodeId}/rounds/${round}`, {
    method: "POST"
  });
}

function expectPublicPayload(payload, kind) {
  if (payload?.public_payload?.kind !== kind) {
    throw new Error(`expected ${kind}, got ${JSON.stringify(payload)}`);
  }

  if (payload.public_payload.kind === "phase-2-placeholder-dkg-round") {
    throw new Error(`placeholder DKG payload leaked into phase 3: ${JSON.stringify(payload)}`);
  }

  if (kind === "frost-dkg-round2" && payload.public_payload.round2_packages) {
    throw new Error(`round 2 routing packages must not be exposed to frontend responses: ${JSON.stringify(payload)}`);
  }
}

function assertNoForbiddenFields(value) {
  const encoded = JSON.stringify(value);

  for (const field of forbiddenFields) {
    if (encoded.includes(field)) {
      throw new Error(`forbidden private field ${field} appeared in coordinator response: ${encoded}`);
    }
  }
}

async function main() {
  const session = await expectOk("/api/dkg/sessions", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      threshold: 2,
      participants: ["node-a", "node-b"]
    })
  });

  const transcript = [];

  transcript.push(await trigger(session.session_id, "node-a", 1));
  transcript.push(await trigger(session.session_id, "node-b", 1));
  transcript.push(await trigger(session.session_id, "node-a", 2));
  transcript.push(await trigger(session.session_id, "node-b", 2));
  transcript.push(await trigger(session.session_id, "node-a", 3));
  transcript.push(await trigger(session.session_id, "node-b", 3));

  expectPublicPayload(transcript[0], "frost-dkg-round1");
  expectPublicPayload(transcript[1], "frost-dkg-round1");
  expectPublicPayload(transcript[2], "frost-dkg-round2");
  expectPublicPayload(transcript[3], "frost-dkg-round2");
  expectPublicPayload(transcript[4], "frost-dkg-round3");
  expectPublicPayload(transcript[5], "frost-dkg-round3");

  const completedRound = transcript[5];

  if (completedRound.dkg_status !== "COMPLETED") {
    throw new Error(`DKG did not complete: ${JSON.stringify(completedRound)}`);
  }

  const replay = await trigger(session.session_id, "node-a", 3);
  expectPublicPayload(replay, "frost-dkg-round3");

  const active = await expectOk("/api/dkg/sessions/active");

  if (active.status !== "COMPLETED") {
    throw new Error(`active DKG session is not completed: ${JSON.stringify(active)}`);
  }

  if (!/^[1-9A-HJ-NP-Za-km-z]+$/.test(active.master_public_key_base58) || active.master_public_key_base58.length < 32) {
    throw new Error(`master public key is not Base58-like: ${JSON.stringify(active)}`);
  }

  assertNoForbiddenFields(transcript);
  assertNoForbiddenFields(replay);
  assertNoForbiddenFields(active);
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
'

  local node_a_private_count
  node_a_private_count="$(docker compose exec -T postgres psql -U "${POSTGRES_USER:-frost}" -d "${POSTGRES_DB:-frost}" -At -c "SELECT count(*) FROM node_a.node_dkg_state WHERE round1_secret_package_ciphertext LIKE 'v1:%' AND round2_secret_package_ciphertext LIKE 'v1:%' AND key_package_ciphertext LIKE 'v1:%';")"
  if [[ "$node_a_private_count" != "1" ]]; then
    echo "node-a did not persist encrypted DKG private material"
    exit 1
  fi

  local node_b_private_count
  node_b_private_count="$(docker compose exec -T postgres psql -U "${POSTGRES_USER:-frost}" -d "${POSTGRES_DB:-frost}" -At -c "SELECT count(*) FROM node_b.node_dkg_state WHERE round1_secret_package_ciphertext LIKE 'v1:%' AND round2_secret_package_ciphertext LIKE 'v1:%' AND key_package_ciphertext LIKE 'v1:%';")"
  if [[ "$node_b_private_count" != "1" ]]; then
    echo "node-b did not persist encrypted DKG private material"
    exit 1
  fi

  local coordinator_forbidden_count
  coordinator_forbidden_count="$(docker compose exec -T postgres psql -U "${POSTGRES_USER:-frost}" -d "${POSTGRES_DB:-frost}" -At -c "SELECT count(*) FROM coordinator.dkg_node_steps WHERE public_payload::text ~ '(root_share|private_share|nonce_secret|secret_key|key_package_ciphertext|round1_secret_package_ciphertext|round2_secret_package_ciphertext)';")"
  if [[ "$coordinator_forbidden_count" != "0" ]]; then
    echo "coordinator public payloads contain forbidden private field names"
    exit 1
  fi

  docker compose restart coordinator node-a node-b

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
    throw new Error(`completed FROST DKG session did not survive restart: ${JSON.stringify(active)}`);
  }
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
'
}

check_phase_four_stack() {
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

  docker compose exec -T postgres psql -U "${POSTGRES_USER:-frost}" -d "${POSTGRES_DB:-frost}" -c "TRUNCATE coordinator.wallets; TRUNCATE coordinator.dkg_sessions CASCADE; TRUNCATE node_a.node_dkg_state; TRUNCATE node_b.node_dkg_state;"

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

function assertWallet(wallet, expectedIndex) {
  if (wallet.wallet_index !== expectedIndex) {
    throw new Error(`expected wallet index ${expectedIndex}, got ${JSON.stringify(wallet)}`);
  }

  if (wallet.derivation_path !== `m/${expectedIndex}`) {
    throw new Error(`unexpected derivation path: ${JSON.stringify(wallet)}`);
  }

  if (!/^[1-9A-HJ-NP-Za-km-z]+$/.test(wallet.address_base58) || wallet.address_base58.length < 32) {
    throw new Error(`wallet address is not Base58-like: ${JSON.stringify(wallet)}`);
  }

  if (wallet.public_key_base58 !== wallet.address_base58) {
    throw new Error(`Solana address should match the derived public key: ${JSON.stringify(wallet)}`);
  }
}

async function main() {
  await expectStatus("/api/wallets", 409, { method: "POST" });

  const session = await expectOk("/api/dkg/sessions", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      threshold: 2,
      participants: ["node-a", "node-b"]
    })
  });

  await expectStatus("/api/wallets", 409, { method: "POST" });

  await trigger(session.session_id, "node-a", 1);
  await trigger(session.session_id, "node-b", 1);
  await trigger(session.session_id, "node-a", 2);
  await trigger(session.session_id, "node-b", 2);
  await trigger(session.session_id, "node-a", 3);
  const completedRound = await trigger(session.session_id, "node-b", 3);

  if (completedRound.dkg_status !== "COMPLETED") {
    throw new Error(`DKG did not complete: ${JSON.stringify(completedRound)}`);
  }

  const active = await expectOk("/api/dkg/sessions/active");

  if (active.status !== "COMPLETED" || !active.master_public_key_base58) {
    throw new Error(`active DKG session is not completed: ${JSON.stringify(active)}`);
  }

  const wallet0 = await expectOk("/api/wallets", { method: "POST" });
  const wallet1 = await expectOk("/api/wallets", { method: "POST" });
  const wallet2 = await expectOk("/api/wallets", { method: "POST" });

  assertWallet(wallet0, 0);
  assertWallet(wallet1, 1);
  assertWallet(wallet2, 2);

  const addresses = new Set([wallet0.address_base58, wallet1.address_base58, wallet2.address_base58]);

  if (addresses.size !== 3) {
    throw new Error(`derived wallet addresses must be unique: ${JSON.stringify([wallet0, wallet1, wallet2])}`);
  }

  const walletList = await expectOk("/api/wallets");
  const indexes = walletList.wallets.map((wallet) => wallet.wallet_index).join(",");

  if (indexes !== "0,1,2") {
    throw new Error(`wallet list returned unexpected indexes: ${JSON.stringify(walletList)}`);
  }

  const balance = await expectOk(`/api/wallets/${wallet0.wallet_index}/balance`);

  if (balance.address_base58 !== wallet0.address_base58) {
    throw new Error(`balance response used a different address: ${JSON.stringify(balance)}`);
  }

  if (!["AVAILABLE", "UNAVAILABLE"].includes(balance.balance_status)) {
    throw new Error(`balance lookup did not return a graceful status: ${JSON.stringify(balance)}`);
  }
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
'

  docker compose restart coordinator

  docker compose exec -T frontend node -e '
async function fetchWithRetry(url, options = {}, attempts = 60) {
  let lastError;

  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    try {
      const response = await fetch(url, options);
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
  const walletList = await fetchWithRetry("http://coordinator:8080/api/wallets");
  const indexes = walletList.wallets.map((wallet) => wallet.wallet_index).join(",");

  if (indexes !== "0,1,2") {
    throw new Error(`wallet list did not survive restart: ${JSON.stringify(walletList)}`);
  }

  const nextWallet = await fetchWithRetry("http://coordinator:8080/api/wallets", {
    method: "POST"
  });

  if (nextWallet.wallet_index !== 3) {
    throw new Error(`wallet index was reused after restart: ${JSON.stringify(nextWallet)}`);
  }
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
'

  docker compose exec -T frontend node -e '
async function fetchWithRetry(url, attempts = 60) {
  let lastError;

  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    try {
      const response = await fetch(url);
      const text = await response.text();

      if (response.ok) {
        return { response, text };
      }

      lastError = new Error(`${url} returned HTTP ${response.status}: ${text}`);
    } catch (error) {
      lastError = error;
    }

    await new Promise((resolve) => setTimeout(resolve, 1000));
  }

  throw lastError;
}

async function main() {
  let html = "";
  let cssBodies = [];

  for (let attempt = 1; attempt <= 60; attempt += 1) {
    html = (await fetchWithRetry("http://localhost:3000/", 1)).text;

    if (!html.includes("FROST Template") || !html.includes("Wallet Derivation")) {
      await new Promise((resolve) => setTimeout(resolve, 1000));
      continue;
    }

    const stylesheetPaths = Array.from(html.matchAll(/href="([^"]+\.css)"/g), (match) => match[1]);

    if (stylesheetPaths.length === 0) {
      await new Promise((resolve) => setTimeout(resolve, 1000));
      continue;
    }

    cssBodies = await Promise.all(
      stylesheetPaths.map(async (path) => {
        return (await fetchWithRetry(new URL(path, "http://localhost:3000/").toString(), 1)).text;
      })
    );

    if (cssBodies.some((css) => /\.wallet-row\s*\{[^}]*display:\s*grid/.test(css))) {
      return;
    }

    await new Promise((resolve) => setTimeout(resolve, 1000));
  }

  if (!html.includes("FROST Template") || !html.includes("Wallet Derivation")) {
    throw new Error("frontend did not render the DKG and wallet control surface");
  }

  throw new Error("frontend stylesheet did not include wallet row grid styles");
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
'
}

check_phase_five_stack() {
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
async function fetchWithRetry(url, options = {}, attempts = 240) {
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

  docker compose exec -T postgres psql -U "${POSTGRES_USER:-frost}" -d "${POSTGRES_DB:-frost}" -c "TRUNCATE coordinator.signing_requests CASCADE; TRUNCATE coordinator.wallets CASCADE; TRUNCATE coordinator.dkg_sessions CASCADE; TRUNCATE node_a.node_dkg_state; TRUNCATE node_b.node_dkg_state; TRUNCATE node_a.node_signing_states; TRUNCATE node_b.node_signing_states;"

  docker compose exec -T frontend node -e '
const baseUrl = "http://coordinator:8080";
const forbiddenFields = [
  "root_share",
  "private_share",
  "nonce_secret",
  "secret_key",
  "key_package_ciphertext",
  "signing_nonces_ciphertext"
];

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

async function triggerDkg(sessionId, nodeId, round) {
  return expectOk(`/api/dkg/sessions/${sessionId}/nodes/${nodeId}/rounds/${round}`, {
    method: "POST"
  });
}

async function triggerSigning(requestId, nodeId, round) {
  return expectOk(`/api/signing-requests/${requestId}/nodes/${nodeId}/rounds/${round}`, {
    method: "POST"
  });
}

function assertNoForbiddenFields(value) {
  const encoded = JSON.stringify(value);

  for (const field of forbiddenFields) {
    if (encoded.includes(field)) {
      throw new Error(`forbidden private field ${field} appeared in coordinator response: ${encoded}`);
    }
  }
}

async function createSigningRequest(walletIndex, recipientAddress, amountLamports) {
  return expectOk("/api/signing-requests", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      wallet_index: walletIndex,
      recipient_address_base58: recipientAddress,
      amount_lamports: amountLamports
    })
  });
}

async function main() {
  const session = await expectOk("/api/dkg/sessions", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      threshold: 2,
      participants: ["node-a", "node-b"]
    })
  });

  await triggerDkg(session.session_id, "node-a", 1);
  await triggerDkg(session.session_id, "node-b", 1);
  await triggerDkg(session.session_id, "node-a", 2);
  await triggerDkg(session.session_id, "node-b", 2);
  await triggerDkg(session.session_id, "node-a", 3);
  await triggerDkg(session.session_id, "node-b", 3);

  const wallet = await expectOk("/api/wallets", { method: "POST" });
  const recipientWallet = await expectOk("/api/wallets", { method: "POST" });

  await expectStatus("/api/signing-requests", 404, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      wallet_index: 99,
      recipient_address_base58: recipientWallet.address_base58,
      amount_lamports: 1
    })
  });

  const firstRequest = await createSigningRequest(wallet.wallet_index, recipientWallet.address_base58, 1000);
  const secondRequest = await createSigningRequest(wallet.wallet_index, recipientWallet.address_base58, 2000);

  if (firstRequest.request_id === secondRequest.request_id) {
    throw new Error(`multiple signing requests reused an id: ${JSON.stringify([firstRequest, secondRequest])}`);
  }

  const list = await expectOk("/api/signing-requests");
  const ids = new Set(list.requests.map((item) => item.request_id));

  if (!ids.has(firstRequest.request_id) || !ids.has(secondRequest.request_id)) {
    throw new Error(`request list did not include both requests: ${JSON.stringify(list)}`);
  }

  await expectStatus(`/api/signing-requests/${firstRequest.request_id}/nodes/node-a/rounds/2`, 409, {
    method: "POST"
  });

  const nodeARound1 = await triggerSigning(firstRequest.request_id, "node-a", 1);

  if (nodeARound1.signing_status !== "COMMITMENTS_IN_PROGRESS") {
    throw new Error(`unexpected status after node-a round 1: ${JSON.stringify(nodeARound1)}`);
  }

  const nodeARound1Replay = await triggerSigning(firstRequest.request_id, "node-a", 1);

  if (nodeARound1Replay.public_payload?.commitments_hex !== nodeARound1.public_payload?.commitments_hex) {
    throw new Error(`round 1 replay did not return stored commitment: ${JSON.stringify([nodeARound1, nodeARound1Replay])}`);
  }

  const nodeBRound1 = await triggerSigning(firstRequest.request_id, "node-b", 1);

  if (nodeBRound1.signing_status !== "COMMITMENTS_READY") {
    throw new Error(`unexpected status after node-b round 1: ${JSON.stringify(nodeBRound1)}`);
  }

  const nodeARound2 = await triggerSigning(firstRequest.request_id, "node-a", 2);

  if (nodeARound2.signing_status !== "SHARES_IN_PROGRESS" || !nodeARound2.public_payload?.signature_share_hex) {
    throw new Error(`node-a round 2 did not produce a signature share: ${JSON.stringify(nodeARound2)}`);
  }

  await expectStatus(`/api/signing-requests/${firstRequest.request_id}/nodes/node-a/rounds/2`, 409, {
    method: "POST"
  });

  const nodeBRound2 = await triggerSigning(firstRequest.request_id, "node-b", 2);

  if (nodeBRound2.signing_status !== "READY_TO_AGGREGATE" || !nodeBRound2.public_payload?.signature_share_hex) {
    throw new Error(`signing request did not become READY_TO_AGGREGATE: ${JSON.stringify(nodeBRound2)}`);
  }

  const completed = await expectOk(`/api/signing-requests/${firstRequest.request_id}`);

  if (completed.status !== "READY_TO_AGGREGATE" || !completed.message_hash_hex) {
    throw new Error(`completed signing request is not ready: ${JSON.stringify(completed)}`);
  }

  assertNoForbiddenFields(list);
  assertNoForbiddenFields(nodeARound1);
  assertNoForbiddenFields(nodeARound2);
  assertNoForbiddenFields(completed);
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
'

  docker compose exec -T postgres psql -U "${POSTGRES_USER:-frost}" -d "${POSTGRES_DB:-frost}" -c "DELETE FROM node_a.node_dkg_state;"

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

async function main() {
  const walletList = await expectOk("/api/wallets");
  const senderWallet = walletList.wallets.find((wallet) => wallet.wallet_index === 0);

  if (!senderWallet) {
    throw new Error(`wallet 0 was not available for failure regression: ${JSON.stringify(walletList)}`);
  }

  const failedRequest = await expectOk("/api/signing-requests", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      wallet_index: 0,
      recipient_address_base58: senderWallet.address_base58,
      amount_lamports: 3000
    })
  });

  await expectStatus(`/api/signing-requests/${failedRequest.request_id}/nodes/node-a/rounds/1`, 502, {
    method: "POST"
  });

  const failed = await expectOk(`/api/signing-requests/${failedRequest.request_id}`);

  if (failed.status !== "FAILED" || !failed.error_message) {
    throw new Error(`failed node call did not mark request FAILED: ${JSON.stringify(failed)}`);
  }

  await expectStatus(`/api/signing-requests/${failedRequest.request_id}/nodes/node-b/rounds/1`, 409, {
    method: "POST"
  });
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
'

  local node_a_nonce_count
  node_a_nonce_count="$(docker compose exec -T postgres psql -U "${POSTGRES_USER:-frost}" -d "${POSTGRES_DB:-frost}" -At -c "SELECT count(*) FROM node_a.node_signing_states WHERE signing_nonces_ciphertext LIKE 'v1:%' AND signature_share_hex IS NOT NULL AND round2_consumed_at IS NOT NULL;")"
  if [[ "$node_a_nonce_count" != "1" ]]; then
    echo "node-a did not persist encrypted nonce state and consume it once"
    exit 1
  fi

  local node_b_nonce_count
  node_b_nonce_count="$(docker compose exec -T postgres psql -U "${POSTGRES_USER:-frost}" -d "${POSTGRES_DB:-frost}" -At -c "SELECT count(*) FROM node_b.node_signing_states WHERE signing_nonces_ciphertext LIKE 'v1:%' AND signature_share_hex IS NOT NULL AND round2_consumed_at IS NOT NULL;")"
  if [[ "$node_b_nonce_count" != "1" ]]; then
    echo "node-b did not persist encrypted nonce state and consume it once"
    exit 1
  fi

  local coordinator_forbidden_count
  coordinator_forbidden_count="$(docker compose exec -T postgres psql -U "${POSTGRES_USER:-frost}" -d "${POSTGRES_DB:-frost}" -At -c "SELECT count(*) FROM coordinator.signing_node_steps WHERE public_payload::text ~ '(root_share|private_share|nonce_secret|secret_key|key_package_ciphertext|signing_nonces_ciphertext)';")"
  if [[ "$coordinator_forbidden_count" != "0" ]]; then
    echo "coordinator signing payloads contain forbidden private field names"
    exit 1
  fi

  docker compose restart coordinator node-a node-b

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
  const list = await fetchWithRetry("http://coordinator:8080/api/signing-requests");
  const ready = list.requests.filter((request) => request.status === "READY_TO_AGGREGATE");

  if (ready.length !== 1) {
    throw new Error(`ready signing request did not survive restart: ${JSON.stringify(list)}`);
  }
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
'

  docker compose exec -T frontend node -e '
async function fetchWithRetry(url, attempts = 60) {
  let lastError;

  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    try {
      const response = await fetch(url);
      const text = await response.text();

      if (response.ok) {
        return { response, text };
      }

      lastError = new Error(`${url} returned HTTP ${response.status}: ${text}`);
    } catch (error) {
      lastError = error;
    }

    await new Promise((resolve) => setTimeout(resolve, 1000));
  }

  throw lastError;
}

async function main() {
  let html = "";
  let cssBodies = [];

  for (let attempt = 1; attempt <= 60; attempt += 1) {
    html = (await fetchWithRetry("http://localhost:3000/", 1)).text;

    if (!html.includes("FROST Template") || !html.includes("Signing Requests")) {
      await new Promise((resolve) => setTimeout(resolve, 1000));
      continue;
    }

    const stylesheetPaths = Array.from(html.matchAll(/href="([^"]+\.css)"/g), (match) => match[1]);

    if (stylesheetPaths.length === 0) {
      await new Promise((resolve) => setTimeout(resolve, 1000));
      continue;
    }

    cssBodies = await Promise.all(
      stylesheetPaths.map(async (path) => {
        return (await fetchWithRetry(new URL(path, "http://localhost:3000/").toString(), 1)).text;
      })
    );

    if (cssBodies.some((css) => /\.signing-round-grid\s*\{[^}]*display:\s*grid/.test(css))) {
      return;
    }

    await new Promise((resolve) => setTimeout(resolve, 1000));
  }

  if (!html.includes("FROST Template") || !html.includes("Signing Requests")) {
    throw new Error("frontend did not render the signing request control surface");
  }

  throw new Error("frontend stylesheet did not include signing round grid styles");
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
'
}

check_phase_six_stack() {
  phase_six_compose config >/dev/null
  phase_six_compose run --rm --no-deps coordinator cargo test --workspace
  npm --prefix frontend run lint
  npm --prefix frontend run build
  phase_six_compose up -d --force-recreate
  phase_six_compose ps

  phase_six_compose exec -T postgres psql -U "${POSTGRES_USER:-frost}" -d "${POSTGRES_DB:-frost}" -c "TRUNCATE coordinator.signing_requests CASCADE; TRUNCATE coordinator.wallets CASCADE; TRUNCATE coordinator.dkg_sessions CASCADE; TRUNCATE node_a.node_dkg_state; TRUNCATE node_b.node_dkg_state; TRUNCATE node_a.node_signing_states; TRUNCATE node_b.node_signing_states;"

  phase_six_compose exec -T frontend node -e '
const baseUrl = "http://coordinator:8080";
const forbiddenFields = [
  "root_share",
  "private_share",
  "nonce_secret",
  "secret_key",
  "key_package_ciphertext",
  "signing_nonces_ciphertext"
];

async function sleep(ms) {
  await new Promise((resolve) => setTimeout(resolve, ms));
}

async function fetchWithRetry(url, options = {}, attempts = 120) {
  let lastError;

  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    try {
      const response = await fetch(url, options);
      const text = await response.text();
      const body = text ? JSON.parse(text) : null;

      if (response.ok) {
        return body;
      }

      lastError = new Error(`${url} returned HTTP ${response.status}: ${JSON.stringify(body)}`);
    } catch (error) {
      lastError = error;
    }

    await sleep(1000);
  }

  throw lastError;
}

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

async function triggerDkg(sessionId, nodeId, round) {
  return expectOk(`/api/dkg/sessions/${sessionId}/nodes/${nodeId}/rounds/${round}`, {
    method: "POST"
  });
}

async function triggerSigning(requestId, nodeId, round) {
  return expectOk(`/api/signing-requests/${requestId}/nodes/${nodeId}/rounds/${round}`, {
    method: "POST"
  });
}

function assertNoForbiddenFields(value) {
  const encoded = JSON.stringify(value);

  for (const field of forbiddenFields) {
    if (encoded.includes(field)) {
      throw new Error(`forbidden private field ${field} appeared in public payload: ${encoded}`);
    }
  }
}

async function main() {
  await fetchWithRetry(`${baseUrl}/health`);
  await fetchWithRetry("http://node-a:8081/health");
  await fetchWithRetry("http://node-b:8081/health");

  const session = await expectOk("/api/dkg/sessions", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      threshold: 2,
      participants: ["node-a", "node-b"]
    })
  });

  await triggerDkg(session.session_id, "node-a", 1);
  await triggerDkg(session.session_id, "node-b", 1);
  await triggerDkg(session.session_id, "node-a", 2);
  await triggerDkg(session.session_id, "node-b", 2);
  await triggerDkg(session.session_id, "node-a", 3);
  await triggerDkg(session.session_id, "node-b", 3);

  const wallet = await expectOk("/api/wallets", { method: "POST" });
  const recipientWallet = await expectOk("/api/wallets", { method: "POST" });
  const signingRequest = await expectOk("/api/signing-requests", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      wallet_index: wallet.wallet_index,
      recipient_address_base58: recipientWallet.address_base58,
      amount_lamports: 1000
    })
  });

  await expectStatus(`/api/signing-requests/${signingRequest.request_id}/broadcast`, 409, {
    method: "POST"
  });
  await triggerSigning(signingRequest.request_id, "node-a", 1);
  await triggerSigning(signingRequest.request_id, "node-b", 1);
  const nodeARound2 = await triggerSigning(signingRequest.request_id, "node-a", 2);
  const nodeBRound2 = await triggerSigning(signingRequest.request_id, "node-b", 2);

  if (!nodeARound2.public_payload?.child_verifying_share_hex || !nodeBRound2.public_payload?.signature_share_hex) {
    throw new Error(`signing rounds did not return public child verifying material: ${JSON.stringify([nodeARound2, nodeBRound2])}`);
  }

  const ready = await expectOk(`/api/signing-requests/${signingRequest.request_id}`);

  if (ready.status !== "READY_TO_AGGREGATE" || !ready.message_hash_hex || !ready.recent_blockhash) {
    throw new Error(`request is not ready for broadcast: ${JSON.stringify(ready)}`);
  }

  const broadcasted = await expectOk(`/api/signing-requests/${signingRequest.request_id}/broadcast`, {
    method: "POST"
  });

  if (broadcasted.status !== "BROADCASTED" || !broadcasted.transaction_signature || !broadcasted.explorer_url) {
    throw new Error(`broadcast did not store transaction metadata: ${JSON.stringify(broadcasted)}`);
  }

  await expectStatus(`/api/signing-requests/${signingRequest.request_id}/nodes/node-a/rounds/1`, 409, {
    method: "POST"
  });

  const confirmed = await expectOk(`/api/signing-requests/${signingRequest.request_id}/confirm`, {
    method: "POST"
  });

  if (confirmed.status !== "CONFIRMED") {
    throw new Error(`confirmation did not advance request: ${JSON.stringify(confirmed)}`);
  }

  assertNoForbiddenFields(ready);
  assertNoForbiddenFields(broadcasted);
  assertNoForbiddenFields(confirmed);
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
'

  phase_six_compose exec -T frontend node -e '
async function fetchWithRetry(url, attempts = 120) {
  let lastError;

  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    try {
      const response = await fetch(url);
      const text = await response.text();

      if (response.ok) {
        return { response, text };
      }

      lastError = new Error(`${url} returned HTTP ${response.status}: ${text}`);
    } catch (error) {
      lastError = error;
    }

    await new Promise((resolve) => setTimeout(resolve, 1000));
  }

  throw lastError;
}

async function main() {
  let html = "";
  let cssBodies = [];

  for (let attempt = 1; attempt <= 120; attempt += 1) {
    html = (await fetchWithRetry("http://localhost:3000/", 1)).text;

    if (!html.includes("FROST Template")) {
      await new Promise((resolve) => setTimeout(resolve, 1000));
      continue;
    }

    const stylesheetPaths = Array.from(html.matchAll(/href="([^"]+\.css)"/g), (match) => match[1]);

    if (stylesheetPaths.length === 0) {
      await new Promise((resolve) => setTimeout(resolve, 1000));
      continue;
    }

    cssBodies = await Promise.all(
      stylesheetPaths.map(async (path) => {
        return (await fetchWithRetry(new URL(path, "http://localhost:3000/").toString(), 1)).text;
      })
    );

    if (cssBodies.some((css) => /\.broadcast-actions\s*\{[^}]*display:\s*flex/.test(css))) {
      return;
    }

    await new Promise((resolve) => setTimeout(resolve, 1000));
  }

  if (!html.includes("FROST Template")) {
    throw new Error("frontend did not render the FROST Template page");
  }

  throw new Error("frontend stylesheet did not include broadcast action styles");
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
  '
}

check_required_text() {
  local file="$1"
  local text="$2"

  if ! grep -Fq "$text" "$file"; then
    echo "${file} is missing required text: ${text}"
    exit 1
  fi
}

check_phase_seven_docs() {
  test -f features/reviewer-experience.feature
  test -f docs/contracts/reviewer-experience.md
  test -f docs/ai-native/logs/phase-7-agent-run-report.md

  check_required_text README.md "Manual Acceptance Checklist"
  check_required_text README.md 'Wallet addresses are visible in the `Wallet Derivation` section.'
  check_required_text README.md "https://faucet.solana.com/"
  check_required_text README.md "solana airdrop 0.5 <WALLET_0_ADDRESS> --url devnet"
  check_required_text README.md "Open Explorer"
  check_required_text README.md "AI Collaboration Evidence"
  check_required_text README.md "CI And Versioning"
  check_required_text README.md "Troubleshooting"
  check_required_text README.md "Out Of Scope"
  check_required_text README.md "./scripts/verify-phase.sh 7"

  check_required_text .env.example "SOLANA_RPC_URL="
  check_required_text .env.example "NEXT_PUBLIC_COORDINATOR_URL="

  check_required_text features/reviewer-experience.feature "Feature: Reviewer experience and handoff"
  check_required_text docs/contracts/reviewer-experience.md "Manual Devnet transfer success must not be claimed unless a funded derived wallet was actually used."
  check_required_text docs/ai-native/05-verification-harness.md "Phase 7: Reviewer Experience And Hardening"
  check_required_text docs/ai-native/logs/ai-collaboration-log.md "Phase 7 Reviewer Experience And Hardening"
  check_required_text docs/ai-native/logs/decision-log.md "Make Reviewer Handoff A First-Class Contract"
}

check_phase_seven_stack() {
  docker compose config >/dev/null
  docker compose run --rm --no-deps coordinator cargo test --workspace
  npm --prefix frontend run lint
  npm --prefix frontend run build
  check_phase_seven_docs
}

check_phase_eight_docs() {
  test -f features/crypto-dashboard-ux.feature
  test -f docs/contracts/crypto-dashboard-ux.md
  test -f docs/ai-native/prompts/08-crypto-dashboard-ux.md
  test -f docs/ai-native/logs/phase-8-agent-run-report.md

  check_required_text frontend/app/page.tsx "MPC Wallet Dashboard"
  check_required_text frontend/app/page.tsx "Key Ceremony"
  check_required_text frontend/app/page.tsx "Derived Vaults"
  check_required_text frontend/app/page.tsx "Transfer Tickets"
  check_required_text frontend/app/page.tsx "Threshold Signing"
  check_required_text frontend/app/page.tsx "Transaction Receipt"
  check_required_text frontend/app/page.tsx "Solana Devnet"
  check_required_text frontend/app/page.tsx "2-of-2 MPC"
  check_required_text frontend/app/page.tsx "Vault Watch"
  check_required_text frontend/app/page.tsx "vault-watch-panel"
  check_required_text frontend/app/page.tsx "terminal-layout"
  check_required_text frontend/app/layout.tsx "FROST MPC Wallet"
  check_required_text frontend/app/globals.css "workflow-step-active"
  check_required_text frontend/app/globals.css "terminal-layout"
  check_required_text frontend/app/globals.css "grid-auto-flow: column"
  check_required_text frontend/app/globals.css "prefers-reduced-motion"

  check_required_text features/crypto-dashboard-ux.feature "Feature: Crypto dashboard UX"
  check_required_text docs/contracts/crypto-dashboard-ux.md "Phase 8 changes the reviewer-facing presentation only."
  check_required_text docs/ai-native/prompts/08-crypto-dashboard-ux.md "Phase 8 Crypto Dashboard UX"
  check_required_text docs/ai-native/05-verification-harness.md "Phase 8: Crypto Dashboard UX"
  check_required_text docs/ai-native/logs/ai-collaboration-log.md "Phase 8 Crypto Dashboard UX"
  check_required_text docs/ai-native/logs/decision-log.md "Reframe UX Without Changing Protocol Semantics"
}

check_phase_eight_stack() {
  check_phase_seven_stack
  check_phase_eight_docs
}

case "$phase" in
  0)
    check_no_sensitive_patterns
    git diff --check
    check_phase_zero_files
    check_release_metadata
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
  3)
    check_no_sensitive_patterns
    git diff --check
    check_phase_three_stack
    ;;
  4)
    check_no_sensitive_patterns
    git diff --check
    check_phase_four_stack
    ;;
  5)
    check_no_sensitive_patterns
    git diff --check
    check_phase_five_stack
    ;;
  6)
    trap cleanup_phase_six_stack EXIT
    cleanup_phase_six_stack
    check_no_sensitive_patterns
    git diff --check
    check_phase_six_stack
    cleanup_phase_six_stack
    trap - EXIT
    ;;
  7)
    check_no_sensitive_patterns
    git diff --check
    check_phase_seven_stack
    ;;
  8)
    check_no_sensitive_patterns
    git diff --check
    check_phase_eight_stack
    ;;
  *)
    echo "No verification harness is defined for phase ${phase} yet."
    exit 2
    ;;
esac

echo "Phase ${phase} verification passed."

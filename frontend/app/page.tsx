"use client";

import { useEffect, useMemo, useState } from "react";

type NodeId = "node-a" | "node-b";
type Round = 1 | 2 | 3;

type DkgStep = {
  node_id: NodeId;
  round: Round;
  status: string;
};

type DkgSession = {
  session_id: string;
  status: string;
  master_public_key_base58: string | null;
  node_steps: DkgStep[];
};

type TriggerRoundResponse = {
  session_id: string;
  node_id: NodeId;
  round: Round;
  status: string;
  dkg_status: string;
  public_payload: Record<string, unknown> | null;
};

type ActionEntry = {
  label: string;
  status: "idle" | "ok" | "error";
  message: string;
};

const nodes: Array<{ id: NodeId; label: string }> = [
  { id: "node-a", label: "Node A" },
  { id: "node-b", label: "Node B" },
];

const rounds: Round[] = [1, 2, 3];

export default function Home() {
  const [session, setSession] = useState<DkgSession | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [pendingAction, setPendingAction] = useState<string | null>(null);
  const [lastAction, setLastAction] = useState<ActionEntry>({
    label: "Ready",
    status: "idle",
    message: "Create or load a DKG session to begin.",
  });

  useEffect(() => {
    void loadActiveSession();
  }, []);

  const completedSteps = useMemo(() => {
    return (
      session?.node_steps.filter((step) => step.status === "COMPLETED").length ??
      0
    );
  }, [session]);

  async function loadActiveSession() {
    setIsLoading(true);

    try {
      const response = await fetch("/api/coordinator/api/dkg/sessions/active", {
        cache: "no-store",
      });

      if (response.status === 404) {
        setSession(null);
        setLastAction({
          label: "No active session",
          status: "idle",
          message: "Create a DKG session when you are ready to drive Round 1.",
        });
        return;
      }

      setSession(await readJson<DkgSession>(response));
    } catch (error) {
      setLastAction({
        label: "Load failed",
        status: "error",
        message: errorMessage(error),
      });
    } finally {
      setIsLoading(false);
    }
  }

  async function createSession() {
    setPendingAction("create-session");

    try {
      const response = await fetch("/api/coordinator/api/dkg/sessions", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          threshold: 2,
          participants: ["node-a", "node-b"],
        }),
      });
      const nextSession = await readJson<DkgSession>(response);

      setSession(nextSession);
      setLastAction({
        label: "Session ready",
        status: "ok",
        message: `Session ${shortId(nextSession.session_id)} is ${nextSession.status}.`,
      });
    } catch (error) {
      setLastAction({
        label: "Create failed",
        status: "error",
        message: errorMessage(error),
      });
    } finally {
      setPendingAction(null);
    }
  }

  async function triggerRound(nodeId: NodeId, round: Round) {
    if (!session) {
      return;
    }

    const actionKey = `${nodeId}-${round}`;
    setPendingAction(actionKey);

    try {
      const response = await fetch(
        `/api/coordinator/api/dkg/sessions/${session.session_id}/nodes/${nodeId}/rounds/${round}`,
        { method: "POST" },
      );
      const result = await readJson<TriggerRoundResponse>(response);

      await loadActiveSession();
      setLastAction({
        label: `${nodeLabel(nodeId)} Round ${round}`,
        status: "ok",
        message: `${result.status}; DKG is now ${result.dkg_status}.`,
      });
    } catch (error) {
      await loadActiveSession();
      setLastAction({
        label: `${nodeLabel(nodeId)} Round ${round}`,
        status: "error",
        message: errorMessage(error),
      });
    } finally {
      setPendingAction(null);
    }
  }

  return (
    <main className="page-shell">
      <section className="top-band" aria-labelledby="page-title">
        <div>
          <p className="eyebrow">FROST Template</p>
          <h1 id="page-title">DKG Control Surface</h1>
          <p className="intro">
            Drive each 2-of-2 DKG round independently and watch coordinator
            state advance without exposing private node material.
          </p>
        </div>
        <div className="session-actions">
          <button
            className="primary-button"
            disabled={pendingAction === "create-session"}
            onClick={createSession}
            type="button"
          >
            {pendingAction === "create-session" ? "Creating..." : "Create Session"}
          </button>
          <button
            className="secondary-button"
            disabled={isLoading}
            onClick={loadActiveSession}
            type="button"
          >
            {isLoading ? "Refreshing..." : "Refresh"}
          </button>
        </div>
      </section>

      <section className="metric-strip" aria-label="DKG session summary">
        <Metric label="Session" value={session ? shortId(session.session_id) : "None"} />
        <Metric label="Status" value={session?.status ?? "NOT_CREATED"} />
        <Metric label="Steps" value={`${completedSteps}/6 completed`} />
        <Metric
          label="Master Key"
          value={session?.master_public_key_base58 ?? "Pending"}
        />
      </section>

      <section className="workflow-layout">
        <div className="control-panel">
          <div className="section-heading">
            <div>
              <p className="eyebrow">Manual Protocol Driver</p>
              <h2>Node Round Controls</h2>
            </div>
            <span className={statusClass(session?.status ?? "NOT_CREATED")}>
              {session?.status ?? "NOT_CREATED"}
            </span>
          </div>

          <div className="round-grid" role="list">
            {nodes.map((node) =>
              rounds.map((round) => {
                const step = findStep(session, node.id, round);
                const actionKey = `${node.id}-${round}`;
                const isPending = pendingAction === actionKey;
                const isReplay = step.status === "COMPLETED";

                return (
                  <article
                    className="round-cell"
                    key={actionKey}
                    role="listitem"
                  >
                    <div>
                      <p className="round-node">{node.label}</p>
                      <h3>Round {round}</h3>
                    </div>
                    <span className={statusClass(step.status)}>{step.status}</span>
                    <button
                      className="round-button"
                      disabled={!session || pendingAction !== null}
                      onClick={() => void triggerRound(node.id, round)}
                      type="button"
                    >
                      {isPending ? "Running..." : isReplay ? "Replay" : "Run"}
                    </button>
                  </article>
                );
              }),
            )}
          </div>
        </div>

        <aside className="state-panel" aria-label="DKG state detail">
          <div>
            <p className="eyebrow">Coordinator State</p>
            <h2>Latest Result</h2>
          </div>
          <div className={actionClass(lastAction.status)}>
            <strong>{lastAction.label}</strong>
            <p>{lastAction.message}</p>
          </div>
          <div className="boundary-list">
            <h3>Protocol Boundary</h3>
            <p>Browser calls Coordinator only.</p>
            <p>TSS nodes return public DKG payloads only.</p>
            <p>Private shares stay node-local for Phase 3 crypto integration.</p>
          </div>
        </aside>
      </section>
    </main>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="metric">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function findStep(
  session: DkgSession | null,
  nodeId: NodeId,
  round: Round,
): DkgStep {
  return (
    session?.node_steps.find(
      (step) => step.node_id === nodeId && step.round === round,
    ) ?? {
      node_id: nodeId,
      round,
      status: "NOT_STARTED",
    }
  );
}

async function readJson<T>(response: Response): Promise<T> {
  const text = await response.text();
  const payload: unknown = text ? JSON.parse(text) : null;

  if (!response.ok) {
    if (hasErrorMessage(payload)) {
      throw new Error(payload.error);
    }

    throw new Error(`Request failed with HTTP ${response.status}.`);
  }

  return payload as T;
}

function hasErrorMessage(payload: unknown): payload is { error: string } {
  return (
    typeof payload === "object" &&
    payload !== null &&
    "error" in payload &&
    typeof (payload as { error?: unknown }).error === "string"
  );
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : "Unknown error";
}

function shortId(value: string): string {
  return value.slice(0, 8);
}

function nodeLabel(nodeId: NodeId): string {
  return nodeId === "node-a" ? "Node A" : "Node B";
}

function statusClass(status: string): string {
  if (status === "COMPLETED") {
    return "status-pill status-completed";
  }

  if (status.includes("IN_PROGRESS") || status === "RUNNING") {
    return "status-pill status-running";
  }

  if (status === "FAILED") {
    return "status-pill status-failed";
  }

  return "status-pill status-idle";
}

function actionClass(status: ActionEntry["status"]): string {
  return `action-result action-${status}`;
}

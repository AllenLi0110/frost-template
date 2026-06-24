"use client";

import { useEffect, useMemo, useState } from "react";

type NodeId = "node-a" | "node-b";
type Round = 1 | 2 | 3;
type SigningRound = 1 | 2;

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

type Wallet = {
  wallet_index: number;
  dkg_session_id: string;
  derivation_path: string;
  public_key_base58: string;
  address_base58: string;
  balance_lamports: number | null;
  balance_status: string;
  balance_error_message: string | null;
  balance_checked_at: string | null;
  created_at: string;
};

type WalletListResponse = {
  wallets: Wallet[];
};

type WalletBalanceResponse = {
  wallet_index: number;
  address_base58: string;
  balance_lamports: number | null;
  balance_status: string;
  balance_error_message: string | null;
};

type SigningStep = {
  node_id: NodeId;
  round: SigningRound;
  status: string;
};

type SigningRequest = {
  request_id: string;
  wallet_index: number;
  sender_address_base58: string;
  recipient_address_base58: string;
  amount_lamports: number;
  status: string;
  message_hash_hex: string | null;
  recent_blockhash: string | null;
  transaction_signature: string | null;
  explorer_url: string | null;
  error_message: string | null;
  created_at: string;
  updated_at: string;
  node_steps: SigningStep[];
};

type SigningRequestListResponse = {
  requests: SigningRequest[];
};

type TriggerSigningRoundResponse = {
  request_id: string;
  node_id: NodeId;
  round: SigningRound;
  status: string;
  signing_status: string;
  public_payload: Record<string, unknown> | null;
};

const nodes: Array<{ id: NodeId; label: string }> = [
  { id: "node-a", label: "Node A" },
  { id: "node-b", label: "Node B" },
];

const rounds: Round[] = [1, 2, 3];
const signingRounds: SigningRound[] = [1, 2];
const defaultRecipientAddress = "11111111111111111111111111111111";

export default function Home() {
  const [session, setSession] = useState<DkgSession | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [pendingAction, setPendingAction] = useState<string | null>(null);
  const [wallets, setWallets] = useState<Wallet[]>([]);
  const [isWalletLoading, setIsWalletLoading] = useState(true);
  const [pendingWalletAction, setPendingWalletAction] = useState<string | null>(
    null,
  );
  const [selectedSenderIndex, setSelectedSenderIndex] = useState<number | null>(
    null,
  );
  const [signingRequests, setSigningRequests] = useState<SigningRequest[]>([]);
  const [isSigningLoading, setIsSigningLoading] = useState(true);
  const [pendingSigningAction, setPendingSigningAction] = useState<
    string | null
  >(null);
  const [selectedSigningRequestId, setSelectedSigningRequestId] = useState<
    string | null
  >(null);
  const [transferForm, setTransferForm] = useState({
    recipientAddress: defaultRecipientAddress,
    amountLamports: "1000",
  });
  const [lastAction, setLastAction] = useState<ActionEntry>({
    label: "Ready",
    status: "idle",
    message: "Create or load a DKG session to begin.",
  });

  useEffect(() => {
    void loadActiveSession();
    void loadWallets();
    void loadSigningRequests();
  }, []);

  const completedSteps = useMemo(() => {
    return (
      session?.node_steps.filter((step) => step.status === "COMPLETED").length ??
      0
    );
  }, [session]);

  const selectedSigningRequest = useMemo(() => {
    return (
      signingRequests.find(
        (request) => request.request_id === selectedSigningRequestId,
      ) ??
      signingRequests[0] ??
      null
    );
  }, [selectedSigningRequestId, signingRequests]);

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

  async function loadWallets() {
    setIsWalletLoading(true);

    try {
      const response = await fetch("/api/coordinator/api/wallets", {
        cache: "no-store",
      });
      const payload = await readJson<WalletListResponse>(response);

      setWallets(payload.wallets);
    } catch (error) {
      setLastAction({
        label: "Wallet load failed",
        status: "error",
        message: errorMessage(error),
      });
    } finally {
      setIsWalletLoading(false);
    }
  }

  async function loadSigningRequests() {
    setIsSigningLoading(true);

    try {
      const response = await fetch("/api/coordinator/api/signing-requests", {
        cache: "no-store",
      });
      const payload = await readJson<SigningRequestListResponse>(response);

      setSigningRequests(payload.requests);
      setSelectedSigningRequestId((currentId) => {
        if (
          currentId &&
          payload.requests.some((request) => request.request_id === currentId)
        ) {
          return currentId;
        }

        return payload.requests[0]?.request_id ?? null;
      });
    } catch (error) {
      setLastAction({
        label: "Signing load failed",
        status: "error",
        message: errorMessage(error),
      });
    } finally {
      setIsSigningLoading(false);
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
      await loadWallets();
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
      await loadWallets();
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

  async function createWallet() {
    setPendingWalletAction("create-wallet");

    try {
      const response = await fetch("/api/coordinator/api/wallets", {
        method: "POST",
      });
      const wallet = await readJson<Wallet>(response);

      await loadWallets();
      setLastAction({
        label: "Wallet created",
        status: "ok",
        message: `Wallet ${wallet.wallet_index} is ready at ${shortAddress(
          wallet.address_base58,
        )}.`,
      });
    } catch (error) {
      setLastAction({
        label: "Wallet create failed",
        status: "error",
        message: errorMessage(error),
      });
    } finally {
      setPendingWalletAction(null);
    }
  }

  async function refreshBalance(walletIndex: number) {
    const actionKey = `balance-${walletIndex}`;
    setPendingWalletAction(actionKey);

    try {
      const response = await fetch(
        `/api/coordinator/api/wallets/${walletIndex}/balance`,
        { cache: "no-store" },
      );
      const balance = await readJson<WalletBalanceResponse>(response);

      setWallets((currentWallets) =>
        currentWallets.map((wallet) =>
          wallet.wallet_index === walletIndex
            ? {
                ...wallet,
                balance_lamports: balance.balance_lamports,
                balance_status: balance.balance_status,
                balance_error_message: balance.balance_error_message,
              }
            : wallet,
        ),
      );
      setLastAction({
        label: `Wallet ${walletIndex} balance`,
        status: balance.balance_status === "AVAILABLE" ? "ok" : "error",
        message:
          balance.balance_status === "AVAILABLE"
            ? formatBalance(balance.balance_lamports)
            : balance.balance_error_message ?? "Balance unavailable.",
      });
    } catch (error) {
      setLastAction({
        label: `Wallet ${walletIndex} balance`,
        status: "error",
        message: errorMessage(error),
      });
    } finally {
      setPendingWalletAction(null);
    }
  }

  function selectSender(walletIndex: number) {
    setSelectedSenderIndex(walletIndex);
    setLastAction({
      label: "Sender selected",
      status: "ok",
      message: `Wallet ${walletIndex} is selected for signing requests.`,
    });
  }

  async function createSigningRequest() {
    if (selectedSenderIndex === null) {
      setLastAction({
        label: "Signing request",
        status: "error",
        message: "Select a sender wallet first.",
      });
      return;
    }

    const amountLamports = Number(transferForm.amountLamports);

    if (!Number.isInteger(amountLamports) || amountLamports <= 0) {
      setLastAction({
        label: "Signing request",
        status: "error",
        message: "Amount must be a positive lamport integer.",
      });
      return;
    }

    setPendingSigningAction("create-signing-request");

    try {
      const response = await fetch("/api/coordinator/api/signing-requests", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          wallet_index: selectedSenderIndex,
          recipient_address_base58: transferForm.recipientAddress.trim(),
          amount_lamports: amountLamports,
        }),
      });
      const signingRequest = await readJson<SigningRequest>(response);

      await loadSigningRequests();
      setSelectedSigningRequestId(signingRequest.request_id);
      setLastAction({
        label: "Signing request created",
        status: "ok",
        message: `Request ${shortId(signingRequest.request_id)} is ${signingRequest.status}.`,
      });
    } catch (error) {
      setLastAction({
        label: "Signing request failed",
        status: "error",
        message: errorMessage(error),
      });
    } finally {
      setPendingSigningAction(null);
    }
  }

  async function triggerSigningRound(nodeId: NodeId, round: SigningRound) {
    if (!selectedSigningRequest) {
      return;
    }

    const actionKey = `signing-${selectedSigningRequest.request_id}-${nodeId}-${round}`;
    setPendingSigningAction(actionKey);

    try {
      const response = await fetch(
        `/api/coordinator/api/signing-requests/${selectedSigningRequest.request_id}/nodes/${nodeId}/rounds/${round}`,
        { method: "POST" },
      );
      const result = await readJson<TriggerSigningRoundResponse>(response);

      await loadSigningRequests();
      setSelectedSigningRequestId(result.request_id);
      setLastAction({
        label: `${nodeLabel(nodeId)} Signing Round ${round}`,
        status: "ok",
        message: `${result.status}; request is now ${result.signing_status}.`,
      });
    } catch (error) {
      await loadSigningRequests();
      setLastAction({
        label: `${nodeLabel(nodeId)} Signing Round ${round}`,
        status: "error",
        message: errorMessage(error),
      });
    } finally {
      setPendingSigningAction(null);
    }
  }

  return (
    <main className="page-shell">
      <section className="top-band" aria-labelledby="page-title">
        <div>
          <p className="eyebrow">FROST Template</p>
          <h1 id="page-title">DKG Control Surface</h1>
          <p className="intro">
            Drive DKG, derive wallets, and manually advance signing requests
            without exposing private node material.
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

      <section className="wallet-panel" aria-labelledby="wallet-title">
        <div className="section-heading">
          <div>
            <p className="eyebrow">Phase 4</p>
            <h2 id="wallet-title">Wallet Derivation</h2>
          </div>
          <div className="wallet-actions">
            <button
              className="primary-button"
              disabled={
                session?.status !== "COMPLETED" || pendingWalletAction !== null
              }
              onClick={() => void createWallet()}
              type="button"
            >
              {pendingWalletAction === "create-wallet"
                ? "Creating..."
                : "Create Wallet"}
            </button>
            <button
              className="secondary-button"
              disabled={isWalletLoading}
              onClick={() => void loadWallets()}
              type="button"
            >
              {isWalletLoading ? "Refreshing..." : "Refresh"}
            </button>
          </div>
        </div>

        {wallets.length === 0 ? (
          <div className="empty-wallet-state">
            <strong>No wallets</strong>
            <p>
              {session?.status === "COMPLETED"
                ? "Create the first derived wallet from the completed DKG."
                : "Complete DKG before deriving wallets."}
            </p>
          </div>
        ) : (
          <div className="wallet-list" role="list">
            {wallets.map((wallet) => {
              const balanceActionKey = `balance-${wallet.wallet_index}`;
              const isSelected = selectedSenderIndex === wallet.wallet_index;

              return (
                <article className="wallet-row" key={wallet.wallet_index} role="listitem">
                  <div className="wallet-index">
                    <span>Index</span>
                    <strong>{wallet.wallet_index}</strong>
                  </div>
                  <div className="wallet-address">
                    <span>{wallet.derivation_path}</span>
                    <strong>{wallet.address_base58}</strong>
                  </div>
                  <div className="wallet-balance">
                    <span className={statusClass(wallet.balance_status)}>
                      {wallet.balance_status}
                    </span>
                    <strong>{formatBalance(wallet.balance_lamports)}</strong>
                    {wallet.balance_error_message ? (
                      <p>{wallet.balance_error_message}</p>
                    ) : null}
                  </div>
                  <div className="wallet-row-actions">
                    <button
                      className="round-button"
                      disabled={pendingWalletAction !== null}
                      onClick={() => void refreshBalance(wallet.wallet_index)}
                      type="button"
                    >
                      {pendingWalletAction === balanceActionKey
                        ? "Checking..."
                        : "Balance"}
                    </button>
                    <button
                      className={isSelected ? "primary-button" : "secondary-button"}
                      disabled={pendingWalletAction !== null}
                      onClick={() => selectSender(wallet.wallet_index)}
                      type="button"
                    >
                      {isSelected ? "Selected" : "Select"}
                    </button>
                  </div>
                </article>
              );
            })}
          </div>
        )}
      </section>

      <section className="signing-panel" aria-labelledby="signing-title">
        <div className="section-heading">
          <div>
            <p className="eyebrow">Phase 5</p>
            <h2 id="signing-title">Signing Requests</h2>
          </div>
          <button
            className="secondary-button"
            disabled={isSigningLoading}
            onClick={() => void loadSigningRequests()}
            type="button"
          >
            {isSigningLoading ? "Refreshing..." : "Refresh"}
          </button>
        </div>

        <div className="transfer-form" aria-label="Create transfer intent">
          <label>
            <span>Sender</span>
            <select
              disabled={wallets.length === 0 || pendingSigningAction !== null}
              onChange={(event) => {
                if (event.target.value) {
                  selectSender(Number(event.target.value));
                }
              }}
              value={selectedSenderIndex ?? ""}
            >
              <option value="">Select wallet</option>
              {wallets.map((wallet) => (
                <option key={wallet.wallet_index} value={wallet.wallet_index}>
                  Wallet {wallet.wallet_index} - {shortAddress(wallet.address_base58)}
                </option>
              ))}
            </select>
          </label>
          <label>
            <span>Recipient</span>
            <input
              onChange={(event) =>
                setTransferForm((current) => ({
                  ...current,
                  recipientAddress: event.target.value,
                }))
              }
              value={transferForm.recipientAddress}
            />
          </label>
          <label>
            <span>Lamports</span>
            <input
              inputMode="numeric"
              min="1"
              onChange={(event) =>
                setTransferForm((current) => ({
                  ...current,
                  amountLamports: event.target.value,
                }))
              }
              type="number"
              value={transferForm.amountLamports}
            />
          </label>
          <button
            className="primary-button"
            disabled={
              selectedSenderIndex === null || pendingSigningAction !== null
            }
            onClick={() => void createSigningRequest()}
            type="button"
          >
            {pendingSigningAction === "create-signing-request"
              ? "Creating..."
              : "Create Request"}
          </button>
        </div>

        <div className="signing-layout">
          <div className="request-list" role="list">
            {signingRequests.length === 0 ? (
              <div className="empty-wallet-state">
                <strong>No signing requests</strong>
                <p>Create a transfer intent after selecting a sender wallet.</p>
              </div>
            ) : (
              signingRequests.map((request) => {
                const isSelected =
                  selectedSigningRequest?.request_id === request.request_id;

                return (
                  <button
                    className={
                      isSelected ? "request-row request-row-selected" : "request-row"
                    }
                    key={request.request_id}
                    onClick={() => setSelectedSigningRequestId(request.request_id)}
                    role="listitem"
                    type="button"
                  >
                    <span>
                      <strong>{shortId(request.request_id)}</strong>
                      <small>Wallet {request.wallet_index}</small>
                    </span>
                    <span className={statusClass(request.status)}>
                      {request.status}
                    </span>
                    <span className="request-amount">
                      {formatLamports(request.amount_lamports)}
                    </span>
                  </button>
                );
              })
            )}
          </div>

          <div className="signing-controls">
            <div className="selected-request-summary">
              <div>
                <p className="eyebrow">Selected Request</p>
                <h3>
                  {selectedSigningRequest
                    ? shortId(selectedSigningRequest.request_id)
                    : "None"}
                </h3>
              </div>
              <span
                className={statusClass(
                  selectedSigningRequest?.status ?? "NOT_CREATED",
                )}
              >
                {selectedSigningRequest?.status ?? "NOT_CREATED"}
              </span>
            </div>

            {selectedSigningRequest ? (
              <>
                <dl className="request-facts">
                  <div>
                    <dt>Sender</dt>
                    <dd>{shortAddress(selectedSigningRequest.sender_address_base58)}</dd>
                  </div>
                  <div>
                    <dt>Recipient</dt>
                    <dd>
                      {shortAddress(
                        selectedSigningRequest.recipient_address_base58,
                      )}
                    </dd>
                  </div>
                  <div>
                    <dt>Message</dt>
                    <dd>
                      {selectedSigningRequest.message_hash_hex
                        ? shortId(selectedSigningRequest.message_hash_hex)
                        : "Pending"}
                    </dd>
                  </div>
                </dl>

                <div className="signing-round-grid" role="list">
                  {nodes.map((node) =>
                    signingRounds.map((round) => {
                      const step = findSigningStep(
                        selectedSigningRequest,
                        node.id,
                        round,
                      );
                      const actionKey = `signing-${selectedSigningRequest.request_id}-${node.id}-${round}`;
                      const isPending = pendingSigningAction === actionKey;
                      const isRoundTwoReplay =
                        round === 2 && step.status === "COMPLETED";

                      return (
                        <article
                          className="round-cell signing-round-cell"
                          key={actionKey}
                          role="listitem"
                        >
                          <div>
                            <p className="round-node">{node.label}</p>
                            <h3>Signing Round {round}</h3>
                          </div>
                          <span className={statusClass(step.status)}>
                            {step.status}
                          </span>
                          <button
                            className="round-button"
                            disabled={
                              pendingSigningAction !== null || isRoundTwoReplay
                            }
                            onClick={() =>
                              void triggerSigningRound(node.id, round)
                            }
                            type="button"
                          >
                            {isPending
                              ? "Running..."
                              : step.status === "COMPLETED"
                                ? round === 1
                                  ? "Replay"
                                  : "Consumed"
                                : "Run"}
                          </button>
                        </article>
                      );
                    }),
                  )}
                </div>
              </>
            ) : (
              <div className="empty-wallet-state">
                <strong>No request selected</strong>
                <p>Create or select a request to trigger signing rounds.</p>
              </div>
            )}
          </div>
        </div>
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

function findSigningStep(
  request: SigningRequest,
  nodeId: NodeId,
  round: SigningRound,
): SigningStep {
  return (
    request.node_steps.find(
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

function shortAddress(value: string): string {
  return `${value.slice(0, 4)}...${value.slice(-4)}`;
}

function formatBalance(lamports: number | null): string {
  if (lamports === null) {
    return "Not checked";
  }

  return `${lamports} lamports (${(lamports / 1_000_000_000).toFixed(9)} SOL)`;
}

function formatLamports(lamports: number): string {
  return `${lamports} lamports`;
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

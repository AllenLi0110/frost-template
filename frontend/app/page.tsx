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
  dkg_session_id: string;
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
const defaultRecipientAddress = "";
const workflowSteps = [
  {
    label: "Key Ceremony",
    detail: "Create the 2-of-2 root key",
  },
  {
    label: "Vault Funding",
    detail: "Fund a Devnet sender vault",
  },
  {
    label: "Transfer Intent",
    detail: "Prepare recipient and lamports",
  },
  {
    label: "Threshold Signing",
    detail: "Collect signer commitments",
  },
  {
    label: "Broadcast",
    detail: "Send and verify the receipt",
  },
];

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
  const [selectedWorkflowIndex, setSelectedWorkflowIndex] = useState<
    number | null
  >(null);
  const [isVaultFundingConfirmed, setIsVaultFundingConfirmed] =
    useState(false);
  const [isThresholdSigningConfirmed, setIsThresholdSigningConfirmed] =
    useState(false);
  const [lastAction, setLastAction] = useState<ActionEntry>({
    label: "Ready",
    status: "idle",
    message: "Start or load a key ceremony to begin the MPC wallet workflow.",
  });

  useEffect(() => {
    void loadActiveSession();
    void loadWallets();
    void loadSigningRequests();
  }, []);

  const selectedSigningRequest = useMemo(() => {
    return (
      signingRequests.find(
        (request) => request.request_id === selectedSigningRequestId,
      ) ??
      signingRequests[0] ??
      null
    );
  }, [selectedSigningRequestId, signingRequests]);

  const completedSigningSteps = useMemo(() => {
    return (
      selectedSigningRequest?.node_steps.filter(
        (step) => step.status === "COMPLETED",
      ).length ?? 0
    );
  }, [selectedSigningRequest]);

  const copyableWallet = useMemo(() => {
    return (
      wallets.find((wallet) => wallet.wallet_index === selectedSenderIndex) ??
      wallets[0] ??
      null
    );
  }, [selectedSenderIndex, wallets]);

  const hasFundedVault = useMemo(() => {
    return wallets.some((wallet) => wallet.balance_status === "AVAILABLE");
  }, [wallets]);

  const checkedVaultCount = useMemo(() => {
    return wallets.filter((wallet) => wallet.balance_status !== "UNKNOWN")
      .length;
  }, [wallets]);

  const isVaultFundingReady = useMemo(() => {
    return wallets.length >= 2 && checkedVaultCount >= 2 && hasFundedVault;
  }, [checkedVaultCount, hasFundedVault, wallets.length]);

  const isThresholdSigningReady = useMemo(() => {
    return (
      selectedSigningRequest?.status === "READY_TO_AGGREGATE" &&
      completedSigningSteps === 4
    );
  }, [completedSigningSteps, selectedSigningRequest?.status]);

  const hasEnteredBroadcast = useMemo(() => {
    return (
      selectedSigningRequest?.status === "BROADCASTED" ||
      selectedSigningRequest?.status === "CONFIRMED"
    );
  }, [selectedSigningRequest?.status]);

  const activeWorkflowIndex = useMemo(() => {
    if (session?.status !== "COMPLETED") {
      return 0;
    }

    if (!isVaultFundingConfirmed && !selectedSigningRequest) {
      return 1;
    }

    if (!selectedSigningRequest) {
      return 2;
    }

    if (!isThresholdSigningConfirmed && !hasEnteredBroadcast) {
      return 3;
    }

    return 4;
  }, [
    hasEnteredBroadcast,
    isThresholdSigningConfirmed,
    isVaultFundingConfirmed,
    selectedSigningRequest,
    session?.status,
  ]);

  const workflowStepStates = useMemo(() => {
    return workflowSteps.map((_, index) => {
      const isComplete =
        (index === 0 && session?.status === "COMPLETED") ||
        (index === 1 &&
          (isVaultFundingConfirmed || selectedSigningRequest !== null)) ||
        (index === 2 && selectedSigningRequest !== null) ||
        (index === 3 &&
          selectedSigningRequest !== null &&
          (isThresholdSigningConfirmed || hasEnteredBroadcast)) ||
        (index === 4 && selectedSigningRequest?.status === "CONFIRMED");

      if (isComplete) {
        return "complete";
      }

      return index === activeWorkflowIndex ? "active" : "idle";
    });
  }, [
    activeWorkflowIndex,
    hasEnteredBroadcast,
    isVaultFundingConfirmed,
    isThresholdSigningConfirmed,
    selectedSigningRequest,
    session,
  ]);

  useEffect(() => {
    setSelectedWorkflowIndex(activeWorkflowIndex);
  }, [activeWorkflowIndex]);

  useEffect(() => {
    if (session?.status !== "COMPLETED") {
      setIsVaultFundingConfirmed(false);
    }
  }, [session?.status]);

  useEffect(() => {
    if (!isVaultFundingReady && !selectedSigningRequest) {
      setIsVaultFundingConfirmed(false);
    }
  }, [isVaultFundingReady, selectedSigningRequest]);

  useEffect(() => {
    setIsThresholdSigningConfirmed(false);
  }, [selectedSigningRequestId]);

  const visibleWorkflowIndex = selectedWorkflowIndex ?? activeWorkflowIndex;
  const visibleWorkflowStep = workflowSteps[visibleWorkflowIndex];

  async function loadActiveSession() {
    setIsLoading(true);

    try {
      const response = await fetch("/api/coordinator/api/dkg/sessions/active", {
        cache: "no-store",
      });

      if (response.status === 404) {
        setSession(null);
        setLastAction({
          label: "No active ceremony",
          status: "idle",
          message: "Start a key ceremony when you are ready to drive Round 1.",
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
        label: "Vault load failed",
        status: "error",
        message: errorMessage(error),
      });
    } finally {
      setIsWalletLoading(false);
    }
  }

  async function refreshAllWalletBalances() {
    setIsWalletLoading(true);

    try {
      const response = await fetch("/api/coordinator/api/wallets", {
        cache: "no-store",
      });
      const payload = await readJson<WalletListResponse>(response);

      if (payload.wallets.length === 0) {
        setWallets([]);
        return;
      }

      const refreshedWallets = await Promise.all(
        payload.wallets.map(async (wallet) => {
          try {
            const balanceResponse = await fetch(
              `/api/coordinator/api/wallets/${wallet.wallet_index}/balance`,
              { cache: "no-store" },
            );
            const balance =
              await readJson<WalletBalanceResponse>(balanceResponse);

            return {
              ...wallet,
              balance_lamports: balance.balance_lamports,
              balance_status: balance.balance_status,
              balance_error_message: balance.balance_error_message,
            };
          } catch {
            return wallet;
          }
        }),
      );

      setWallets(refreshedWallets);
    } catch {
      await loadWallets();
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
        label: "Vault created",
        status: "ok",
        message: `Vault ${wallet.wallet_index} is ready at ${shortAddress(
          wallet.address_base58,
        )}.`,
      });
    } catch (error) {
      setLastAction({
        label: "Vault create failed",
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
        label: `Vault ${walletIndex} balance`,
        status: balance.balance_status === "AVAILABLE" ? "ok" : "error",
        message:
          balance.balance_status === "AVAILABLE"
            ? formatBalance(balance.balance_lamports)
            : balance.balance_error_message ?? "Balance unavailable.",
      });
    } catch (error) {
      setLastAction({
        label: `Vault ${walletIndex} balance`,
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
      message: `Vault ${walletIndex} is selected for transfer tickets.`,
    });
  }

  function advanceToTransferIntent() {
    const fundedWallet = wallets.find(
      (wallet) => wallet.balance_status === "AVAILABLE",
    );

    if (!isVaultFundingReady || !fundedWallet) {
      setLastAction({
        label: "Vault funding incomplete",
        status: "error",
        message:
          "Create two vaults, refresh both balances, and fund at least one sender vault before continuing.",
      });
      return;
    }

    setSelectedSenderIndex((currentIndex) => currentIndex ?? fundedWallet.wallet_index);
    setIsVaultFundingConfirmed(true);
    setSelectedWorkflowIndex(2);
    setLastAction({
      label: "Vaults ready",
      status: "ok",
      message: `Vault ${fundedWallet.wallet_index} is selected for the transfer intent.`,
    });
  }

  async function copyWalletAddress(wallet: Wallet | null = copyableWallet) {
    if (!wallet) {
      setLastAction({
        label: "Copy unavailable",
        status: "error",
        message: "Create a derived vault before copying a wallet address.",
      });
      return;
    }

    try {
      await writeClipboardText(wallet.address_base58);
      setLastAction({
        label: "Vault address copied",
        status: "ok",
        message: `Vault ${wallet.wallet_index} address copied: ${shortAddress(
          wallet.address_base58,
        )}.`,
      });
    } catch (error) {
      setLastAction({
        label: "Copy failed",
        status: "error",
        message: errorMessage(error),
      });
    }
  }

  async function createSigningRequest() {
    if (selectedSenderIndex === null) {
      setLastAction({
        label: "Transfer ticket",
        status: "error",
        message: "Select a sender vault first.",
      });
      return;
    }

    const amountLamports = Number(transferForm.amountLamports);

    if (!Number.isInteger(amountLamports) || amountLamports <= 0) {
      setLastAction({
        label: "Transfer ticket",
        status: "error",
        message: "Amount must be a positive lamport integer.",
      });
      return;
    }

    if (!transferForm.recipientAddress.trim()) {
      setLastAction({
        label: "Transfer ticket",
        status: "error",
        message: "Recipient address is required.",
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
        label: "Transfer ticket created",
        status: "ok",
        message: `Ticket ${shortId(signingRequest.request_id)} is ${signingRequest.status}.`,
      });
    } catch (error) {
      setLastAction({
        label: "Transfer ticket failed",
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
        label: `${nodeLabel(nodeId)} ${
          round === 1 ? "Commitment Round" : "Signature Share Round"
        }`,
        status: "ok",
        message: `${result.status}; ticket is now ${result.signing_status}.`,
      });
    } catch (error) {
      await loadSigningRequests();
      setLastAction({
        label: `${nodeLabel(nodeId)} ${
          round === 1 ? "Commitment Round" : "Signature Share Round"
        }`,
        status: "error",
        message: errorMessage(error),
      });
    } finally {
      setPendingSigningAction(null);
    }
  }

  function advanceToBroadcast() {
    if (!selectedSigningRequest || !isThresholdSigningReady) {
      setLastAction({
        label: "Signing incomplete",
        status: "error",
        message:
          "Run both signing rounds for Node A and Node B before broadcasting.",
      });
      return;
    }

    setIsThresholdSigningConfirmed(true);
    setSelectedWorkflowIndex(4);
    setLastAction({
      label: "Signature shares ready",
      status: "ok",
      message: `Ticket ${shortId(
        selectedSigningRequest.request_id,
      )} is ready to aggregate and broadcast.`,
    });
  }

  async function broadcastSigningRequest() {
    if (!selectedSigningRequest) {
      return;
    }

    const actionKey = `broadcast-${selectedSigningRequest.request_id}`;
    setPendingSigningAction(actionKey);

    try {
      const response = await fetch(
        `/api/coordinator/api/signing-requests/${selectedSigningRequest.request_id}/broadcast`,
        { method: "POST" },
      );
      const request = await readJson<SigningRequest>(response);

      await loadSigningRequests();
      await refreshAllWalletBalances();
      setSelectedSigningRequestId(request.request_id);
      setLastAction({
        label: "Broadcast submitted",
        status: "ok",
        message: request.transaction_signature
          ? `Transaction ${shortId(request.transaction_signature)} is ${request.status}.`
          : `Ticket ${shortId(request.request_id)} is ${request.status}.`,
      });
    } catch (error) {
      await loadSigningRequests();
      setLastAction({
        label: "Broadcast failed",
        status: "error",
        message: errorMessage(error),
      });
    } finally {
      setPendingSigningAction(null);
    }
  }

  async function refreshSigningConfirmation() {
    if (!selectedSigningRequest) {
      return;
    }

    const actionKey = `confirm-${selectedSigningRequest.request_id}`;
    setPendingSigningAction(actionKey);

    try {
      const response = await fetch(
        `/api/coordinator/api/signing-requests/${selectedSigningRequest.request_id}/confirm`,
        { method: "POST" },
      );
      const request = await readJson<SigningRequest>(response);

      await loadSigningRequests();
      if (request.status !== "FAILED") {
        await refreshAllWalletBalances();
      }
      setSelectedSigningRequestId(request.request_id);
      setLastAction({
        label: "Confirmation refreshed",
        status: request.status === "FAILED" ? "error" : "ok",
        message: request.error_message ?? `Ticket is ${request.status}.`,
      });
    } catch (error) {
      await loadSigningRequests();
      setLastAction({
        label: "Confirmation failed",
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
          <p className="eyebrow">FROST MPC Wallet</p>
          <h1 id="page-title">MPC Wallet Dashboard</h1>
          <p className="intro">
            Run a 2-of-2 key ceremony, derive Solana Devnet vaults, collect
            threshold signatures, and broadcast test transfers without exposing
            node-local private material.
          </p>
          <div className="network-badges" aria-label="wallet environment">
            <span>Solana Devnet</span>
            <span>2-of-2 MPC</span>
            <span>Test SOL only</span>
          </div>
        </div>
        <div className="session-actions">
          <button
            className="primary-button"
            disabled={pendingAction === "create-session"}
            onClick={createSession}
            type="button"
          >
            {pendingAction === "create-session"
              ? "Starting..."
              : "Start Key Ceremony"}
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

      <section className="workflow-steps" aria-label="MPC wallet workflow">
        {workflowSteps.map((step, index) => {
          const canOpenStep =
            index <= activeWorkflowIndex ||
            workflowStepStates[index] === "complete";

          return (
            <button
              aria-current={
                visibleWorkflowIndex === index ? "step" : undefined
              }
              className={`workflow-step workflow-step-${workflowStepStates[index]} ${
                visibleWorkflowIndex === index ? "workflow-step-selected" : ""
              }`}
              disabled={!canOpenStep}
              key={step.label}
              onClick={() => setSelectedWorkflowIndex(index)}
              type="button"
            >
              <span>{index + 1}</span>
              <div>
                <strong>{step.label}</strong>
                <em>{step.detail}</em>
              </div>
              <small>{workflowStepLabel(workflowStepStates[index])}</small>
            </button>
          );
        })}
      </section>

      <section className="terminal-layout" aria-label="MPC wallet terminal">
        <section className="scene-panel" aria-live="polite">
          <div className="scene-heading">
            <div>
              <p className="eyebrow">Scene {visibleWorkflowIndex + 1}</p>
              <h2>{visibleWorkflowStep.label}</h2>
              <p>{visibleWorkflowStep.detail}</p>
            </div>
            <span className={statusClass(workflowStepStates[visibleWorkflowIndex])}>
              {workflowStepLabel(workflowStepStates[visibleWorkflowIndex])}
            </span>
          </div>

          <div className="scene-body">
            {visibleWorkflowIndex === 0 ? (
              <div className="scene-stack">
                <div className="scene-copy">
                  <strong>2-of-2 MPC key ceremony</strong>
                  <p>
                    Drive the public DKG transcript one node round at a time.
                    Private root shares stay sealed inside the TSS nodes.
                  </p>
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
                            <h3>DKG Round {round}</h3>
                          </div>
                          <span className={statusClass(step.status)}>
                            {step.status}
                          </span>
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
            ) : null}

            {visibleWorkflowIndex === 1 ? (
              <div className="scene-stack">
                <div className="scene-toolbar">
                  <div className="scene-copy">
                    <strong>Derived Vaults</strong>
                    <p>
                      Create public Solana Devnet vault addresses from the
                      completed key ceremony, then refresh balances after funding.
                    </p>
                  </div>
                  <div className="wallet-actions">
                    <button
                      className="primary-button"
                      disabled={
                        session?.status !== "COMPLETED" ||
                        pendingWalletAction !== null
                      }
                      onClick={() => void createWallet()}
                      type="button"
                    >
                      {pendingWalletAction === "create-wallet"
                        ? "Creating..."
                        : "Create Vault"}
                    </button>
                    <button
                      className="secondary-button"
                      disabled={isWalletLoading}
                      onClick={() => void loadWallets()}
                      type="button"
                    >
                      {isWalletLoading ? "Refreshing..." : "Refresh"}
                    </button>
                    <button
                      className="primary-button"
                      disabled={!isVaultFundingReady}
                      onClick={advanceToTransferIntent}
                      type="button"
                    >
                      Next Step
                    </button>
                  </div>
                </div>

                {wallets.length === 0 ? (
                  <div className="empty-wallet-state">
                    <strong>No derived vaults</strong>
                    <p>
                      {session?.status === "COMPLETED"
                        ? "Create the first derived vault from the completed key ceremony."
                        : "Complete the key ceremony before deriving vaults."}
                    </p>
                  </div>
                ) : (
                  <div className="wallet-list" role="list">
                    {wallets.map((wallet) => {
                      const balanceActionKey = `balance-${wallet.wallet_index}`;
                      const isSelected =
                        selectedSenderIndex === wallet.wallet_index;

                      return (
                        <article
                          className="wallet-row"
                          key={wallet.wallet_index}
                          role="listitem"
                        >
                          <div className="wallet-index">
                            <span>Vault</span>
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
                              onClick={() =>
                                void refreshBalance(wallet.wallet_index)
                              }
                              type="button"
                            >
                              {pendingWalletAction === balanceActionKey
                                ? "Checking..."
                                : "Balance"}
                            </button>
                            <button
                              className={
                                isSelected ? "primary-button" : "secondary-button"
                              }
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
              </div>
            ) : null}

            {visibleWorkflowIndex === 2 ? (
              <div className="scene-stack">
                <div className="scene-toolbar">
                  <div className="scene-copy">
                    <strong>Transfer Tickets</strong>
                    <p>
                      Pick a funded sender vault, enter a Devnet recipient, and
                      create the ticket that will be signed by both TSS nodes.
                    </p>
                  </div>
                  <div className="wallet-actions">
                    <button
                      className="secondary-button"
                      disabled={isSigningLoading}
                      onClick={() => void loadSigningRequests()}
                      type="button"
                    >
                      {isSigningLoading ? "Refreshing..." : "Refresh Tickets"}
                    </button>
                  </div>
                </div>
                <div className="transfer-form" aria-label="Create transfer ticket">
                  <div className="transfer-fields">
                    <label>
                      <span>Sender Vault</span>
                      <select
                        disabled={
                          wallets.length === 0 || pendingSigningAction !== null
                        }
                        onChange={(event) => {
                          if (event.target.value) {
                            selectSender(Number(event.target.value));
                          }
                        }}
                        value={selectedSenderIndex ?? ""}
                      >
                        <option value="">Select vault</option>
                        {wallets.map((wallet) => (
                          <option
                            key={wallet.wallet_index}
                            value={wallet.wallet_index}
                          >
                            Vault {wallet.wallet_index} -{" "}
                            {shortAddress(wallet.address_base58)}
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
                        placeholder="Paste a Devnet wallet address"
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
                  </div>
                  <button
                    className="primary-button transfer-submit"
                    disabled={
                      selectedSenderIndex === null || pendingSigningAction !== null
                    }
                    onClick={() => void createSigningRequest()}
                    type="button"
                  >
                    {pendingSigningAction === "create-signing-request"
                      ? "Creating..."
                      : "Create Ticket"}
                  </button>
                </div>
                <div className="request-list" role="list">
                  {signingRequests.length === 0 ? (
                    <div className="empty-wallet-state">
                      <strong>No transfer tickets</strong>
                      <p>Create a ticket after selecting a sender vault.</p>
                    </div>
                  ) : (
                    signingRequests.map((request) => {
                      const isSelected =
                        selectedSigningRequest?.request_id === request.request_id;

                      return (
                        <button
                          className={
                            isSelected
                              ? "request-row request-row-selected"
                              : "request-row"
                          }
                          key={request.request_id}
                          onClick={() =>
                            setSelectedSigningRequestId(request.request_id)
                          }
                          role="listitem"
                          type="button"
                        >
                          <span>
                            <strong>{shortId(request.request_id)}</strong>
                            <small>Vault {request.wallet_index}</small>
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
              </div>
            ) : null}

            {visibleWorkflowIndex === 3 ? (
              <div className="scene-stack">
                <div className="scene-copy">
                  <strong>Threshold Signing</strong>
                  <p>
                    Each node first publishes signing commitments, then returns
                    one consumed signature share for the selected transfer ticket.
                  </p>
                </div>
                {selectedSigningRequest ? (
                  <>
                    <div className="selected-request-summary">
                      <div>
                        <p className="eyebrow">Selected Ticket</p>
                        <h3>{shortId(selectedSigningRequest.request_id)}</h3>
                      </div>
                      <span className={statusClass(selectedSigningRequest.status)}>
                        {selectedSigningRequest.status}
                      </span>
                      <button
                        className="primary-button signing-next-button"
                        disabled={!isThresholdSigningReady}
                        onClick={advanceToBroadcast}
                        type="button"
                      >
                        Next Step
                      </button>
                    </div>
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
                                <h3>
                                  {round === 1
                                    ? "Commitment Round"
                                    : "Signature Share Round"}
                                </h3>
                              </div>
                              <span className={statusClass(step.status)}>
                                {step.status}
                              </span>
                              <button
                                className="round-button"
                                disabled={
                                  pendingSigningAction !== null ||
                                  isRoundTwoReplay
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
                    <strong>No transfer ticket selected</strong>
                    <p>Create or select a ticket before collecting signatures.</p>
                  </div>
                )}
              </div>
            ) : null}

            {visibleWorkflowIndex === 4 ? (
              <div className="scene-stack">
                <div className="scene-copy">
                  <strong>Broadcast</strong>
                  <p>
                    Aggregate both signature shares, submit the Devnet
                    transaction, and open the Solana Explorer receipt.
                  </p>
                </div>
                {selectedSigningRequest ? (
                  <>
                    <dl className="request-facts">
                      <div>
                        <dt>Sender Vault</dt>
                        <dd>
                          {shortAddress(
                            selectedSigningRequest.sender_address_base58,
                          )}
                        </dd>
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
                      <div>
                        <dt>Transaction Receipt</dt>
                        <dd>
                          {selectedSigningRequest.transaction_signature
                            ? shortId(selectedSigningRequest.transaction_signature)
                            : "Not sent"}
                        </dd>
                      </div>
                    </dl>
                    <div className="broadcast-actions">
                      <div className="receipt-heading">
                        <p className="eyebrow">Transaction Receipt</p>
                        <strong>
                          {selectedSigningRequest.transaction_signature
                            ? shortId(selectedSigningRequest.transaction_signature)
                            : "Not broadcast"}
                        </strong>
                      </div>
                      <button
                        className="primary-button"
                        disabled={
                          selectedSigningRequest.status !==
                            "READY_TO_AGGREGATE" ||
                          pendingSigningAction !== null
                        }
                        onClick={() => void broadcastSigningRequest()}
                        type="button"
                      >
                        {pendingSigningAction ===
                        `broadcast-${selectedSigningRequest.request_id}`
                          ? "Broadcasting..."
                          : "Aggregate & Broadcast"}
                      </button>
                      <button
                        className="secondary-button"
                        disabled={
                          selectedSigningRequest.status !== "BROADCASTED" ||
                          pendingSigningAction !== null
                        }
                        onClick={() => void refreshSigningConfirmation()}
                        type="button"
                      >
                        {pendingSigningAction ===
                        `confirm-${selectedSigningRequest.request_id}`
                          ? "Checking..."
                          : "Refresh Confirmation"}
                      </button>
                      {selectedSigningRequest.explorer_url ? (
                        <a
                          className="explorer-link"
                          href={selectedSigningRequest.explorer_url}
                          rel="noreferrer"
                          target="_blank"
                        >
                          Open Explorer
                        </a>
                      ) : null}
                    </div>
                    {selectedSigningRequest.error_message ? (
                      <p className="request-error">
                        {selectedSigningRequest.error_message}
                      </p>
                    ) : null}
                  </>
                ) : (
                  <div className="empty-wallet-state">
                    <strong>No transfer ticket selected</strong>
                    <p>Create or select a ticket before broadcasting.</p>
                  </div>
                )}
              </div>
            ) : null}
          </div>
        </section>

        <aside className="terminal-side" aria-label="Wallet status">
          <div className="summary-stack" aria-label="MPC wallet summary">
            <div className="summary-card">
              <div className="summary-card-row">
                <span>Ceremony</span>
                <strong>{session ? shortId(session.session_id) : "Not started"}</strong>
              </div>
              <div className="summary-card-row">
                <span>MPC Status</span>
                <strong>{session?.status ?? "NOT_CREATED"}</strong>
              </div>
              <div className="summary-key">
                <span>Master Key</span>
                <strong>
                  {session?.master_public_key_base58
                    ? shortAddress(session.master_public_key_base58)
                    : "Pending"}
                </strong>
              </div>
            </div>

          </div>

          <div className="vault-watch-panel" aria-label="Vault balances">
            <div className="vault-watch-header">
              <div>
                <p className="eyebrow">Vault Watch</p>
                <h2>Wallet Balances</h2>
              </div>
              <button
                className="secondary-button vault-watch-refresh"
                disabled={isWalletLoading}
                onClick={() => void loadWallets()}
                type="button"
              >
                {isWalletLoading ? "Refreshing..." : "Refresh"}
              </button>
            </div>
            {wallets.length === 0 ? (
              <div className="vault-watch-empty">
                <strong>No vaults yet</strong>
                <p>Create vaults after completing the key ceremony.</p>
              </div>
            ) : (
              <div className="vault-watch-list">
                {wallets.map((wallet) => (
                  <article
                    className="vault-watch-row"
                    key={wallet.wallet_index}
                  >
                    <div className="vault-watch-main">
                      <span>Vault {wallet.wallet_index}</span>
                      <strong>{shortAddress(wallet.address_base58)}</strong>
                      <small>{wallet.derivation_path}</small>
                    </div>
                    <div className="vault-watch-balance">
                      <span className={statusClass(wallet.balance_status)}>
                        {wallet.balance_status}
                      </span>
                      <strong>{formatSolBalance(wallet.balance_lamports)}</strong>
                    </div>
                    <button
                      className="copy-address-button"
                      onClick={() => void copyWalletAddress(wallet)}
                      type="button"
                    >
                      Copy
                    </button>
                  </article>
                ))}
              </div>
            )}
            <div className={actionClass(lastAction.status)}>
              <strong>{lastAction.label}</strong>
              <p>{lastAction.message}</p>
            </div>
          </div>
        </aside>
      </section>
    </main>
  );
}

function workflowStepLabel(state: string) {
  if (state === "complete") {
    return "Done";
  }

  if (state === "active") {
    return "Now";
  }

  return "Queued";
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

async function writeClipboardText(value: string) {
  try {
    await navigator.clipboard.writeText(value);
    return;
  } catch {
    const textArea = document.createElement("textarea");
    textArea.value = value;
    textArea.setAttribute("readonly", "");
    textArea.style.position = "fixed";
    textArea.style.left = "-9999px";
    textArea.style.top = "0";
    document.body.appendChild(textArea);
    textArea.focus();
    textArea.select();

    const didCopy = document.execCommand("copy");
    document.body.removeChild(textArea);

    if (!didCopy) {
      throw new Error("Clipboard copy was blocked by the browser.");
    }
  }
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

function formatSolBalance(lamports: number | null): string {
  if (lamports === null) {
    return "Not checked";
  }

  return `${(lamports / 1_000_000_000).toFixed(9)} SOL`;
}

function formatLamports(lamports: number): string {
  return `${lamports} lamports`;
}

function nodeLabel(nodeId: NodeId): string {
  return nodeId === "node-a" ? "Node A" : "Node B";
}

function statusClass(status: string): string {
  if (status === "COMPLETED" || status === "CONFIRMED") {
    return "status-pill status-completed";
  }

  if (
    status.includes("IN_PROGRESS") ||
    status === "RUNNING" ||
    status === "BROADCASTED"
  ) {
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

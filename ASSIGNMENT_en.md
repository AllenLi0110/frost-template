# Blockchain Threshold Signature Wallet Implementation Assignment (FROST & HD Wallet on Solana)

Welcome to this technical assignment! We are excited to see your creativity and technical capabilities in tackling this challenging project.

## 🎯 Assignment Objective

This assignment is designed to evaluate your ability to quickly build complex systems using **AI Agents (such as Claude Code, Codex, OpenCode, Cursor, Windsurf, Antigravity, etc.)**, as well as to observe your understanding of cryptographic protocols and system architecture partitioning.

Please implement a minimal viable **2-of-2 TSS Solana Wallet Demo System**. The scope covers applied modern cryptography (FROST threshold signatures), Rust backend development, frontend interface design, and cross-node state management. The goal here is "not" to build a production-ready MPC infrastructure, but rather to ensure that the core data flow, protocol steps, state management, and frontend-backend integration are correctly executed. Furthermore, we **strongly encourage you to showcase how you guided AI to solve domain-specific technical problems**.

The focus is on **functionality and observable behavior verification**, not on forcing you to mathematically prove a specific set of cryptographic derivations. As long as you properly utilize `frost-ed25519` together with the `hd-wallet`'s non-hardened Edwards derivation, complete a root-level 2-of-2 DKG, successfully derive multiple wallet addresses on the frontend continuously, and allow the nodes to utilize the exact same root key shares to complete threshold signatures for transactions across different wallet indices, you will have met our expectations.

This task adopts a "**Specified Tech Stack + Specified Observable Behavior Verification**" approach: apart from the explicitly stated technical limitations, protocol boundaries, and environment conditions, you are completely free to design the internal abstractions, APIs, database schemas, state transition logic, and implementation details. You don't need to copy a rigid playbook; ensuring the final system behavior meets the acceptance criteria is what matters most.

---

## 🛠️ Scope & Parameters

- **Blockchain Environment**: Solana (Devnet)
- **Curve / Signature Algorithm**: Ed25519 / FROST
- **Threshold**: 2-of-2
- **Wallet Derivation**: Non-hardened Edwards derivation for Ed25519 (Compatibility with current mainstream Solana wallet derivations is not required)
- **RPC Endpoint**: Please strictly use `https://api.devnet.solana.com`
- **Funding**: During verification, the reviewer will manually transfer Devnet SOL to the derived wallet addresses; providing an integrated airdrop feature is considered a nice-to-have bonus, but is not strictly necessary.
- **Transaction Type**: The system must support at least basic Solana Devnet SOL transfers.

## 💻 Tech Stack Requirements

To maintain a consistent evaluation environment, please adhere to the following tech stack:

- **Backend:** Rust `1.94.0` + `axum` `0.8.8` + `snafu` `0.8.7` + `sqlx` `0.8.6` + PostgreSQL `18`
- **Frontend:** Next.js `16` + React `19` + TypeScript `5.9`
- **Solana SDK:** `solana-client` `3.1.8` / `solana-sdk` `3.0.0`
- **Core Crypto:** [`frost-ed25519` `2.1.0`](https://github.com/ZcashFoundation/frost), [`hd-wallet` `0.6.1`](https://docs.rs/hd-wallet/latest/hd_wallet/) (Using `Edwards` non-hardened derivation)

You are welcome to integrate other necessary packages, provided the system architecture remains clean, the code readable, and the application easy to run. You are completely free to decide how to bridge the FROST root share with the `hd-wallet` derivation process; we do not mandate the internal mathematical abstraction. The primary focus is whether the final observable behavior satisfies the requirements.

---

## 📐 System Architecture & Component Responsibilities

To simulate a real-world TSS (Threshold Signature Scheme) environment, the system is separated into the following key components:

### 1. TSS Coordinator

Acts as the central orchestrator:

- **Coordinate DKG Flow**: Forwards trigger commands from the frontend to the specified Node, then aggregates and persists the exchange data across different rounds.
- **Coordinate Signing Flow**: Receives pending signature requests from the frontend, sends Signing Round start triggers to the target Nodes, and collects Commitments and Signature Shares.
- **Aggregation & Broadcast**: Aggregates the collected Signature Shares into the final Aggregated Signature, constructs the complete transaction, and broadcasts it to the Solana Devnet.
- **General Wallet Operations**: Manages application-layer logic such as wallet address derivations and balance inquiries.

### 2. TSS Nodes (Node A & Node B)

Each Node represents an independent Signer and is responsible for all cryptographic computations. **The private key (Share) must be securely kept within the Node and must NEVER leave it**:

- Executes the cryptographic operations for every round of the DKG.
- Executes the operations for every round of the Signing phase: generating Nonces/Commitments, deriving Child Key Shares, and calculating the Signature Share.
- **On-the-fly Derivation (Core Requirement)**: During idle states, the node merely needs to securely store the Root Share generated from the DKG. The system should be designed such that when generating new wallet addresses for the frontend to display, it **does not require the TSS Nodes to communicate with one another**. Derivation happens seamlessly based on the wallet index. During a Signing phase, nodes must be able to instantly and seamlessly derive the cryptographic material corresponding to a wallet index in memory, based on the shared Root Share, to successfully complete the threshold signature for the derived wallet.

### 3. Frontend (State Visualizer & Protocol Driver)

The frontend serves as the sole interactive interface for the system, driving all underlying flows via the Coordinator API. We expect the frontend to **visually represent the multi-round interactive nature of FROST**. Users should clearly see the protocol's state and manually trigger individual steps between the nodes and the coordinator.

---

## 💾 Data Persistence

- Please use a **single PostgreSQL instance** for the entire system (different services can utilize different databases or schemas).
- The Coordinator and each Node are individually responsible for persisting their own required operational state. **The explicit data schema design and persistence strategy are entirely up to you**, as long as the system behavior behaves as expected.
- **Basic Requirement**: Following a system reboot, previously completed DKG results, a list of derived wallets, and historical transaction records must be accurately restored and must not be lost.

---

## ✅ Core Use Cases & Acceptance Criteria

Below are the three core workflows that we will operate and evaluate during our review:

### A. DKG (Distributed Key Generation) Flow

The frontend interface must provide a "**step-by-step / independent trigger**" interaction model for the DKG. The behavior of each node in every Round must have an independent operation button or equivalent control UI element; **do NOT design a single "Run All" button that hides the complex DKG process behind the scenes**.

**Acceptance Criteria:**

1. The user can independently trigger Node A and Node B to execute each consecutive phase of the DKG (Round 1 / Round 2 / Round 3).
2. Upon successful DKG completion, the system must hold the necessary root-level material for subsequent Wallet Derivation, enabling the frontend to seamlessly and continuously generate derived wallets. You must definitively use `frost-ed25519` and `hd-wallet`'s non-hardened Edwards derivation for implementation.
3. The frontend must clearly display the operational completion status of each Node at every phase, an overall DKG progress indicator, and the finally generated Master Public Key (in Base58 format).

### B. Wallet Derivation

Utilizing the root-level material generated by the DKG, execute a Non-hardened Edwards Derivation to branch out child wallet addresses.

**Acceptance Criteria:**

1. The UI provides a "Create Wallet" button. Upon clicking, the system automatically infers a brand-new wallet using the next sequentially increasing index.
2. The frontend must list **all historically derived wallets**, detailing at minimum the "Wallet Index" and its corresponding "Solana Address (Base58)", alongside a query/display function for that address's SOL balance on the Devnet.
3. Deriving a new wallet address must not demand or rely on further network interactivity between the TSS Nodes; this criterion serves to prove that a single set of root key shares can successfully correspond to multiple different wallet indices.
4. The user is able to select any existing wallet from the list to act as the **Sender for an impending transfer**.

### C. Threshold Signatures & Transaction Transfer (FROST Signing Flow)

Like DKG, the frontend must break down the internal progression of a single transfer into individually **triggerable** operational steps. Every node in every round of the Signing process must feature a correlating UI control component; **do NOT implement a generic one-click "Sign & Send" button**.

**Acceptance Criteria:**

1. **Transaction Creation**: The user selects a sender wallet from the list, inputs a recipient address, and an amount to transfer. Upon submission, the Coordinator generates an isolated and trackable **Signing Request**.
2. **Pending Signature Request List**: The frontend should query and list all currently pending Signing Requests, showing information such as: sender wallet, target address, amount, creation timestamp, and current status. The interface must allow users to distinguish between different requests and select one to proceed with its signing flow.
3. **Signing Round 1**: For a specified pending request, the user can independently trigger Node A or Node B to execute the first phase of the signing computation.
4. **Signing Round 2**: For a specified request, the user can independently trigger Node A or Node B to execute the second phase. In this step, nodes must guarantee they are drawing upon the unified Root Share and the explicitly designated wallet index to satisfy the threshold signature at the derived wallet tier.
5. **Aggregation & Broadcast**: After all signature shares are assembled, the user can prompt the Coordinator to perform signature aggregation for that request, composing a legally formatted Solana transaction and broadcasting it to the Solana Devnet.
6. **State Visualization:**
   - Must comprehensively and intelligibly display the progression of a Signing Request's state (e.g., Pending → Signing → Broadcasted → Confirmed → Failed), where the "Confirmed" state should strictly reflect the Solana network's `confirmed` signal return.
   - Must distinctly reflect the completion indicators/status of Node A and Node B during each phase of the signature process for that particular request.
   - Upon successful broadcast, provide a hyperlink to the transaction's hash on the Solana Explorer.

> **⏰ Expected Time Commitment: Roughly One Week (7 Days).** <br/> Please do not feel rushed. Organize your time sensibly, and we recommend prioritizing the validation of correct logic underlying the core DKG and Signing flows.

---

## 📋 Evaluation Rubric

The grading dimensions for this assignment are as follows:

| Evaluation Dimension            | Weight  | Description                                                                                                                                                                                                                                                                                                                                          |
| ------------------------------- | ------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Acceptance Criteria**         | **40%** | Does the system successfully run through the three core workflows (DKG, Wallet Derivation, Signing & Broadcast), and does the frontend satisfy the requirements for step-by-step triggering and layered status visualization?<br/> **⚠️ This is a pass/fail threshold: A workflow that fails basic acceptance verification will receive no points.** |
| **AI Collaboration Journey**    | **40%** | We heavily weigh how you collaborate with AI, including your complete journey of guiding an AI through challenging cryptographic engineering challenges (see detailed requirements below).                                                                                                                                                           |
| **Engineering Quality & Bonus** | **20%** | Security architecture (e.g., input validation, robust error handling, nonce protection), code structure and quality, test coverage, comprehensiveness of system architecture design, documentation quality, and any additional value-add features beyond basic designs.                                                                              |

### 1. AI Development Journey & Documentation (40% of Final Score)

In today's engineering climate, utilizing AI effectively is a massive advantage. We place **great emphasis** on how you prompt, pair-program with, and navigate AI tools to complete this demanding project. Therefore, please document your workflow religiously.

- Accepted formats are diverse: including, but not limited to, prompt histories, verbatim chat exports, crucial screenshots, organized Markdown documentation, an architectural Decision Log, or any materials that elucidate your interaction process with the AI.
- When you hit a roadblock, or the AI suggests an incorrect path, **please meticulously document how you corrected and redirected it**.
- If your dialogues contain sensitive information or prohibitively long texts, feel free to sanitize or trim them appropriately. However, you must include the critical exchanges that highlight your problem-solving thought process.

We primarily want to uncover:

- **Exploring the Unknown & Accelerated Learning**: When faced with unfamiliar tech stacks (like Blockchain, Rust or FROST cryptography), how did you utilize AI as an accelerator to quickly grasp new knowledge and implement it practically?
- **Complex Problem Decomposition**: Confronted with a broad architecture, how did you parse it into digestible subtasks that an AI could effectively process and respond to?
- **Prompt Engineering & Context Management**: Were your prompts precise? Did you supply adequate and necessary context to guide the AI toward correct outputs?
- **Course Correction & Debugging Strategies**: When an AI produced "hallucinations" or buggy code, what were your troubleshooting strategies and perspectives? How did you guide the AI to self-correct?
- **Autonomy & Technical Decision Making**: At which crucial crossroads did you step in to assert professional human engineering judgment over blindly accepting AI suggestions?

### 2. Run Guide (README & Docker Compose)

To expedite our testing and review process, please provide a clear local launch guide:

- **No Cloud Deployment Required**: Simply provide a thoroughly documented `README.md` guiding us on compiling and running the full system natively on our locals.
- **Docker Compose**: You **must** supply a `docker-compose.yml` file. We hope to launch the complete ecosystem—inclusive of Frontend, Coordinator, Node A, Node B, and PostgreSQL (a single instance is fine)—through a single, straightforward `docker compose up` command.
- In your README, please explicitly inform to set the default RPC Endpoint to `https://api.devnet.solana.com`. In addition, explain to the reviewer where they can view derived wallet addresses so they may manually send Devnet SOL as testing funds. If you've gone the extra mile to integrate a Faucet/Airdrop service, please introduce that as well!

### 3. Automated Testing Experience

While we heavily focus on manual acceptance testing and your AI collaboration journal, exceptional software engineering is deeply tied to automated testing.
You are more than welcome to showcase your proficiency by writing Unit tests or Integration tests! You have absolute liberty over the test coverage and strategy. We will utilize this aspect to gauge your protective awareness concerning the system's critical paths, as well as your profound understanding of engineering quality as a whole.

Thank you again for dedicating your time. We hope you genuinely enjoy the challenge and achieve great results! 🎉

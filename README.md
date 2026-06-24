# FROST Interview Assignment — Initial Environment Setup Guide

This repository provides the starting skeleton for the implementation assignment. Please read `ASSIGNMENT_en.md` (or `ASSIGNMENT_zh.md` for Chinese) carefully to understand the complete requirements, and follow the steps below to set up your environment.

---

## 📦 Prerequisites: Install `mise`

This project uses [mise](https://mise.jdx.dev/) to consistently manage Node.js and Rust versions. Please install it first:

```bash
# macOS / Linux
curl https://mise.run | sh
```

```bash
# Brew
brew install mise
```

Once installed, add `mise` to your shell configuration (choose the one corresponding to your shell):

```bash
# zsh (Default on macOS)
echo 'eval "$(~/.local/bin/mise activate zsh)"' >> ~/.zshrc
source ~/.zshrc

# bash
echo 'eval "$(~/.local/bin/mise activate bash)"' >> ~/.bashrc
source ~/.bashrc
```

> For more installation options, please refer to the [official mise documentation](https://mise.jdx.dev/getting-started.html).

---

## 🚀 Environment Startup

### Step 1: Restart Your Terminal

After adding `mise` to your shell configuration above, **restart your terminal** (or `source` your shell config) so that the `mise activate` hook takes effect.

Once activated, **`mise` automatically switches to the correct tool versions whenever you `cd` into this project directory** — no manual steps needed.

### Step 2: Install Tool Versions

Navigate to the project root and run:

```bash
mise install
```

`mise` will automatically install the Node.js and Rust versions specified in `mise.toml`. After installation, verify that the correct versions are active:

```bash
node --version   # Expected: v24.14.0
rustc --version  # Expected: rustc 1.94.0 (...)
```

---

### Step 3: Verify the Frontend Environment (Next.js)

```bash
cd frontend
npm install
npm run dev
```

Open your browser and navigate to [http://localhost:3000](http://localhost:3000). Seeing the default Next.js welcome page means your frontend environment is working correctly.

Press `Ctrl+C` to stop the server when you are done.

---

### Step 4: Verify the Backend Environment (Rust Workspace)

```bash
cd backend

# Run the coordinator
cargo run -p coordinator
# Expected output: Hello, world! (coordinator)

# Run the tss-node
cargo run -p tss-node
# Expected output: Hello, world! (tss-node)
```

If both commands output successfully, your backend environment is ready to go.

---

## 📁 Project Structure

```
frost-assignment/
├── ASSIGNMENT_en.md    # Assignment requirements (English)
├── ASSIGNMENT_zh.md    # Assignment requirements (Chinese)
├── mise.toml           # Node.js & Rust version management
├── frontend/           # Next.js frontend (Implement your frontend here)
│   └── ...
└── backend/            # Rust Workspace backend (Implement your backend here)
    ├── Cargo.toml      # Workspace root, manages all crate dependencies
    ├── coordinator/    # Coordinator Server
    │   └── src/main.rs
    └── tss-node/       # TSS Node (Can be executed multiple times to simulate multiple nodes)
        └── src/main.rs
```

---

## 💡 Development Tips

- **Rust dependencies:** Please add them uniformly to the `[workspace.dependencies]` section in `backend/Cargo.toml`. Individual crates should inherit them using `.workspace = true` to avoid version conflicts.
- **Frontend:** Work within the `frontend/` directory and start the development mode using `npm run dev`.
- **Backend:** Work within the `backend/` directory. Use `cargo run -p <crate-name>` to run a specific crate, and `cargo build --workspace` to compile everything.
- For detailed assignment requirements, please see `ASSIGNMENT_en.md` or `ASSIGNMENT_zh.md`.

---

## ✅ CI And Versioning

Pull requests are expected to pass GitHub Actions before merge:

- Repository hygiene and release metadata
- Backend Rust tests
- Frontend lint and build
- Phase integration verification

Release checkpoints use `VERSION`, `CHANGELOG.md`, package metadata, and Git tags. See `docs/release-process.md` for the branch protection and release flow.

---

## 🧪 Full Devnet Demo Workflow

This workflow runs the complete FROST wallet demo against Solana Devnet. Devnet SOL is test money only; it has no mainnet value. The transaction is still a real Devnet transaction and can be inspected in Solana Explorer.

### 1. Start The Full Stack

From the project root:

```bash
docker compose up -d
```

Verify that every service is running:

```bash
docker compose ps
curl -s http://localhost:8080/health
curl -s http://localhost:8080/health/nodes
curl -s http://localhost:8081/health
curl -s http://localhost:8082/health
```

Open the frontend:

```bash
open http://localhost:3000
```

If `open` is not available, manually visit [http://localhost:3000](http://localhost:3000).

Phase 6 verification uses an isolated Docker Compose project with `SOLANA_RPC_URL=mock://phase6`, so it does not overwrite this Devnet demo stack. If you ever need to force the local demo back onto Devnet, recreate the coordinator:

```bash
docker compose up -d --force-recreate coordinator
```

### 2. Complete DKG

In the `DKG Control Surface` section:

1. Click `Create Session`.
2. Run `Node A` Round 1.
3. Run `Node B` Round 1.
4. Run `Node A` Round 2.
5. Run `Node B` Round 2.
6. Run `Node A` Round 3.
7. Run `Node B` Round 3.
8. Confirm the session status is `COMPLETED`.

The coordinator only stores public DKG state. Private root key shares stay inside the TSS nodes.

### 3. Create A Derived Wallet

In the `Wallet Derivation` section:

1. Set `Index` to `0`.
2. Click `Create Wallet`.
3. Copy the displayed wallet address.
4. Click `Balance` or `Refresh` to load its Devnet balance.

Example wallet address:

```text
2gSj4TXHJteBPamDy1BtLNxAD4sLZEd3BbPRgJNEpAAa
```

### 4. Fund The Wallet With Devnet SOL

The sender wallet must have Devnet SOL before it can pay transaction fees or send transfers.

Option A: use the Solana Devnet Faucet in a browser:

```text
https://faucet.solana.com/
```

Paste the derived wallet address from the frontend and request Devnet SOL.

Option B: use the Solana CLI if it is installed:

```bash
solana airdrop 0.5 <DERIVED_WALLET_ADDRESS> --url devnet
```

Then return to the frontend and click `Balance` or `Refresh`. A funded wallet should show a balance such as:

```text
500000000 lamports (0.500000000 SOL)
```

### 5. Pick A Recipient

For the clearest demo, create a second derived wallet:

1. In `Wallet Derivation`, set `Index` to `1`.
2. Click `Create Wallet`.
3. Copy the Wallet 1 address.
4. Use Wallet 1 as the recipient in the signing request.

For a quick smoke test, you may also send from Wallet 0 to Wallet 0. In that case, only the transaction fee is deducted because the transfer amount leaves and returns to the same account.

Do not use this address as a recipient:

```text
11111111111111111111111111111111
```

That is the Solana System Program ID, not a normal recipient wallet.

### 6. Create A Signing Request

In the `Signing Requests` section:

1. Select `Wallet 0` as the sender.
2. Paste the recipient wallet address.
3. Enter a small amount, such as `100` lamports for a smoke test or `1000000` lamports for a visible transfer.
4. Click `Create Request`.
5. Select the newly created request.

### 7. Complete The Signing Rounds

For the selected request:

1. Run `Node A` Signing Round 1.
2. Run `Node B` Signing Round 1.
3. Run `Node A` Signing Round 2.
4. Run `Node B` Signing Round 2.
5. Confirm the request reaches `READY_TO_AGGREGATE`.

Each TSS node signs with its local child signing share. The coordinator receives public signing payloads and signature shares, but it never receives private root shares or child private shares.

### 8. Broadcast And Confirm

1. Click `Aggregate & Broadcast`.
2. Wait for the request status to become `BROADCASTED`.
3. Click `Refresh Confirmation`.
4. Confirm the request status becomes `CONFIRMED`.
5. Click `Open Explorer` to inspect the Devnet transaction.

A successful self-transfer may show details like:

```text
Status: Success
Confirmation: Finalized
Fee: 0.000005 SOL
System Program: Transfer
```

### 9. Common Demo Errors

If the sender is not funded, Solana RPC may return:

```text
Attempt to debit an account but found no record of a prior credit.
```

Fund the sender wallet with Devnet SOL, then create a new signing request.

If the recipient is `11111111111111111111111111111111`, the app rejects the signing request because that address is the System Program, not a wallet. Older builds may have reached Solana RPC and returned:

```text
instruction changed the balance of a read-only account
```

Use a normal Devnet wallet address instead.

If broadcast fails after waiting too long, create a new signing request. Solana recent blockhashes expire, so the transaction must be signed and broadcast soon after the signing request is prepared.

---

Best of luck! 🎉

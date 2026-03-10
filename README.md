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

Best of luck! 🎉

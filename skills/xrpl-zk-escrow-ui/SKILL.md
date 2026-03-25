---
name: xrpl-zk-escrow-ui
description: Web UI for quickly deploying and testing XRPL Smart Escrows. No-code interface for hackathon participants to experiment before diving into Rust code.
globs:
  - "ui/**/*"
  - "ui/*.html"
  - "ui/*.css"
  - "ui/*.js"
alwaysApply: true
---

# XRPL ZK Escrow Web UI

**Start here for hackathon experimentation.** The Web UI provides a no-code interface for deploying and testing Smart Escrows without touching Rust code. Perfect for understanding the concept before building custom guest programs.

## Quick Start (No Code Required)

### 1. Open the Hosted UI

Navigate to: **`https://boundless-xyz.github.io/xrpl-risc0-starter/`**

Or open `ui/index.html` locally in your browser.

### 2. Connect to Network

Choose your target:
- **Groth Devnet** — Public devnet with Groth5 precompiles (recommended for beginners)
- **Local Node** — Your own rippled instance (for advanced testing)

Click **Connect**. The status bar shows "Connected" when ready.

### 3. Load WASM Code

Three ways to load escrow logic:

| Method | How | Best For |
|--------|-----|----------|
| **Examples** | Click "Hello World" button | Quick testing with pre-built examples |
| **Upload** | Select `.wasm` file from `target/wasm32v1-none/release/` | Your compiled Rust escrow |
| **Paste Hex** | Copy hex output from `xxd -p escrow.wasm` | Debugging or CI integration |

### 4. Generate & Fund Accounts

1. Click **Generate New Account** — creates a wallet + funds it via faucet
2. The account appears in the Accounts list with balance
3. Click any account to select it (highlighted)

For additional accounts, repeat the process. You need at least 2 accounts (owner + destination) to create an escrow.

### 5. Deploy Smart Escrow

In the **Transaction Interface → Deploy WASM** tab:

1. **Source Account** — Select the escrow owner (creates the escrow)
2. **Destination Account** — Select who receives funds when finished
3. **Escrow Amount** — Default 100000 drops (0.1 XRP)
4. Click **Deploy WASM as Smart Escrow**

The UI auto-fills:
- `CancelAfter` — 2000 seconds from current ledger time
- `FinishFunction` — Your loaded WASM code
- `Data` — 70 XRP (computation allowance)

Success: Escrow appears in the **Escrow Management** list with sequence number.

### 6. Finish the Escrow

In the **Transaction Interface → Finish Escrow** tab:

1. **Select Escrow** — Pick from your deployed escrows (auto-fills sequence/owner)
2. **Source Account** — Typically the destination account (who finishes it)
3. **Owner Account** — Auto-filled from escrow selection
4. **Offer Sequence** — Auto-filled from escrow selection
5. (Optional) Add memos, fulfillment, or custom fields via **Advanced Options**
6. Click **Finish Escrow**

The WASM executes on-chain. If it returns `1`, funds release to destination. Otherwise, the escrow stays locked.

## UI Features

### WASM Code Management

- **Examples Tab** — Pre-built WASM examples (hello_world, etc.)
- **Upload Tab** — File picker for `.wasm` files
- **Paste Hex Tab** — Direct hex input for debugging
- **README Links** — Click "View README" to understand example logic

### Account Management

- **Generate New Account** — Creates + funds wallet
- **Fund Selected** — Add more XRP to existing account
- **Copy Address** — 📋 button copies address to clipboard
- **Balance Display** — Real-time XRP balance from ledger

### Escrow Management

- List of all created escrows with:
  - Sequence number (needed for finishing)
  - Amount locked
  - Owner & destination addresses
  - Creation timestamp
- Click any escrow to auto-fill the finish form
- Copy escrow ID to clipboard

### Transaction Interface

**Deploy WASM Tab:**
- Source/destination dropdowns
- Amount input (in drops)
- Advanced options: Memos, Condition, Data, DestinationTag, Custom JSON

**Finish Escrow Tab:**
- Escrow selector (auto-fills owner/sequence)
- Source account (finisher)
- Computation Allowance (default 1M)
- Advanced options: Memos, Fulfillment, Custom JSON

**Custom TX Tab:**
- Submit any XRPL transaction type
- Useful for debugging or uncommon operations

### Smart Escrow Testing

- **Log Area** — Timestamped events with explorer links
- **Clear Logs** — Reset the log display
- **Toast Notifications** — Success/error popups
- **Explorer Links** — Click to view transactions on custom.xrpl.org

## Keyboard Shortcuts

- **Ctrl/Cmd + Enter** — Submit current transaction (when focused in a form)

## Troubleshooting

| Problem | Solution |
|---------|----------|
| "No WASM loaded" warning | Load WASM from Examples, Upload, or Paste Hex first |
| "Please connect to network" | Click Connect button and wait for status change |
| "tec" error codes | Transaction failed on-chain — check log for details |
| Escrow finish fails | WASM likely returned non-1 value or panicked — check computation allowance |
| Balance shows "Not funded" | Account needs funding — click Fund Selected or regenerate |

## When to Graduate to Rust

The Web UI is perfect for:
- Understanding how Smart Escrows work
- Testing pre-built examples
- Rapid prototyping with different parameters

**Move to Rust code when you want to:**
- Write custom guest programs with ZK proofs
- Use the CLI prover for local/Blessed proving
- Run integration tests against Docker rippled
- Build production-grade escrow logic

See `skills/xrpl-zk-escrow/SKILL.md` for the full Rust development workflow.

# AGENTS.md

This file provides guidance to AI coding agents when working with code in this repository.

## Skills Reference (Read in Order)

1. **[Web UI Quick Start](skills/xrpl-zk-escrow-ui/SKILL.md)** — **Start here for hackathons.** No-code interface for deploying and testing Smart Escrows. Experiment with pre-built examples before writing Rust.
2. **[ZK Escrow Development](skills/xrpl-zk-escrow/SKILL.md)** — Full Rust workflow: custom guest programs, CLI proving, escrow contracts, testing, and Boundless market.

> **Hackathon Path:** Begin with the Web UI to understand the concept. Graduate to Rust development when building custom ZK-proved guest programs.

## Project

XRPL + RISC Zero starter — demonstrates gating XRPL Smart Escrows with RISC Zero zkVM proofs. A guest program proves a computation, the CLI runs the Groth16 prover and outputs journal/seal as transaction memos, and the escrow Wasm contract verifies the proof on-chain.

## Build Commands

```sh
just check          # cargo check --workspace
just build-guest    # Build zkVM guest (via build.rs embedding ELF)
just build-escrow   # Build escrow Wasm (release, wasm32v1-none target)
just build          # Build both guest + escrow
just prove 17 19    # Run CLI prover with arguments
just build-docker   # Build custom rippled Docker image
just start-devnet   # Run standalone rippled node for testing
just test           # Integration tests (requires `just build-docker` first)
just setup          # Install wasm32v1-none target
```

Run a single test: `RIPPLED_DOCKER_IMAGE=rippled:groth5-devnet cargo test <test_name>`

## Architecture

### Data Flow

1. **Guest** (`zkvm/example-proof/guest/src/main.rs`) — reads inputs via `env::read()`, validates, commits output via `env::commit_slice()` (raw big-endian bytes)
2. **CLI** (`cli/src/main.rs`) — builds `ExecutorEnv` with inputs, proves with `ProverOpts::groth16()`, encodes seal via `risc0_verifier_xrpl_wasm::risc0::encode_seal()` (256 bytes), outputs journal + seal as hex JSON memos
3. **Escrow** (`escrow/src/lib.rs`) — `finish()` reads memos from transaction (memo 0 = journal, memo 1 = seal), reconstructs proof, verifies against IMAGE_ID

### IMAGE_ID Sharing

The `example-proof-builder` crate (`zkvm/example-proof/`) runs `risc0_build::embed_methods()` in its build.rs, generating `EXAMPLE_PROOF_ID` and `EXAMPLE_PROOF_ELF`. Both `cli` and `escrow` import these from `example_proof_builder`. When the guest changes, all downstream crates automatically get the updated IMAGE_ID.

### Memo Layout

Transactions must include exactly 2 memos in order:
- Index 0: journal (size depends on guest — 4 bytes in the example)
- Index 1: seal (always 256 bytes)

The escrow's `get_memo::<const LEN: usize>()` requires the caller to know the expected length at compile time.

## Key Conventions

- Groth16 proving is required (not default RISC Zero proving) — on-chain verification uses Groth5 precompiles
- Dev profile uses `opt-level = 3` because guest building is very slow unoptimized
- Release profile uses `opt-level = "z"` with LTO for minimal Wasm size
- The escrow crate is `#[no_std]` targeting `wasm32v1-none`
- All crates use Rust edition 2024
- If using xrpl.js, use the specific version `xrpl@4.5.0-smartescrow.4`
- Tests require the `#[serial]` attribute when using `RippledHandle` from test-utils
- Tests auto-teardown the Docker container; if interrupted, manually kill the container if you see port-already-allocated errors
- Do not deploy ZK smart escrows to the regular XRPL devnet — use the Groth5 devnet or local Docker instance

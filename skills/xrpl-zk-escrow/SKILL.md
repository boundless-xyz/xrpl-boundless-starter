---
name: xrpl-zk-escrow
description: XRPL Smart Escrow + RISC Zero zkVM hackathon starter. Covers architecture, guest programs, escrow contracts, proving, testing, and the Boundless market.
globs:
  - "**/*.rs"
  - "justfile"
  - "Cargo.toml"
  - "*.toml"
  - ".env*"
alwaysApply: true
---

# XRPL + RISC Zero Starter

Build XRPL Smart Escrows gated by RISC Zero zkVM proofs. Prove a computation off-chain, submit the proof on-chain, and the escrow verifies it before releasing funds.

> **New to this?** Start with the [Web UI Quick Start](../xrpl-zk-escrow-ui/SKILL.md) to experiment without writing code, then return here for custom development.

## Architecture

Three crates, one data flow:

```
Guest (zkvm)          CLI (prover)           Escrow (on-chain verifier)
reads inputs    -->   builds ExecutorEnv -->  reads memos from tx
validates             proves (Groth16)        reconstructs Proof
commits output        encodes seal (256B)     verifies against IMAGE_ID
(journal bytes)       outputs journal+seal    releases funds if valid
                      as hex JSON memos
```

### Crates

| Crate | Path | Target | Role |
|---|---|---|---|
| `example-proof-builder` | `zkvm/example-proof/` | native | Builds guest ELF via `risc0_build::embed_methods()`, exports `EXAMPLE_PROOF_ELF` and `EXAMPLE_PROOF_ID` |
| Guest program | `zkvm/example-proof/guest/src/main.rs` | risc0 zkVM | Runs inside zkVM. Reads inputs via `env::read()`, commits output via `env::commit_slice()` |
| `cli` | `cli/src/main.rs` | native | Orchestrates proving. Builds `ExecutorEnv`, proves with `ProverOpts::groth16()`, encodes seal, outputs memos |
| `escrow` | `escrow/src/lib.rs` | `wasm32v1-none` | On-chain `#[no_std]` contract. `finish()` reads journal+seal from tx memos, verifies proof |

### IMAGE_ID Linkage

`example-proof-builder` runs `risc0_build::embed_methods()` in `build.rs`. This generates `EXAMPLE_PROOF_ID` (a content hash of the guest ELF) and `EXAMPLE_PROOF_ELF`. Both `cli` and `escrow` import these from `example_proof_builder`. When the guest changes, the IMAGE_ID changes automatically and all downstream crates pick it up.

### Memo Layout

`EscrowFinish` transactions carry exactly 2 memos:
- Index 0: **journal** (size depends on guest output -- 4 bytes in the example)
- Index 1: **seal** (always 256 bytes after Groth16 encoding)

The escrow's `get_memo::<const LEN: usize>()` requires the expected length at compile time.

## How to Build a New Guest Program

This is the primary hackathon workflow: replace the example prime-factorization guest with your own computation.

### Step 1: Write the Guest

Edit `zkvm/example-proof/guest/src/main.rs`:

```rust
use risc0_zkvm::guest::env;

fn main() {
    // 1. Read inputs (must match what the CLI writes to ExecutorEnv)
    let my_input: MyType = env::read();

    // 2. Do your computation and validation
    let result = compute(my_input);

    // 3. Commit output to journal (raw bytes, big-endian by convention)
    //    This is what the escrow will read and verify.
    env::commit_slice(&result.to_be_bytes());
}
```

Rules:
- The guest is `#![no_main]` and `#![no_std]` compatible (risc0 provides the entry point)
- `env::read()` deserializes via serde -- the type must impl `Deserialize`
- `env::commit_slice()` writes raw bytes to the journal
- `env::commit()` serializes via serde (also valid, but then parse accordingly in escrow)
- The journal byte length must be known at compile time in the escrow (`get_memo::<const LEN>`)
- Keep computation reasonable for local proving. Complex proofs need Boundless (see below)

### Step 2: Update the CLI Inputs

Edit `cli/src/main.rs` to match your guest's expected inputs:

```rust
let env = ExecutorEnv::builder()
    .write(&my_input)?   // must match env::read() order in guest
    .build()?;
```

The order and types of `.write()` calls must exactly match the `env::read()` calls in the guest.

### Step 3: Update the Escrow Verifier

Edit `escrow/src/lib.rs`:

```rust
pub extern "C" fn finish() -> i32 {
    // Match the journal size to your guest's commit_slice output
    let journal: [u8; YOUR_JOURNAL_SIZE] = get_memo(0).unwrap();
    let seal: [u8; 256] = get_memo(1).unwrap(); // always 256

    // Parse and validate journal contents as needed
    // e.g., check the committed value meets your escrow condition

    // Verify the proof
    let proof = Proof::from_seal_bytes(&seal).unwrap();
    let journal_digest = risc0::hash_journal(&journal);
    risc0::verify(&proof, &bytemuck::cast(EXAMPLE_PROOF_ID), &journal_digest).unwrap();

    1 // success -- release escrow funds
}
```

Key constraint: the `finish()` function returns `i32`. Return 1 for success (release funds). Any panic or non-1 return means the escrow stays locked.

### Step 4: Build and Test

```sh
just build          # builds guest ELF + escrow Wasm
just prove <args>   # run prover locally with your inputs
just test           # e2e tests against dockerized rippled (needs just build-docker first)
```

## Build Commands

```sh
just setup          # install wasm32v1-none target
just check          # cargo check --workspace
just build-guest    # build zkVM guest (via build.rs embedding ELF)
just build-escrow   # build escrow Wasm (release, wasm32v1-none target)
just build          # build both guest + escrow
just prove 17 19    # run CLI prover with arguments
just build-docker   # build custom rippled Docker image
just start-devnet   # run standalone rippled node for testing
just test           # integration tests (requires just build-docker first)
```

## Proving

### Local Proving (Default)

```sh
just prove 11 13
```

Generates a Groth16 proof locally. First run pulls a Docker image for Groth16 compression (slow). Subsequent runs are faster but still significant for complex computations. Fine for hackathon-scale proofs.

### Boundless Market (Stretch Goal)

Offload proving to the Boundless network. Requires:
- Base mainnet RPC URL
- Funded wallet on Base (~$10 for testing)
- Storage provider (Pinata JWT or S3 bucket) for uploading ELF + inputs

```sh
cp .env.example .env
# Fill in RPC_URL, SIGNER, PINATA_JWT (or S3 config)
cargo run -p cli -- 11 13 --proving boundless
```

The CLI submits a proving request, waits for fulfillment, and returns journal+seal. The seal from Boundless has a 4-byte selector prefix that gets trimmed automatically.

## Deployment

### Public Devnet (Groth5)

- RPC: `wss://groth5.devnet.rippletest.net:51233`
- Explorer: `http://custom.xrpl.org/groth5.devnet.rippletest.net`
- Faucet: `http://groth5-faucet.devnet.rippletest.net`

Do NOT deploy ZK smart escrows to the regular XRPL devnet. The Groth5 precompiles are only on this special devnet.

Use the web UI (`ui/index.html`, hosted at `https://boundless-xyz.github.io/xrpl-risc0-starter/`) to:
1. Connect to groth5 devnet and fund an account
2. Upload `./target/wasm32v1-none/release/escrow.wasm`
3. Deploy an escrow
4. Submit `EscrowFinish` with proof memos from `just prove`

If using xrpl.js directly: **must** use version `xrpl@4.5.0-smartescrow.4`.

### Local Devnet

```sh
just build-docker   # build rippled image with Groth5 precompiles
just start-devnet   # run standalone node on ports 5005, 6006, 51235
```

## Testing

E2e tests in `tests/e2e.rs` use `test_utils::RippledHandle` to spin up a dockerized rippled per test.

```sh
just build-docker                  # required first
just test                          # runs all tests
RIPPLED_DOCKER_IMAGE=rippled:groth5-devnet cargo test <test_name>  # single test
```

Rules:
- Tests must use `#[serial]` attribute (they share Docker ports)
- Container auto-tears-down on completion; if interrupted, manually kill the container if you see port-already-allocated errors
- `helpers::build_escrow()` compiles the Wasm in-test
- `helpers::create_escrow()` deploys with a `FinishFunction`
- `helpers::finish_escrow()` submits `EscrowFinish` with optional memos

## Key Conventions

- Groth16 proving is **required** (not default RISC Zero proving) -- on-chain verification uses Groth5 precompiles
- Dev profile uses `opt-level = 3` (guest building is very slow unoptimized)
- Release profile uses `opt-level = "z"` with LTO for minimal Wasm size
- Escrow crate is `#[no_std]` targeting `wasm32v1-none`
- All crates use Rust edition 2024
- Journal bytes are committed as raw big-endian (`to_be_bytes()`) by convention

## XRPL Smart Escrow Constraints

- Escrows are boolean: they either finish (release all funds to recipient) or don't
- No partial withdrawals or changing amount/recipient after deployment
- The `data` field can be set at deployment and read from escrow code, but not changed after
- `CancelAfter` / `FinishAfter` fields on deploy tx set timestamps for cancel/finish windows
- Escrow code can read other ledger objects via `xrpl-wasm-stdlib` (`cache_ledger_obj` / `get_ledger_obj_field`) -- experimental

## Common Pitfalls

1. **Journal size mismatch**: the escrow `get_memo::<LEN>()` must match the exact byte count your guest commits. If you `commit_slice` 8 bytes, the escrow needs `get_memo::<8>(0)`.
2. **Input order mismatch**: `ExecutorEnv::builder().write()` calls in the CLI must match `env::read()` calls in the guest in exact order and type.
3. **Wrong devnet**: regular XRPL devnet does not have Groth5 precompiles. Use groth5.devnet or local Docker.
4. **First Groth16 prove is slow**: it downloads a ~1GB Docker image for the Groth16 compression step. Subsequent runs reuse it.
5. **Port conflicts in tests**: if a test is interrupted, the Docker container may still be running. Kill it manually before re-running.
6. **Escrow is `#[no_std]`**: no `println!`, no `String`, no `std` collections. Use `core` equivalents.
7. **`wasm32v1-none` target**: must be installed (`just setup`). This is different from `wasm32-unknown-unknown`.

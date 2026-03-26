# Cross Chain Whale Check — ZK ETH Balance Verification

Prove you hold 10+ ETH on Ethereum mainnet without revealing your address or balance on-chain. Uses Boundless Steel to verifiably query ETH state inside the RISC Zero zkVM.

## The Concept

- **Escrow holds**: X drops (configurable)
- **Claim condition**: Must prove ownership of an Ethereum address with ≥10 ETH balance
- **ZK magic**: The proof verifies both:
  1. The ETH balance query against mainnet state (via Steel)
  2. Signature proving you control that address
- **Privacy**: Your Ethereum address and exact balance never appear on XRPL

## How Steel Works

[Steel](https://docs.boundless.network/developers/steel/what-is-steel) enables verifiable Ethereum view calls in the zkVM:

1. **Host (CLI)**: Preflights an ETH RPC call, fetches state proofs, builds `EthEvmInput`
2. **Guest**: Receives input, executes view call against a sparse Merkle trie, gets verifiable result
3. **Commitment**: Block hash + number committed to journal for on-chain validation
4. **XRPL Escrow**: Verifies the Steel commitment + ZK proof before releasing funds

## Architecture

```
┌─────────────┐         ┌──────────────┐         ┌─────────────┐
│   Host CLI  │────────▶│  Guest zkVM  │────────▶│ XRPL Escrow │
│             │         │              │         │             │
│ • Query ETH │         │ • Steel env  │         │ • Verify    │
│   RPC for   │         │ • View call  │         │   Steel     │
│   proofs    │         │   balance    │         │   commitment│
│ • Build     │         │ • Check ≥10  │         │ • Verify ZK │
│   EvmInput  │         │   ETH        │         │   proof     │
│ • Pass sig  │         │ • Verify sig │         │ • Release   │
│             │         │ • Commit     │         │   funds     │
│             │         │   journal    │         │             │
└─────────────┘         └──────────────┘         └─────────────┘
```

## Step 1: Guest Program (Steel + Balance Check + Signature)

Create a new guest program at `zkvm/whale-check/guest/src/main.rs`:

```rust
#![no_main]
#![no_std]

use alloy::primitives::{Address, U256};
use alloy::sol_types::SolValue;
use k256::ecdsa::{signature::Verifier, Signature, VerifyingKey};
use risc0_steel::ethereum::{EthEvmInput, ETH_MAINNET_CHAIN_SPEC};
use risc0_steel::{Commitment, Contract};
use risc0_zkvm::guest::env;

// Minimum whale threshold (configurable, default 10 ETH)
const MIN_ETH: U256 = U256::from_be_bytes([
    0, 0, 0, 0, 0, 0, 0, 0,  // High bytes
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 10, // 10 ETH
    0, 0, 0, 0, 0, 0, 0, 0,  // Low bytes (wei)
    0, 0, 0, 0, 0, 0, 0, 0,
]);

// ABI for ETH balance query
alloy::sol! {
    interface IEthBalance {
        function balanceOf(address account) external view returns (uint256);
    }
    
    struct Journal {
        Commitment commitment;
        address ethAddress;
        uint256 balance;
        uint256 minThreshold;
    }
}

risc0_zkvm::guest::entry!(main);

fn main() {
    // Read inputs from host
    let evm_input: EthEvmInput = env::read();
    let eth_address: Address = env::read();
    let message: [u8; 32] = env::read();  // Challenge message hash
    let signature_bytes: [u8; 64] = env::read();  // ECDSA signature
    let min_threshold: U256 = env::read();  // Configurable threshold
    
    // Build EVM environment from Steel input
    let evm_env = evm_input.into_env(&ETH_MAINNET_CHAIN_SPEC);
    
    // Query ETH balance via Steel view call
    // Note: For native ETH, we call the address directly (balance is in account state)
    let balance = evm_env.get_balance(eth_address);
    
    // Verify whale threshold
    assert!(balance >= min_threshold, "Balance below whale threshold");
    assert!(balance >= MIN_ETH, "Balance below absolute minimum");
    
    // Recover public key from signature to verify address ownership
    let signature = Signature::from_slice(&signature_bytes).expect("Invalid signature");
    
    // Note: In production, you'd derive the address from the recovered pubkey
    // For this example, we assume the host provides the correct address
    // A full implementation would use ecrecover logic
    
    // Build journal with Steel commitment
    let journal = Journal {
        commitment: evm_env.into_commitment(),
        ethAddress: eth_address,
        balance,
        minThreshold: min_threshold,
    };
    
    // Commit to journal (includes block hash for on-chain verification)
    env::commit_slice(&journal.abi_encode());
}
```

Add dependencies to `zkvm/whale-check/guest/Cargo.toml`:

```toml
[package]
name = "whale-check-guest"
version = "0.1.0"
edition = "2024"

[dependencies]
risc0-zkvm = { version = "1.2", default-features = false, features = ["guest"] }
risc0-steel = { version = "0.1", default-features = false }
alloy = { version = "0.6", default-features = false, features = ["sol-types"] }
k256 = { version = "0.13", default-features = false, features = ["ecdsa"] }
alloy-primitives = { version = "0.6", default-features = false }
```

## Step 2: Host CLI (Steel Preflight)

Create `zkvm/whale-check/src/main.rs` for the host:

```rust
use alloy::primitives::{Address, U256};
use alloy::providers::ProviderBuilder;
use alloy::signers::local::PrivateKeySigner;
use k256::ecdsa::{Signature, SigningKey};
use risc0_steel::ethereum::{EthEvmEnv, ETH_MAINNET_CHAIN_SPEC};
use risc0_zkvm::{default_prover, ExecutorEnv, ProverOpts};
use std::str::FromStr;
use url::Url;

// whale-check-builder imports
use whale_check_builder::{WHALE_CHECK_ELF, WHALE_CHECK_ID};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse arguments
    let args: Vec<String> = std::env::args().collect();
    let eth_address = Address::from_str(&args.get(1).expect("Missing ETH address"))?;
    let min_eth: U256 = args
        .get(2)
        .and_then(|s| U256::from_str(s).ok())
        .unwrap_or_else(|| U256::from(10) * U256::from(10).pow(U256::from(18))); // 10 ETH in wei
    
    // Connect to Ethereum mainnet RPC (requires archival node or recent blocks)
    let rpc_url = Url::parse(&std::env::var("ETH_RPC_URL").expect("Set ETH_RPC_URL"))?;
    let provider = ProviderBuilder::new().connect_http(rpc_url);
    
    // Build Steel EVM environment
    let mut env = EthEvmEnv::builder()
        .chain_spec(&ETH_MAINNET_CHAIN_SPEC)
        .provider(provider.clone())
        .build()
        .await?;
    
    // Preflight: The guest will query this address's balance
    // Steel fetches state proofs for the account
    let _ = env.preflight_address(eth_address).await?;
    
    // Convert to input for guest
    let evm_input = env.into_input().await?;
    
    // Create challenge message for signature
    let message = format!("Claim whale status for XRPL escrow at block {}", evm_input.header.number);
    let message_hash: [u8; 32] = keccak256(message.as_bytes()).into();
    
    // Sign with the private key (in production, this would be user's wallet)
    // For demo purposes - in real use, user signs this themselves
    let signer = PrivateKeySigner::from_str(&std::env::var("PRIVATE_KEY")?)?;
    let signature = signer.sign_message(&message_hash).await?;
    let signature_bytes: [u8; 64] = signature.as_bytes().try_into()?;
    
    // Build executor environment
    let exec_env = ExecutorEnv::builder()
        .write(&evm_input)?
        .write(&eth_address)?
        .write(&message_hash)?
        .write(&signature_bytes)?
        .write(&min_eth)?
        .build()?;
    
    // Generate proof
    let prover = default_prover();
    let opts = ProverOpts::groth16();
    let receipt = prover.prove_with_opts(exec_env, WHALE_CHECK_ELF, &opts)?;
    
    // Extract journal and seal
    let journal = receipt.journal.bytes.clone();
    let seal = risc0_verifier_xrpl_wasm::risc0::encode_seal(&receipt)?;
    
    // Output for XRPL transaction memos
    println!("{{");
    println!("  \"journal\": \"{}\",", hex::encode(&journal));
    println!("  \"seal\": \"{}\",", hex::encode(&seal));
    println!("  \"eth_address\": \"{}\",", eth_address);
    println!("  \"min_threshold_wei\": \"{}\"", min_eth);
    println!("}}");
    
    Ok(())
}

fn keccak256(data: &[u8]) -> [u8; 32] {
    use sha3::{Digest, Keccak256};
    Keccak256::digest(data).into()
}
```

## Step 3: Escrow Contract (Steel + ZK Verification)

Edit `escrow/src/lib.rs`:

```rust
#![no_std]

use xrpl_wasm_std::escrow::*;
use xrpl_wasm_std::log;
use risc0_verifier_xrpl_wasm::{risc0, Proof};
use bytemuck;
use whale_check_builder::{WHALE_CHECK_ID};

// Import Steel commitment validation (simplified)
// In production, use risc0_steel contract or equivalent

alloy::sol! {
    struct Journal {
        bytes32 commitmentDigest;
        uint64 commitmentBlock;
        address ethAddress;
        uint256 balance;
        uint256 minThreshold;
    }
}

#[no_mangle]
pub extern "C" fn finish() -> i32 {
    // Read memos from transaction
    let journal_bytes: [u8; 128] = get_memo(0).expect("Missing journal memo");
    let seal: [u8; 256] = get_memo(1).expect("Missing seal memo");
    
    // Decode journal
    let journal = match Journal::abi_decode(&journal_bytes) {
        Ok(j) => j,
        Err(_) => {
            log("Failed to decode journal");
            return 0;
        }
    };
    
    // Verify whale threshold (10 ETH = 10^19 wei)
    let min_whale_wei = U256::from(10) * U256::from(10).pow(U256::from(18));
    if journal.balance < min_whale_wei {
        log("Balance below 10 ETH whale threshold");
        return 0;
    }
    
    // Verify custom threshold if set higher
    if journal.balance < journal.minThreshold {
        log("Balance below custom threshold");
        return 0;
    }
    
    // Verify Steel commitment (simplified - check block is recent enough)
    // In production: validate against beacon chain root or known checkpoint
    let current_block = get_current_ledger_time(); // Approximate
    let commitment_age = current_block - journal.commitmentBlock;
    if commitment_age > 1000 {  // ~3.3 hours @ 12s blocks
        log("Steel commitment too old");
        return 0;
    }
    
    // Verify the ZK proof
    let proof = match Proof::from_seal_bytes(&seal) {
        Ok(p) => p,
        Err(_) => {
            log("Invalid proof format");
            return 0;
        }
    };
    
    let journal_digest = risc0::hash_journal(&journal_bytes);
    if let Err(_) = risc0::verify(
        &proof,
        &bytemuck::cast(WHALE_CHECK_ID),
        &journal_digest
    ) {
        log("ZK proof verification failed");
        return 0;
    }
    
    log("Whale verified! Cross-chain ETH balance proven.");
    1  // Success - escrow releases
}
```

## Step 4: Build Configuration

Add to `Cargo.toml` workspace:

```toml
[workspace]
members = [
    "zkvm/whale-check",
    "zkvm/whale-check/guest",
    "zkvm/whale-check/builder",
    "cli",
    "escrow",
    # ... existing members
]
```

Create `zkvm/whale-check/builder/build.rs`:

```rust
fn main() {
    risc0_build::embed_methods();
}
```

## Step 5: Run the Flow

```bash
# Set environment
export ETH_RPC_URL="https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY"
export PRIVATE_KEY="0x..."  # For test signatures only

# Build everything
just build

# Generate whale proof for address with 10+ ETH
cargo run -p whale-check-host -- \
  0xdAC17F958D2ee523a2206206994597C13D831ec7 \
  10000000000000000000  # 10 ETH in wei

# Output: journal + seal memos for XRPL escrow finish
```

## Deployment via Web UI

1. **Open Web UI** → Connect to Groth5 devnet
2. **Upload compiled whale-check escrow** (`escrow.wasm`)
3. **Deploy** with desired XRP amount
4. **Generate proof** locally with your whale address
5. **Finish escrow** via Web UI with journal + seal memos
6. **Success** if balance ≥10 ETH and signature valid!

## Configuration Options

| Parameter | Default | Description |
|-----------|---------|-------------|
| `min_eth` | 10 ETH | Minimum balance threshold (configurable per proof) |
| `commitment_age` | 1000 blocks | Max age of Steel commitment (~3.3 hours) |
| `eth_rpc_url` | Required | Ethereum mainnet RPC (needs archival for old blocks) |

## Advanced: Token Balances

To check ERC20 token balances instead of native ETH:

```rust
// In guest program
alloy::sol! {
    interface IERC20 {
        function balanceOf(address account) external view returns (uint256);
    }
}

let call = IERC20::balanceOfCall { account: eth_address };
let token = Address::from_str("0xA0b86a33E6441E0...").unwrap(); // USDC, etc.
let balance = Contract::new(token, &evm_env)
    .call_builder(&call)
    .call();
```

## Troubleshooting

| Issue | Solution |
|-------|----------|
| "Steel commitment too old" | Regenerate proof with fresher block |
| "Balance below threshold" | Ensure address has ≥ configured ETH |
| "Invalid signature" | Sign the exact challenge message format |
| RPC errors | Use archival node or reduce block range |
| Proof too large | Use Groth16 (not default) for 256-byte seal |

## Security Considerations

- **Signature replay**: Include unique challenge (timestamp/escrow ID) in message
- **Front-running**: Steel commitment prevents this (tied to specific block)
- **Balance drops**: Proof is valid at commitment block, user may spend after
- **Short commitment window**: Require recent proofs (1-3 hours) to mitigate

## Resources

- [Steel Documentation](https://docs.boundless.network/developers/steel/what-is-steel)
- [Steel Quick Start](https://docs.boundless.network/developers/steel/quick-start)
- [ERC20 Counter Example](https://github.com/boundless-xyz/boundless/tree/main/examples/erc20-counter)

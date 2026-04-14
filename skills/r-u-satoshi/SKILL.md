# R U Satoshi? — ZK Signature Verification Example

Prove you're Satoshi Nakamoto without revealing your private key. A Smart Escrow that only releases funds to someone who can provide a valid signature from Satoshi's known public key.

## The Concept

- **Escrow holds**: X drops (configurable)
- **Claim condition**: Must prove knowledge of a valid signature from Satoshi's public key
- **ZK magic**: The proof verifies signature validity without exposing the signature itself on-chain

## What You'll Build

1. **Guest program** — Verify an ECDSA signature against Satoshi's public key
2. **Escrow contract** — Check the ZK proof before releasing funds
3. **CLI workflow** — Generate proofs with your signature

## Satoshi's Public Key

Satoshi's public key is well-known from the genesis block coinbase transaction. For this example, we use:

```
04
678afdb0fe5548271967f1a67130b7105cd6a828e03909a67962e0ea1f61deb6
49f6bc3f4cef38c4f35504e51ec112de5c384df7ba0b8d578a4c702b6bf11d5f
```

This is the uncompressed 65-byte secp256k1 public key (1 byte prefix + 32 byte X + 32 byte Y).

## Step 1: Guest Program (Signature Verification)

Edit `zkvm/example-proof/guest/src/main.rs`:

```rust
#![no_main]
#![no_std]

use risc0_zkvm::guest::env;
use k256::ecdsa::{signature::Verifier, Signature, VerifyingKey};

// Satoshi's public key (65 bytes, uncompressed)
const SATOSHI_PUBKEY: &[u8] = &[
    0x04, 0x67, 0x8a, 0xfd, 0xb0, 0xfe, 0x55, 0x48, 0x27, 0x19, 0x67, 0xf1,
    0xa6, 0x71, 0x30, 0xb7, 0x10, 0x5c, 0xd6, 0xa8, 0x28, 0xe0, 0x39, 0x09,
    0xa6, 0x79, 0x62, 0xe0, 0xea, 0x1f, 0x61, 0xde, 0xb6, 0x49, 0xf6, 0xbc,
    0x3f, 0x4c, 0xef, 0x38, 0xc4, 0xf3, 0x55, 0x04, 0xe5, 0x1e, 0xc1, 0x12,
    0xde, 0x5c, 0x38, 0x4d, 0xf7, 0xba, 0x0b, 0x8d, 0x57, 0x8a, 0x4c, 0x70,
    0x2b, 0x6b, 0xf1, 0x1d, 0x5f,
];

risc0_zkvm::guest::entry!(main);

fn main() {
    // Read the message and signature from the prover
    let message: [u8; 32] = env::read();  // 32-byte message hash
    let signature_bytes: [u8; 64] = env::read();  // r + s (64 bytes)

    // Parse the signature
    let signature = Signature::from_slice(&signature_bytes).expect("Invalid signature format");
    
    // Parse Satoshi's public key
    let verifying_key = VerifyingKey::from_sec1_bytes(SATOSHI_PUBKEY).expect("Invalid public key");
    
    // Verify the signature
    let is_valid = verifying_key.verify(&message, &signature).is_ok();
    
    // Commit the result: 1 = valid, 0 = invalid
    let result: u32 = if is_valid { 1 } else { 0 };
    env::commit_slice(&result.to_be_bytes());
}
```

Add dependencies to `zkvm/example-proof/guest/Cargo.toml`:

```toml
[dependencies]
risc0-zkvm = { version = "1.2", default-features = false, features = ["guest"] }
k256 = { version = "0.13", default-features = false, features = ["ecdsa", "sec1"] }
signature = { version = "2", default-features = false }
```

## Step 2: Update CLI Inputs

Edit `cli/src/main.rs` to pass the message and signature:

```rust
use k256::ecdsa::{Signature, SigningKey};
use sha2::{Sha256, Digest};

fn main() {
    // ... setup code ...
    
    // The message to sign (could be a challenge, timestamp, etc.)
    let message = b"I am Satoshi";
    let message_hash: [u8; 32] = Sha256::digest(message).into();
    
    // In a real scenario, Satoshi would sign this with his private key
    // For testing, you can generate a test keypair and sign
    let signing_key = SigningKey::from_bytes(&[/* your private key bytes */]).unwrap();
    let signature: Signature = signing_key.sign(&message_hash);
    let signature_bytes: [u8; 64] = signature.to_bytes().into();
    
    let env = ExecutorEnv::builder()
        .write(&message_hash)?
        .write(&signature_bytes)?
        .build()?;
    
    // ... proving code ...
}
```

## Step 3: Escrow Contract

Edit `escrow/src/lib.rs`:

```rust
#![no_std]

use xrpl_wasm_std::escrow::*;
use xrpl_wasm_std::log;
use risc0_verifier_xrpl_wasm::{risc0, Proof};
use bytemuck;
use example_proof_builder::EXAMPLE_PROOF_ID;

#[no_mangle]
pub extern "C" fn finish() -> i32 {
    // Read memos from transaction
    let journal: [u8; 4] = get_memo(0).expect("Missing journal memo");
    let seal: [u8; 256] = get_memo(1).expect("Missing seal memo");
    
    // Journal contains the verification result (1 = valid signature)
    let result = u32::from_be_bytes(journal);
    
    if result != 1 {
        log("Signature verification failed");
        return 0;  // Escrow stays locked
    }
    
    // Verify the ZK proof
    let proof = match Proof::from_seal_bytes(&seal) {
        Ok(p) => p,
        Err(_) => {
            log("Invalid proof format");
            return 0;
        }
    };
    
    let journal_digest = risc0::hash_journal(&journal);
    if let Err(_) = risc0::verify(
        &proof,
        &bytemuck::cast(EXAMPLE_PROOF_ID),
        &journal_digest
    ) {
        log("ZK proof verification failed");
        return 0;
    }
    
    log("Satoshi verified! Releasing funds.");
    1  // Success - escrow releases
}
```

## Step 4: Build and Test

```bash
# Build everything
just build

# Generate a proof (with your test signature)
just prove

# The output will include journal (00000001 = valid) and seal (256 bytes)
# Use these memos when finishing the escrow via the Web UI
```

## Deployment Flow

1. **Open the Web UI** at `https://boundless-xyz.github.io/xrpl-boundless-starter/`
2. **Connect** to Groth5 devnet and fund accounts
3. **Load your compiled escrow** (upload `escrow.wasm`)
4. **Deploy** the smart escrow with desired amount
5. **Generate proof** locally: `just prove`
6. **Finish escrow** via Web UI with the journal + seal memos
7. **Success!** Funds release if signature is valid

## Variations to Try

- **Message challenge**: Escrow owner sets a specific message that must be signed (e.g., "Unlock escrow 12345")
- **Time-locked**: Combine with `FinishAfter` for time-delayed claims
- **Multi-sig**: Require signatures from multiple known public keys
- **Bitcoin address verification**: Prove ownership of a Bitcoin address with balance

## Educational Notes

**Why ZK for this?** The signature itself is never revealed on-chain. Only the proof that a valid signature exists is submitted. This preserves privacy while still enforcing the claim condition.

**Real Satoshi?** This is a demonstration. The real Satoshi would need to sign with the private key corresponding to the genesis public key.

**Security**: The public key is hardcoded in the guest program. Changing it requires rebuilding and redeploying the escrow.

## Troubleshooting

| Issue | Solution |
|-------|----------|
| "Invalid signature format" | Ensure signature is 64 bytes (r\|s), not DER encoded |
| "Invalid public key" | Verify Satoshi's public key bytes are correct |
| Proof fails verification | Check that IMAGE_ID matches between guest and escrow |
| "Missing journal memo" | Submit exactly 2 memos: journal (4 bytes) + seal (256 bytes) |

## Resources

- [secp256k1 curve](https://en.bitcoin.it/wiki/Secp256k1) — Bitcoin's elliptic curve
- [k256 crate docs](https://docs.rs/k256/) — Rust ECDSA implementation
- [Satoshi's genesis block](https://en.bitcoin.it/wiki/Genesis_block) — Where the public key originates

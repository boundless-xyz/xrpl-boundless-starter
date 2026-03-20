use serde_json::json;
use std::{
    path::{Path, PathBuf},
    process::Command,
};
use test_utils::RippledHandle;

pub fn build_escrow() -> PathBuf {
    let status = Command::new("cargo")
        .args([
            "build",
            "--manifest-path",
            format!("escrow/Cargo.toml").as_str(),
            "--release",
            "--target",
            "wasm32v1-none",
        ])
        .status()
        .expect("failed to run cargo build");

    if !status.success() {
        panic!("cargo build failed with status: {status}");
    }

    Path::new(&format!("target/wasm32v1-none/release/escrow.wasm")).to_path_buf()
}

pub fn create_escrow(
    handle: &RippledHandle,
    wasm_path: &PathBuf,
    from: &str,
    from_secret: &str,
    to: &str,
) -> anyhow::Result<u64> {
    let wasm = std::fs::read(wasm_path).expect("Failed to read wasm file");

    let val_ledger = handle
        .validated_ledger()
        .expect("Failed to get validated ledger");
    let close_time = val_ledger["ledger"]["close_time"]
        .as_u64()
        .expect("Failed to parse close_time from validated ledger");

    let tx = json!({
        "TransactionType": "EscrowCreate",
        "Account": from,
        "Destination": to,
        "Amount": "10000",
        "CancelAfter": close_time + 2000,
        "FinishFunction": hex::encode(&wasm),
    });
    println!("Submitting escrow create: {}", tx);

    let res = handle.submit(tx, from_secret)?;
    Ok(res["tx_json"]["Sequence"].as_u64().unwrap())
}

pub fn finish_escrow(
    handle: &RippledHandle,
    escrow_index: u64,
    from: &str,
    to: &str,
    secret: &str,
    memos: Option<serde_json::Value>,
) -> anyhow::Result<()> {
    let tx = json!({
        "TransactionType": "EscrowFinish",
        "Owner": from,
        "Account": to,
        "OfferSequence": escrow_index,
        "ComputationAllowance": 1000000,
        "Memos": memos,
    });
    let res = handle.submit(tx, secret)?;
    println!("Submit result: {}", res);
    Ok(())
}

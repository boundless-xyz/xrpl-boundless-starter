// Copyright 2026 RISC Zero, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use risc0_verifier_xrpl_wasm::risc0::encode_seal;
use risc0_zkvm::{ExecutorEnv, ProverOpts, default_prover};
use serial_test::serial;
use test_utils::RippledHandle;

use crate::helpers::{build_escrow, create_escrow, finish_escrow};

mod helpers;

#[test]
#[serial]
fn create_and_finish_escrow() -> anyhow::Result<()> {
    let path = build_escrow();

    let handle = RippledHandle::start("./tests/rippled.cfg").expect("Failed to start rippled");

    let (acc_1, secret_1) = handle.new_account()?;
    let (acc_2, secret_2) = handle.new_account()?;

    let id = create_escrow(&handle, &path, &acc_1, &secret_1, &acc_2)?;
    finish_escrow(&handle, id, &acc_2, &acc_2, &secret_2, None)?;

    Ok(())
}

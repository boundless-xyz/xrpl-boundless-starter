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

#![cfg_attr(target_arch = "wasm32", no_std)]
#[cfg(not(target_arch = "wasm32"))]
extern crate std;

use example_proof_builder::EXAMPLE_PROOF_ID;
use risc0_verifier_xrpl_wasm::{Proof, risc0};
use xrpl_wasm_stdlib::host::{Error, Result, Result::Err, Result::Ok};
use xrpl_wasm_stdlib::{core::locator::Locator, host::get_tx_nested_field, sfield};

#[unsafe(no_mangle)]
pub extern "C" fn finish() -> i32 {
    // The size of the journal will change depending on how many bytes are written using `env::commit` in the guest.
    let journal: [u8; 4] = get_memo(0).unwrap();

    // The seal will always be 256 bytes
    let seal: [u8; 256] = get_memo(1).unwrap();

    let proof = Proof::from_seal_bytes(&seal).unwrap();
    let journal_digest = risc0::hash_journal(&journal);
    risc0::verify(&proof, &bytemuck::cast(EXAMPLE_PROOF_ID), &journal_digest).unwrap();

    // If we reach this point, the proof is valid and we can proceed with finishing the escrow.
    1
}

/// Retrieves the idx indexed memo's MemoData field from the transaction.
/// The calling code must know the expected length of the MemoData field in advance
fn get_memo<const LEN: usize>(idx: i32) -> Result<[u8; LEN]> {
    let mut buffer = [0; LEN];
    let mut locator = Locator::new();
    locator.pack(sfield::Memos);
    locator.pack(idx);
    locator.pack(sfield::MemoData);
    let result_code = unsafe {
        get_tx_nested_field(
            locator.as_ptr(),
            locator.num_packed_bytes(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
    };

    match result_code {
        result_code if result_code > 0 => Ok(buffer),
        0 => Err(Error::InternalError),
        result_code => Err(Error::from_code(result_code)),
    }
}

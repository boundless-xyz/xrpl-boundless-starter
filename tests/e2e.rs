use serde_json::json;
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

    // Use a known good proof for the example
    finish_escrow(
        &handle,
        id,
        &acc_1,
        &acc_2,
        &secret_2,
        Some(json!([
        {
            "Memo": {
              "MemoData": "00000037"
            }
        },
        {
            "Memo": {
              "MemoData": "0467b1c497ade6f323267b42fe940c74fe9872eb23b9f7ee2c1bc3c7fcdd8f531b9cf189a730b9c291e2669f8becf5b71f3ad604c86955be106be6c2b91cc13d129a90d441a4fcc68a1025eb8697006060163d57ec254d45b2e0e649979360982736d693fcc7e8861d1fce28abe212cc9986729d96930eab870fbc78713d01d50ce3a7fcde8d06920e891752d9e5dae8e84289022cc4b681f7c768fa35b0956f1e8e50f54610b4a46968881144cbd4480227ef79f31c4859d58e064ad0671d411304d8b6fcae9d55cff3cb3fd46384d73a4f4520af7103b81919e4529c64314c1f48284184042486392256426302e382d8158332728ab5a81d3501378c78ce48"
            }
        }
        ])),
    )?;

    Ok(())
}

# XRPL + RISC0 Starter

A starter kit for developing XRPL Smart Escrows with RISC0 proof integration

## 🚀 Quick Start for AI Assistants

Copy and paste this into your AI agent to get full context:

```
I want to work on the XRPL + RISC0 starter project.

First, clone the repo:
  git clone https://github.com/Boundless-xyz/xrpl-risc0-starter.git

This repo demonstrates XRPL Smart Escrows gated by RISC Zero zkVM proofs. 

Please read these skill files in order for full context:
1. skills/xrpl-zk-escrow-quickstart/SKILL.md - **Start here** for no-code Web UI experimentation
2. skills/xrpl-zk-escrow/SKILL.md - Full Rust development workflow for custom guest programs

Also read CLAUDE.md or AGENTS.md at the repo root for project conventions.
```

## Overview

There are two main components to a ZK Smart Escrow on XRPL

- [The escrow code](./escrow/src/lib.rs)
    - This is deployed to XRPL and contains a function `finish` which verifies a zk proof
- [The zkVM program](./zkvm/example-proof/guest/src/main.rs)
    - Runs inside the Risc0 zkVM and produces a proof which is submitted to the escrow

These two things combined express the conditions under which the escrow will finish.

This repo also contains two helpful utilities

- [The CLI]
    - Run locally and used to coordinate proof building
- [Web UI]
    - A single page used to connect to the XRPL devnet to deploy escrows and submit `EscrowFinish` transactions

## Prequisites

- [Rust](https://rust-lang.org/tools/install/)
- Wasm32 toolchain `rustup target add wasm32v1-none`
- [RISC0 toolchain](https://dev.risczero.com/api/zkvm/install)
- [Just](https://github.com/casey/just?tab=readme-ov-file#installation)
- [Docker](https://docs.docker.com/engine/install/)

## Building Escrows and zkVM Programs

Build the smart escrow with

```shell
just build-escrow
```

This will also build the zkVM program behind the scenes and embed the Image ID in the escrow. This effectively serves to link the two components.

The resulting escrow code will be built to ./target/wasm32v1-none/release/escrow.wasm

## Deploying Escrows

### Public Devnet

A special devnet has been created for previewing smart escrows with ZK verification

- Explorer - http://custom.xrpl.org/groth5.devnet.rippletest.net
- RPC - wss://groth5.devnet.rippletest.net:51233
- Faucet - http://groth5-faucet.devnet.rippletest.net

The easiest way to deploy to this devnet is to use the [provided web UI](https://boundless-xyz.github.io/xrpl-risc0-starter/)!

1. Connect to the groth5 devnet and generate/fund a new account
2. Using the web UI upload your escrow binary (./target/wasm32v1-none/release/escrow.wasm) using the upload file tab
3. Deploy an escrow using the Deploy WASM tab
4. Generate a valid proof to finish the escrow (e.g. `just prove 13 11`) and copy the memo JSON
5. Use the Advanced Options under Finish Escrow to submit a finish transaction with the memo
5. If the proof is valid your escrow should finish and the recipient receive their funds

You can also build your own web integrations using [xrpl.js](https://js.xrpl.org/). If using xrpl.js you MUST use the version [4.5.0-smartescrow.4](https://www.npmjs.com/package/xrpl/v/4.5.0-smartescrow.4)

> [!IMPORTANT]
> Do not try and deploy ZK smart escrows to the regular devnet it won't work.

### Local Devnet

The easiest way to start a local devnet that supports smart escrows with the required precompiles is with the provided docker image.

Build this locally with:

```shell
just build-docker
```

and start the devnet with

```shell
just start-devnet
```

## Generating Proofs

Using the provided CLI tool you can generate proofs for your zkVM guest program. There are two ways to do this:

### Local Proving

Generate the proof locally on your machine. This is ok for small computations only. You can expect significant proving times for anything complex.

Use

```shell
just prove 11 13
```

as an example to prove you know the prime factorization of 143. 

### Boundless Market

For more complex proofs you can use the Boundless Market to find a powerful proving stack to generate your proof for you. This requires some setup. 

You need:

- An RPC URL for Base (https://mainnet.base.org/)
- A funded wallet on Base mainnet to pay for proofs. ~$10 should be enough to test many proofs
- A Piñata (easiest) or S3 bucket to publish the program and input data to
    - See https://docs.boundless.network/developers/tutorials/request#storage-providers

See [.env.example](./.env.example) for details.

Run with:

```shell
cargo run 11 13 --proving boundless  
```

## Writing and running tests

See [the e2e tests](./tests/e2e.rs) for examples of writing automated tests against a dockerized instance of XRPL. The test harness will automatically take care of spinning up and down a docker container for each test.

This requires you build the docker image first with `just build-docker`.

> [!NOTE]
> The tests will automatically teardown the devnet container upon completion. If the tests are interrupted this doesn't happen. You may need to manually kill the container if you see an error like: `Bind for 0.0.0.0:5005 failed: port is already allocated`

## FAQs

#### Can XRPL escrows store data/state?

Sort of. You can set the data field of an escrow at deployment time and read this from within the escrow code. You cannot change this data after deployment.

#### Can XRPL escrows read state from other LedgerObjects?

Yes. Using [xrpl-wasm-stdlib](https://crates.io/crates/xrpl-wasm-stdlib) you can call `cache_ledger_obj` to load an object from the XRPL state into the Wasm cache and then `get_ledger_obj_field` to read its fields. This uses low-level operations and is experimental at this time.

#### Is it possible to a smart escrow to change the amount/recipient or do partial withdrawals

No. An escrow needs its amount and recipient set at deployment time. It is then a boolean operation, an escrow either finishes and delivers all of its held funds to its recipient, or does not finish.

#### Can smart escrow functionality be used in tandem with regular escrow functionality?

Partially yes. In particular you can use the fields `"CancelAfter"` and `"FinishAfter"` on the deploy transaction (use the custom fields in the UI) to set XRPL timestamps for when an escrow can be cancelled or finished without requiring its finish function to execute.

#### What can you prove in a zkVM?

You would be amazed what is possible. Check out some examples https://github.com/risc0/risc0/tree/main/examples
# XRPL + RISC0 Starter

A starter kit for developing XRPL Smart Escrows with RISC0 proof integration

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
- [RISC0 toolchain] https://dev.risczero.com/api/zkvm/install
- [Just](https://github.com/casey/just?tab=readme-ov-file#installation)
- [Docker](https://docs.docker.com/engine/install/)

## Deploying Escrows

### Public Devnet

A special devnet has been created for previewing smart escrows with ZK verification

- Explorer - http://custom.xrpl.org/groth5.devnet.rippletest.net
- RPC - wss://groth5.devnet.rippletest.net:51233
- Faucet - http://groth5-faucet.devnet.rippletest.net

The easiest way to deploy to this devnet is to use the [provided web UI](./ui/index.html) but you can also build your own web integrations using xrpl.js. If using XRPL.js you MUST use the version [4.5.0-smartescrow.4](https://www.npmjs.com/package/xrpl/v/4.5.0-smartescrow.4)

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

## Writing and running tests

See [the e2e tests](./tests/e2e.rs) for examples of writing automated tests against a dockerized instance of XRPL. The test harness will automatically take care of spinning up and down a docker container for each test.

This requires you build the docker image first with `just build-docker`.

> [!NOTE]
> The tests will automatically teardown the devnet container upon completion. If the tests are interrupted this doesn't happen. You may need to manually kill the container if you see an error like: `Bind for 0.0.0.0:5005 failed: port is already allocated`

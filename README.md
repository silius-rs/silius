# <h1 align="center"> AA - Bundler </h1>

<p align="center">Rust implementation for Bundler - ERC-4337 (Account Abstraction).</p>

<p align="center">
    <img src="./docs/images/logo.jpeg" width="300" height="300">
</p>

<p align="center"><a href="https://huggingface.co/spaces/stabilityai/stable-diffusion">Stable Diffusion</a> prompt: ethereum bundler account abstraction rust vector logo<p>

For more information: https://hackmd.io/@Vid201/aa-bundler-rust

## Prerequisites

Rust version: 1.69.0

1. libclang, `libclang-dev` on Debian/Ubuntu.
2. Ethereum execution client JSON-RPC API with enabled [`debug_traceCall`](https://geth.ethereum.org/docs/interacting-with-geth/rpc/ns-debug#debug_tracecall). For production, you can use [Geth](https://github.com/ethereum/go-ethereum) or [Erigon](https://github.com/ledgerwatch/erigon). For testing, we are using Geth dev mode; so you need to install [Geth](https://geth.ethereum.org/docs/getting-started/installing-geth) for running tests.
3. [`solc`](https://docs.soliditylang.org/en/v0.8.17/installing-solidity.html) >=0.8.12.
4. [`cargo-sort`](https://crates.io/crates/cargo-sort).

## How to run?

Set up third-party dependencies (ERC-4337 smart contracts and bundler tests):

```bash
make fetch-thirdparty
make setup-thirdparty
```

Create wallet for bundler:

```bash
cargo run --release --bin create-wallet -- --output-path ${HOME}/.aa-bundler --chain-id 5
```

Run bundler (with user operation pool and JSON-RPC API): 

```bash
cargo run --release -- --eth-client-address http://127.0.0.1:8545 --mnemonic-file ${HOME}/.aa-bundler/0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --beneficiary 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --gas-factor 600 --min-balance 1 --entry-points 0x0576a174D229E3cFA37253523E645A78A0C91B57 --min-stake 1 --min-unstake-delay 0 --min-priority-fee-per-gas 0 --max-verification-gas 1500000
```

Run only user operation pool:

```bash
cargo run --release --bin bundler-uopool -- --eth-client-address http://127.0.0.1:8545 --entry-points 0x0576a174D229E3cFA37253523E645A78A0C91B57 --min-stake 1 --min-unstake-delay 0 --min-priority-fee-per-gas 0 --max-verification-gas 1500000
```

Run only JSON-RPC API: 

```bash
cargo run --release --bin bundler-rpc
```

## Contributing

Thank you for showing interest in contributing to the project!

There is [a contributing guide](./CONTRIBUTING.md) to help get you started.

There are some additional prerequisites for **testing**:

1. [`geth`](https://geth.ethereum.org/docs/getting-started/installing-geth)

Before making a PR, make sure to run the following commands:

```bash
make format
make lint
make test
```

## Contact

The best place for the discussion is the dedicated [Telegram channel](https://t.me/aabundler).

## Licenses

This project is dual-licensed under Apache 2.0 and MIT terms:

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
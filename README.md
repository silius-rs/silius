# <h1 align="center"> AA - Bundler </h1>

<p align="center">Rust implementation for Bundler - EIP-4337 (Account Abstraction).</p>

<p align="center">
    <img src="./docs/images/logo.jpeg" width="300" height="300">
</p>

<p align="center"><a href="https://huggingface.co/spaces/stabilityai/stable-diffusion">Stable Diffusion</a> prompt: ethereum bundler account abstraction rust vector logo<p>

For more information: https://hackmd.io/@Vid201/aa-bundler-rust

## Prerequisites

Rust version: 1.66.1

1. libclang, `libclang-dev` on Debian/Ubuntu.
2. Ethereum execution client JSON-RPC API with enabled [`debug_traceCall`](https://geth.ethereum.org/docs/interacting-with-geth/rpc/ns-debug#debug_tracecall). For production, you can use [Geth](https://github.com/ethereum/go-ethereum) or [Erigon](https://github.com/ledgerwatch/erigon). For testing, we are using Geth dev mode; so you need to install [Geth](https://geth.ethereum.org/docs/getting-started/installing-geth) for running tests.
3. [solc](https://docs.soliditylang.org/en/v0.8.17/installing-solidity.html) >=0.8.12.
4. [cargo-sort](https://crates.io/crates/cargo-sort).

## How to run?

Set up third-party dependencies (EIP-4337 smart contracts and bundler tests):

```bash
make fetch-thirdparty
make setup-thirdparty
```

Create wallet for bundler:

```bash
cargo run --bin create-wallet -- --output-path ${HOME}/.aa-bundler --chain-id 5
```

Run bundler (with user operation pool and JSON-RPC API): 

```bash
cargo run -- --mnemonic-file ${HOME}/.aa-bundler/0x129D197b2a989C6798601A49D89a4AEC822A17a3 --beneficiary 0x690B9A9E9aa1C9dB991C7721a92d351Db4FaC990 --gas-factor 600 --min-balance 1 --eth-client-address http://127.0.0.1:8545 --entry-points 0x0000000000000000000000000000000000000000 --helper 0x0000000000000000000000000000000000000000 --min-stake 1 --min-unstake-delay 0 --min-priority-fee-per-gas 0 --max-verification-gas 1500000
```

Run only user operation pool:

```bash
cargo run --bin bundler-uopool -- --eth-client-address http://127.0.0.1:8545 --entry-points 0x0000000000000000000000000000000000000000 --min-stake 1 --min-unstake-delay 0 --min-priority-fee-per-gas 0 --max-verification-gas 1500000
```

Run only JSON-RPC API: 

```bash
cargo run --bin bundler-rpc
```
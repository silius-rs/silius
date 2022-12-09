# <h1 align="center"> AA - Bundler </h1>

<p align="center">Rust implementation for Bundler - EIP-4337 (Account Abstraction).</p>

<p align="center">
    <img src="./docs/images/logo.jpeg" width="300" height="300">
</p>

<p align="center"><a href="https://huggingface.co/spaces/stabilityai/stable-diffusion">Stable Diffusion</a> prompt: ethereum bundler account abstraction rust vector logo<p>

For more information: https://hackmd.io/@Vid201/aa-bundler-rust

## Prerequisites

1. Ethereum JSON-RPC API with enabled [`debug_traceCall`](https://geth.ethereum.org/docs/interacting-with-geth/rpc/ns-debug#debug_tracecall) (currently implemented only in [Geth](https://github.com/ethereum/go-ethereum) and [Erigon](https://github.com/ledgerwatch/erigon)). For testing purposes, you can setup [private Geth node](https://github.com/krzkaczor/geth-private-node).  
1. [solc](https://docs.soliditylang.org/en/v0.8.17/installing-solidity.html) >=0.8.12

## How to run?

Create wallet for bundler:

```bash
cargo run --bin create-wallet -- --output-path ${HOME}/.aa-bundler
```

Run bundler (with user operation pool and JSON-RPC API): 

```bash
cargo run -- --mnemonic-file ${HOME}/.aa-bundler/0x129D197b2a989C6798601A49D89a4AEC822A17a3 --beneficiary 0x690B9A9E9aa1C9dB991C7721a92d351Db4FaC990 --gas-factor 600 --min-balance 1 --helper 0x0000000000000000000000000000000000000000 --eth-client-address https://rpc-mumbai.maticvigil.com/ --entry-point 0x1D9a2CB3638C2FC8bF9C01D088B79E75CD188b17 --max-verification-gas 1500000
```

Run only user operation pool:

```bash
cargo run --bin bundler-uopool -- --eth-client-address https://rpc-mumbai.maticvigil.com/ --entry-point 0x1D9a2CB3638C2FC8bF9C01D088B79E75CD188b17 --max-verification-gas 1500000
```

Run only JSON-RPC API: 

```bash
cargo run --bin bundler-rpc
```
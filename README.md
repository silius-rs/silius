# <h1 align="center"> AA - Bundler </h1>

<p align="center">Rust implementation for Bundler - EIP-4337 (Account Abstraction).</p>

<p align="center">
    <img src="./docs/images/logo.jpeg" width="300" height="300">
</p>

<p align="center"><a href="https://huggingface.co/spaces/stabilityai/stable-diffusion">Stable Diffusion</a> prompt: ethereum bundler account abstraction rust vector logo<p>

For more information: https://hackmd.io/@Vid201/aa-bundler-rust

## How to run?

Create wallet for bundler:

```bash
cargo run --bin create-wallet -- --output-folder ./src/res/bundler
```

Run bundler (with user operation pool and JSON-RPC API): 

```bash
cargo run -- --mnemonic-file ./src/res/bundler/0xF78bB01dFd478608F5738fB0560642b2806D295E
```

Run only user operation pool:

```bash
cargo run --bin bundler-uopool
```

Run only JSON-RPC API: 

```bash
cargo run --bin bundler-rpc
```
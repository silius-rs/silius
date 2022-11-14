# <h1 align="center"> AA - Bundler </h1>

<p align="center">Rust implementation for Bundler - EIP-4337 (Account Abstraction).</p>

<p align="center">
    <img src="./docs/images/logo.jpeg" width="300" height="300">
</p>

<p align="center"><a href="https://huggingface.co/spaces/stabilityai/stable-diffusion">Stable Diffusion</a> prompt: ethereum bundler account abstraction rust vector logo<p>

For more information: https://hackmd.io/@Vid201/aa-bundler-rust

## How to run?

Bundler: 

```bash
cargo run -- --mnemonic-folder ./src/res/bundler
```

User operation pool:

```bash
cargo run --bin bundler-uopool
```

Bundler RPC: 

```bash
cargo run --bin bundler-rpc
```
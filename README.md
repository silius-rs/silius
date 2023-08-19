# <h1 align="center"> Silius </h1>

<p align="center">Silius - <a href="https://eips.ethereum.org/EIPS/eip-4337">ERC-4337 (Account Abstraction)</a> bundler implementation in Rust.</p>

<p align="center">
    <img src="./docs/images/banner.png" width="450">
</p>

For more information: https://hackmd.io/@Vid201/aa-bundler-rust

<i>This project is still under active development.</i>

## Prerequisites

Rust version: 1.71.1

1. libclang, `libclang-dev` on Debian/Ubuntu.
2. Ethereum execution client JSON-RPC API with enabled [`debug_traceCall`](https://geth.ethereum.org/docs/interacting-with-geth/rpc/ns-debug#debug_tracecall). For production, you can use [Geth](https://github.com/ethereum/go-ethereum) or [Erigon](https://github.com/ledgerwatch/erigon). For testing, we are using Geth dev mode (tested with [v1.11.6](https://github.com/ethereum/go-ethereum/releases/tag/v1.11.6)); so you need to install [Geth](https://geth.ethereum.org/docs/getting-started/installing-geth) for running tests.
3. [`solc`](https://docs.soliditylang.org/en/v0.8.17/installing-solidity.html) >=0.8.12.
4. [`cargo-sort`](https://crates.io/crates/cargo-sort) and [`cargo-udeps`](https://crates.io/crates/cargo-udeps).

## How to run?

Set up third-party dependencies (ERC-4337 smart contracts and bundler tests):

```bash
make fetch-thirdparty
make setup-thirdparty
```

Create wallet for bundler:

```bash
cargo run --release --bin create-wallet -- --output-path ${HOME}/.silius --chain-id 5
```

Run bundler (with user operation pool and JSON-RPC API):

```bash
cargo run --release -- --eth-client-address http://127.0.0.1:8545 --mnemonic-file ${HOME}/.silius/0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --beneficiary 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --entry-points 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789 --http --ws
```

Run only user operation pool:

```bash
cargo run --release --bin silius-uopool -- --eth-client-address http://127.0.0.1:8545 --entry-points 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789
```

Run only JSON-RPC API:

```bash
cargo run --release --bin silius-rpc --http --ws
```

## Supported networks

Bundler was tested on the following networks:

| Chain         | Mainnet   | Testnet                                       |
| :--------:    | :-------: | :-------:                                     |
| Ethereum      | :soon:    | :soon: (Goerli), :heavy_check_mark: (Sepolia) |
| Polygon PoS   | :soon:    | :heavy_check_mark: (Mumbai)                   |

## Supported entry point
The address of the entry point smart contract is the same on all EVM networks.
| Address         | Commit   | Audited                                       |
| :--------:      | :-------:| :-------:                                     |
| [0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789](https://blockscan.com/address/0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789) | [9b5f2e4](https://github.com/eth-infinitism/account-abstraction/commit/9b5f2e4bb30a81aa30761749d9e2e43fee64c768) | [April 2023](https://blog.openzeppelin.com/eip-4337-ethereum-account-abstraction-incremental-audit)


## Examples

To get started, check the examples [here](./examples/). More examples will be added in the future.

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

Official [bundler spec tests](https://github.com/eth-infinitism/bundler-spec-tests) developed by the [eth-infinitism](https://github.com/eth-infinitism/) team are also included in the repo's CI pipeline (commit: [f7c993031d4bb5f0940c5282298c911ec15a5fb7](https://github.com/eth-infinitism/bundler-spec-tests/tree/f7c993031d4bb5f0940c5282298c911ec15a5fb7)). You can find more information on how to run tests [here](https://github.com/eth-infinitism/bundler-spec-tests). Make sure your contribution doesn't break the tests!

## Contact

The best place for the discussion is the dedicated [Telegram group](https://t.me/+sKeRcN4j3MM3NmNk).

## Authors

- Vid Kersic: [GitHub](https://github.com/Vid201), [Twitter](https://twitter.com/vidkersic)
- WillQ: [GitHub](https://github.com/zsluedem), [Twitter](https://twitter.com/zsluedem06)

## Licenses

This project is dual-licensed under Apache 2.0 and MIT terms:

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

## Donations

Silius is an open-source project and a public good. If you want to help the project, you can send donations of any size via:

- Ethereum address: `0x7cB801446AC4f5EA8f7333EFc58ab787eB611558`

## Acknowledgements

- [Bundler - eth-infinitism](https://github.com/eth-infinitism/bundler)
- [Akula](https://github.com/akula-bft/akula)
- [ethers-rs](https://github.com/gakonst/ethers-rs)
- [Reth](https://github.com/paradigmxyz/reth)

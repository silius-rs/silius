# Build from Source

You can build Silius on Linux.

## Dependencies

First install Rust using <a href="https://rustup.rs/">rustup</a>:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

There are some other dependencies you need to install based on your operating system (OS):

- **Ubuntu/Debian**: `apt-get install libclang-dev pkg-config libssl-dev build-essential`

Solidity compiler (solc) is also neded to build from source. Find instructions <a href="https://docs.soliditylang.org/en/v0.8.17/installing-solidity.html">here</a>.

## Build Silius

Clone the repository and move to the directory:

```bash
git clone git@github.com:silius-rs/silius.git
cd silius
```

There are also some other third-party dependencies you need to setup (mainly ERC-4337 related smart contracts). These commands will clone the account abstraction repos and compile the smart contracts.

```bash
make fetch-thirdparty
make setup-thirdparty
```

After everything is setup, you can start the build:

```bash
make build // cargo build --release
```

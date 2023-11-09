# Prerequirements for building

1. Install rust
2. make fetch-thirdparty
3. make setup-thirdparty

# How to run silius with p2p enabled

## Run silius as bootnodes

```
cargo run -- bundler --eth-client-address http://localhost:8545 --mnemonic-file ./bundler-spec-tests/keys/0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --beneficiary 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --entry-points 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789 --http --http.port 4000 --eth-client-proxy-address http://localhost:8545 --p2p-broadcast-address 127.0.0.1 --enable-p2p
```
## Run silius with bootnodes

```
cargo run -- bundler --eth-client-address http://localhost:8545 --mnemonic-file ./bundler-spec-tests/keys/0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --beneficiary 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --entry-points 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789 --http --http.port 4000 --eth-client-proxy-address http://localhost:8545 --p2p-broadcast-address 127.0.0.1 --bootnodes "enr:-Iu4QBh2tesC8BokO61v1w43MnbfHF5H95ZJNHVEQaRq_MjFFuxmeVQnoEXxUDk5qKJCHM944gC72Xg4dYwRkGt9zA4BgmlkgnY0gmlwhH8AAAGJc2VjcDI1NmsxoQKRdyIA8OvArCcZbt3hoJHu4nVe6CblqjO0CnrbGACi-IN0Y3CCEPGDdWRwghDx" --enable-p2p --discovery.port 4338 --p2p.port 4338 --datadir ./.local/node1
```

## Run cluster of Silius bundler


Run as a  bootnode
```
cargo run -- bundler --eth-client-address http://localhost:8545 --mnemonic-file ./bundler-spec-tests/keys/0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --beneficiary 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --entry-points 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789 --http --http.port 4000 --eth-client-proxy-address http://localhost:8545 --p2p-broadcast-address 127.0.0.1 --enable-p2p
```

Run a second node
```
cargo run -- bundler --eth-client-address http://localhost:8545 --mnemonic-file ./bundler-spec-tests/keys/0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --beneficiary 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --entry-points 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789 --http --http.port 4001 --eth-client-proxy-address http://localhost:8545 --p2p-broadcast-address 127.0.0.1 --bootnodes "enr:-Iu4QBh2tesC8BokO61v1w43MnbfHF5H95ZJNHVEQaRq_MjFFuxmeVQnoEXxUDk5qKJCHM944gC72Xg4dYwRkGt9zA4BgmlkgnY0gmlwhH8AAAGJc2VjcDI1NmsxoQKRdyIA8OvArCcZbt3hoJHu4nVe6CblqjO0CnrbGACi-IN0Y3CCEPGDdWRwghDx" --enable-p2p --discovery.port 4338 --p2p.port 4338 --datadir ./.local/node1
```

Run a third node
```
cargo run -- bundler --eth-client-address http://localhost:8545 --mnemonic-file ./bundler-spec-tests/keys/0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --beneficiary 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --entry-points 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789 --http --http.port 4002 --eth-client-proxy-address http://localhost:8545 --p2p-broadcast-address 127.0.0.1 --bootnodes "enr:-Iu4QBh2tesC8BokO61v1w43MnbfHF5H95ZJNHVEQaRq_MjFFuxmeVQnoEXxUDk5qKJCHM944gC72Xg4dYwRkGt9zA4BgmlkgnY0gmlwhH8AAAGJc2VjcDI1NmsxoQKRdyIA8OvArCcZbt3hoJHu4nVe6CblqjO0CnrbGACi-IN0Y3CCEPGDdWRwghDx" --enable-p2p --discovery.port 4339 --p2p.port 4339 --datadir ./.local/node2
```
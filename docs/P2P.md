# Silius P2P instructions

## How to run Silius with P2P enabled

### Run bootnode

```bash
cargo run --release -- node --eth-client-address http://127.0.0.1:8545 --mnemonic-file ./bundler-spec-tests/keys/0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --beneficiary 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --entry-points 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789 --http --http.port 4000 --eth-client-proxy-address http://127.0.0.1:8545 --p2p.baddr 127.0.0.1 --enable-p2p
```

### Run peer node

```bash
cargo run --release -- node --eth-client-address http://127.0.0.1:8545 --mnemonic-file ./bundler-spec-tests/keys/0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --beneficiary 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --entry-points 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789 --http --http.port 4000 --eth-client-proxy-address http://127.0.0.1:8545 --p2p.baddr 127.0.0.1 --bootnodes "enr:-J24QFyIGX9IG6_4WO6F40-BXH0b4gChUm3zTOkYNoYBOWX5LTq7ubqm5oaFjwcg5r1YesmllbqNvKAapeM2JK8fkKoBiGNoYWluX2lkiDkFAAAAAAAAgmlkgnY0gmlwhH8AAAGJc2VjcDI1NmsxoQMm_tiGzC78d2_BvxJAUX9hRzBQv9QUmgU4OB4Pv1eVE4N0Y3CCIyiDdWRwgiMo" --enable-p2p --discovery.port 4338 --p2p.port 4338 --datadir ./.local/node1
```

### Run Silius bundler with env seed set (used for generation of P2P peer keys)

```bash
P2P_SEED=1 cargo run --release -- node --eth-client-address http://127.0.0.1:8545 --mnemonic-file ./bundler-spec-tests/keys/0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --beneficiary 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --entry-points 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789 --http --http.port 4000 --eth-client-proxy-address http://127.0.0.1:8545 --p2p.baddr 127.0.0.1 --bootnodes "enr:-J24QFyIGX9IG6_4WO6F40-BXH0b4gChUm3zTOkYNoYBOWX5LTq7ubqm5oaFjwcg5r1YesmllbqNvKAapeM2JK8fkKoBiGNoYWluX2lkiDkFAAAAAAAAgmlkgnY0gmlwhH8AAAGJc2VjcDI1NmsxoQMm_tiGzC78d2_BvxJAUX9hRzBQv9QUmgU4OB4Pv1eVE4N0Y3CCIyiDdWRwgiMo" --enable-p2p --discovery.port 4338 --p2p.port 4338 --datadir ./.local/node1
```

### Run cluster of Silius bundlers

Run bootnode

```bash
cargo run --release -- node --eth-client-address http://127.0.0.1:8545 --mnemonic-file ./bundler-spec-tests/keys/0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --beneficiary 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --entry-points 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789 --http --http.port 4000 --eth-client-proxy-address http://127.0.0.1:8545 --p2p.baddr 127.0.0.1 --enable-p2p
```

Run first peer node

```bash
cargo run --release -- node --eth-client-address http://127.0.0.1:8545 --mnemonic-file ./bundler-spec-tests/keys/0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --beneficiary 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --entry-points 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789 --uopool.port 3004 --bundler.port 3005 --http --http.port 4001 --eth-client-proxy-address http://127.0.0.1:8545 --p2p.baddr 127.0.0.1 --bootnodes "enr:-J24QFyIGX9IG6_4WO6F40-BXH0b4gChUm3zTOkYNoYBOWX5LTq7ubqm5oaFjwcg5r1YesmllbqNvKAapeM2JK8fkKoBiGNoYWluX2lkiDkFAAAAAAAAgmlkgnY0gmlwhH8AAAGJc2VjcDI1NmsxoQMm_tiGzC78d2_BvxJAUX9hRzBQv9QUmgU4OB4Pv1eVE4N0Y3CCIyiDdWRwgiMo" --enable-p2p --discovery.port 4338 --p2p.port 4338 --datadir ./.local/node1
```

Run second peer node

```bash
cargo run --release -- node --eth-client-address http://127.0.0.1:8545 --mnemonic-file ./bundler-spec-tests/keys/0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --beneficiary 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --entry-points 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789 --uopool.port 3006 --bundler.port 3007 --http --http.port 4002 --eth-client-proxy-address http://127.0.0.1:8545 --p2p.baddr 127.0.0.1 --bootnodes "enr:-J24QFyIGX9IG6_4WO6F40-BXH0b4gChUm3zTOkYNoYBOWX5LTq7ubqm5oaFjwcg5r1YesmllbqNvKAapeM2JK8fkKoBiGNoYWluX2lkiDkFAAAAAAAAgmlkgnY0gmlwhH8AAAGJc2VjcDI1NmsxoQMm_tiGzC78d2_BvxJAUX9hRzBQv9QUmgU4OB4Pv1eVE4N0Y3CCIyiDdWRwgiMo" --enable-p2p --discovery.port 4339 --p2p.port 4339 --datadir ./.local/node2
```

build:
	cargo build --release

build-debug-mode:
	cargo build

run-silius:
	cargo run --release -- node --eth-client-address http://127.0.0.1:8545 --mnemonic-file ${HOME}/.silius/0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --beneficiary 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --entry-points 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789 --http --ws

run-silius-bundling:
	cargo run --release -- bundler --eth-client-address http://127.0.0.1:8545 --mnemonic-file ${HOME}/.silius/0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --beneficiary 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --entry-points 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789

run-silius-uopool:
	cargo run --release -- uopool --eth-client-address http://127.0.0.1:8545 --entry-points 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789

run-silius-rpc:
	cargo run --release -- rpc --http --ws

run-silius-create-wallet:
	cargo run --release -- create-wallet --output-path ${HOME}/.silius

run-silius-p2p-bootnode:
	cargo run --release -- node --eth-client-address http://127.0.0.1:8545 --mnemonic-file ./bundler-spec-tests/keys/0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --beneficiary 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --entry-points 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789 --http --http.port 4000 --eth-client-proxy-address http://127.0.0.1:8545 --p2p.baddr 127.0.0.1 --enable-p2p

run-silius-p2p-peer:
	cargo run --release -- node --eth-client-address http://127.0.0.1:8545 --mnemonic-file ./bundler-spec-tests/keys/0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --beneficiary 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --entry-points 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789 --http --http.port 4000 --eth-client-proxy-address http://127.0.0.1:8545 --p2p.baddr 127.0.0.1 --bootnodes "enr:-J24QFyIGX9IG6_4WO6F40-BXH0b4gChUm3zTOkYNoYBOWX5LTq7ubqm5oaFjwcg5r1YesmllbqNvKAapeM2JK8fkKoBiGNoYWluX2lkiDkFAAAAAAAAgmlkgnY0gmlwhH8AAAGJc2VjcDI1NmsxoQMm_tiGzC78d2_BvxJAUX9hRzBQv9QUmgU4OB4Pv1eVE4N0Y3CCIyiDdWRwgiMo" --enable-p2p --discovery.port 4338 --p2p.port 4338 --datadir ./.local/node1

run-silius-debug:
	cargo run --release -- node --eth-client-address ws://127.0.0.1:8546 --mnemonic-file ${HOME}/.silius/0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --beneficiary 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --entry-points 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789 --http --ws --http.api eth,debug,web3 --ws.api eth,debug,web3

run-silius-debug-mode:
	cargo run --profile debug-fast -- node --verbosity 4 --eth-client-address ws://127.0.0.1:8546 --mnemonic-file /home/vid/.silius/0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --beneficiary 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --entry-points 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789 --http --ws --http.api eth,debug,web3 --ws.api eth,debug,web3

fetch-thirdparty:
	git submodule update --init

setup-thirdparty:
	cd crates/contracts/thirdparty/account-abstraction && yarn install --frozen-lockfile --immutable && yarn compile && cd ../../../..
	cd tests/thirdparty/bundler && yarn install --frozen-lockfile --immutable && yarn preprocess && cd ../../..

run-examples:
	./scripts/run_examples.sh

test:
	cargo test --workspace

format:
	cargo +nightly fmt --all
	cargo sort --workspace --grouped

lint:
	cargo +nightly fmt --all -- --check
	cargo clippy --all -- -D warnings -A clippy::derive_partial_eq_without_eq -D clippy::unwrap_used -D clippy::uninlined_format_args
	cargo sort --check --workspace --grouped
	cargo +nightly udeps --workspace

run-geth:
	cd bundler-spec-tests && docker compose up -d && cd ..

deploy-contracts:
	cd crates/contracts/thirdparty/account-abstraction && yarn deploy --network localhost && cd ../../..

clean:
	cd crates/contracts/thirdparty/account-abstraction && yarn clean && cd ../..
	cd tests/thirdparty/bundler && yarn clear && cd ../..
	cargo clean

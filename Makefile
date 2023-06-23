build:
	cargo build --release

run-bundler:
	cargo run --release -- --eth-client-address http://127.0.0.1:8545 --mnemonic-file ${HOME}/.aa-bundler/0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --beneficiary 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --gas-factor 600 --min-balance 1 --entry-points 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789 --min-stake 1 --min-unstake-delay 0 --min-priority-fee-per-gas 0 --max-verification-gas 1500000

run-bundler-uopool:
	cargo run --release --bin bundler-uopool -- --eth-client-address http://127.0.0.1:8545 --entry-points 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789 --min-stake 1 --min-unstake-delay 0 --min-priority-fee-per-gas 0 --max-verification-gas 1500000

run-bundler-rpc:
	cargo run --release --bin bundler-rpc

run-create-wallet:
	cargo run --release --bin create-wallet -- --output-path ${HOME}/.aa-bundler

run-bundler-debug:
	cargo run --release -- --eth-client-address http://127.0.0.1:8545 --mnemonic-file ${HOME}/.aa-bundler/0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --beneficiary 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --gas-factor 600 --min-balance 1 --entry-points 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789 --min-stake 1 --min-unstake-delay 0 --min-priority-fee-per-gas 0 --max-verification-gas 1500000 --rpc-api eth,debug

run-bundler-debug-mode:
	RUST_LOG=aa_bundler=TRACE cargo run --profile debug-fast -- --eth-client-address http://127.0.0.1:8545 --mnemonic-file /home/vid/.aa-bundler/0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --beneficiary 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --gas-factor 600 --min-balance 1 --entry-points 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789 --min-stake 1 --min-unstake-delay 0 --min-priority-fee-per-gas 0 --max-verification-gas 1500000 --rpc-api eth,debug

fetch-thirdparty:
	git submodule update --init

setup-thirdparty:
	cd thirdparty/account-abstraction && yarn install --frozen-lockfile --immutable && yarn compile && cd ../..
	cd thirdparty/bundler && yarn install --frozen-lockfile --immutable && yarn preprocess && cd ../..

test:
	cargo test --workspace

format:
	cargo fmt --all

lint:
	cargo fmt --all -- --check
	cargo clippy --all -- -D warnings -A clippy::derive_partial_eq_without_eq -D clippy::unwrap_used -D clippy::uninlined_format_args
	cargo sort --check --workspace
	cargo udeps --workspace

clean:
	cd thirdparty/account-abstraction && yarn clean && cd ../..
	cd thirdparty/bundler && yarn clear && cd ../..
	cargo clean

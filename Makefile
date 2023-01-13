build:
	make fetch-thirdparty
	cd thirdparty/account-abstraction && yarn install && yarn compile && cd ../..
	cargo build

run-bundler:
	cargo run -- --mnemonic-file ${HOME}/.aa-bundler/0x129D197b2a989C6798601A49D89a4AEC822A17a3 --beneficiary 0x690B9A9E9aa1C9dB991C7721a92d351Db4FaC990 --gas-factor 600 --min-balance 1 --eth-client-address https://rpc-mumbai.maticvigil.com/ --entry-points 0x0000000000000000000000000000000000000000 --chain-id 5 --helper 0x0000000000000000000000000000000000000000 --max-verification-gas 1500000

run-bundler-uopool:
	cargo run --bin bundler-uopool -- --entry-points 0x0000000000000000000000000000000000000000 --chain-id 5 --eth-client-address https://rpc-mumbai.maticvigil.com/ --max-verification-gas 1500000

run-bundler-rpc:
	cargo run --bin bundler-rpc

run-create-wallet:
	cargo run --bin create-wallet -- --output-path ${HOME}/.aa-bundler

fetch-thirdparty:
	git submodule update --init 

test:	
	cd thirdparty/bundler && yarn install && yarn preprocess && cd ../..
	cargo test 

format:
	cargo fmt --all

lint:
	cd thirdparty/bundler && yarn install && yarn preprocess && cd ../..
	cargo fmt --all -- --check
	cargo clippy --tests -- -D warnings -A clippy::derive_partial_eq_without_eq

clean:
	cd thirdparty/account-abstraction && yarn clean && cd ../..
	cd thirdparty/bundler && yarn clear && cd ../..
	cargo clean

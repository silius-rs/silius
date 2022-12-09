run-bundler:
	cargo run -- --mnemonic-file ${HOME}/.aa-bundler/0x129D197b2a989C6798601A49D89a4AEC822A17a3 --beneficiary 0x690B9A9E9aa1C9dB991C7721a92d351Db4FaC990 --gas-factor 600 --min-balance 1 --helper 0x0000000000000000000000000000000000000000 --eth-client-address https://rpc-mumbai.maticvigil.com/ --entry-point 0x1D9a2CB3638C2FC8bF9C01D088B79E75CD188b17 --max-verification-gas 1500000

run-bundler-uopool:
	cargo run --bin bundler-uopool -- --eth-client-address https://rpc-mumbai.maticvigil.com/ --entry-point 0x1D9a2CB3638C2FC8bF9C01D088B79E75CD188b17 --max-verification-gas 1500000

run-bundler-rpc:
	cargo run --bin bundler-rpc

run-create-wallet:
	cargo run --bin create-wallet -- --output-path ${HOME}/.aa-bundler

cargo-fmt:
	cargo fmt --all

lint:
	cargo fmt --all -- --check
	cargo clippy -- -D warnings -A clippy::derive_partial_eq_without_eq

build:
	cargo build

cargo-test:
	cargo test 

fetch-thirdparty:
	git submodule update --init
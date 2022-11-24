run-bundler:
	cargo run -- --mnemonic-file ./src/res/bundler/0xF78bB01dFd478608F5738fB0560642b2806D295E

run-bundler-uopool:
	cargo run --bin bundler-uopool

run-bundler-rpc:
	cargo run --bin bundler-rpc

run-create-wallet:
	cargo run --bin create-wallet -- --output-folder ./src/res/bundler

cargo-fmt:
	cargo fmt --all

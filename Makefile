run-bundler:
	cargo run -- --mnemonic-file ./src/res/bundler/0xD00D3EEc454D05d3d9bB48532BabED0c89941f17

run-bundler-uopool:
	cargo run --bin bundler-uopool

run-bundler-rpc:
	cargo run --bin bundler-rpc

cargo-fmt:
	cargo fmt --all

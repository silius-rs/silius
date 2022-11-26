run-bundler:
	cargo run -- --mnemonic-file ${HOME}/.aa-bundler/0x129D197b2a989C6798601A49D89a4AEC822A17a3

run-bundler-uopool:
	cargo run --bin bundler-uopool

run-bundler-rpc:
	cargo run --bin bundler-rpc

run-create-wallet:
	cargo run --bin create-wallet -- --output-path ${HOME}/.aa-bundler

cargo-fmt:
	cargo fmt --all

fetch-thirdparty:
	git submodule update --init
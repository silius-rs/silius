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
	cargo run --release -- node --eth-client-address http://127.0.0.1:8545 --mnemonic-file ./bundler-spec-tests/keys/0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --beneficiary 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 --entry-points 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789 --http --http.port 4000 --eth-client-proxy-address http://127.0.0.1:8545 --p2p.baddr 127.0.0.1 --bootnodes "enr:-J24QDPWQny36hS9qIFZcbIVSj2APHVP6cGT8hMc-365q2tjWU9Wq_NTyo0QMiXWaGkFfyeE32Pj2HetGHBfEL2QgpQBiGNoYWluX2lkiDkFAAAAAAAAgmlkgnY0gmlwhH8AAAGJc2VjcDI1NmsxoQL_3hI8PSmgjpFl83Nps5MTBjBf3pMm8Bo2TysjqnMGBoN0Y3CCIyiDdWRwgiMo" --enable-p2p --discovery.port 4338 --p2p.port 4338 --datadir ./.local/node1

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

mdbook:
	mdbook build
	mdbook watch --open

doc:
	cargo docs --open

run-geth:
	cd bundler-spec-tests && docker compose up -d && cd ..

deploy-contracts:
	cd crates/contracts/thirdparty/account-abstraction && yarn deploy --network localhost && cd ../../..

clean:
	cd crates/contracts/thirdparty/account-abstraction && yarn clean && cd ../..
	cd tests/thirdparty/bundler && yarn clear && cd ../..
	cargo clean

# Heavily inspired by Lighthouse: https://github.com/sigp/lighthouse/blob/693886b94176faa4cb450f024696cb69cda2fe58/Makefile
# docker image cross-platform build

GIT_TAG = $(shell git describe --tags)
BIN_DIR = dist/bin
BUILD_PATH = target
PROFILE = release

DOCKER_IMAGE_NAME = ghcr.io/silius-rs/silius

docker-build-push:
	$(call docker_build_push)

define docker_build_push
	$(MAKE) fetch-thirdparty
	$(MAKE) setup-thirdparty

	RUSTFLAGS="-C link-arg=-lgcc -Clink-arg=-static-libgcc" \
		cross build --target x86_64-unknown-linux-gnu --features "" --profile "$(PROFILE)"
	mkdir -p $(BIN_DIR)/amd64
	cp $(BUILD_PATH)/x86_64-unknown-linux-gnu/$(PROFILE)/silius $(BIN_DIR)/amd64/silius

	RUSTFLAGS="-C link-arg=-lgcc -Clink-arg=-static-libgcc" JEMALLOC_SYS_WITH_LG_PAGE=16 \
		cross build --target aarch64-unknown-linux-gnu --features "" --profile "$(PROFILE)"
	mkdir -p $(BIN_DIR)/arm64
	cp $(BUILD_PATH)/aarch64-unknown-linux-gnu/$(PROFILE)/silius $(BIN_DIR)/arm64/silius

	docker buildx build --file ./Dockerfile.cross . \
		--platform linux/amd64,linux/arm64 \
		--tag $(DOCKER_IMAGE_NAME):$(GIT_TAG) \
		--tag $(DOCKER_IMAGE_NAME):$(GIT_TAG) \
		--provenance=false \
		--push
endef

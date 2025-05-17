set fallback := true

FEATURES := "--all-features"
PROFILE := "release"

# default recipe to display help information
default:
  @just --list

build:
  cargo build --bin silius {{FEATURES}} --profile {{PROFILE}}

install:
  cargo install --path bin/silius --force --locked {{FEATURES}} --profile {{PROFILE}}

clean:
  cargo clean

lint:
  cargo +nightly fmt --all
  cargo clippy --all --all-targets --features "$(FEATURES)" --no-deps -- --deny warnings
  cargo sort --grouped

clean-deps:
  cargo +nightly udeps --workspace --tests --all-targets --release

test:
  cargo test --workspace -- --nocapture

build-debug:
  cargo build --bin silius {{FEATURES}}

run:
  ./target/{{PROFILE}}/silius

pr:
  just lint && just test

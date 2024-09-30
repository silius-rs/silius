FROM lukemathwalker/cargo-chef:latest-rust-1.81.0 AS chef
WORKDIR /app

LABEL org.opencontainers.image.source=https://github.com/silius-rs/silius
LABEL org.opencontainers.image.description="Silius - ERC-4337 (Account Abstraction) bundler implementation in Rust."
LABEL org.opencontainers.image.licenses="MIT OR Apache-2.0"

# Install system dependencies
RUN apt-get update && apt-get -y upgrade && apt-get install -y wget pkg-config libclang-dev libssl-dev ca-certificates

# Install solc
RUN wget -c "https://github.com/ethereum/solidity/releases/download/v0.8.27/solc-static-linux"
RUN mv solc-static-linux /usr/local/bin/solc
RUN chmod a+x /usr/local/bin/solc

# build cargo-chef plan
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json

# Set the build profile to be release
ARG BUILD_PROFILE=release
ENV BUILD_PROFILE $BUILD_PROFILE

# Extra Cargo features
ARG FEATURES=""
ENV FEATURES $FEATURES

# Builds dependencies
RUN cargo chef cook --profile $BUILD_PROFILE --features "$FEATURES" --recipe-path recipe.json

# Build application
COPY . .
RUN cargo build --profile $BUILD_PROFILE --features "$FEATURES" --locked

# Copy application
RUN cp /app/target/$BUILD_PROFILE/silius /app/silius

# Use ubuntu as a runtime image
FROM ubuntu:22.04 AS runtime

# Create data folder
RUN mkdir -p /data/silius

# Copy silus binary
COPY --from=builder /app/silius /usr/local/bin/silius

# Copy licenses
COPY LICENSE-* ./

# Expose ports
EXPOSE 3000 3001

ENTRYPOINT ["/usr/local/bin/silius"]

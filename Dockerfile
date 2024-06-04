FROM lukemathwalker/cargo-chef:latest-rust-1.76.0 AS chef
WORKDIR /app

LABEL org.opencontainers.image.source=https://github.com/silius-rs/silius
LABEL org.opencontainers.image.description="Silius - ERC-4337 (Account Abstraction) bundler implementation in Rust."
LABEL org.opencontainers.image.licenses="MIT OR Apache-2.0"

# build cargo-chef plan
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json

# Set the build profile to be release
ARG BUILD_PROFILE=release
ENV BUILD_PROFILE $BUILD_PROFILE

# Set the build target platform
ARG TARGETPLATFORM

# Install system dependencies
RUN apt-get update && apt-get -y upgrade && apt-get install -y pkg-config libclang-dev libssl-dev

# Install solc
RUN wget -c "https://github.com/ethereum/solidity/releases/download/v0.8.20/solc-static-linux"
RUN mv solc-static-linux /usr/local/bin/solc
RUN chmod a+x /usr/local/bin/solc

# Builds dependencies
RUN cargo chef cook --profile $BUILD_PROFILE --recipe-path recipe.json

# Build application
COPY . .
RUN if [ "$TARGETPLATFORM" = "linux/arm64" ]; then JEMALLOC_SYS_WITH_LG_PAGE=16 ; fi && \
    cargo build --profile $BUILD_PROFILE --locked

# Copy application
RUN cp /app/target/$BUILD_PROFILE/silius /app/silius

# Use alpine as a runtime image
FROM frolvlad/alpine-glibc:alpine-3.17 AS runtime

# Create data folder
RUN mkdir -p /data/silius

# Install system dependencies
RUN apk add openssl1.1-compat

# Copy silus binary
COPY --from=builder /app/silius /usr/local/bin/silius

# Copy licenses
COPY LICENSE-* ./

# Expose ports
EXPOSE 3000 3001

ENTRYPOINT ["/usr/local/bin/silius"]

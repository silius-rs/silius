# build
FROM ubuntu:18.04 AS builder

RUN apt-get update && apt-get -y upgrade && apt-get install -y build-essential software-properties-common curl git clang pkg-config libclang-dev libssl-dev
RUN add-apt-repository ppa:ethereum/ethereum && apt-get update && apt-get install -y solc

RUN curl -sL https://deb.nodesource.com/setup_14.x | sh -
RUN curl -sS https://dl.yarnpkg.com/debian/pubkey.gpg | apt-key add -
RUN echo "deb https://dl.yarnpkg.com/debian/ stable main" | tee /etc/apt/sources.list.d/yarn.list

RUN apt-get update && apt-get install -y nodejs yarn

WORKDIR /rust

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:$PATH"

WORKDIR /silius
COPY . .

RUN make fetch-thirdparty
RUN make setup-thirdparty
RUN make build

# run
FROM frolvlad/alpine-glibc

COPY --from=builder /silius/target/release/silius /usr/local/bin/silius

EXPOSE 3000

ENTRYPOINT ["usr/local/bin/silius"]

LABEL org.opencontainers.image.source=https://github.com/Vid201/silius

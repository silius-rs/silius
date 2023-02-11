# build
FROM ubuntu:18.04 AS builder

RUN apt-get update && apt-get -y upgrade && apt-get install -y build-essential software-properties-common curl git
RUN add-apt-repository ppa:ethereum/ethereum && apt-get update && apt-get install -y solc

RUN curl -sL https://deb.nodesource.com/setup_14.x | sh -
RUN curl -sS https://dl.yarnpkg.com/debian/pubkey.gpg | apt-key add -
RUN echo "deb https://dl.yarnpkg.com/debian/ stable main" | tee /etc/apt/sources.list.d/yarn.list

RUN apt-get update && apt-get install -y nodejs yarn

WORKDIR /rust

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:$PATH"

WORKDIR /aa-bundler
COPY . .

RUN make fetch-thirdparty
RUN make setup-thirdparty
RUN make build

# run
FROM frolvlad/alpine-glibc

COPY --from=builder /aa-bundler/target/debug/create-wallet /usr/local/bin/create-wallet
COPY --from=builder /aa-bundler/target/debug/bundler /usr/local/bin/bundler

# $HOME == /root
RUN if [ -z "$(ls -A $HOME)" ]; then /usr/local/bin/create-wallet --output-path $HOME ; fi

EXPOSE 3000

ENTRYPOINT usr/local/bin/bundler --rpc-listen-address 0.0.0.0:3000 --eth-client-address http://geth-dev:8545 --mnemonic-file $HOME/$(ls $HOME | head -1) --beneficiary 0x690B9A9E9aa1C9dB991C7721a92d351Db4FaC990 --gas-factor 600 --min-balance 1 --entry-points 0x0000000000000000000000000000000000000000 --chain-id 5 --helper 0x0000000000000000000000000000000000000000 --min-stake 1 --min-unstake-delay 0
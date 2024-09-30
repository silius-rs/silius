#!/usr/bin/env bash

set -eo pipefail

# install solc

SOLC_VERSION="v0.8.27"

wget -c "https://github.com/ethereum/solidity/releases/download/$SOLC_VERSION/solc-static-linux"
mv solc-static-linux /usr/local/bin/solc
chmod a+x /usr/local/bin/solc

solc --version

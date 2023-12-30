#!/usr/bin/env bash

set -eo pipefail

# install geth

GETH_VERSION="1.12.0-e501b3b0"

wget -c "https://gethstore.blob.core.windows.net/builds/geth-linux-amd64-$GETH_VERSION.tar.gz"
tar -xf "geth-linux-amd64-$GETH_VERSION.tar.gz"
mv geth-linux-amd64-$GETH_VERSION/geth /usr/local/bin/
chmod a+x /usr/local/bin/geth

geth version

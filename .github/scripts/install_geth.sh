#!/usr/bin/env bash

set -eo pipefail

# install geth

GETH_VERSION="1.14.11-f3c696fa"

wget -c "https://gethstore.blob.core.windows.net/builds/geth-linux-amd64-$GETH_VERSION.tar.gz"
tar -xf "geth-linux-amd64-$GETH_VERSION.tar.gz"
mv geth-linux-amd64-$GETH_VERSION/geth /usr/local/bin/
chmod a+x /usr/local/bin/geth

geth version

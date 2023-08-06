#!/bin/bash 
# Launcher script for the Silius.
set -x 
pushd `dirname \`realpath $0\``
case $1 in

 name)
	echo "Silius - ERC-4337 bundler in Rust"
	;;

 start)
	docker-compose up -d
    RUST_LOG=silius=TRACE silius \
        --rpc-listen-address 0.0.0.0:3000 \
        --eth-client-address http://localhost:8545 \
        --mnemonic-file keys/0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 \
        --beneficiary 0x690B9A9E9aa1C9dB991C7721a92d351Db4FaC990 \
        --entry-points 0x5ff137d4b0fdcd49dca30c7cf57e578a026d2789 \
        --rpc-api eth,debug,web3 & echo $! > bundler.pid
    popd
	cd @account-abstraction && yarn deploy --network localhost
	;;
 stop)
 	docker-compose down
    kill $(cat bundler.pid)
	;;

 *)
	echo "usage: $0 {start|stop|name}"
esac
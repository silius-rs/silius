#!/bin/sh -e

test -n "$VERBOSE" && set -x

#fund all addresses in the FUND array
#each entry is either an address (42-char hex) or privatekey (66-char hex)
#use ether "FUND_PRIVATEKEY" or accounts[0]

function fatal {
  echo "FATAL: $@"
  exit 1
}

#ETH_RPC_URL=http://localhost:8545

if [ -n "$FUND_PRIVATEKEY" ]; then
  funder=`cast w address $FUND_PRIVATEKEY`
  echo "using account for FUND_PRIVATEKEY: $funder"
  SENDER="--private-key $FUND_PRIVATEKEY"
else
  funder=`cast rpc eth_accounts|jq -r .[0]`
  SENDER="--unlocked --from $funder"
fi

test  -z "$funder" && fatal "unable to find a funder account: no FUND_PRIVATEKEY and no accounts[0] in node"

funderBal=`cast balance $funder`
test "$funderBal" = "0" && fatal "Funder account $funder has no balance"

for addr in $FUND; do
  len=`echo -n $addr | wc -c | xargs`
  case $len in
	   42) ;;
	   64|66) addr=`cast wallet address $addr` ;;
	   *) fatal "not an address and not privatekey: $addr" ;;
  esac

  cast send --gas-price 1000000000 --priority-gas-price 1000000000 --async $SENDER $addr --value `cast to-wei 10 eth` > /dev/null
  echo funded: $addr

done

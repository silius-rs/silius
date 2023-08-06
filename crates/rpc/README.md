# Silius RPC

Silius RPC crate provides an interface for handling RPC methods according to the [ERC-4337 specs](https://eips.ethereum.org/EIPS/eip-4337#rpc-methods-eth-namespace).

## Supported RPC Methods
### `eth` name space
* `eth_sendUserOperation`
  * submits a UserOperation to the client's [UserOperation alternative mempools](https://eips.ethereum.org/EIPS/eip-4337#alternative-mempools). The client must validate the UserOperation, and return a result accordingly.
* `eth_estimateUserOperationGas`
  * estimates the gas values for a UserOperation. Given UserOperation optionally without gas limits and gas prices, return the needed gas limits. The signature field is ignored by the wallet, so that the operation will not require user’s approval.
* `eth_getUserOperationReceipt`
  *  returns a UserOperation receipt based on a hash (`userOpHash`) returned by `eth_sendUserOperation`.
* `eth_supportedEntryPoints`
  * returns an array of the `entryPoint` addresses supported by the client.
* `eth_getUserOperationByHash`
  * returns a UserOperation based on a hash (`userOpHash`) returned by `eth_sendUserOperation`.
* `eth_chainId`
  * returns [EIP-155](https://eips.ethereum.org/EIPS/eip-155) Chain ID.
### `debug` name space
* `debug_clearState`
  * clears the bundler's [alternative mempool](https://eips.ethereum.org/EIPS/eip-4337#alternative-mempools) and reputation data of paymasters/accounts/factories/aggregators.
* `debug_dumpMempool`
  * dumps the current UserOperations mempool.
* `debug_setReputation`
  * sets reputation of given addresses.
* `debug_dumpReputation`
  * returns the reputation data of all observed addresses.
* `debug_setBundlingMode`
  * sets the bundling mode. After setting mode to `manual`, an explicit call to `debug_sendBundleNow` is required to send a bundle.
* `debug_sendBundleNow`
  * forces the bundler to build and execute a bundle from the mempool as [`handleOps()`](https://github.com/eth-infinitism/account-abstraction/blob/12be13e2e97b763e1ef294602b3f2072bc301443/contracts/core/EntryPoint.sol#L92) transaction.
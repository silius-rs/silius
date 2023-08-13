# Silius UserOperation Pool

The UserOperation alternative mempool implementation according to the [ERC-4337 specifications](https://eips.ethereum.org/EIPS/eip-4337#Alternative%20Mempools).

## Components Breakdown
### `uopool` module 
* `uopool` module provides the `UoPool` type whose interface encapsulates `validator`, `mempool` and `reputation` trait implementations for a customizable alternative mempool.
### `validate` module
* `validate` module provides the `Validator` trait that defines the interface for constructing flexible alternative memppool validation rules.
### `mempool` module
* `mempool` module provides the `Mempool` trait that defines the interface for mempool-related operations.
### `reputation` module
* `reputation` module provides the `Reputation` trait that defines the interface for reputation-realted operations.
### `database` and `memory` modules
* `database` and `memory` modules provide different implementations of the `Mempool` trait, depending on the use case.
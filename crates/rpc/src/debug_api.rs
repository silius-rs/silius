pub use crate::debug::DebugApiServerImpl;
use ethers::types::{Address, H256};
use jsonrpsee::{core::RpcResult, proc_macros::rpc};
use silius_primitives::{reputation::ReputationEntry, BundlerMode, UserOperation};

/// The ERC-4337 `debug` namespace RPC methods trait
#[rpc(server, namespace = "debug_bundler")]
pub trait DebugApi {
    /// Clears the bundler mempool
    ///
    ///
    /// # Returns
    /// * `RpcResult<()>` - None
    #[method(name = "clearState")]
    async fn clear_state(&self) -> RpcResult<()>;

    /// Get all [UserOperations](UserOperation) of the mempool
    ///
    /// # Arguments
    /// * `entry_point: Address` - The address of the entry point.
    ///
    /// # Returns
    /// * `RpcResult<Vec<UserOperation>>` - A vector of [UserOperations](UserOperation) returned
    #[method(name = "dumpMempool")]
    async fn dump_mempool(&self, entry_point: Address) -> RpcResult<Vec<UserOperation>>;

    /// Set the reputations for the given array of [ReputationEntry](ReputationEntry)
    ///
    /// # Arguments
    /// * `reputation_entries: Vec<ReputationEntry>` - The [ReputationEntry](ReputationEntry) to be set.
    /// * `entry_point: Address` - The address of the entry point.
    ///
    /// # Returns
    /// * `RpcResult<()>` - None
    #[method(name = "setReputation")]
    async fn set_reputation(
        &self,
        reputation_entries: Vec<ReputationEntry>,
        entry_point: Address,
    ) -> RpcResult<()>;

    /// Return the all of [ReputationEntries](ReputationEntry) in the mempool.
    ///
    /// # Arguments
    /// * `entry_point: Address` - The address of the entry point.
    ///
    /// # Returns
    /// * `RpcResult<Vec<ReputationEntry>>` - An array of [ReputationEntry](ReputationEntry)
    #[method(name = "dumpReputation")]
    async fn dump_reputation(&self, entry_point: Address) -> RpcResult<Vec<ReputationEntry>>;

    /// Set the bundling mode.
    ///
    /// # Arguments
    /// * `mode: BundlerMode` - The [BundlingMode](BundlingMode) to be set.
    ///
    /// # Returns
    /// * `RpcResult<()>` - None
    #[method(name = "setBundlingMode")]
    async fn set_bundling_mode(&self, mode: BundlerMode) -> RpcResult<()>;

    /// Immediately send the current bundle of user operations.
    /// This is useful for testing or in situations where waiting for the next scheduled bundle is not desirable.
    ///
    ///
    /// # Returns
    /// * `RpcResult<H256>` - The hash of the bundle that was sent.
    #[method(name = "sendBundleNow")]
    async fn send_bundle_now(&self) -> RpcResult<H256>;
}

pub use crate::debug::DebugApiServerImpl;
use ethers::types::{Address, H256};
use jsonrpsee::{core::RpcResult, proc_macros::rpc};
use serde::{Deserialize, Serialize};
use silius_primitives::{
    reputation::{ReputationEntry, StakeInfoResponse},
    BundleMode, UserOperationRequest,
};

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResponseSuccess {
    Ok,
}

/// The ERC-4337 `debug` namespace RPC methods trait
#[rpc(server, namespace = "debug_bundler")]
pub trait DebugApi {
    /// Clears the bundler mempool
    ///
    ///
    /// # Returns
    /// * `RpcResult<ResponseSuccess>` - Ok
    #[method(name = "clearMempool")]
    async fn clear_mempool(&self) -> RpcResult<ResponseSuccess>;

    /// Clears the bundler reputation
    ///
    ///
    /// # Returns
    /// * `RpcResult<ResponseSuccess>` - Ok
    #[method(name = "clearReputation")]
    async fn clear_reputation(&self) -> RpcResult<ResponseSuccess>;

    /// Clears the bundler mempool and reputation
    ///
    ///
    /// # Returns
    /// * `RpcResult<ResponseSuccess>` - Ok
    #[method(name = "clearState")]
    async fn clear_state(&self) -> RpcResult<ResponseSuccess>;

    /// Set the mempool for the given array of [UserOperation](UserOperationRequest)
    ///
    /// # Arguments
    /// * `user_operations: Vec<UserOperationRequest>` - The [UserOperation](UserOperationRequest)
    ///   to be set.
    /// * `entry_point: Address` - The address of the entry point.
    ///
    /// # Returns
    /// * `RpcResult<ResponseSuccess>` - Ok
    #[method(name = "addUserOps")]
    async fn add_user_ops(
        &self,
        user_operations: Vec<UserOperationRequest>,
        entry_point: Address,
    ) -> RpcResult<ResponseSuccess>;

    /// Get all [UserOperations](UserOperationRequest) of the mempool
    ///
    /// # Arguments
    /// * `entry_point: Address` - The address of the entry point.
    ///
    /// # Returns
    /// * `RpcResult<Vec<UserOperationRequest>>` - A vector of
    ///   [UserOperations](UserOperationRequest) returned
    #[method(name = "dumpMempool")]
    async fn dump_mempool(&self, entry_point: Address) -> RpcResult<Vec<UserOperationRequest>>;

    /// Set the reputations for the given array of [ReputationEntry](ReputationEntry)
    ///
    /// # Arguments
    /// * `reputation_entries: Vec<ReputationEntry>` - The [ReputationEntry](ReputationEntry) to be
    ///   set.
    /// * `entry_point: Address` - The address of the entry point.
    ///
    /// # Returns
    /// * `RpcResult<ResponseSuccess>` - Ok
    #[method(name = "setReputation")]
    async fn set_reputation(
        &self,
        reputation_entries: Vec<ReputationEntry>,
        entry_point: Address,
    ) -> RpcResult<ResponseSuccess>;

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
    /// * `mode: BundleMode` - The [BundleMode](BundleMode) to be set.
    ///
    /// # Returns
    /// * `RpcResult<ResponseSuccess>` - Ok
    #[method(name = "setBundlingMode")]
    async fn set_bundling_mode(&self, mode: BundleMode) -> RpcResult<ResponseSuccess>;

    /// Immediately send the current bundle of user operations.
    /// This is useful for testing or in situations where waiting for the next scheduled bundle is
    /// not desirable.
    ///
    ///
    /// # Returns
    /// * `RpcResult<H256>` - The hash of the bundle that was sent.
    #[method(name = "sendBundleNow")]
    async fn send_bundle_now(&self) -> RpcResult<H256>;

    /// Returns the stake info of the given address.
    ///
    /// # Arguments
    /// * `address: Address` - The address of the entity.
    /// * `entry_point: Address` - The address of the entry point.
    ///
    /// # Returns
    /// * `RpcResult<StakeInfoResponse>` - Stake info of the entity.
    #[method(name = "getStakeStatus")]
    async fn get_stake_status(
        &self,
        address: Address,
        entry_point: Address,
    ) -> RpcResult<StakeInfoResponse>;
}

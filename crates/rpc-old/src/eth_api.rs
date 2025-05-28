pub use crate::eth::EthApiServerImpl;
use ethers::types::{Address, U64};
use jsonrpsee::{core::RpcResult, proc_macros::rpc};
use silius_primitives::{
    UserOperationByHash, UserOperationGasEstimation, UserOperationHash, UserOperationReceipt,
    UserOperationRequest,
};

/// The ERC-4337 `eth` namespace RPC methods trait
#[rpc(server, namespace = "eth")]
pub trait EthApi {
    /// Retrieve the current [EIP-155](https://eips.ethereum.org/EIPS/eip-155) chain ID.
    ///
    ///
    /// # Returns
    /// * `RpcResult<U64>` - The chain ID as a U64.
    #[method(name = "chainId")]
    async fn chain_id(&self) -> RpcResult<U64>;

    /// Get the supported entry points of the bundler.
    ///
    /// # Returns
    /// * `RpcResult<Vec<String>>` - A array of the entry point addresses as strings.
    #[method(name = "supportedEntryPoints")]
    async fn supported_entry_points(&self) -> RpcResult<Vec<String>>;

    /// Send a user operation.
    ///
    /// # Arguments
    /// * `user_operation: UserOperation` - The [UserOperation](UserOperationRequest) to be sent.
    /// * `entry_point: Address` - The address of the entry point.
    ///
    /// # Returns
    /// * `RpcResult<UserOperationHash>` - The hash of the sent user operation.
    #[method(name = "sendUserOperation")]
    async fn send_user_operation(
        &self,
        user_operation: UserOperationRequest,
        entry_point: Address,
    ) -> RpcResult<UserOperationHash>;

    /// Estimate the gas required for a user operation.
    /// This allows you to gauge the computational cost of the operation.
    /// See [How ERC-4337 Gas Estimation Works](https://www.alchemy.com/blog/erc-4337-gas-estimation).
    ///
    /// # Arguments
    /// * `user_operation: [UserOperation](UserOperationRequest)` - User operation for which to
    ///   estimate the gas.
    /// * `entry_point: Address` - The address of the entry point.
    ///
    /// # Returns
    /// * `RpcResult<UserOperationGasEstimation>` - The estimated gas for the user operation.
    #[method(name = "estimateUserOperationGas")]
    async fn estimate_user_operation_gas(
        &self,
        user_operation: UserOperationRequest,
        entry_point: Address,
    ) -> RpcResult<UserOperationGasEstimation>;

    /// Retrieve the receipt of a user operation.
    /// The receipt contains the results of the operation, such as the amount of gas used.
    ///
    /// # Arguments
    /// * `user_operation_hash: String` - The hash of a user operation.
    ///
    /// # Returns
    /// * `RpcResult<Option<UserOperationReceipt>>` - The receipt of the user operation, or None if
    ///   it does not exist.
    #[method(name = "getUserOperationReceipt")]
    async fn get_user_operation_receipt(
        &self,
        user_operation_hash: String,
    ) -> RpcResult<Option<UserOperationReceipt>>;

    /// Retrieve a user operation by its hash.
    /// The hash serves as a unique identifier for the operation.
    ///
    /// # Arguments
    /// * `user_operation_hash: String` - The hash of the user operation.
    ///
    /// # Returns
    /// * `RpcResult<Option<UserOperationByHash>>` - The user operation associated with the hash, or
    ///   None if it does not exist.
    #[method(name = "getUserOperationByHash")]
    async fn get_user_operation_by_hash(
        &self,
        user_operation_hash: String,
    ) -> RpcResult<Option<UserOperationByHash>>;
}

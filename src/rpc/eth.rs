use crate::{rpc::eth_api::EthApiServer, types::user_operation::UserOperation};
use async_trait::async_trait;
use ethereum_types::{Address, U64};
use jsonrpsee::core::RpcResult;

pub struct EthApiServerImpl {
    pub call_gas_limit: u64,
}

#[async_trait]
impl EthApiServer for EthApiServerImpl {
    async fn chain_id(&self) -> RpcResult<U64> {
        Ok(U64::default())
    }

    async fn supported_entry_points(&self) -> RpcResult<Vec<Address>> {
        Ok(vec![Address::default()])
    }

    async fn send_user_operation(
        &self,
        user_operation: UserOperation,
        entry_point: Address,
    ) -> RpcResult<bool> {
        println!("{:?}", user_operation);
        println!("{:?}", entry_point);
        Ok(true)
    }
}

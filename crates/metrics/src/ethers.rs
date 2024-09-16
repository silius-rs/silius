use async_trait::async_trait;
use ethers::{
    providers::{
        FilterWatcher, LogQuery, Middleware, MiddlewareError, PendingTransaction, PubsubClient,
        SubscriptionStream,
    },
    types::{
        transaction::{eip2718::TypedTransaction, eip2930::AccessListWithGasUsed},
        Address, Block, BlockId, BlockNumber, Bytes, FeeHistory, Filter,
        GethDebugTracingCallOptions, GethDebugTracingOptions, GethTrace, Log, NameOrAddress,
        Signature, Transaction, TransactionReceipt, H256, U256, U64,
    },
};
use metrics::counter;
use serde::Serialize;
use std::fmt::Debug;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct MetricsMiddleware<M> {
    inner: M,
}

impl<M> MetricsMiddleware<M>
where
    M: Middleware,
{
    pub fn new(inner: M) -> Self {
        Self { inner }
    }
}

#[derive(Error, Debug)]
pub enum MetricError<M: Middleware> {
    /// Thrown when the internal middleware errors
    #[error("{0}")]
    MiddlewareError(M::Error),
}

impl<M: Middleware> MiddlewareError for MetricError<M> {
    type Inner = M::Error;

    fn from_err(src: M::Error) -> Self {
        MetricError::MiddlewareError(src)
    }

    fn as_inner(&self) -> Option<&Self::Inner> {
        match self {
            MetricError::MiddlewareError(e) => Some(e),
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<M> Middleware for MetricsMiddleware<M>
where
    M: Middleware,
{
    type Error = MetricError<M>;

    type Provider = M::Provider;

    type Inner = M;

    fn inner(&self) -> &Self::Inner {
        &self.inner
    }

    async fn client_version(&self) -> Result<String, Self::Error> {
        counter!("silius_ethers_client_version").increment(1);
        result_counter(self.inner().client_version().await, "silius_ethers_client_version")
    }

    /// Get the block number
    async fn get_block_number(&self) -> Result<U64, Self::Error> {
        counter!("silius_ethers_get_block_number").increment(1);
        result_counter(self.inner().get_block_number().await, "silius_ethers_get_block_number")
    }

    async fn send_transaction<'a, T: Into<TypedTransaction> + Send + Sync>(
        &'a self,
        tx: T,
        block: Option<BlockId>,
    ) -> Result<PendingTransaction<'a, Self::Provider>, Self::Error> {
        counter!("silius_ethers_send_transaction").increment(1);
        result_counter(
            self.inner().send_transaction(tx, block).await,
            "silius_ethers_send_transaction",
        )
    }

    async fn get_block<T: Into<BlockId> + Send + Sync>(
        &self,
        block_hash_or_number: T,
    ) -> Result<Option<Block<H256>>, Self::Error> {
        counter!("silius_ethers_get_block").increment(1);
        result_counter(
            self.inner().get_block(block_hash_or_number).await,
            "silius_ethers_get_block",
        )
    }

    async fn get_block_with_txs<T: Into<BlockId> + Send + Sync>(
        &self,
        block_hash_or_number: T,
    ) -> Result<Option<Block<Transaction>>, Self::Error> {
        counter!("silius_ethers_get_block_with_txs").increment(1);
        result_counter(
            self.inner().get_block_with_txs(block_hash_or_number).await,
            "silius_ethers_get_block_with_txs",
        )
    }

    async fn get_transaction_count<T: Into<NameOrAddress> + Send + Sync>(
        &self,
        from: T,
        block: Option<BlockId>,
    ) -> Result<U256, Self::Error> {
        counter!("silius_ethers_get_transaction_count").increment(1);
        result_counter(
            self.inner().get_transaction_count(from, block).await,
            "silius_ethers_get_transaction_count",
        )
    }

    async fn estimate_gas(
        &self,
        tx: &TypedTransaction,
        block: Option<BlockId>,
    ) -> Result<U256, Self::Error> {
        counter!("silius_ethers_estimate_gas").increment(1);
        result_counter(self.inner().estimate_gas(tx, block).await, "silius_ethers_estimate_gas")
    }

    async fn call(
        &self,
        tx: &TypedTransaction,
        block: Option<BlockId>,
    ) -> Result<Bytes, Self::Error> {
        counter!("silius_ethers_call").increment(1);
        result_counter(self.inner().call(tx, block).await, "silius_ethers_call")
    }

    async fn get_chainid(&self) -> Result<U256, Self::Error> {
        counter!("silius_ethers_get_chainid").increment(1);
        result_counter(self.inner().get_chainid().await, "silius_ethers_get_chainid")
    }

    async fn get_balance<T: Into<NameOrAddress> + Send + Sync>(
        &self,
        from: T,
        block: Option<BlockId>,
    ) -> Result<U256, Self::Error> {
        counter!("silius_ethers_get_balance").increment(1);
        result_counter(self.inner().get_balance(from, block).await, "silius_ethers_get_balance")
    }

    async fn get_transaction<T: Send + Sync + Into<H256>>(
        &self,
        transaction_hash: T,
    ) -> Result<Option<Transaction>, Self::Error> {
        counter!("silius_ethers_get_transaction").increment(1);
        result_counter(
            self.inner().get_transaction(transaction_hash).await,
            "silius_ethers_get_transaction",
        )
    }

    async fn get_transaction_receipt<T: Send + Sync + Into<H256>>(
        &self,
        transaction_hash: T,
    ) -> Result<Option<TransactionReceipt>, Self::Error> {
        counter!("silius_ethers_get_transaction_receipt").increment(1);
        result_counter(
            self.inner().get_transaction_receipt(transaction_hash).await,
            "silius_ethers_get_transaction_receipt",
        )
    }

    async fn get_block_receipts<T: Into<BlockNumber> + Send + Sync>(
        &self,
        block: T,
    ) -> Result<Vec<TransactionReceipt>, Self::Error> {
        counter!("silius_ethers_get_block_receipts").increment(1);
        result_counter(
            self.inner().get_block_receipts(block).await,
            "silius_ethers_get_block_receipts",
        )
    }

    async fn get_gas_price(&self) -> Result<U256, Self::Error> {
        counter!("silius_ethers_get_gas_price").increment(1);
        result_counter(self.inner().get_gas_price().await, "silius_ethers_get_gas_price")
    }

    async fn estimate_eip1559_fees(
        &self,
        estimator: Option<fn(U256, Vec<Vec<U256>>) -> (U256, U256)>,
    ) -> Result<(U256, U256), Self::Error> {
        counter!("silius_ethers_estimate_eip1559_fees").increment(1);
        result_counter(
            self.inner().estimate_eip1559_fees(estimator).await,
            "silius_ethers_estimate_eip1559_fees",
        )
    }

    async fn get_accounts(&self) -> Result<Vec<Address>, Self::Error> {
        counter!("silius_ethers_get_accounts").increment(1);
        result_counter(self.inner().get_accounts().await, "silius_ethers_get_accounts")
    }

    async fn send_raw_transaction<'a>(
        &'a self,
        tx: Bytes,
    ) -> Result<PendingTransaction<'a, Self::Provider>, Self::Error> {
        counter!("silius_ethers_send_raw_transaction").increment(1);
        result_counter(
            self.inner().send_raw_transaction(tx).await,
            "silius_ethers_send_raw_transaction",
        )
    }

    async fn sign<T: Into<Bytes> + Send + Sync>(
        &self,
        data: T,
        from: &Address,
    ) -> Result<Signature, Self::Error> {
        counter!("silius_ethers_sign").increment(1);
        result_counter(self.inner().sign(data, from).await, "silius_ethers_sign")
    }

    async fn sign_transaction(
        &self,
        tx: &TypedTransaction,
        from: Address,
    ) -> Result<Signature, Self::Error> {
        counter!("silius_ethers_sign_transaction").increment(1);
        result_counter(
            self.inner().sign_transaction(tx, from).await,
            "silius_ethers_sign_transaction",
        )
    }

    async fn get_logs(&self, filter: &Filter) -> Result<Vec<Log>, Self::Error> {
        counter!("silius_ethers_get_logs").increment(1);
        result_counter(self.inner().get_logs(filter).await, "silius_ethers_get_logs")
    }

    fn get_logs_paginated<'a>(
        &'a self,
        filter: &Filter,
        page_size: u64,
    ) -> LogQuery<'a, Self::Provider> {
        counter!("silius_ethers_get_logs_paginated").increment(1);
        self.inner().get_logs_paginated(filter, page_size)
    }

    async fn watch<'a>(
        &'a self,
        filter: &Filter,
    ) -> Result<FilterWatcher<'a, Self::Provider, Log>, Self::Error> {
        counter!("silius_ethers_watch").increment(1);
        result_counter(self.inner().watch(filter).await, "silius_ethers_watch")
    }

    async fn watch_pending_transactions<'a>(
        &'a self,
    ) -> Result<FilterWatcher<'a, Self::Provider, H256>, Self::Error> {
        counter!("silius_ethers_watch_pending_transactions").increment(1);
        result_counter(
            self.inner().watch_pending_transactions().await,
            "silius_ethers_watch_pending_transactions",
        )
    }
    async fn watch_blocks<'a>(
        &'a self,
    ) -> Result<FilterWatcher<'a, Self::Provider, H256>, Self::Error> {
        counter!("silius_ethers_watch_blocks").increment(1);
        result_counter(self.inner().watch_blocks().await, "silius_ethers_watch_blocks")
    }
    async fn get_code<T: Into<NameOrAddress> + Send + Sync>(
        &self,
        at: T,
        block: Option<BlockId>,
    ) -> Result<Bytes, Self::Error> {
        counter!("silius_ethers_get_code").increment(1);
        result_counter(self.inner().get_code(at, block).await, "silius_ethers_get_code")
    }

    async fn get_storage_at<T: Into<NameOrAddress> + Send + Sync>(
        &self,
        from: T,
        location: H256,
        block: Option<BlockId>,
    ) -> Result<H256, Self::Error> {
        counter!("silius_ethers_get_storage_at").increment(1);
        result_counter(
            self.inner().get_storage_at(from, location, block).await,
            "silius_ethers_get_storage_at",
        )
    }

    async fn debug_trace_transaction(
        &self,
        tx_hash: H256,
        trace_options: GethDebugTracingOptions,
    ) -> Result<GethTrace, Self::Error> {
        counter!("silius_ethers_debug_trace_transaction").increment(1);
        result_counter(
            self.inner().debug_trace_transaction(tx_hash, trace_options).await,
            "silius_ethers_debug_trace_transaction",
        )
    }

    async fn debug_trace_call<T: Into<TypedTransaction> + Send + Sync>(
        &self,
        req: T,
        block: Option<BlockId>,
        trace_options: GethDebugTracingCallOptions,
    ) -> Result<GethTrace, Self::Error> {
        counter!("silius_ethers_debug_trace_call").increment(1);
        result_counter(
            self.inner().debug_trace_call(req, block, trace_options).await,
            "silius_ethers_debug_trace_call",
        )
    }

    async fn debug_trace_block_by_number(
        &self,
        block: Option<BlockNumber>,
        trace_options: GethDebugTracingOptions,
    ) -> Result<Vec<GethTrace>, Self::Error> {
        counter!("silius_ethers_debug_trace_block_by_number").increment(1);
        result_counter(
            self.inner().debug_trace_block_by_number(block, trace_options).await,
            "silius_ethers_debug_trace_block_by_number",
        )
    }

    async fn debug_trace_block_by_hash(
        &self,
        block: H256,
        trace_options: GethDebugTracingOptions,
    ) -> Result<Vec<GethTrace>, Self::Error> {
        counter!("silius_ethers_debug_trace_block_by_hash").increment(1);
        result_counter(
            self.inner().debug_trace_block_by_hash(block, trace_options).await,
            "silius_ethers_debug_trace_block_by_hash",
        )
    }

    async fn subscribe_blocks<'a>(
        &'a self,
    ) -> Result<SubscriptionStream<'a, Self::Provider, Block<H256>>, Self::Error>
    where
        <Self as Middleware>::Provider: PubsubClient,
    {
        counter!("silius_ethers_subscribe_blocks").increment(1);
        result_counter(self.inner().subscribe_blocks().await, "silius_ethers_subscribe_blocks")
    }

    async fn subscribe_pending_txs<'a>(
        &'a self,
    ) -> Result<SubscriptionStream<'a, Self::Provider, H256>, Self::Error>
    where
        <Self as Middleware>::Provider: PubsubClient,
    {
        counter!("silius_ethers_subscribe_pending_txs").increment(1);
        result_counter(
            self.inner().subscribe_pending_txs().await,
            "silius_ethers_subscribe_pending_txs",
        )
    }

    async fn subscribe_full_pending_txs<'a>(
        &'a self,
    ) -> Result<SubscriptionStream<'a, Self::Provider, Transaction>, Self::Error>
    where
        <Self as Middleware>::Provider: PubsubClient,
    {
        counter!("silius_ethers_subscribe_full_pending_txs").increment(1);
        result_counter(
            self.inner().subscribe_full_pending_txs().await,
            "silius_ethers_subscribe_full_pending_txs",
        )
    }
    async fn subscribe_logs<'a>(
        &'a self,
        filter: &Filter,
    ) -> Result<SubscriptionStream<'a, Self::Provider, Log>, Self::Error>
    where
        <Self as Middleware>::Provider: PubsubClient,
    {
        counter!("silius_ethers_subscribe_logs").increment(1);
        result_counter(self.inner().subscribe_logs(filter).await, "silius_ethers_subscribe_logs")
    }

    async fn fee_history<T: Into<U256> + Serialize + Send + Sync>(
        &self,
        block_count: T,
        last_block: BlockNumber,
        reward_percentiles: &[f64],
    ) -> Result<FeeHistory, Self::Error> {
        counter!("silius_ethers_fee_history").increment(1);
        result_counter(
            self.inner().fee_history(block_count, last_block, reward_percentiles).await,
            "silius_ethers_fee_history",
        )
    }

    async fn create_access_list(
        &self,
        tx: &TypedTransaction,
        block: Option<BlockId>,
    ) -> Result<AccessListWithGasUsed, Self::Error> {
        counter!("silius_ethers_create_access_list").increment(1);
        result_counter(
            self.inner().create_access_list(tx, block).await,
            "silius_ethers_create_access_list",
        )
    }
}

fn result_counter<M, T, E>(result: Result<T, E>, request_type: &str) -> Result<T, MetricError<M>>
where
    M: Middleware<Error = E>,
    E: Send + Sync + Debug,
{
    match result {
        Ok(res) => {
            counter!(format!("{request_type}_success")).increment(1);
            Ok(res)
        }
        Err(e) => {
            counter!(format!("{request_type}_failed")).increment(1);
            Err(MiddlewareError::from_err(e))
        }
    }
}

use alloy_chains::{Chain, NamedChain};
use ethers::{
    providers::Middleware,
    signers::Signer,
    types::{
        transaction::eip2718::TypedTransaction, Address, Eip1559TransactionRequest, H256, U256, U64,
    },
};
use silius_contracts::entry_point::EntryPointAPI;
use silius_primitives::{UserOperation, UserOperationHash, Wallet};
use std::sync::Arc;
use tracing::{info, trace};

/// A trait for sending the bundler of user operations
#[async_trait::async_trait]
pub trait SendBundleOp: Send + Sync + 'static {
    /// Send a bundle of [UserOperations](UserOperation).
    ///
    /// # Arguments
    /// * `bundle` - Bundle of [UserOperations](UserOperation)
    ///
    /// # Returns
    /// * `H256` - The hash
    async fn send_bundle(&self, bundle: TypedTransaction) -> eyre::Result<H256>;
}

/// The `Bundler` struct is used to represent a bundler with necessary properties
#[derive(Clone, Debug)]
pub struct Bundler<M, S>
where
    M: Middleware + 'static,
    S: SendBundleOp,
{
    /// Bundler's wallet
    pub wallet: Wallet,
    /// Beneficiary address where the gas is refunded after execution
    pub beneficiary: Address,
    /// Entry point contract address
    pub entry_point: Address,
    /// Chain the bundler is running on
    pub chain: Chain,
    /// Minimum balance required
    pub min_balance: U256,
    /// Ethereum execution client
    pub eth_client: Arc<M>,
    /// Client that sends the bundle to some network
    pub client: Arc<S>,
}

impl<M, S> Bundler<M, S>
where
    M: Middleware + 'static,
    S: SendBundleOp,
{
    /// Create a new Bundler thats bundles multiple user operations and sends them as bundle
    ///
    /// # Returns
    /// * `Self` - A new `Bundler` instance
    pub fn new(
        wallet: Wallet,
        beneficiary: Address,
        entry_point: Address,
        chain: Chain,
        min_balance: U256,
        eth_client: Arc<M>,
        client: Arc<S>,
    ) -> Self {
        Self { wallet, beneficiary, entry_point, chain, min_balance, eth_client, client }
    }

    /// Functions that generates a bundle of user operations (i.e.,
    /// [TypedTransaction](TypedTransaction)).
    ///
    /// # Arguments
    /// * `uos` - Slice of [UserOperations](UserOperation)
    ///
    /// # Returns
    /// * `TypedTransaction` - A [TypedTransaction](TypedTransaction)
    async fn create_bundle(&self, uos: &[UserOperation]) -> eyre::Result<TypedTransaction> {
        let ep = EntryPointAPI::new(self.entry_point, self.eth_client.clone());

        let nonce =
            self.eth_client.get_transaction_count(self.wallet.signer.address(), None).await?;
        let balance = self.eth_client.get_balance(self.wallet.signer.address(), None).await?;
        let beneficiary = if balance < self.min_balance {
            self.wallet.signer.address()
        } else {
            self.beneficiary
        };

        let mut tx: TypedTransaction = ep
            .handle_ops(
                uos.iter().cloned().map(|uo| uo.user_operation.into()).collect(),
                beneficiary,
            )
            .tx;

        match Chain::from_id(self.chain.id()).named() {
            // Mumbai
            Some(NamedChain::PolygonMumbai) => {
                tx.set_nonce(nonce).set_chain_id(self.chain.id());
            }
            // All other surpported networks, including Mainnet, Goerli
            _ => {
                let accesslist = self.eth_client.create_access_list(&tx, None).await?.access_list;
                tx.set_access_list(accesslist.clone());
                let estimated_gas = self.eth_client.estimate_gas(&tx, None).await?;

                let (max_fee_per_gas, max_priority_fee) =
                    self.eth_client.estimate_eip1559_fees(None).await?;

                tx = TypedTransaction::Eip1559(Eip1559TransactionRequest {
                    to: tx.to().cloned(),
                    from: Some(self.wallet.signer.address()),
                    data: tx.data().cloned(),
                    chain_id: Some(U64::from(self.chain.id())),
                    max_priority_fee_per_gas: Some(max_priority_fee),
                    max_fee_per_gas: Some(max_fee_per_gas),
                    gas: Some(estimated_gas),
                    nonce: Some(nonce),
                    value: None,
                    access_list: accesslist,
                });
            }
        };

        Ok(tx)
    }

    /// Send a bundle of [UserOperations](UserOperation)
    ///
    /// # Arguments
    /// * `uos` - An array of [UserOperations](UserOperation)
    ///
    /// # Returns
    /// * `H256` - The hash
    pub async fn send_bundle(&self, uos: &Vec<UserOperation>) -> eyre::Result<H256> {
        if uos.is_empty() {
            info!("Skipping creating a new bundle, no user operations");
            return Ok(H256::default());
        };

        info!(
            "Creating a new bundle with {} user operations: {:?}",
            uos.len(),
            uos.iter().map(|uo| uo.hash).collect::<Vec<UserOperationHash>>()
        );
        trace!("Bundle content: {uos:?}");

        let bundle = self.create_bundle(uos).await?;
        let hash = self.client.send_bundle(bundle).await?;

        info!(
            "Bundle successfully sent, hash: {:?}, account: {:?}, entry point: {:?}, beneficiary: {:?}",
            hash,
            self.wallet.signer.address(),
            self.entry_point,
            self.beneficiary
        );

        Ok(hash)
    }
}

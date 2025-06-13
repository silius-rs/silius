use alloy_chains::Chain;
use ethers::{
    providers::Middleware,
    signers::Signer,
    types::{
        transaction::eip2718::TypedTransaction, Address, Eip1559TransactionRequest, H256, U256, U64,
    },
};
use silius_contracts::entry_point::EntryPointAPI;
use silius_primitives::{simulation::StorageMap, UserOperation, UserOperationHash, Wallet};
use std::sync::Arc;
use tracing::{info, trace};

/// A trait for sending the bundler of user operations
#[async_trait::async_trait]
pub trait SendBundleOp: Send + Sync + 'static {
    /// Send a bundle of user operations.
    ///
    /// # Arguments
    /// * `bundle` - Bundle of user operations as [TypedTransaction](TypedTransaction).
    /// * 'storage_map' - Storage map
    ///
    /// # Returns
    /// * `H256` - The hash
    async fn send_bundle(
        &self,
        bundle: TypedTransaction,
        storage_map: StorageMap,
    ) -> eyre::Result<H256>;
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
    /// Whether add access list into tx
    pub enable_access_list: bool,
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
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        wallet: Wallet,
        beneficiary: Address,
        entry_point: Address,
        chain: Chain,
        min_balance: U256,
        eth_client: Arc<M>,
        client: Arc<S>,
        enable_access_list: bool,
    ) -> Self {
        Self {
            wallet,
            beneficiary,
            entry_point,
            chain,
            min_balance,
            eth_client,
            client,
            enable_access_list,
        }
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

        let accesslist = if self.enable_access_list {
            let accesslist = self.eth_client.create_access_list(&tx, None).await?.access_list;
            tx.set_access_list(accesslist.clone());
            accesslist
        } else {
            Default::default()
        };
        let estimated_gas = self.eth_client.estimate_gas(&tx, None).await?;

        let mut max_fee_per_gas: U256 = U256::zero();
        let mut max_priority_fee_per_gas: U256 = U256::zero();

        for uo in uos {
            max_fee_per_gas += uo.max_fee_per_gas;
            max_priority_fee_per_gas += uo.max_priority_fee_per_gas;
        }

        tx = TypedTransaction::Eip1559(Eip1559TransactionRequest {
            to: tx.to().cloned(),
            from: Some(self.wallet.signer.address()),
            data: tx.data().cloned(),
            chain_id: Some(U64::from(self.chain.id())),
            max_priority_fee_per_gas: Some(max_priority_fee_per_gas / uos.len()),
            max_fee_per_gas: Some(max_fee_per_gas / uos.len()),
            gas: Some(estimated_gas),
            nonce: Some(nonce),
            value: None,
            access_list: accesslist,
        });

        Ok(tx)
    }

    /// Send a bundle of [UserOperations](UserOperation)
    ///
    /// # Arguments
    /// * `uos` - An array of [UserOperations](UserOperation)
    /// * `storage_map` - Storage map
    ///
    /// # Returns
    /// * `H256` - The hash
    pub async fn send_bundle(
        &self,
        uos: &Vec<UserOperation>,
        storage_map: StorageMap,
    ) -> eyre::Result<Option<H256>> {
        if uos.is_empty() {
            info!("Skipping creating a new bundle, no user operations");
            return Ok(None);
        };

        info!(
            "Creating a new bundle with {} user operations: {:?}",
            uos.len(),
            uos.iter().map(|uo| uo.hash).collect::<Vec<UserOperationHash>>()
        );
        trace!("Bundle content: {uos:?}");

        let bundle = self.create_bundle(uos).await?;
        let hash = self.client.send_bundle(bundle, storage_map).await?;

        info!(
            "Bundle successfully sent, hash: {:?}, account: {:?}, entry point: {:?}, beneficiary: {:?}",
            hash,
            self.wallet.signer.address(),
            self.entry_point,
            self.beneficiary
        );

        Ok(Some(hash))
    }
}

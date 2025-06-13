use crate::bundler::SendBundleOp;
use ethers::{
    middleware::SignerMiddleware,
    providers::Middleware,
    signers::{LocalWallet, Signer},
    types::{transaction::eip2718::TypedTransaction, H256},
};
use ethers_flashbots::{BundleRequest, FlashbotsMiddleware, PendingBundleError, SimulatedBundle};
use silius_primitives::{simulation::StorageMap, Wallet};
use std::sync::Arc;
use tracing::{info, trace};
use url::Url;

/// A struct for the Flashbots Signer client
#[derive(Clone)]
pub struct FlashbotsClient<M>(
    pub Arc<SignerMiddleware<FlashbotsMiddleware<Arc<M>, LocalWallet>, LocalWallet>>,
);

#[async_trait::async_trait]
impl<M> SendBundleOp for FlashbotsClient<M>
where
    M: Middleware + 'static,
{
    // TODO: add more relay endpoints support
    /// Send a bundle of user operations to the Flashbots relay.
    ///
    /// # Arguments
    /// * `bundle` - Bundle of user operations as [TypedTransaction](TypedTransaction).
    /// * 'storage_map' - Storage map
    ///
    /// # Returns
    /// * `H256` - The transaction hash of the bundle
    async fn send_bundle(
        &self,
        bundle: TypedTransaction,
        _storage_map: StorageMap,
    ) -> eyre::Result<H256> {
        let bundle_req = self.generate_bundle_req(vec![bundle], false).await?;

        match self.simulate_flashbots_bundle(&bundle_req).await {
            Ok(_) => {}
            Err(e) => return Err(eyre::eyre!("Bundle simulation failed: {:?}", e)),
        };

        let bundle_hash = self.send_flashbots_bundle(bundle_req.clone()).await?;

        Ok(bundle_hash)
    }
}

impl<M> FlashbotsClient<M>
where
    M: Middleware + 'static,
{
    /// Create a new Flashbots client
    ///
    /// # Arguments
    /// * `eth_client` - Connection to the Ethereum execution client
    /// * `relay_endpoints` - An array of Flashbots relay endpoints
    /// * `wallet` - A [Wallet](Wallet) instance
    ///
    /// # Returns
    /// * `FlashbotsClient` - A [Flashbots Signer Middleware](FlashbotsClient)
    pub fn new(
        eth_client: Arc<M>,
        relay_endpoints: Option<Vec<String>>,
        wallet: Wallet,
    ) -> eyre::Result<Self> {
        // Only support one relay endpoint for now
        let relay_endpoint: &str = relay_endpoints
            .as_ref()
            .expect("No Flashbots relay endpoint provided")
            .first()
            .expect("No Flashbots relay endpoint provided");

        let bundle_signer = match wallet.flashbots_signer {
            Some(ref signer) => signer,
            None => return Err(eyre::eyre!("No Flashbots signer provided")),
        };

        let mut flashbots_middleware = FlashbotsMiddleware::new(
            eth_client,
            Url::parse(relay_endpoint)?,
            bundle_signer.clone(),
        );
        flashbots_middleware.set_simulation_relay(
            Url::parse(relay_endpoint).expect("Failed to parse simulation relay URL"),
            bundle_signer.clone(),
        );

        let client = Arc::new(SignerMiddleware::new(flashbots_middleware, wallet.signer.clone()));

        Ok(Self(client))
    }

    /// Generate a Flashbots bundle request
    ///
    /// # Arguments
    /// * `tx` - A [EIP-1559 TypedTransaction](TypedTransaction)
    /// * `revertible` - If true the bundle is revertible, otherwise any transactions in the bundle
    ///   revert will revert the whole bundle
    ///
    /// # Returns
    /// * `BundleRequest` - A [BundleRequest](BundleRequest)
    pub async fn generate_bundle_req(
        &self,
        txs: Vec<TypedTransaction>,
        revertible: bool,
    ) -> eyre::Result<BundleRequest> {
        let mut bundle_req = BundleRequest::new();
        for tx in txs {
            let typed_tx = TypedTransaction::Eip1559(tx.into());
            let raw_signed_tx = match self.0.signer().sign_transaction(&typed_tx).await {
                Ok(tx) => typed_tx.rlp_signed(&tx),
                Err(e) => return Err(eyre::eyre!("Failed to sign transaction: {:?}", e)),
            };

            if revertible {
                bundle_req = bundle_req.push_revertible_transaction(raw_signed_tx);
            } else {
                bundle_req = bundle_req.push_transaction(raw_signed_tx);
            };
        }

        // Simulate the Flashbots bundle
        let block_num = self.0.get_block_number().await?;
        bundle_req = bundle_req
            .set_block(block_num + 1)
            .set_simulation_block(block_num)
            .set_simulation_timestamp(0);

        Ok(bundle_req)
    }

    /// Send a Flashbots bundle and check for status
    ///
    /// # Arguments
    /// * `bundle` - A [BundleRequest](BundleRequest) sent to Flashbots relay
    ///
    /// # Returns
    /// * `H256` - The transaction hash of the bundle
    pub async fn send_flashbots_bundle(&self, bundle: BundleRequest) -> eyre::Result<H256> {
        // Send the Flashbots bundle and check for status
        let pending_bundle = match self.0.inner().send_bundle(&bundle).await {
            Ok(bundle) => bundle,
            Err(e) => return Err(eyre::eyre!("Failed to send bundle: {:?}", e)),
        };

        info!("Bundle received at block: {:?}", pending_bundle.block);

        match pending_bundle.await {
            Ok(bundle_hash) => Ok(bundle_hash),
            Err(err) => match err {
                PendingBundleError::BundleNotIncluded => {
                    Err(eyre::eyre!("Bundle not included in the target block"))
                }
                _ => Err(eyre::eyre!("Bundle rejected: {:?}", err)),
            },
        }
    }

    /// Simulate a Flashbots bundle
    ///
    /// # Arguments
    /// * `bundle` - A [BundleRequest](BundleRequest) sent to Flashbots relay
    ///
    /// # Returns
    /// * `SimulatedBundle` - Simulated Flashbots bundle
    pub async fn simulate_flashbots_bundle(
        &self,
        bundle: &BundleRequest,
    ) -> eyre::Result<SimulatedBundle> {
        let simulated_bundle = self.0.inner().simulate_bundle(bundle).await?;

        // Currently there's only 1 tx per bundle
        for tx in &simulated_bundle.transactions {
            trace!("Simulate bundle: {:?}", tx);

            if let Some(err) = &tx.error {
                return Err(eyre::eyre!("Transaction failed simulation with error: {:?}", err));
            }
            if let Some(revert) = &tx.revert {
                return Err(eyre::eyre!("Transaction failed simulation with revert: {:?}", revert));
            }
        }

        Ok(simulated_bundle)
    }
}

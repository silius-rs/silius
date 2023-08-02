use aa_bundler_contracts::entry_point::EntryPointAPI;
use aa_bundler_primitives::{consts::flashbots_relay_endpoints, Chain, UserOperation, Wallet};
use anvil::eth::util::get_precompiles_for;
use bytes::Bytes;
use ethers::{
    prelude::SignerMiddleware,
    providers::{Http, Middleware, Provider},
    signers::Signer,
    types::{
        transaction::eip2718::TypedTransaction,
        transaction::eip2930::AccessList,
        Address, Eip1559TransactionRequest, Eip2930TransactionRequest, H256, U256, U64,
    },
    
};
use ethers_flashbots::{BundleRequest, FlashbotsMiddleware, PendingBundleError::BundleNotIncluded};
use foundry_evm::revm::{
    db::CacheDB,
    primitives::{Address as rAddress, U256 as rU256},
    EVM,
};
use foundry_evm::{
    executor::{
        fork::{BlockchainDb, BlockchainDbMeta, SharedBackend},
        inspector::AccessListTracer,
        TxEnv,
    },
    utils::{eval_to_instruction_result, halt_to_instruction_result},
};
use revm::{
    interpreter::InstructionResult,
    primitives::{ExecutionResult, Output, TransactTo},
};
use std::collections::BTreeSet;
use std::{sync::Arc, time::Duration};
use tokio::task::JoinHandle;
use tracing::{info, trace};
use url::Url;

/// The `SendBundleMode` determines whether to send the bundle to a Ethereum execution client or to Flashbots relay
#[derive(Clone, Debug, PartialEq)]
pub enum SendBundleMode {
    /// Send the bundle to a Ethereum execution client
    EthClient,
    /// Send the bundle to Flashbots relay
    Flashbots,
}

/// The `Bundler` struct is used to represent a bundler with necessary properties
#[derive(Clone, Debug)]
pub struct Bundler {
    /// Wallet instance representing the bundler's wallet.
    pub wallet: Wallet,
    /// URL of an Ethereum client.
    pub eth_client_address: String,
    /// Beneficiary address where the gas is refunded after execution
    pub beneficiary: Address,
    /// Entry point contract address
    pub entry_point: Address,
    /// [Chain](Chain) instance representing the blockchain network to be used
    pub chain: Chain,
    /// Send bundle mode determines whether to send the bundle to a regular Eth execution client or to Flashbots
    pub send_bundle_mode: SendBundleMode,
    /// Block Builder relay endpoints
    pub relay_endpoints: Option<Vec<String>>,
}

impl Bundler {
    /// Create a new `Bundler` instance
    /// if `send_bundle_mode` is `SendBundleMode::Flashbots` and `relay_endpoints` is `None`, the default Flashbots relay endpoint will be used
    ///
    /// # Returns
    /// * `Self` - A new `Bundler` instance
    pub fn new(
        wallet: Wallet,
        eth_client_address: String,
        beneficiary: Address,
        entry_point: Address,
        chain: Chain,
        send_bundle_mode: SendBundleMode,
        relay_endpoints: Option<Vec<String>>,
    ) -> Self {
        if !(chain.id() == 1 || chain.id() == 5 || chain.id() == 11155111)
            && send_bundle_mode == SendBundleMode::Flashbots
        {
            panic!("Flashbots is only supported on mainnet, goerli and Sepolia");
        };

        match send_bundle_mode {
            SendBundleMode::EthClient => Self {
                wallet,
                eth_client_address,
                beneficiary,
                entry_point,
                chain,
                send_bundle_mode,
                relay_endpoints: None,
            },
            SendBundleMode::Flashbots => match relay_endpoints {
                None => {
                    let mut relay_endpoints = Vec::new();
                    relay_endpoints.push(flashbots_relay_endpoints::FLASHBOTS.to_string());
                    Self {
                        wallet,
                        eth_client_address,
                        beneficiary,
                        entry_point,
                        chain,
                        send_bundle_mode,
                        relay_endpoints: Some(relay_endpoints),
                    }
                }
                Some(relay_endpoints) => Self {
                    wallet,
                    eth_client_address,
                    beneficiary,
                    entry_point,
                    chain,
                    send_bundle_mode,
                    relay_endpoints: Some(relay_endpoints),
                },
            },
        }
    }

    /// Send a bundle of [UserOperations](UserOperation) to the Ethereum execution client or Flashbots' relay, depending on the [SendBundleMode](SendBundleMode)
    ///
    /// # Arguments
    /// * `uos` - An array of [UserOperations](UserOperation)
    ///
    /// # Returns
    /// * `H256` - The transaction hash of the bundle or transaction
    pub async fn send_next_bundle(&self, uos: &Vec<UserOperation>) -> anyhow::Result<H256> {
        if uos.is_empty() {
            info!("Skipping creating a new bundle, no user operations");
            return Ok(H256::default());
        };

        info!("Creating a new bundle with {} user operations", uos.len());
        trace!("Bundle content: {uos:?}");

        match self.send_bundle_mode {
            SendBundleMode::EthClient => self.send_next_bundle_eth(uos).await,
            SendBundleMode::Flashbots => self.send_next_bundle_flashbots(uos).await,
        }
    }

    /// Based on Reth [`create_access_list_at`](https://github.com/paradigmxyz/reth/blob/b46101afb5e549d40b7b2537fff9b67e05ad4448/crates/rpc/rpc/src/eth/api/call.rs#L237) method
    async fn create_access_list<M: Middleware + 'static>(
        &self,
        client: Arc<M>,
        call_data: Bytes,
    ) -> anyhow::Result<(InstructionResult, Option<Output>, u64, AccessList)> 
    {
        let block_number = client.get_block_number().await?;
        let current_block = match client.get_block(block_number.clone()).await {
            Ok(block) => match block {
                Some(block) => block,
                None => return Err(anyhow::anyhow!("Failed to get block")),
            },
            Err(e) => return Err(anyhow::anyhow!("Failed to get block: {:?}", e)),
        };

        let shared_backend = SharedBackend::spawn_backend_thread(
            client.clone(),
            BlockchainDb::new(
                BlockchainDbMeta {
                    cfg_env: Default::default(),
                    block_env: Default::default(),
                    hosts: BTreeSet::from(["".to_string()]),
                },
                None,
            ),
            Some(block_number.clone().into()),
        );

        let fork_db = CacheDB::new(shared_backend);
        let mut evm = EVM::new();
        evm.database(fork_db);

        // Set up the EVM block environment
        evm.env.block.number = rU256::from(block_number.as_u64());
        evm.env.block.timestamp = current_block.timestamp.into();
        evm.env.block.basefee = current_block
            .base_fee_per_gas
            .unwrap_or(U256::from(21000))
            .into();
        // Using builder0x69's address as a mock
        evm.env.block.coinbase = "0x690B9A9E9aa1C9dB991C7721a92d351Db4FaC990"
            .parse::<rAddress>()
            .expect("Failed to parse address");

        // Set up the EVM transaction environment
        let tx = TxEnv {
            caller: self.wallet.signer.address().into(),
            gas_limit: u64::MAX,
            gas_price: current_block
                .base_fee_per_gas
                .unwrap_or(U256::from(21000))
                .into(),
            gas_priority_fee: None,
            transact_to: TransactTo::Call(self.entry_point.into()),
            value: Default::default(),
            data: call_data,
            chain_id: self.chain.id().into(),
            nonce: None,
            access_list: Default::default(),
        };
        evm.env.tx = tx;

        let mut access_list_inspector = AccessListTracer::new(
            Default::default(),
            self.wallet.signer.address().into(),
            self.entry_point.into(),
            get_precompiles_for(evm.env.cfg.spec_id),
        );

        let res_and_state = match evm.inspect_ref(&mut access_list_inspector) {
            Ok(res_and_state) => res_and_state,
            Err(e) => return Err(anyhow::anyhow!("Failed to inspect transaction: {:?}", e)),
        };

        let (exit_reason, gas_used, out) = match res_and_state.result {
            ExecutionResult::Success {
                reason,
                gas_used,
                output,
                ..
            } => (eval_to_instruction_result(reason), gas_used, Some(output)),
            ExecutionResult::Revert { gas_used, output } => (
                InstructionResult::Revert,
                gas_used,
                Some(Output::Call(output)),
            ),
            ExecutionResult::Halt { reason, gas_used } => {
                (halt_to_instruction_result(reason), gas_used, None)
            }
        };

        let access_list = access_list_inspector.access_list();

        Ok((exit_reason, out, gas_used, access_list))
    }

    /// Helper function to generate a [TypedTransaction](TypedTransaction) from an array of user operations.
    ///
    /// # Arguments
    /// * `client` - An a provider that implements [Middleware](Middleware) trait
    ///
    /// # Returns
    /// * `TypedTransaction` - A [TypedTransaction](TypedTransaction) instance
    #[allow(clippy::ptr_arg)]
    async fn generate_tx<M>(
        &self,
        client: Arc<M>,
        uos: &Vec<UserOperation>,
    ) -> anyhow::Result<TypedTransaction>
    where
        M: Middleware + 'static,
    {
        let ep = EntryPointAPI::new(self.entry_point, client.clone());

        let nonce = client
            .clone()
            .get_transaction_count(self.wallet.signer.address(), None)
            .await?;
        let mut tx: TypedTransaction = ep
            .handle_ops(
                uos.clone().into_iter().map(Into::into).collect(),
                self.beneficiary,
            )
            .tx;


        match self.chain.id() {
            // Mumbai
            80001u64 => {
                tx.set_nonce(nonce).set_chain_id(self.chain.id());
            }
            _ => {

                
                let accesslist = client.clone().create_access_list(&tx, None).await?.access_list;
                tx.set_access_list(accesslist);
                let estimated_gas = client.clone().estimate_gas(&tx, None).await?;

                let (max_fee_per_gas, max_priority_fee) =
                    client.clone().estimate_eip1559_fees(None).await?;

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
                    access_list: accesslist.access_list,
                });
            }
        };

        println!("tx: {:?}", tx);

        Ok(tx.into())
    }

    /// Send a bundle of [UserOperations](UserOperation) to the Ethereum execution client
    ///
    /// # Arguments
    /// * `uos` - An array of [UserOperations](UserOperation)
    ///
    /// # Returns
    /// * `H256` - The transaction hash of the bundle
    async fn send_next_bundle_eth(&self, uos: &Vec<UserOperation>) -> anyhow::Result<H256> {
        let eth_client = Provider::<Http>::try_from(self.eth_client_address.clone())?;
        let client = Arc::new(SignerMiddleware::new(
            eth_client.clone(),
            self.wallet.signer.clone(),
        ));

        let tx = self.generate_tx(client.clone(), uos).await?;

        trace!("Sending transaction to the execution client: {tx:?}");

        let tx = client
            .send_transaction(tx, None)
            .await?
            .interval(Duration::from_millis(75));
        let tx_hash = tx.tx_hash();

        let tx_receipt = tx.await?;

        trace!("Transaction receipt: {tx_receipt:?}");

        Ok(tx_hash)
    }

    // TODO: add more relay endpoints support
    /// Send a bundle of [UserOperations](UserOperation) to the Flashbots relay.
    ///
    /// # Arguments
    /// * `uos` - An array of [UserOperations](UserOperation)
    ///
    /// # Returns
    /// * `H256` - The transaction hash of the bundle
    #[allow(clippy::needless_return)]
    async fn send_next_bundle_flashbots(&self, uos: &Vec<UserOperation>) -> anyhow::Result<H256> {
        // TODO: add support for multiple relay endpoints
        let relay_endpoint: &str = self.relay_endpoints.as_ref().unwrap().first().unwrap();

        let provider = Provider::<Http>::try_from(self.eth_client_address.clone())?;

        let bundle_signer = match self.wallet.fb_signer {
            Some(ref signer) => signer,
            None => return Err(anyhow::anyhow!("No Flashbots signer provided")),
        };

        let mut fb_middleware = FlashbotsMiddleware::new(
            provider.clone(),
            Url::parse(relay_endpoint.clone())?,
            bundle_signer.clone(),
        );
        fb_middleware.set_simulation_relay(
            Url::parse(relay_endpoint.clone()).expect("Failed to parse simulation relay URL"),
            bundle_signer.clone(),
        );

        let client = Arc::new(SignerMiddleware::new(
            fb_middleware,
            self.wallet.signer.clone(),
        ));

        let tx = self.generate_tx(client.clone(), uos).await?;

        trace!("Sending transaction to the execution client: {tx:?}");

        // Sign the tx
        let typed_tx = TypedTransaction::Eip1559(tx.clone().into());
        let raw_signed_tx = match client.signer().sign_transaction(&typed_tx).await {
            Ok(tx) => typed_tx.rlp_signed(&tx),
            Err(e) => return Err(anyhow::anyhow!("Failed to sign transaction: {:?}", e)),
        };

        // Add tx to Flashbots bundle
        let mut bundle_req = BundleRequest::new();
        bundle_req = bundle_req.push_transaction(raw_signed_tx);

        // Simulate the Flashbots bundle
        let block_num = client.get_block_number().await?;
        bundle_req = bundle_req
            .set_block(block_num + 1)
            .set_simulation_block(block_num)
            .set_simulation_timestamp(0);
        let simulated_bundle = client.inner().simulate_bundle(&bundle_req).await?;

        // Currently there's only 1 tx per bundle
        for tx in simulated_bundle.transactions {
            trace!("Simulate bundle: {:?}", tx);
            if let Some(err) = &tx.error {
                return Err(anyhow::anyhow!(
                    "Transaction failed simulation with error: {:?}",
                    err
                ));
            }
            if let Some(revert) = &tx.revert {
                return Err(anyhow::anyhow!(
                    "Transaction failed simulation with revert: {:?}",
                    revert
                ));
            }
        }

        // Send the Flashbots bundle and check for status
        let handle: JoinHandle<Result<(bool, H256), anyhow::Error>> = tokio::spawn(async move {
            let pending_bundle = match client.inner().send_bundle(&bundle_req).await {
                Ok(bundle) => bundle,
                Err(e) => return Err(anyhow::anyhow!("Failed to send bundle: {:?}", e)),
            };

            let bundle_hash = pending_bundle.bundle_hash;

            match pending_bundle.await {
                Ok(_) => return Ok((true, bundle_hash)),
                Err(BundleNotIncluded) => {
                    return Err(anyhow::anyhow!("Bundle not included in the target block"));
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Bundle rejected: {:?}", e));
                }
            };
        });

        match handle.await {
            Ok(Ok((_, bundle_hash))) => {
                info!("Bundle included");
                Ok(bundle_hash)
            }
            Ok(Err(e)) => Err(e),
            Err(e) => Err(anyhow::anyhow!("Task panicked: {:?}", e)),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{Bundler, SendBundleMode};
    use aa_bundler_primitives::{consts::flashbots_relay_endpoints, Chain, UserOperation, Wallet};
    use alloy_primitives::{Address as alloy_Address, U256 as alloy_U256};
    use alloy_sol_types::{sol, SolCall};
    use ethers::{
        signers::Signer,
        types::{Address, U256},
    };
    use std::env;

    sol! {
        #[derive(Debug)]
        function swapExactETHForTokens(uint amountOutMin, address[] calldata path, address to, uint deadline) external payable returns (uint[] memory amounts);
    }

    // Testing key
    const KEY_PHRASE: &str = "test test test test test test test test test test test junk";
    // Deplolyed SimpleAccount address on Goerli
    const SIMPLE_ACCOUNT: &str = "0xf850679aFA7A1675D24D21092b584AbA709d35F8";
    // UO signing key
    const SIGNING_KEY: &str = "0xdf1f39dd322a0cb54da8724bf1baf639f0d34916d529adbe2942a28b47dbed4a";

    struct TestContext {
        pub bundler: Bundler,
        pub entry_point: Address,
    }

    async fn setup() -> anyhow::Result<TestContext> {
        std::env::set_var("RUST_LOG", "info");
        tracing_subscriber::fmt::init();

        dotenv::dotenv().ok();
        let eth_client_address = env::var("HTTP_RPC").expect("HTTP_RPC env var not set");
        let ep_address = "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789".parse::<Address>()?;

        let wallet = Wallet::from_phrase(KEY_PHRASE, &U256::from(5), true)?;

        let bundler = Bundler::new(
            wallet.clone(),
            eth_client_address.clone(),
            wallet.signer.address(),
            ep_address,
            Chain::from(5),
            SendBundleMode::Flashbots,
            Some(vec![flashbots_relay_endpoints::FLASHBOTS_GOERLI.to_string()]),
        );

        Ok(TestContext {
            bundler,
            entry_point: ep_address,
        })
    }

    #[tokio::test]
    async fn test_send_bundle_flashbots() -> anyhow::Result<()> {
        let ctx = setup().await?;

        let path = vec![
            // WETH address
            alloy_Address::parse_checksummed("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", None)
                .unwrap(),
            // USDt address
            alloy_Address::parse_checksummed("0xdAC17F958D2ee523a2206206994597C13D831ec7", None)
                .unwrap(),
        ];

        let swap_eth = swapExactETHForTokensCall {
            amountOutMin: alloy_U256::from(0),
            path: path.clone(),
            to: SIMPLE_ACCOUNT.parse::<alloy_Address>().unwrap(),
            deadline: alloy_U256::from(0),
        };
        let call_data = swap_eth.encode();

        let uo = UserOperation::default()
            .call_data(call_data.into())
            .sender(SIMPLE_ACCOUNT.parse::<Address>().unwrap())
            // .verification_gas_limit(100_000.into())
            // .pre_verification_gas(21_000.into())
            // .max_priority_fee_per_gas(1_000_000_000.into())
            // .call_gas_limit(200_000.into())
            // .max_fee_per_gas(3_000_000_000_u64.into())
            // .max_priority_fee_per_gas(1_000_000_000.into())
            .signature(SIGNING_KEY.parse().unwrap());

        let bundle_hash = ctx.bundler.send_next_bundle(&vec![uo]).await?;
        println!("Bundle hash: {}", bundle_hash);

        Ok(())
    }
}

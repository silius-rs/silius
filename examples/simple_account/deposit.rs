use std::{env, sync::Arc, time::Duration};

use ethers::{
    prelude::{gas_oracle::ProviderOracle, MiddlewareBuilder, SignerMiddleware},
    providers::{Http, Middleware, Provider},
    signers::{coins_bip39::English, MnemonicBuilder, Signer},
    types::{transaction::eip2718::TypedTransaction, Address, U256},
    utils::parse_ether,
};
use serde_json::Value;
use silius_contracts::entry_point::EntryPointAPI;
use silius_tests::common::gen::SimpleAccountFactory;

// stackup simple account factory
const SIMPLE_ACCOUNT_FACTORY: &str = "0x9406Cc6185a346906296840746125a0E44976454";
const ENTRYPOINT_ADDRESS: &str = "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789";
const CREATE_INDEX: u64 = 1;
const SEND_VALUE: &str = "0.01"; // ether unit

#[derive(Debug, serde::Serialize)]
pub struct Request {
    jsonrpc: String,
    id: u64,
    method: String,
    params: Vec<Value>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let key_phrase = env::var("KEY_PHRASE").unwrap();
    let provider_url = env::var("PROVIDER_URL").unwrap();
    let wallet = MnemonicBuilder::<English>::default()
        .phrase(key_phrase.as_str())
        .build()?;
    let provider = Provider::<Http>::try_from(provider_url)?.interval(Duration::from_millis(10u64));
    let client = SignerMiddleware::new(provider.clone(), wallet.clone().with_chain_id(5u64))
        .nonce_manager(wallet.address())
        .gas_oracle(ProviderOracle::new(provider.clone()));
    let provider = Arc::new(client);

    let simple_account_factory_address = SIMPLE_ACCOUNT_FACTORY.to_string().parse::<Address>()?;
    let simple_account_factory =
        SimpleAccountFactory::new(simple_account_factory_address, provider.clone());

    let owner_address = wallet.address();
    println!("simple_account_factory: {:?}", simple_account_factory);
    println!("Signer address: {:x}", owner_address);
    let address = simple_account_factory
        .get_address(owner_address, U256::from(CREATE_INDEX))
        .call()
        .await?;
    println!("Smart account addresss: {:?}", address);

    let entrypoint = EntryPointAPI::new(ENTRYPOINT_ADDRESS.parse::<Address>()?, provider.clone());
    let call = entrypoint.deposit_to(address);
    let mut tx: TypedTransaction = call.tx;
    tx.set_value(parse_ether(SEND_VALUE)?);
    println!("tx: {:?}", tx);
    let pending_tx = provider.send_transaction(tx, None).await?;
    println!("pending_tx: {:?}", pending_tx);
    let receipt = pending_tx.await?;
    println!("receipt: {:?}", receipt);
    Ok(())
}

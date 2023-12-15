use ethers::{
    prelude::{gas_oracle::ProviderOracle, MiddlewareBuilder, SignerMiddleware},
    providers::{Http, Middleware, Provider},
    signers::{coins_bip39::English, MnemonicBuilder, Signer},
    types::{transaction::eip2718::TypedTransaction, Address, U256},
    utils::parse_ether,
};
use silius_contracts::entry_point::EntryPointAPI;
use silius_primitives::constants::entry_point::ADDRESS;
use silius_tests::common::gen::SimpleAccountFactory;
use std::{env, sync::Arc, time::Duration};

// stackup simple account factory
const SIMPLE_ACCOUNT_FACTORY: &str = "0x9406Cc6185a346906296840746125a0E44976454";
const CREATE_INDEX: u64 = 2;
const SEND_VALUE: &str = "0.01"; // ether unit

#[tokio::main]
async fn main() -> eyre::Result<()> {
    if let Ok(provider_url) = env::var("PROVIDER_URL") {
        let seed_phrase = env::var("SEED_PHRASE").unwrap();

        let provider =
            Provider::<Http>::try_from(provider_url)?.interval(Duration::from_millis(10u64));
        let chain_id = provider.get_chainid().await?.as_u64();

        let wallet = MnemonicBuilder::<English>::default().phrase(seed_phrase.as_str()).build()?;
        let client =
            SignerMiddleware::new(provider.clone(), wallet.clone().with_chain_id(chain_id))
                .nonce_manager(wallet.address())
                .gas_oracle(ProviderOracle::new(provider.clone()));
        let provider = Arc::new(client);

        let simple_account_factory_address =
            SIMPLE_ACCOUNT_FACTORY.to_string().parse::<Address>()?;
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

        let entrypoint = EntryPointAPI::new(ADDRESS.parse::<Address>()?, provider.clone());
        let call = entrypoint.deposit_to(address);
        let mut tx: TypedTransaction = call.tx;
        tx.set_value(parse_ether(SEND_VALUE)?);
        println!("tx: {:?}", tx);
        let pending_tx = provider.send_transaction(tx, None).await?;
        println!("pending_tx: {:?}", pending_tx);
        let receipt = pending_tx.await?;
        println!("receipt: {:?}", receipt);
    }

    Ok(())
}

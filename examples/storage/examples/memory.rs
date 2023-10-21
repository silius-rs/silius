use ethers::types::{Address, U256};
use silius_primitives::{consts::entry_point::ADDRESS, provider::create_http_provider, Chain};
use silius_uopool::{MemoryMempool, MemoryReputation, UoPoolBuilder};
use std::{env, str::FromStr, sync::Arc};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    //  uopool needs connection to the execution client
    let provider_url = env::var("PROVIDER_URL").unwrap();

    // creating uopool with builder
    let builder = UoPoolBuilder::new(
        false, // whether uoppol is in unsafe mode
        Arc::new(create_http_provider(provider_url.as_str()).await?), // provider
        Address::from_str(ADDRESS)?, // entry point address
        Chain::Named(ethers::types::Chain::Dev), // chain information
        U256::from(5000000), // max verification gas
        U256::from(1), // min stake
        U256::from(0), // min priority fee per gas
        vec![], // whitelisted entities
        MemoryMempool::default(), // in-memory mempool of user operations
        MemoryReputation::default(), // in-memory reputation
    );

    // optional: subscription to block updates and reputation updates
    // builder.register_block_updates(block_stream);
    // builder.register_reputation_updates();

    println!("In-memory uopool created!");

    // size of mempool
    println!(
        "Mempool size: {size}",
        size = builder.uopool().get_all().len()
    );

    Ok(())
}

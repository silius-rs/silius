use ethers::types::{Address, U256};
use futures::channel::mpsc::unbounded;
use silius_primitives::{
    consts::entry_point::ADDRESS, provider::create_http_provider, Chain, UserOperation,
};
use silius_uopool::{init_env, DatabaseMempool, DatabaseReputation, UoPoolBuilder, WriteMap};
use std::{env, str::FromStr, sync::Arc};
use tempdir::TempDir;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    //  uopool needs connection to the execution client
    let provider_url = env::var("PROVIDER_URL").unwrap();

    // initialize database env
    let dir = TempDir::new("silius-db").unwrap();
    let env = Arc::new(init_env::<WriteMap>(dir.into_path()).expect("Init mdbx failed"));
    env.create_tables()
        .expect("Create mdbx database tables failed");
    let (waiting_to_pub_sd, _) = unbounded::<(UserOperation, U256)>();
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
        DatabaseMempool::new(env.clone()), // database mempool of user operations
        DatabaseReputation::new(env), // database reputation
        Some(waiting_to_pub_sd), // waiting to publish user operations, for p2p part
    );

    // optional: subscription to block updates and reputation updates
    // builder.register_block_updates(block_stream);
    // builder.register_reputation_updates();

    println!("Database uopool created!");

    // size of mempool
    println!(
        "Mempool size: {size}",
        size = builder.uopool().get_all().len()
    );

    Ok(())
}

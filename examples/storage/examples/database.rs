use alloy_chains::Chain;
use ethers::types::{Address, U256};
use parking_lot::RwLock;
use silius_contracts::EntryPoint;
use silius_mempool::{
    init_env, validate::validator::new_canonical, CodeHashes, DatabaseTable, Mempool, Reputation,
    UoPoolBuilder, UserOperations, UserOperationsByEntity, UserOperationsBySender, WriteMap,
};
use silius_primitives::{
    constants::{
        entry_point::ADDRESS,
        validation::reputation::{
            BAN_SLACK, MIN_INCLUSION_RATE_DENOMINATOR, MIN_UNSTAKE_DELAY, THROTTLING_SLACK,
        },
    },
    provider::create_http_provider,
    reputation::ReputationEntry,
    UoPoolMode,
};
use std::{
    collections::{HashMap, HashSet},
    env,
    str::FromStr,
    sync::Arc,
    time::Duration,
};
use tempfile::TempDir;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    //  uopool needs connection to the execution client
    if let Ok(provider_url) = env::var("PROVIDER_URL") {
        // initialize database env
        let dir = TempDir::new().unwrap();
        let env = Arc::new(init_env::<WriteMap>(dir.into_path()).expect("Init mdbx failed"));
        env.create_tables().expect("Create mdbx database tables failed");
        println!("Database uopool created!");

        let provider =
            Arc::new(create_http_provider(provider_url.as_str(), Duration::from_secs(1)).await?);
        let ep = Address::from_str(ADDRESS)?;
        let chain = Chain::dev();
        let entry_point = EntryPoint::new(provider.clone(), ep);
        let mempool = Mempool::new(
            Box::new(DatabaseTable::<WriteMap, UserOperations>::new(env.clone())),
            Box::new(DatabaseTable::<WriteMap, UserOperationsBySender>::new(env.clone())),
            Box::new(DatabaseTable::<WriteMap, UserOperationsByEntity>::new(env.clone())),
            Box::new(DatabaseTable::<WriteMap, CodeHashes>::new(env.clone())),
        );
        let reputation = Reputation::new(
            MIN_INCLUSION_RATE_DENOMINATOR,
            THROTTLING_SLACK,
            BAN_SLACK,
            1.into(),
            MIN_UNSTAKE_DELAY.into(),
            Arc::new(RwLock::new(HashSet::<Address>::default())),
            Arc::new(RwLock::new(HashSet::<Address>::default())),
            Box::new(Arc::new(RwLock::new(HashMap::<Address, ReputationEntry>::default()))),
        );
        let builder = UoPoolBuilder::new(
            UoPoolMode::Standard,
            provider.clone(),
            ep.clone(),
            chain,
            U256::from(5000000),
            mempool,
            reputation,
            new_canonical(entry_point, chain, U256::from(5000000), U256::from(1)),
            None,
        );

        // size of mempool
        println!(
            "Mempool size: {size}",
            size = builder.uopool().get_all().expect("should work").len()
        );
    }

    Ok(())
}

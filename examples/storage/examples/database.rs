use alloy_chains::Chain;
use ethers::types::{Address, U256};
use parking_lot::RwLock;
use silius_contracts::EntryPoint;
use silius_mempool::{
    init_db, validate::validator::new_canonical, CodeHashes, DatabaseArguments, DatabaseTable,
    Mempool, Reputation, Tables, UoPoolBuilder, UserOperations, UserOperationsByEntity,
    UserOperationsBySender,
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
use tempdir::TempDir;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    //  uopool needs connection to the execution client
    if let Ok(provider_url) = env::var("PROVIDER_URL") {
        // initialize database env
        let dir = TempDir::new("silius-db").unwrap();
        let env =
            init_db(dir.into_path(), DatabaseArguments::default().with_default_tables(Some(false)))
                .unwrap();

        for table in Tables::ALL {
            env.create_table(table.name(), table.is_dupsort()).unwrap();
        }

        let env = Arc::new(env);

        println!("Database uopool created!");

        let provider =
            Arc::new(create_http_provider(provider_url.as_str(), Duration::from_secs(1)).await?);
        let ep = Address::from_str(ADDRESS)?;
        let chain = Chain::dev();
        let entry_point = EntryPoint::new(provider.clone(), ep);
        let mempool = Mempool::new(
            Box::new(DatabaseTable::<UserOperations>::new(env.clone())),
            Box::new(DatabaseTable::<UserOperationsBySender>::new(env.clone())),
            Box::new(DatabaseTable::<UserOperationsByEntity>::new(env.clone())),
            Box::new(DatabaseTable::<CodeHashes>::new(env)),
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

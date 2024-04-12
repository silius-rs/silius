use alloy_chains::Chain;
use ethers::types::{Address, U256};
use parking_lot::RwLock;
use silius_contracts::EntryPoint;
use silius_mempool::{validate::validator::new_canonical, Mempool, Reputation, UoPoolBuilder};
use silius_primitives::{
    constants::{
        entry_point::ADDRESS,
        validation::reputation::{
            BAN_SLACK, MIN_INCLUSION_RATE_DENOMINATOR, MIN_UNSTAKE_DELAY, THROTTLING_SLACK,
        },
    },
    provider::create_http_provider,
    reputation::ReputationEntry,
    simulation::CodeHash,
    UoPoolMode, UserOperationHash, UserOperationSigned,
};
use std::{
    collections::{HashMap, HashSet},
    env,
    str::FromStr,
    sync::Arc,
    time::Duration,
};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    //  uopool needs connection to the execution client
    if let Ok(provider_url) = env::var("PROVIDER_URL") {
        let provider =
            Arc::new(create_http_provider(provider_url.as_str(), Duration::from_secs(1)).await?);
        let ep = Address::from_str(ADDRESS)?;
        let chain = Chain::dev();
        let entry_point = EntryPoint::new(provider.clone(), ep);
        let mempool = Mempool::new(
            Box::new(Arc::new(RwLock::new(
                HashMap::<UserOperationHash, UserOperationSigned>::default(),
            ))),
            Box::new(Arc::new(RwLock::new(
                HashMap::<Address, HashSet<UserOperationHash>>::default(),
            ))),
            Box::new(Arc::new(RwLock::new(
                HashMap::<Address, HashSet<UserOperationHash>>::default(),
            ))),
            Box::new(Arc::new(RwLock::new(HashMap::<UserOperationHash, Vec<CodeHash>>::default()))),
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

        // optional: subscription to block updates and reputation updates
        // builder.register_block_updates(block_stream);
        // builder.register_reputation_updates();

        println!("In-memory uopool created!");

        // size of mempool
        println!("Mempool size: {size}", size = builder.uopool().get_all().expect("work").len());
    };

    Ok(())
}

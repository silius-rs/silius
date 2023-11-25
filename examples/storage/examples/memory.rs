use silius_uopool::UserOperationOp;
use std::{collections::HashMap, env};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    //  uopool needs connection to the execution client
    if let Ok(_) = env::var("PROVIDER_URL") {
        let hashmap: HashMap<_, _> = HashMap::new();
        // optional: subscription to block updates and reputation updates
        // builder.register_block_updates(block_stream);
        // builder.register_reputation_updates();

        println!("In-memory uopool created!");

        // size of mempool
        println!("Mempool size: {size}", size = hashmap.get_all().len());
    };

    Ok(())
}

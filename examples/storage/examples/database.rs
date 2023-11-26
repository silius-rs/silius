use silius_uopool::{init_env, DatabaseTable, UserOperationOp, UserOperations, WriteMap};
use std::{env, sync::Arc};
use tempdir::TempDir;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    //  uopool needs connection to the execution client
    if let Ok(_) = env::var("PROVIDER_URL") {
        // initialize database env
        let dir = TempDir::new("silius-db").unwrap();
        let env = Arc::new(init_env::<WriteMap>(dir.into_path()).expect("Init mdbx failed"));
        env.create_tables()
            .expect("Create mdbx database tables failed");

        let database: DatabaseTable<WriteMap, UserOperations> = DatabaseTable::new(env);
        println!("Database uopool created!");

        // size of mempool
        println!(
            "Mempool size: {size}",
            size = database.get_all().unwrap().len()
        );
    }

    Ok(())
}

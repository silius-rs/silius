use super::{tables::EntitiesReputation, utils::WrapAddress, DatabaseTable};
use crate::{mempool::ClearOp, reputation::ReputationEntryOp, ReputationError};
use ethers::types::Address;
use reth_db::{
    cursor::DbCursorRO,
    database::Database,
    transaction::{DbTx, DbTxMut},
};
use silius_primitives::reputation::ReputationEntry;

impl ClearOp for DatabaseTable<EntitiesReputation> {
    fn clear(&mut self) {
        let tx = self.env.tx_mut().expect("clear database tx should work");
        tx.clear::<EntitiesReputation>().expect("clear succeed");
        tx.commit().expect("clear commit succeed");
    }
}

impl ReputationEntryOp for DatabaseTable<EntitiesReputation> {
    fn get_entry(&self, addr: &Address) -> Result<Option<ReputationEntry>, ReputationError> {
        let addr_wrap: WrapAddress = (*addr).into();

        let tx = self.env.tx()?;
        let res = tx.get::<EntitiesReputation>(addr_wrap)?;
        tx.commit()?;
        Ok(res.map(|o| o.into()))
    }

    fn set_entry(
        &mut self,
        entry: ReputationEntry,
    ) -> Result<Option<ReputationEntry>, ReputationError> {
        let tx = self.env.tx_mut()?;
        let original = tx.get::<EntitiesReputation>((entry.address).into())?;
        tx.put::<EntitiesReputation>((entry.address).into(), entry.into())?;
        tx.commit()?;
        Ok(original.map(|o| o.into()))
    }

    fn contains_entry(&self, addr: &Address) -> Result<bool, ReputationError> {
        Ok(self.get_entry(addr)?.is_some())
    }

    fn get_all(&self) -> Vec<ReputationEntry> {
        self.env
            .tx()
            .and_then(|tx| {
                let mut c = tx.cursor_read::<EntitiesReputation>()?;
                let res: Vec<ReputationEntry> = c
                    .walk(Some(WrapAddress::default()))?
                    .map(|a| a.map(|(_, v)| v.into()))
                    .collect::<Result<Vec<_>, _>>()?;
                tx.commit()?;
                Ok(res)
            })
            .unwrap_or_else(|_| vec![])
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        database::tables::{EntitiesReputation, Tables},
        utils::tests::reputation_test_case,
        DatabaseTable, Reputation,
    };
    use ethers::types::{Address, U256};
    use parking_lot::RwLock;
    use reth_db::{init_db, mdbx::DatabaseArguments};
    use silius_primitives::constants::validation::reputation::{
        BAN_SLACK, MIN_INCLUSION_RATE_DENOMINATOR, THROTTLING_SLACK,
    };
    use std::{collections::HashSet, sync::Arc};
    use tempdir::TempDir;

    #[tokio::test]
    async fn database_reputation() {
        let data_dir = TempDir::new("test-silius-db").unwrap();
        let env = init_db(&data_dir, DatabaseArguments::default().with_default_tables(Some(false)))
            .unwrap();

        for table in Tables::ALL {
            env.create_table(table.name(), table.is_dupsort()).unwrap();
        }

        let entry: Box<DatabaseTable<EntitiesReputation>> =
            Box::new(DatabaseTable::new(Arc::new(env)));
        let reputation = Reputation::new(
            MIN_INCLUSION_RATE_DENOMINATOR,
            THROTTLING_SLACK,
            BAN_SLACK,
            U256::from(1),
            U256::from(0),
            Arc::new(RwLock::new(HashSet::<Address>::default())),
            Arc::new(RwLock::new(HashSet::<Address>::default())),
            entry,
        );

        reputation_test_case(reputation);
    }
}

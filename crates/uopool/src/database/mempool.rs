use super::env::Env;
use super::{
    env::DBError,
    tables::{CodeHashes, UserOperations, UserOperationsBySender},
    utils::{WrapAddress, WrapUserOperation, WrapUserOperationHash},
};
use crate::mempool::Mempool;
use ethers::types::{Address, U256};
use reth_db::{
    cursor::{DbCursorRO, DbDupCursorRO},
    database::Database,
    mdbx::EnvironmentKind,
    transaction::{DbTx, DbTxMut},
};
use silius_primitives::{simulation::CodeHash, UserOperation, UserOperationHash};
use std::sync::Arc;

#[derive(Debug)]
pub struct DatabaseMempool<E: EnvironmentKind> {
    env: Arc<Env<E>>,
}

impl<E: EnvironmentKind> DatabaseMempool<E> {
    pub fn new(env: Arc<Env<E>>) -> Self {
        Self { env }
    }
}

impl<E: EnvironmentKind> Mempool for DatabaseMempool<E> {
    type UserOperations = Vec<UserOperation>;
    type CodeHashes = Vec<CodeHash>;
    type Error = DBError;

    fn add(
        &mut self,
        uo: UserOperation,
        ep: &Address,
        chain_id: &U256,
    ) -> Result<UserOperationHash, DBError> {
        let hash = uo.hash(ep, chain_id);
        let tx = self.env.tx_mut()?;

        let uo_hash_wrap: WrapUserOperationHash = hash.into();
        let uo_wrap: WrapUserOperation = uo.clone().into();

        tx.put::<UserOperations>(uo_hash_wrap, uo_wrap.clone())?;
        tx.put::<UserOperationsBySender>(uo.sender.into(), uo_wrap)?;
        tx.commit()?;
        Ok(hash)
    }

    fn get(&self, uo_hash: &UserOperationHash) -> Result<Option<UserOperation>, DBError> {
        let uo_hash_wrap: WrapUserOperationHash = (*uo_hash).into();

        let tx = self.env.tx()?;
        let res = tx.get::<UserOperations>(uo_hash_wrap)?;
        tx.commit()?;

        Ok(res.map(|uo| uo.into()))
    }

    fn get_all_by_sender(&self, sender: &Address) -> Self::UserOperations {
        let sender_wrap: WrapAddress = (*sender).into();
        self.env
            .tx()
            .and_then(|tx| {
                let mut cursor = tx.cursor_dup_read::<UserOperationsBySender>()?;
                let res: Vec<UserOperation> = cursor
                    .walk_dup(Some(sender_wrap.clone()), Some(Address::default().into()))?
                    .map(|a| a.map(|(_, v)| v.into()))
                    .collect::<Result<Vec<_>, _>>()?;
                tx.commit()?;
                Ok(res)
            })
            .unwrap_or_else(|_| vec![])
    }

    fn get_number_by_sender(&self, sender: &Address) -> usize {
        let sender_wrap: WrapAddress = (*sender).into();
        self.env
            .tx()
            .and_then(|tx| {
                let mut cursor = tx.cursor_dup_read::<UserOperationsBySender>()?;
                let res = cursor
                    .walk_dup(Some(sender_wrap.clone()), Some(Address::default().into()))?
                    .count();
                tx.commit()?;
                Ok(res)
            })
            .unwrap_or(0)
    }

    fn has_code_hashes(&self, uo_hash: &UserOperationHash) -> Result<bool, Self::Error> {
        let uo_hash_wrap: WrapUserOperationHash = (*uo_hash).into();

        let tx = self.env.tx()?;
        let res = tx.get::<CodeHashes>(uo_hash_wrap)?;
        tx.commit()?;
        Ok(res.is_some())
    }

    fn get_code_hashes(&self, uo_hash: &UserOperationHash) -> Self::CodeHashes {
        let uo_hash_wrap: WrapUserOperationHash = (*uo_hash).into();

        self.env
            .tx()
            .and_then(|tx| {
                let mut cursor = tx.cursor_dup_read::<CodeHashes>()?;
                let res: Vec<CodeHash> = cursor
                    .walk_dup(Some(uo_hash_wrap), Some(Address::default().into()))?
                    .map(|a| a.map(|(_, v)| v.into()))
                    .collect::<Result<Vec<_>, _>>()?;
                tx.commit()?;
                Ok(res)
            })
            .unwrap_or_else(|_| vec![])
    }

    fn set_code_hashes(
        &mut self,
        uo_hash: &UserOperationHash,
        hashes: &Self::CodeHashes,
    ) -> Result<(), Self::Error> {
        let uo_hash_wrap: WrapUserOperationHash = (*uo_hash).into();

        let tx = self.env.tx_mut()?;
        let res = tx.get::<CodeHashes>(uo_hash_wrap.clone())?;
        if res.is_some() {
            tx.delete::<CodeHashes>(uo_hash_wrap.clone(), None)?;
        }
        for hash in hashes {
            tx.put::<CodeHashes>(uo_hash_wrap.clone(), hash.clone().into())?;
        }
        tx.commit()?;
        Ok(())
    }

    fn remove(&mut self, uo_hash: &UserOperationHash) -> Result<(), DBError> {
        let uo_hash_wrap: WrapUserOperationHash = (*uo_hash).into();

        let tx = self.env.tx_mut()?;
        if let Some(uo) = tx.get::<UserOperations>(uo_hash_wrap.clone())? {
            tx.delete::<UserOperations>(uo_hash_wrap.clone(), None)?;
            tx.delete::<UserOperationsBySender>(uo.0.sender.into(), Some(uo))?;
            tx.delete::<CodeHashes>(uo_hash_wrap, None)?;
            tx.commit()?;
            Ok(())
        } else {
            Err(DBError::NotFound)
        }
    }

    fn get_sorted(&self) -> Result<Self::UserOperations, DBError> {
        self.env
            .tx()
            .and_then(|tx| {
                let mut cursor = tx.cursor_read::<UserOperations>()?;
                let mut uos: Vec<UserOperation> = cursor
                    .walk(Some(WrapUserOperationHash::default()))?
                    .map(|a| a.map(|(_, uo)| uo.into()))
                    .collect::<Result<Vec<_>, _>>()?;
                uos.sort_by(|a, b| {
                    if a.max_priority_fee_per_gas != b.max_priority_fee_per_gas {
                        b.max_priority_fee_per_gas.cmp(&a.max_priority_fee_per_gas)
                    } else {
                        a.nonce.cmp(&b.nonce)
                    }
                });
                Ok(uos)
            })
            .map_err(DBError::DBInternalError)
    }

    fn get_all(&self) -> Self::UserOperations {
        self.env
            .tx()
            .and_then(|tx| {
                let mut c = tx.cursor_read::<UserOperations>()?;
                let res: Vec<UserOperation> = c
                    .walk(Some(WrapUserOperationHash::default()))?
                    .map(|a| a.map(|(_, v)| v.into()))
                    .collect::<Result<Vec<_>, _>>()?;
                tx.commit()?;
                Ok(res)
            })
            .unwrap_or_else(|_| vec![])
    }

    fn clear(&mut self) {
        self.env
            .tx_mut()
            .and_then(|tx| {
                tx.clear::<UserOperations>()?;
                tx.clear::<UserOperationsBySender>()?;
                tx.commit()
            })
            .expect("Clear database failed");
    }
}

#[cfg(test)]
mod tests {
    use crate::{database::init_env, utils::tests::mempool_test_case, DatabaseMempool};
    use reth_libmdbx::WriteMap;
    use std::sync::Arc;
    use tempdir::TempDir;

    #[allow(clippy::unit_cmp)]
    #[tokio::test]
    async fn database_mempool() {
        let dir = TempDir::new("test-silius-db").unwrap();

        let env = init_env::<WriteMap>(dir.into_path()).unwrap();
        env.create_tables()
            .expect("Create mdbx database tables failed");
        let mempool: DatabaseMempool<WriteMap> = DatabaseMempool::new(Arc::new(env));

        mempool_test_case(mempool, "NotFound");
    }
}

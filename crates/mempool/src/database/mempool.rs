use super::{
    env::DatabaseError,
    tables::{CodeHashes, UserOperations, UserOperationsByEntity, UserOperationsBySender},
    utils::{
        WrapAddress, WrapCodeHash, WrapCodeHashVec, WrapUserOpSet, WrapUserOperationHash,
        WrapUserOperationSigned,
    },
    DatabaseTable,
};
use crate::{
    mempool::{
        AddRemoveUserOp, AddRemoveUserOpHash, ClearOp, UserOperationAddrOp,
        UserOperationCodeHashOp, UserOperationOp,
    },
    MempoolErrorKind,
};
use ethers::types::Address;
use reth_db::{
    cursor::DbCursorRO,
    database::Database,
    mdbx::EnvironmentKind,
    transaction::{DbTx, DbTxMut},
};
use silius_primitives::{simulation::CodeHash, UserOperation, UserOperationHash};

impl<E: EnvironmentKind> AddRemoveUserOp for DatabaseTable<E, UserOperations> {
    fn add(&mut self, uo: UserOperation) -> Result<UserOperationHash, MempoolErrorKind> {
        let tx = self.env.tx_mut()?;
        let uo_hash_wrap: WrapUserOperationHash = uo.hash.into();
        let uo_wrap: WrapUserOperationSigned = uo.user_operation.into();
        tx.put::<UserOperations>(uo_hash_wrap, uo_wrap)?;
        tx.commit()?;
        Ok(uo.hash)
    }

    fn remove_by_uo_hash(&mut self, uo_hash: &UserOperationHash) -> Result<bool, MempoolErrorKind> {
        let uo_hash_wrap: WrapUserOperationHash = (*uo_hash).into();

        let tx = self.env.tx_mut()?;
        let original_value = tx.get::<UserOperations>(uo_hash_wrap.clone())?;
        tx.delete::<UserOperations>(uo_hash_wrap.clone(), None)?;
        tx.commit()?;
        Ok(original_value.is_some())
    }
}

macro_rules! impl_add_remove_user_op_hash {
    ($table: ident) => {
        impl<E: EnvironmentKind> AddRemoveUserOpHash for DatabaseTable<E, $table> {
            fn add(
                &mut self,
                address: &Address,
                uo_hash: UserOperationHash,
            ) -> Result<(), MempoolErrorKind> {
                let tx = self.env.tx_mut()?;
                let uo_hash_wrap: WrapUserOperationHash = uo_hash.into();
                if let Some(mut uo_hash_set) = tx.get::<$table>(address.clone().into())? {
                    uo_hash_set.insert(uo_hash_wrap);
                    tx.put::<$table>(address.clone().into(), uo_hash_set)?;
                } else {
                    let mut uo_hash_set = WrapUserOpSet::default();
                    uo_hash_set.insert(uo_hash_wrap);
                    tx.put::<$table>(address.clone().into(), uo_hash_set)?;
                }
                tx.commit()?;
                Ok(())
            }

            fn remove_uo_hash(
                &mut self,
                address: &Address,
                uo_hash: &UserOperationHash,
            ) -> Result<bool, MempoolErrorKind> {
                let tx = self.env.tx_mut()?;
                if let Some(mut uo_hash_set) =
                    tx.get::<UserOperationsBySender>(address.clone().into())?
                {
                    uo_hash_set.remove(&uo_hash.clone().into());
                    if uo_hash_set.is_empty() {
                        tx.delete::<UserOperationsBySender>(address.clone().into(), None)?;
                    } else {
                        tx.put::<UserOperationsBySender>(address.clone().into(), uo_hash_set)?;
                    }
                    tx.commit()?;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
        }
    };
}

impl_add_remove_user_op_hash!(UserOperationsBySender);
impl_add_remove_user_op_hash!(UserOperationsByEntity);

impl<E: EnvironmentKind> UserOperationOp for DatabaseTable<E, UserOperations> {
    fn get_by_uo_hash(
        &self,
        uo_hash: &UserOperationHash,
    ) -> Result<Option<UserOperation>, MempoolErrorKind> {
        let uo_hash_wrap: WrapUserOperationHash = (*uo_hash).into();

        let tx = self.env.tx()?;
        let res = tx.get::<UserOperations>(uo_hash_wrap)?;
        tx.commit()?;

        Ok(res.map(|uo| UserOperation::from_user_operation_signed(*uo_hash, uo.into())))
    }

    fn get_sorted(&self) -> Result<Vec<UserOperation>, MempoolErrorKind> {
        self.env
            .tx()
            .and_then(|tx| {
                let mut cursor = tx.cursor_read::<UserOperations>()?;
                let mut uos: Vec<UserOperation> = cursor
                    .walk(Some(WrapUserOperationHash::default()))?
                    .map(|a| {
                        a.map(|(hash, uo)| {
                            UserOperation::from_user_operation_signed(hash.into(), uo.into())
                        })
                    })
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
            .map_err(|e| MempoolErrorKind::Database(DatabaseError::Internal(e)))
    }

    fn get_all(&self) -> Result<Vec<UserOperation>, MempoolErrorKind> {
        let tx = self.env.tx()?;
        let mut c = tx.cursor_read::<UserOperations>()?;
        let mut res = Vec::new();
        while let Some((hash, uo)) = c.next()? {
            res.push(UserOperation::from_user_operation_signed(hash.into(), uo.into()))
        }

        Ok(res)
    }
}
macro_rules! impl_user_op_addr_op {
    ($table:ident) => {
        impl<E: EnvironmentKind> UserOperationAddrOp for DatabaseTable<E, $table> {
            fn get_all_by_address(&self, address: &Address) -> Vec<UserOperationHash> {
                let address_wrap: WrapAddress = (*address).into();
                self.env
                    .tx()
                    .and_then(|tx| {
                        if let Some(uo_hash_set) = tx.get::<$table>(address_wrap)? {
                            Ok(uo_hash_set.to_vec())
                        } else {
                            Ok(vec![])
                        }
                    })
                    .unwrap_or_else(|_| vec![])
            }
        }
    };
}
impl_user_op_addr_op!(UserOperationsBySender);
impl_user_op_addr_op!(UserOperationsByEntity);

impl<E: EnvironmentKind> UserOperationCodeHashOp for DatabaseTable<E, CodeHashes> {
    fn has_code_hashes(&self, uo_hash: &UserOperationHash) -> Result<bool, MempoolErrorKind> {
        let uo_hash_wrap: WrapUserOperationHash = (*uo_hash).into();

        let tx = self.env.tx()?;
        let res = tx.get::<CodeHashes>(uo_hash_wrap)?;
        Ok(res.is_some())
    }

    fn set_code_hashes(
        &mut self,
        uo_hash: &UserOperationHash,
        hashes: Vec<CodeHash>,
    ) -> Result<(), MempoolErrorKind> {
        let uo_hash_wrap: WrapUserOperationHash = (*uo_hash).into();

        let tx = self.env.tx_mut()?;
        let wrap_hashes: WrapCodeHashVec =
            hashes.into_iter().map(Into::into).collect::<Vec<WrapCodeHash>>().into();
        tx.put::<CodeHashes>(uo_hash_wrap, wrap_hashes)?;
        tx.commit()?;
        Ok(())
    }

    fn get_code_hashes(
        &self,
        uo_hash: &UserOperationHash,
    ) -> Result<Vec<CodeHash>, MempoolErrorKind> {
        let uo_hash_wrap: WrapUserOperationHash = (*uo_hash).into();
        let tx = self.env.tx_mut()?;
        let res = tx.get::<CodeHashes>(uo_hash_wrap)?;
        Ok(res
            .map(|hashes| {
                let hashes: Vec<WrapCodeHash> = hashes.into();
                hashes.into_iter().map(Into::into).collect::<Vec<CodeHash>>()
            })
            .unwrap_or(vec![]))
    }

    fn remove_code_hashes(
        &mut self,
        uo_hash: &UserOperationHash,
    ) -> Result<bool, MempoolErrorKind> {
        let uo_hash_wrap: WrapUserOperationHash = (*uo_hash).into();
        let tx = self.env.tx_mut()?;
        if tx.get::<CodeHashes>(uo_hash_wrap.clone())?.is_some() {
            tx.delete::<CodeHashes>(uo_hash_wrap, None)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

macro_rules! impl_clear {
    ($table: ident) => {
        impl<E: EnvironmentKind> ClearOp for DatabaseTable<E, $table> {
            fn clear(&mut self) {
                self.env
                    .tx_mut()
                    .and_then(|tx| {
                        tx.clear::<$table>()?;
                        tx.commit()
                    })
                    .expect("Clear database failed");
            }
        }
    };
}
impl_clear!(UserOperations);
impl_clear!(UserOperationsBySender);
impl_clear!(UserOperationsByEntity);
impl_clear!(CodeHashes);

#[cfg(test)]
mod tests {
    use crate::{
        database::{
            init_env,
            tables::{CodeHashes, UserOperations, UserOperationsByEntity, UserOperationsBySender},
            DatabaseTable,
        },
        utils::tests::mempool_test_case,
        Mempool,
    };
    use reth_libmdbx::WriteMap;
    use std::sync::Arc;
    use tempdir::TempDir;

    #[allow(clippy::unit_cmp)]
    #[tokio::test]
    async fn database_mempool() {
        let dir = TempDir::new("test-silius-db").unwrap();

        let env = init_env::<WriteMap>(dir.into_path()).unwrap();
        env.create_tables().expect("Create mdbx database tables failed");
        let env = Arc::new(env);
        let uo_ops: DatabaseTable<WriteMap, UserOperations> = DatabaseTable::new(env.clone());
        let uo_ops_sender: DatabaseTable<WriteMap, UserOperationsBySender> =
            DatabaseTable::new(env.clone());
        let uo_ops_entity: DatabaseTable<WriteMap, UserOperationsByEntity> =
            DatabaseTable::new(env.clone());
        let uo_ops_codehashes: DatabaseTable<WriteMap, CodeHashes> =
            DatabaseTable::new(env.clone());
        let mempool = Mempool::new(uo_ops, uo_ops_sender, uo_ops_entity, uo_ops_codehashes);

        mempool_test_case(mempool);
    }
}

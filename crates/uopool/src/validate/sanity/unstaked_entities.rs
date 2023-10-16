use crate::{
    mempool::Mempool,
    reputation::Reputation as Rep,
    uopool::{VecCh, VecUo},
    validate::{SanityCheck, SanityHelper},
};
use ethers::{
    providers::Middleware,
    types::{Address, U256},
};
use silius_primitives::{
    consts::{
        entities::{FACTORY, PAYMASTER, SENDER},
        reputation::{
            INCLUSION_RATE_FACTOR, SAME_SENDER_MEMPOOL_COUNT, SAME_UNSTAKED_ENTITY_MEMPOOL_COUNT,
        },
    },
    reputation::{ReputationEntry, ReputationError, StakeInfo},
    sanity::SanityCheckError,
    UserOperation,
};
use std::{cmp, fmt::Debug};

pub struct UnstakedEntities;

impl UnstakedEntities {
    /// Gets the deposit info for entity.
    async fn get_stake<'a, M: Middleware, P, R, E>(
        &self,
        addr: &Address,
        helper: &SanityHelper<'a, M, P, R, E>,
    ) -> Result<StakeInfo, SanityCheckError>
    where
        P: Mempool<UserOperations = VecUo, CodeHashes = VecCh, Error = E> + Send + Sync,
        R: Rep<ReputationEntries = Vec<ReputationEntry>, Error = E> + Send + Sync,
        E: Debug,
    {
        let info = helper
            .entry_point
            .get_deposit_info(addr)
            .await
            .map_err(|_| SanityCheckError::UnknownError {
                message: "Couldn't retrieve deposit info from entry point".to_string(),
            })?;

        Ok(StakeInfo {
            address: *addr,
            stake: U256::from(info.stake),
            unstake_delay: U256::from(info.unstake_delay_sec),
        })
    }

    /// Gets the reputation entry for entity.
    fn get_entity<M: Middleware, P, R, E>(
        &self,
        addr: &Address,
        helper: &SanityHelper<M, P, R, E>,
    ) -> Result<ReputationEntry, SanityCheckError>
    where
        P: Mempool<UserOperations = VecUo, CodeHashes = VecCh, Error = E> + Send + Sync,
        R: Rep<ReputationEntries = Vec<ReputationEntry>, Error = E> + Send + Sync,
        E: Debug,
    {
        helper
            .reputation
            .get(addr)
            .map_err(|_| SanityCheckError::UnknownError {
                message: "Failed to retrieve reputation entry".into(),
            })
    }

    /// Calculates allowed number of user operations
    fn calculate_allowed_user_operations(entity: ReputationEntry) -> u64 {
        if entity.uo_seen == 0 {
            SAME_UNSTAKED_ENTITY_MEMPOOL_COUNT as u64
        } else {
            SAME_UNSTAKED_ENTITY_MEMPOOL_COUNT as u64
                + ((entity.uo_included as f64 / entity.uo_seen as f64)
                    * INCLUSION_RATE_FACTOR as f64) as u64
                + cmp::min(entity.uo_included, 10000)
        }
    }
}

#[async_trait::async_trait]
impl<M: Middleware, P, R, E> SanityCheck<M, P, R, E> for UnstakedEntities
where
    P: Mempool<UserOperations = VecUo, CodeHashes = VecCh, Error = E> + Send + Sync,
    R: Rep<ReputationEntries = Vec<ReputationEntry>, Error = E> + Send + Sync,
    E: Debug,
{
    /// The [check_user_operation] method implementation that performs the sanity check for the unstaked entities.
    ///
    /// # Arguments
    /// `uo` - The user operation to be checked.
    /// `helper` - The [sanity check helper](SanityHelper) that contains the necessary data to perform the sanity check.
    ///
    /// # Returns
    /// None if the sanity check is successful, otherwise a [SanityCheckError] is returned.
    async fn check_user_operation(
        &self,
        uo: &UserOperation,
        helper: &SanityHelper<M, P, R, E>,
    ) -> Result<(), SanityCheckError> {
        let (sender, factory, paymaster) = uo.get_entities();

        // sender
        // [STO-040]
        if helper.mempool.get_number_by_entity(&sender) > 0 {
            return Err(SanityCheckError::EntityVerification {
                entity: SENDER.to_string(),
                address: sender,
                message:
                    "is used as a different entity in another UserOperation currently in mempool"
                        .to_string(),
            });
        }

        let sender_stake = self.get_stake(&sender, helper).await?;
        if helper
            .reputation
            .verify_stake(SENDER, Some(sender_stake))
            .is_err()
        {
            // [UREP-010]
            if helper.mempool.get_number_by_sender(&uo.sender) >= SAME_SENDER_MEMPOOL_COUNT {
                return Err(ReputationError::UnstakedEntityVerification {
                    entity: SENDER.to_string(),
                    address: uo.sender,
                    message: "has too many user operations in the mempool".into(),
                }
                .into());
            }
        }

        // factory
        if let Some(factory) = factory {
            // [STO-040]
            if helper.mempool.get_number_by_sender(&factory) > 0 {
                return Err(SanityCheckError::EntityVerification {
                    entity: FACTORY.to_string(),
                    address: factory,
                    message:
                        "is used as a sender entity in another UserOperation currently in mempool"
                            .to_string(),
                });
            }

            let factory_stake = self.get_stake(&factory, helper).await?;
            if helper
                .reputation
                .verify_stake(FACTORY, Some(factory_stake))
                .is_err()
            {
                // [UREP-020]
                let entity = self.get_entity(&factory, helper)?;
                let uos_allowed = Self::calculate_allowed_user_operations(entity);
                if helper.mempool.get_number_by_entity(&factory) as u64 >= uos_allowed {
                    return Err(ReputationError::UnstakedEntityVerification {
                        entity: FACTORY.to_string(),
                        address: factory,
                        message: "has too many user operations in the mempool".into(),
                    }
                    .into());
                }
            }
        }

        // paymaster
        if let Some(paymaster) = paymaster {
            // [STO-040]
            if helper.mempool.get_number_by_sender(&paymaster) > 0 {
                return Err(SanityCheckError::EntityVerification {
                    entity: PAYMASTER.to_string(),
                    address: paymaster,
                    message:
                        "is used as a sender entity in another UserOperation currently in mempool"
                            .to_string(),
                });
            }

            let paymaster_stake = self.get_stake(&paymaster, helper).await?;
            if helper
                .reputation
                .verify_stake(PAYMASTER, Some(paymaster_stake))
                .is_err()
            {
                // [UREP-020]
                let entity = self.get_entity(&paymaster, helper)?;
                let uos_allowed = Self::calculate_allowed_user_operations(entity);
                if helper.mempool.get_number_by_entity(&paymaster) as u64 >= uos_allowed {
                    return Err(ReputationError::UnstakedEntityVerification {
                        entity: PAYMASTER.to_string(),
                        address: paymaster,
                        message: "has too many user operations in the mempool".into(),
                    }
                    .into());
                }
            }
        }

        Ok(())
    }
}

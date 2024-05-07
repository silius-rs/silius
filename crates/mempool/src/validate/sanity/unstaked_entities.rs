use crate::{
    mempool::Mempool,
    reputation::Reputation,
    validate::{SanityCheck, SanityHelper},
    ReputationError, SanityError,
};
use ethers::{
    providers::Middleware,
    types::{Address, U256},
};
use silius_primitives::{
    constants::validation::{
        entities::{FACTORY, PAYMASTER, SENDER},
        reputation::{
            INCLUSION_RATE_FACTOR, SAME_SENDER_MEMPOOL_COUNT, SAME_UNSTAKED_ENTITY_MEMPOOL_COUNT,
        },
    },
    reputation::{ReputationEntry, StakeInfo},
    UserOperation,
};
use std::cmp;

#[derive(Clone)]
pub struct UnstakedEntities;

impl UnstakedEntities {
    /// Gets the deposit info for entity.
    async fn get_stake<'a, M: Middleware>(
        &self,
        addr: &Address,
        helper: &SanityHelper<'a, M>,
    ) -> Result<StakeInfo, SanityError> {
        let info = helper.entry_point.get_deposit_info(addr).await?;

        Ok(StakeInfo {
            address: *addr,
            stake: U256::from(info.stake),
            unstake_delay: U256::from(info.unstake_delay_sec),
        })
    }

    /// Gets the reputation entry for entity.
    fn get_entity<M: Middleware>(
        &self,
        addr: &Address,
        _helper: &SanityHelper<M>,
        reputation: &Reputation,
    ) -> Result<ReputationEntry, SanityError> {
        reputation.get(addr).map_err(|e| e.into())
    }

    /// Calculates allowed number of user operations
    fn calculate_allowed_user_operations(entity: ReputationEntry) -> u64 {
        if entity.uo_seen == 0 {
            SAME_UNSTAKED_ENTITY_MEMPOOL_COUNT as u64
        } else {
            SAME_UNSTAKED_ENTITY_MEMPOOL_COUNT as u64 +
                ((entity.uo_included as f64 / entity.uo_seen as f64) * INCLUSION_RATE_FACTOR as f64)
                    as u64 +
                cmp::min(entity.uo_included, 10000)
        }
    }
}

#[async_trait::async_trait]
impl<M: Middleware> SanityCheck<M> for UnstakedEntities {
    /// The method implementation that performs the sanity check for the unstaked entities.
    ///
    /// # Arguments
    /// `uo` - The user operation to be checked.
    /// `helper` - The [sanity check helper](SanityHelper) that contains the necessary data to
    /// perform the sanity check.
    ///
    /// # Returns
    /// None if the sanity check is successful, otherwise a [SanityError] is returned.
    async fn check_user_operation(
        &self,
        uo: &UserOperation,
        mempool: &Mempool,
        reputation: &Reputation,
        helper: &SanityHelper<M>,
    ) -> Result<(), SanityError> {
        let (sender, factory, paymaster) = uo.get_entities();

        // [SREP-010] - the "canonical mempool" defines a staked entity if it has MIN_STAKE_VALUE
        // and unstake delay of MIN_UNSTAKE_DELAY

        // sender
        // [STO-040] - UserOperation may not use an entity address (factory/paymaster/aggregator)
        // that is used as an "account" in another UserOperation in the mempool
        if mempool.get_number_by_entity(&sender) > 0 {
            return Err(SanityError::EntityRoles {
                entity: SENDER.into(),
                address: sender,
                entity_other: "different".into(),
            });
        }

        // [UREP-010] - UserOperation with unstaked sender are only allowed up to
        // SAME_SENDER_MEMPOOL_COUNT times in the mempool
        let sender_stake = self.get_stake(&sender, helper).await?;
        if reputation
            .verify_stake(
                SENDER,
                Some(sender_stake),
                helper.val_config.min_stake,
                helper.val_config.min_unstake_delay,
            )
            .is_err() &&
            mempool.get_number_by_sender(&uo.sender) >= SAME_SENDER_MEMPOOL_COUNT
        {
            return Err(ReputationError::UnstakedEntity {
                entity: SENDER.into(),
                address: uo.sender,
            }
            .into());
        }

        // factory
        if let Some(factory) = factory {
            // [STO-040] - UserOperation may not use an entity address
            // (factory/paymaster/aggregator) that is used as an "account" in another UserOperation
            // in the mempool
            if mempool.get_number_by_sender(&factory) > 0 {
                return Err(SanityError::EntityRoles {
                    entity: FACTORY.into(),
                    address: sender,
                    entity_other: "sender".into(),
                });
            }

            let factory_stake = self.get_stake(&factory, helper).await?;
            if reputation
                .verify_stake(
                    FACTORY,
                    Some(factory_stake),
                    helper.val_config.min_stake,
                    helper.val_config.min_unstake_delay,
                )
                .is_err()
            {
                // [UREP-020] - for other entities
                let entity = self.get_entity(&factory, helper, reputation)?;
                let uos_allowed = Self::calculate_allowed_user_operations(entity);
                if mempool.get_number_by_entity(&factory) as u64 >= uos_allowed {
                    return Err(ReputationError::UnstakedEntity {
                        entity: FACTORY.into(),
                        address: factory,
                    }
                    .into());
                }
            }
        }

        // paymaster
        if let Some(paymaster) = paymaster {
            // [STO-040] - UserOperation may not use an entity address
            // (factory/paymaster/aggregator) that is used as an "account" in another UserOperation
            // in the mempool
            if mempool.get_number_by_sender(&paymaster) > 0 {
                return Err(SanityError::EntityRoles {
                    entity: PAYMASTER.into(),
                    address: sender,
                    entity_other: "sender".into(),
                });
            }

            let paymaster_stake = self.get_stake(&paymaster, helper).await?;
            if reputation
                .verify_stake(
                    PAYMASTER,
                    Some(paymaster_stake),
                    helper.val_config.min_stake,
                    helper.val_config.min_unstake_delay,
                )
                .is_err()
            {
                // [UREP-020] - for other entities
                let entity = self.get_entity(&paymaster, helper, reputation)?;
                let uos_allowed = Self::calculate_allowed_user_operations(entity);
                if mempool.get_number_by_entity(&paymaster) as u64 >= uos_allowed {
                    return Err(ReputationError::UnstakedEntity {
                        entity: PAYMASTER.into(),
                        address: paymaster,
                    }
                    .into());
                }
            }
        }

        Ok(())
    }
}

use crate::{
    mempool::Mempool,
    reputation::Reputation as Rep,
    uopool::{VecCh, VecUo},
    validate::{SanityCheck, SanityHelper},
};
use ethers::{providers::Middleware, types::Address};
use silius_primitives::{
    consts::{
        entities::{FACTORY, PAYMASTER, SENDER},
        reputation::THROTTLED_ENTITY_MEMPOOL_COUNT,
    },
    reputation::{ReputationEntry, ReputationError, Status},
    sanity::SanityCheckError,
    UserOperation,
};
use std::fmt::Debug;

pub struct Entities;

impl Entities {
    /// Gets the status for entity.
    fn get_status<M: Middleware, P, R, E>(
        &self,
        addr: &Address,
        helper: &SanityHelper<M, P, R, E>,
    ) -> Result<Status, SanityCheckError>
    where
        P: Mempool<UserOperations = VecUo, CodeHashes = VecCh, Error = E> + Send + Sync,
        R: Rep<ReputationEntries = Vec<ReputationEntry>, Error = E> + Send + Sync,
        E: Debug,
    {
        Ok(Status::from(helper.reputation.get_status(addr).map_err(
            |_| SanityCheckError::UnknownError {
                message: "Failed to retrieve reputation status".into(),
            },
        )?))
    }

    /// [SREP-020]: A BANNED address is not allowed into the mempool.
    fn check_banned(
        &self,
        entity: &str,
        addr: &Address,
        status: &Status,
    ) -> Result<(), SanityCheckError> {
        if *status == Status::BANNED {
            return Err(ReputationError::EntityBanned {
                entity: entity.to_string(),
                address: *addr,
            }
            .into());
        }

        Ok(())
    }

    /// [SREP-030]: A THROTTLED address is limited to THROTTLED_ENTITY_MEMPOOL_COUNT entries in the mempool.
    fn check_throttled<M: Middleware, P, R, E>(
        &self,
        entity: &str,
        addr: &Address,
        status: &Status,
        helper: &SanityHelper<M, P, R, E>,
    ) -> Result<(), SanityCheckError>
    where
        P: Mempool<UserOperations = VecUo, CodeHashes = VecCh, Error = E> + Send + Sync,
        R: Rep<ReputationEntries = Vec<ReputationEntry>, Error = E> + Send + Sync,
        E: Debug,
    {
        if *status == Status::THROTTLED
            && (helper.mempool.get_number_by_sender(addr)
                + helper.mempool.get_number_by_entity(addr))
                >= THROTTLED_ENTITY_MEMPOOL_COUNT
        {
            return Err(ReputationError::ThrottledLimit {
                entity: entity.to_string(),
                address: *addr,
            }
            .into());
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl<M: Middleware, P, R, E> SanityCheck<M, P, R, E> for Entities
where
    P: Mempool<UserOperations = VecUo, CodeHashes = VecCh, Error = E> + Send + Sync,
    R: Rep<ReputationEntries = Vec<ReputationEntry>, Error = E> + Send + Sync,
    E: Debug,
{
    /// The [check_user_operation] method implementation that performs the sanity check for the staked entities.
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
        let status = self.get_status(&sender, helper)?;
        self.check_banned(SENDER, &sender, &status)?;
        self.check_throttled(SENDER, &sender, &status, helper)?;

        // factory
        if let Some(factory) = factory {
            let status = self.get_status(&factory, helper)?;
            self.check_banned(FACTORY, &factory, &status)?;
            self.check_throttled(FACTORY, &factory, &status, helper)?;
        }

        // paymaster
        if let Some(paymaster) = paymaster {
            let status = self.get_status(&paymaster, helper)?;
            self.check_banned(PAYMASTER, &paymaster, &status)?;
            self.check_throttled(PAYMASTER, &paymaster, &status, helper)?;
        }

        Ok(())
    }
}

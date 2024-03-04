use crate::{
    mempool::Mempool,
    reputation::Reputation,
    validate::{SanityCheck, SanityHelper},
    ReputationError, SanityError,
};
use ethers::{providers::Middleware, types::Address};
use silius_primitives::{
    constants::validation::{
        entities::{FACTORY, PAYMASTER, SENDER},
        reputation::THROTTLED_ENTITY_MEMPOOL_COUNT,
    },
    reputation::Status,
    UserOperation,
};

#[derive(Clone)]
pub struct Entities;

impl Entities {
    /// Gets the status for entity.
    fn get_status<M: Middleware>(
        &self,
        addr: &Address,
        _helper: &SanityHelper<M>,
        reputation: &Reputation,
    ) -> Result<Status, SanityError> {
        Ok(Status::from(reputation.get_status(addr)?))
    }

    /// [SREP-020] - a BANNED address is not allowed into the mempool.
    fn check_banned(
        &self,
        entity: &str,
        addr: &Address,
        status: &Status,
    ) -> Result<(), SanityError> {
        if *status == Status::BANNED {
            return Err(
                ReputationError::BannedEntity { entity: entity.into(), address: *addr }.into()
            );
        }

        Ok(())
    }

    /// [SREP-030] - THROTTLED address is limited to THROTTLED_ENTITY_MEMPOOL_COUNT entries in the
    /// mempool
    fn check_throttled<M: Middleware>(
        &self,
        entity: &str,
        addr: &Address,
        status: &Status,
        _helper: &SanityHelper<M>,
        mempool: &Mempool,
        _reputation: &Reputation,
    ) -> Result<(), SanityError> {
        if *status == Status::THROTTLED &&
            (mempool.get_number_by_sender(addr) + mempool.get_number_by_entity(addr)) >=
                THROTTLED_ENTITY_MEMPOOL_COUNT
        {
            return Err(
                ReputationError::ThrottledEntity { entity: entity.into(), address: *addr }.into()
            );
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl<M: Middleware> SanityCheck<M> for Entities {
    /// The method implementation that performs the sanity check for the staked entities.
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

        // [SREP-040] - an OK staked entity is unlimited by the reputation rule

        // sender
        let status = self.get_status(&sender, helper, reputation)?;
        self.check_banned(SENDER, &sender, &status)?;
        self.check_throttled(SENDER, &sender, &status, helper, mempool, reputation)?;

        // factory
        if let Some(factory) = factory {
            let status = self.get_status(&factory, helper, reputation)?;
            self.check_banned(FACTORY, &factory, &status)?;
            self.check_throttled(FACTORY, &factory, &status, helper, mempool, reputation)?;
        }

        // paymaster
        if let Some(paymaster) = paymaster {
            let status = self.get_status(&paymaster, helper, reputation)?;
            self.check_banned(PAYMASTER, &paymaster, &status)?;
            self.check_throttled(PAYMASTER, &paymaster, &status, helper, mempool, reputation)?;
        }

        Ok(())
    }
}

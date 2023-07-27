use dashmap::mapref::one::{Ref, RefMut};
use ethers::{
    providers::Middleware,
    types::{Address, H256},
};
use silius_primitives::UserOperation;
use silius_uopool::UoPool as UserOperationPool;
use tonic::{Code, Status};

pub fn parse_addr(h: Option<crate::H160>) -> Result<Address, Status> {
    match h {
        Some(h) => Ok(h.into()),
        None => Err(Status::new(Code::InvalidArgument, "Address is not valid")),
    }
}

pub fn parse_hash(h: Option<crate::H256>) -> Result<H256, Status> {
    match h {
        Some(h) => Ok(h.into()),
        None => Err(Status::new(Code::InvalidArgument, "Hash is not valid")),
    }
}

pub fn parse_uo(uo: Option<crate::UserOperation>) -> Result<UserOperation, Status> {
    match uo {
        Some(uo) => Ok(uo.into()),
        None => Err(Status::new(
            Code::InvalidArgument,
            "User operation is not valid",
        )),
    }
}

pub fn parse_uo_pool<M: Middleware>(
    uo_pool: Option<Ref<H256, UserOperationPool<M>>>,
) -> Result<Ref<H256, UserOperationPool<M>>, Status> {
    match uo_pool {
        Some(uo_pool) => Ok(uo_pool),
        None => Err(Status::new(
            Code::Unavailable,
            "User operation pool is not available",
        )),
    }
}

pub fn parse_uo_pool_mut<M: Middleware>(
    uo_pool: Option<RefMut<H256, UserOperationPool<M>>>,
) -> Result<RefMut<H256, UserOperationPool<M>>, Status> {
    match uo_pool {
        Some(uo_pool) => Ok(uo_pool),
        None => Err(Status::new(
            Code::Unavailable,
            "User operation pool is not available",
        )),
    }
}

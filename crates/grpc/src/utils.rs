use ethers::types::{Address, H256};
use silius_primitives::UserOperation;
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
        None => Err(Status::new(Code::InvalidArgument, "User operation is not valid")),
    }
}

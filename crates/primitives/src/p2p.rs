//! P2P primitives

use crate::UserOperationSigned;
use ethers::types::{Address, U256 as EthersU256};
use ssz_rs::{Vector, U256};
use ssz_rs_derive::Serializable;

/// P2P message type
#[derive(Clone, Debug, Default, Serializable, PartialEq)]
pub struct VerifiedUserOperation {
    user_operation: UserOperationSigned,
    entry_point: Vector<u8, 20>,
    verified_at_block_hash: U256,
}

impl VerifiedUserOperation {
    pub fn new(
        user_operation: UserOperationSigned,
        entry_point: Address,
        verified_at_block_hash: EthersU256,
    ) -> Self {
        let mut buf: [u8; 32] = [0; 32];
        verified_at_block_hash.to_little_endian(&mut buf);
        let verified_at_block_hash = U256::from_bytes_le(buf);

        Self {
            user_operation,
            entry_point: <Vector<u8, 20>>::try_from(entry_point.as_bytes().to_vec())
                .expect("entrypoint address is valid"),
            verified_at_block_hash,
        }
    }

    pub fn user_operation(self) -> UserOperationSigned {
        self.user_operation
    }

    pub fn entry_point(&self) -> Address {
        Address::from_slice(&self.entry_point)
    }
}

use crate::UserOperation;
use alloy_chains::Chain;
use ethers::types::{Address, U256 as EthersU256};
use ssz_rs::{List, Vector, U256};
use ssz_rs_derive::Serializable;

#[derive(Clone, Debug, Default, Serializable, PartialEq)]
pub struct UserOperationsWithEntryPoint {
    // entrypoint address
    entrypoint_contract: Vector<u8, 20>,
    verified_at_block_hash: U256,
    chain_id: U256,
    user_operations: List<UserOperation, 4096>,
}

impl UserOperationsWithEntryPoint {
    pub fn new(
        entrypoint_address: Address,
        verified_at_block_hash: EthersU256,
        chain_id: EthersU256,
        user_operations: Vec<UserOperation>,
    ) -> Self {
        let mut buf: [u8; 32] = [0; 32];
        verified_at_block_hash.to_little_endian(&mut buf);
        let verified_at_block_hash = U256::from_bytes_le(buf);
        let mut buf: [u8; 32] = [0; 32];
        chain_id.to_little_endian(&mut buf);
        let chain_id = U256::from_bytes_le(buf);
        Self {
            entrypoint_contract: <Vector<u8, 20>>::try_from(entrypoint_address.as_bytes().to_vec())
                .expect("entrypoint address is valid"),
            verified_at_block_hash,
            chain_id,
            // FIXME: should have a bound check here or return Err
            user_operations: <List<UserOperation, 4096>>::try_from(user_operations)
                .expect("Too many user operations"),
        }
    }

    pub fn user_operations(self) -> Vec<UserOperation> {
        self.user_operations.to_vec()
    }

    pub fn entrypoint_address(&self) -> Address {
        Address::from_slice(&self.entrypoint_contract)
    }

    pub fn chain(&self) -> Chain {
        Chain::from(EthersU256::from_little_endian(self.chain_id.to_bytes_le().as_ref()).as_u64())
    }
}

#[derive(Clone, Debug, Default, Serializable)]
pub struct PooledUserOps {
    mempool_id: Vector<u8, 32>,
    more_flag: u64,
    user_operations: List<UserOperation, 4096>,
}

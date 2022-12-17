use crate::contracts::gen::entry_point_api;
use ethers::abi::AbiEncode;
use ethers::prelude::{EthAbiCodec, EthAbiType};
use ethers::types::{Address, Bytes, TransactionReceipt, H256, U256};
use ethers::utils::keccak256;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use std::str::FromStr;

pub type UserOperationHash = H256;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, EthAbiCodec, EthAbiType)]
#[serde(rename_all = "camelCase")]
pub struct UserOperation {
    pub sender: Address,
    pub nonce: U256,
    pub init_code: Bytes,
    pub call_data: Bytes,
    pub call_gas_limit: U256,
    pub verification_gas_limit: U256,
    pub pre_verification_gas: U256,
    pub max_fee_per_gas: U256,
    pub max_priority_fee_per_gas: U256,
    pub paymaster_and_data: Bytes,
    pub signature: Bytes,
}

impl From<UserOperation> for entry_point_api::UserOperation {
    fn from(user_operation: UserOperation) -> Self {
        Self {
            sender: user_operation.sender,
            nonce: user_operation.nonce,
            init_code: user_operation.init_code,
            call_data: user_operation.call_data,
            call_gas_limit: user_operation.call_gas_limit,
            verification_gas_limit: user_operation.verification_gas_limit,
            pre_verification_gas: user_operation.pre_verification_gas,
            max_fee_per_gas: user_operation.max_fee_per_gas,
            max_priority_fee_per_gas: user_operation.max_priority_fee_per_gas,
            paymaster_and_data: user_operation.paymaster_and_data,
            signature: user_operation.signature,
        }
    }
}

impl UserOperation {
    pub fn pack(&self) -> Bytes {
        self.clone().encode_hex().parse::<Bytes>().unwrap()
    }

    pub fn pack_for_signature(&self) -> Bytes {
        let mut encoded = String::from("0x");
        let packed = hex::encode(
            UserOperation {
                signature: Bytes::from_str("0x").unwrap(),
                ..self.clone()
            }
            .encode(),
        );
        encoded.push_str(&packed[..packed.len() - 64]);
        encoded.parse::<Bytes>().unwrap()
    }

    pub fn hash(&self, entry_point_address: Address, chain_id: U256) -> UserOperationHash {
        H256::from_slice(
            keccak256(
                [
                    keccak256(self.pack_for_signature().deref()).to_vec(),
                    entry_point_address.encode(),
                    chain_id.encode(),
                ]
                .concat(),
            )
            .as_slice(),
        )
    }
}

#[derive(Serialize, Deserialize)]
pub struct UserOperationReceipt {
    pub user_op_hash: UserOperationHash,
    pub sender: Address,
    pub nonce: U256,
    pub paymaster: Address,
    pub actual_gas_cost: U256,
    pub actual_gas_used: U256,
    pub success: bool,
    pub reason: String,
    pub logs: Vec<String>,
    pub receipt: TransactionReceipt,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_operation_pack() {
        let user_operations =  vec![
            UserOperation {
                sender: Address::zero(),
                nonce: U256::zero(),
                init_code: Bytes::default(),
                call_data: Bytes::default(),
                call_gas_limit: U256::zero(),
                verification_gas_limit: U256::from(100000),
                pre_verification_gas: U256::from(21000),
                max_fee_per_gas: U256::zero(),
                max_priority_fee_per_gas: U256::from(1e9 as u64),
                paymaster_and_data: Bytes::default(),
                signature: Bytes::default(),
            },
            UserOperation {
                sender: "0x663F3ad617193148711d28f5334eE4Ed07016602".parse().unwrap(),
                nonce: U256::zero(),
                init_code: Bytes::default(),
                call_data: Bytes::default(),
                call_gas_limit: U256::from(200000),
                verification_gas_limit: U256::from(100000),
                pre_verification_gas: U256::from(21000),
                max_fee_per_gas: U256::from(3000000000 as u64),
                max_priority_fee_per_gas: U256::from(1000000000),
                paymaster_and_data: Bytes::default(),
                signature: Bytes::from_str("0x7cb39607585dee8e297d0d7a669ad8c5e43975220b6773c10a138deadbc8ec864981de4b9b3c735288a217115fb33f8326a61ddabc60a534e3b5536515c70f931c").unwrap(),
            },
        ];
        assert_eq!(user_operations[0].pack(), "0x0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001600000000000000000000000000000000000000000000000000000000000000180000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000186a000000000000000000000000000000000000000000000000000000000000052080000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000003b9aca0000000000000000000000000000000000000000000000000000000000000001a000000000000000000000000000000000000000000000000000000000000001c00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000".parse::<Bytes>().unwrap());
        assert_eq!(user_operations[1].pack(), "0x000000000000000000000000663f3ad617193148711d28f5334ee4ed070166020000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000016000000000000000000000000000000000000000000000000000000000000001800000000000000000000000000000000000000000000000000000000000030d4000000000000000000000000000000000000000000000000000000000000186a0000000000000000000000000000000000000000000000000000000000000520800000000000000000000000000000000000000000000000000000000b2d05e00000000000000000000000000000000000000000000000000000000003b9aca0000000000000000000000000000000000000000000000000000000000000001a000000000000000000000000000000000000000000000000000000000000001c000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000417cb39607585dee8e297d0d7a669ad8c5e43975220b6773c10a138deadbc8ec864981de4b9b3c735288a217115fb33f8326a61ddabc60a534e3b5536515c70f931c00000000000000000000000000000000000000000000000000000000000000".parse::<Bytes>().unwrap());
    }

    #[test]
    fn user_operation_pack_for_signature() {
        let user_operations =  vec![
            UserOperation {
                sender: Address::zero(),
                nonce: U256::zero(),
                init_code: Bytes::default(),
                call_data: Bytes::default(),
                call_gas_limit: U256::zero(),
                verification_gas_limit: U256::from(100000),
                pre_verification_gas: U256::from(21000),
                max_fee_per_gas: U256::zero(),
                max_priority_fee_per_gas: U256::from(1e9 as u64),
                paymaster_and_data: Bytes::default(),
                signature: Bytes::default(),
            },
            UserOperation {
                sender: "0x663F3ad617193148711d28f5334eE4Ed07016602".parse().unwrap(),
                nonce: U256::zero(),
                init_code: Bytes::default(),
                call_data: Bytes::default(),
                call_gas_limit: U256::from(200000),
                verification_gas_limit: U256::from(100000),
                pre_verification_gas: U256::from(21000),
                max_fee_per_gas: U256::from(3000000000 as u64),
                max_priority_fee_per_gas: U256::from(1000000000),
                paymaster_and_data: Bytes::default(),
                signature: Bytes::from_str("0x7cb39607585dee8e297d0d7a669ad8c5e43975220b6773c10a138deadbc8ec864981de4b9b3c735288a217115fb33f8326a61ddabc60a534e3b5536515c70f931c").unwrap(),
            },
        ];
        assert_eq!(user_operations[0].pack_for_signature(), "0x0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001600000000000000000000000000000000000000000000000000000000000000180000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000186a000000000000000000000000000000000000000000000000000000000000052080000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000003b9aca0000000000000000000000000000000000000000000000000000000000000001a000000000000000000000000000000000000000000000000000000000000001c0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000".parse::<Bytes>().unwrap());
        assert_eq!(user_operations[1].pack_for_signature(), "0x000000000000000000000000663f3ad617193148711d28f5334ee4ed070166020000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000016000000000000000000000000000000000000000000000000000000000000001800000000000000000000000000000000000000000000000000000000000030d4000000000000000000000000000000000000000000000000000000000000186a0000000000000000000000000000000000000000000000000000000000000520800000000000000000000000000000000000000000000000000000000b2d05e00000000000000000000000000000000000000000000000000000000003b9aca0000000000000000000000000000000000000000000000000000000000000001a000000000000000000000000000000000000000000000000000000000000001c0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000".parse::<Bytes>().unwrap());
    }

    #[test]
    fn user_operation_hash() {
        let user_operations =  vec![
            UserOperation {
                sender: Address::zero(),
                nonce: U256::zero(),
                init_code: Bytes::default(),
                call_data: Bytes::default(),
                call_gas_limit: U256::zero(),
                verification_gas_limit: U256::from(100000),
                pre_verification_gas: U256::from(21000),
                max_fee_per_gas: U256::zero(),
                max_priority_fee_per_gas: U256::from(1e9 as u64),
                paymaster_and_data: Bytes::default(),
                signature: Bytes::default(),
            },
            UserOperation {
                sender: "0x663F3ad617193148711d28f5334eE4Ed07016602".parse().unwrap(),
                nonce: U256::zero(),
                init_code: Bytes::default(),
                call_data: Bytes::default(),
                call_gas_limit: U256::from(200000),
                verification_gas_limit: U256::from(100000),
                pre_verification_gas: U256::from(21000),
                max_fee_per_gas: U256::from(3000000000 as u64),
                max_priority_fee_per_gas: U256::from(1000000000),
                paymaster_and_data: Bytes::default(),
                signature: Bytes::from_str("0x7cb39607585dee8e297d0d7a669ad8c5e43975220b6773c10a138deadbc8ec864981de4b9b3c735288a217115fb33f8326a61ddabc60a534e3b5536515c70f931c").unwrap(),
            },
        ];
        assert_eq!(
            user_operations[0].hash(
                "0x2DF1592238420ecFe7f2431360e224707e77fA0E"
                    .parse()
                    .unwrap(),
                U256::from(1)
            ),
            H256::from_str("0x42e145138104ec4124367ea3f7994833071b2011927290f6844d593e05011279")
                .unwrap()
        );
        assert_eq!(
            user_operations[1].hash(
                "0x2DF1592238420ecFe7f2431360e224707e77fA0E"
                    .parse()
                    .unwrap(),
                U256::from(1)
            ),
            H256::from_str("0x583c8fcba470fd9da514f9482ccd31c299b0161a36b365aab353a6bfebaa0bb2")
                .unwrap()
        );
    }
}

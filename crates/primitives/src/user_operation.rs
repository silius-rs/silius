use ethers::{
    abi::AbiEncode,
    prelude::{EthAbiCodec, EthAbiType},
    types::{Address, BlockNumber, Bytes, Log, TransactionReceipt, H256, U256},
    utils::keccak256,
};
use rustc_hex::FromHexError;
use serde::{Deserialize, Serialize};
use std::{ops::Deref, str::FromStr};

use super::utils::as_checksum;

#[derive(
    Eq, Hash, PartialEq, Debug, Serialize, Deserialize, Clone, Copy, Default, PartialOrd, Ord,
)]
pub struct UserOperationHash(pub H256);

impl From<H256> for UserOperationHash {
    fn from(value: H256) -> Self {
        Self(value)
    }
}

impl From<UserOperationHash> for H256 {
    fn from(value: UserOperationHash) -> Self {
        value.0
    }
}

impl From<[u8; 32]> for UserOperationHash {
    fn from(value: [u8; 32]) -> Self {
        Self(H256::from_slice(&value))
    }
}

impl FromStr for UserOperationHash {
    type Err = FromHexError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        H256::from_str(s).map(|h| h.into())
    }
}

impl UserOperationHash {
    #[inline]
    pub const fn as_fixed_bytes(&self) -> &[u8; 32] {
        &self.0 .0
    }

    #[inline]
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.0 .0
    }

    #[inline]
    pub const fn repeat_byte(byte: u8) -> UserOperationHash {
        UserOperationHash(H256([byte; 32]))
    }

    #[inline]
    pub const fn zero() -> UserOperationHash {
        UserOperationHash::repeat_byte(0u8)
    }

    pub fn assign_from_slice(&mut self, src: &[u8]) {
        self.as_bytes_mut().copy_from_slice(src);
    }

    pub fn from_slice(src: &[u8]) -> Self {
        let mut ret = Self::zero();
        ret.assign_from_slice(src);
        ret
    }
}

#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, EthAbiCodec, EthAbiType,
)]
#[serde(rename_all = "camelCase")]
pub struct UserOperation {
    #[serde(serialize_with = "as_checksum")]
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

impl UserOperation {
    pub fn pack(&self) -> Bytes {
        Bytes::from(self.clone().encode())
    }

    pub fn pack_for_signature(&self) -> Bytes {
        let mut packed: Vec<u8> = UserOperation {
            signature: Bytes::default(),
            ..self.clone()
        }
        .encode();
        packed.truncate(packed.len() - 32);
        Bytes::from(packed)
    }

    pub fn hash(&self, entry_point: &Address, chain_id: &U256) -> UserOperationHash {
        H256::from_slice(
            keccak256(
                [
                    keccak256(self.pack_for_signature().deref()).to_vec(),
                    entry_point.encode(),
                    chain_id.encode(),
                ]
                .concat(),
            )
            .as_slice(),
        )
        .into()
    }

    #[cfg(feature = "test-utils")]
    pub fn random() -> Self {
        Self {
            sender: Address::random(),
            nonce: U256::zero(),
            init_code: Bytes::default(),
            call_data: Bytes::default(),
            call_gas_limit: U256::zero(),
            verification_gas_limit: U256::from(100000),
            pre_verification_gas: U256::from(21000),
            max_fee_per_gas: U256::from(0),
            max_priority_fee_per_gas: U256::from(1e9 as u64),
            paymaster_and_data: Bytes::default(),
            signature: Bytes::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UserOperationReceipt {
    pub user_op_hash: UserOperationHash,
    #[serde(serialize_with = "as_checksum")]
    pub sender: Address,
    pub nonce: U256,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paymaster: Option<Address>,
    pub actual_gas_cost: U256,
    pub actual_gas_used: U256,
    pub success: bool,
    pub reason: String,
    pub logs: Vec<Log>,
    pub receipt: TransactionReceipt,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserOperationByHash {
    pub user_operation: UserOperation,
    #[serde(serialize_with = "as_checksum")]
    pub entry_point: Address,
    pub block_number: BlockNumber,
    pub block_hash: H256,
    pub transaction_hash: H256,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserOperationPartial {
    pub sender: Address,
    pub nonce: U256,
    pub init_code: Option<Bytes>,
    pub call_data: Option<Bytes>,
    pub call_gas_limit: Option<U256>,
    pub verification_gas_limit: Option<U256>,
    pub pre_verification_gas: Option<U256>,
    pub max_fee_per_gas: Option<U256>,
    pub max_priority_fee_per_gas: Option<U256>,
    pub paymaster_and_data: Option<Bytes>,
    pub signature: Option<Bytes>,
}

impl From<UserOperationPartial> for UserOperation {
    fn from(user_operation: UserOperationPartial) -> Self {
        Self {
            sender: user_operation.sender,
            nonce: user_operation.nonce,
            init_code: {
                if let Some(init_code) = user_operation.init_code {
                    init_code
                } else {
                    Bytes::default()
                }
            },
            call_data: {
                if let Some(call_data) = user_operation.call_data {
                    call_data
                } else {
                    Bytes::default()
                }
            },
            call_gas_limit: {
                if let Some(call_gas_limit) = user_operation.call_gas_limit {
                    call_gas_limit
                } else {
                    U256::zero()
                }
            },
            verification_gas_limit: {
                if let Some(verification_gas_limit) = user_operation.verification_gas_limit {
                    verification_gas_limit
                } else {
                    U256::from(10000000)
                }
            },
            pre_verification_gas: {
                if let Some(pre_verification_gas) = user_operation.pre_verification_gas {
                    pre_verification_gas
                } else {
                    U256::zero()
                }
            },
            max_fee_per_gas: {
                if let Some(max_fee_per_gas) = user_operation.max_fee_per_gas {
                    max_fee_per_gas
                } else {
                    U256::zero()
                }
            },
            max_priority_fee_per_gas: {
                if let Some(max_priority_fee_per_gas) = user_operation.max_priority_fee_per_gas {
                    max_priority_fee_per_gas
                } else {
                    U256::zero()
                }
            },
            paymaster_and_data: {
                if let Some(paymaster_and_data) = user_operation.paymaster_and_data {
                    paymaster_and_data
                } else {
                    Bytes::default()
                }
            },
            signature: {
                if let Some(signature) = user_operation.signature {
                    signature
                } else {
                    Bytes::from(vec![1; 65])
                }
            },
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserOperationGasEstimation {
    pub pre_verification_gas: U256,
    #[serde(rename = "verificationGas")]
    pub verification_gas_limit: U256,
    pub call_gas_limit: U256,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

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
                max_fee_per_gas: U256::from(3000000000_u64),
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
                max_fee_per_gas: U256::from(3000000000_u64),
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
                max_fee_per_gas: U256::from(3000000000_u64),
                max_priority_fee_per_gas: U256::from(1000000000),
                paymaster_and_data: Bytes::default(),
                signature: Bytes::from_str("0x7cb39607585dee8e297d0d7a669ad8c5e43975220b6773c10a138deadbc8ec864981de4b9b3c735288a217115fb33f8326a61ddabc60a534e3b5536515c70f931c").unwrap(),
            },
        ];
        assert_eq!(
            user_operations[0].hash(
                &"0x2DF1592238420ecFe7f2431360e224707e77fA0E"
                    .parse()
                    .unwrap(),
                &U256::from(1)
            ),
            H256::from_str("0x42e145138104ec4124367ea3f7994833071b2011927290f6844d593e05011279")
                .unwrap()
                .into()
        );
        assert_eq!(
            user_operations[1].hash(
                &"0x2DF1592238420ecFe7f2431360e224707e77fA0E"
                    .parse()
                    .unwrap(),
                &U256::from(1)
            ),
            H256::from_str("0x583c8fcba470fd9da514f9482ccd31c299b0161a36b365aab353a6bfebaa0bb2")
                .unwrap()
                .into()
        );
    }
}

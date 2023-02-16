use crate::contracts::gen::entry_point_api;
use ethers::abi::{AbiDecode, AbiEncode};
use ethers::prelude::{EthAbiCodec, EthAbiType};
use ethers::types::{Address, Bytes, TransactionReceipt, H256, U256};
use ethers::utils::keccak256;
use reth_db::table::{Compress, Decode, Decompress, Encode};
use rustc_hex::FromHexError;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use std::str::FromStr;
use std::vec;

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

impl FromStr for UserOperationHash {
    type Err = FromHexError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        H256::from_str(s).map(|h| h.into())
    }
}

impl Decode for UserOperationHash {
    fn decode<B: Into<prost::bytes::Bytes>>(value: B) -> Result<Self, reth_db::Error> {
        Ok(H256::from_slice(value.into().as_ref()).into())
    }
}

impl Encode for UserOperationHash {
    type Encoded = [u8; 32];
    fn encode(self) -> Self::Encoded {
        *self.0.as_fixed_bytes()
    }
}

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

    #[cfg(test)]
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

impl Compress for UserOperation {
    type Compressed = Bytes;
    fn compress(self) -> Self::Compressed {
        self.pack()
    }
}

impl Decompress for UserOperation {
    fn decompress<B: Into<prost::bytes::Bytes>>(value: B) -> Result<Self, reth_db::Error> {
        Self::decode(value.into()).map_err(|_e| reth_db::Error::DecodeError)
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

#[derive(Serialize, Deserialize)]
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
pub struct UserOperationGasEstimation {
    pub pre_verification_gas: U256,
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

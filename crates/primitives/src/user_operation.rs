use super::utils::as_checksum;
use ethers::{
    abi::AbiEncode,
    prelude::{EthAbiCodec, EthAbiType},
    types::{Address, Bytes, Log, TransactionReceipt, H256, U256, U64},
    utils::keccak256,
};
use rustc_hex::FromHexError;
use serde::{Deserialize, Serialize};
use ssz_rs::Sized;
use std::{
    ops::{AddAssign, Deref},
    slice::Windows,
    str::FromStr,
};

/// Transaction type for ERC-4337 account abstraction
#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    EthAbiCodec,
    EthAbiType,
)]
#[serde(rename_all = "camelCase")]
pub struct UserOperation {
    /// Sender of the user operation
    #[serde(serialize_with = "as_checksum")]
    pub sender: Address,

    /// Nonce (anti replay protection)
    pub nonce: U256,

    /// Init code for the account (needed if account not yet deployed and needs to be created)
    pub init_code: Bytes,

    /// The data that is passed to the sender during the main execution call
    pub call_data: Bytes,

    /// The amount of gas to allocate for the main execution call
    pub call_gas_limit: U256,

    /// The amount of gas to allocate for the verification step
    pub verification_gas_limit: U256,

    /// The amount of gas to pay bundler to compensate for the pre-verification execution and calldata
    pub pre_verification_gas: U256,

    /// Maximum fee per gas (similar to EIP-1559)
    pub max_fee_per_gas: U256,

    /// Maximum priority fee per gas (similar to EIP-1559)
    pub max_priority_fee_per_gas: U256,

    /// Address of paymaster sponsoring the user operation, followed by extra data to send to the paymaster (can be empty)
    pub paymaster_and_data: Bytes,

    /// Data passed to the account along with the nonce during the verification step
    pub signature: Bytes,
}

impl UserOperation {
    /// Packs the user operation into bytes
    pub fn pack(&self) -> Bytes {
        self.clone().encode().into()
    }

    /// Packs the user operation without signature to bytes (used for calculating the hash)
    pub fn pack_without_signature(&self) -> Bytes {
        let user_operation_packed = UserOperationUnsigned::from(self.clone());
        user_operation_packed.encode().into()
    }

    /// Calculates the hash of the user operation
    pub fn hash(&self, entry_point: &Address, chain_id: &U256) -> UserOperationHash {
        H256::from_slice(
            keccak256(
                [
                    keccak256(self.pack_without_signature().deref()).to_vec(),
                    entry_point.encode(),
                    chain_id.encode(),
                ]
                .concat(),
            )
            .as_slice(),
        )
        .into()
    }

    // Builder pattern helpers

    /// Sets the sender of the user operation
    pub fn sender(mut self, sender: Address) -> Self {
        self.sender = sender;
        self
    }

    /// Sets the nonce of the user operation
    pub fn nonce(mut self, nonce: U256) -> Self {
        self.nonce = nonce;
        self
    }

    /// Sets the init code of the user operation
    pub fn init_code(mut self, init_code: Bytes) -> Self {
        self.init_code = init_code;
        self
    }

    /// Sets the call data of the user operation
    pub fn call_data(mut self, call_data: Bytes) -> Self {
        self.call_data = call_data;
        self
    }

    /// Sets the call gas limit of the user operation
    pub fn call_gas_limit(mut self, call_gas_limit: U256) -> Self {
        self.call_gas_limit = call_gas_limit;
        self
    }

    /// Sets the verification gas limit of the user operation
    pub fn verification_gas_limit(mut self, verification_gas_limit: U256) -> Self {
        self.verification_gas_limit = verification_gas_limit;
        self
    }

    /// Sets the pre-verification gas of the user operation
    pub fn pre_verification_gas(mut self, pre_verification_gas: U256) -> Self {
        self.pre_verification_gas = pre_verification_gas;
        self
    }

    /// Sets the max fee per gas of the user operation
    pub fn max_fee_per_gas(mut self, max_fee_per_gas: U256) -> Self {
        self.max_fee_per_gas = max_fee_per_gas;
        self
    }

    /// Sets the max priority fee per gas of the user operation
    pub fn max_priority_fee_per_gas(mut self, max_priority_fee_per_gas: U256) -> Self {
        self.max_priority_fee_per_gas = max_priority_fee_per_gas;
        self
    }

    /// Sets the paymaster and data of the user operation
    pub fn paymaster_and_data(mut self, paymaster_and_data: Bytes) -> Self {
        self.paymaster_and_data = paymaster_and_data;
        self
    }

    /// Sets the signature of the user operation
    pub fn signature(mut self, signature: Bytes) -> Self {
        self.signature = signature;
        self
    }

    /// Creates random user operation (for testing purposes)
    #[cfg(feature = "test-utils")]
    pub fn random() -> Self {
        UserOperation::default()
            .sender(Address::random())
            .verification_gas_limit(100_000.into())
            .pre_verification_gas(21_000.into())
            .max_priority_fee_per_gas(1_000_000_000.into())
    }
}

/// User operation hash
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

/// User operation without signature
#[derive(EthAbiCodec, EthAbiType)]
pub struct UserOperationUnsigned {
    pub sender: Address,
    pub nonce: U256,
    pub init_code: H256,
    pub call_data: H256,
    pub call_gas_limit: U256,
    pub verification_gas_limit: U256,
    pub pre_verification_gas: U256,
    pub max_fee_per_gas: U256,
    pub max_priority_fee_per_gas: U256,
    pub paymaster_and_data: H256,
}

impl From<UserOperation> for UserOperationUnsigned {
    fn from(value: UserOperation) -> Self {
        Self {
            sender: value.sender,
            nonce: value.nonce,
            init_code: keccak256(value.init_code.deref()).into(),
            call_data: keccak256(value.call_data.deref()).into(),
            call_gas_limit: value.call_gas_limit,
            verification_gas_limit: value.verification_gas_limit,
            pre_verification_gas: value.pre_verification_gas,
            max_fee_per_gas: value.max_fee_per_gas,
            max_priority_fee_per_gas: value.max_priority_fee_per_gas,
            paymaster_and_data: keccak256(value.paymaster_and_data.deref()).into(),
        }
    }
}

/// Receipt of the user operation (returned from the RPC endpoint eth_getUserOperationReceipt)
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserOperationReceipt {
    #[serde(rename = "userOpHash")]
    pub user_operation_hash: UserOperationHash,
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
    #[serde(rename = "receipt")]
    pub tx_receipt: TransactionReceipt,
}

/// Struct that is returned from the RPC endpoint eth_getUserOperationByHash
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserOperationByHash {
    pub user_operation: UserOperation,
    #[serde(serialize_with = "as_checksum")]
    pub entry_point: Address,
    pub transaction_hash: H256,
    pub block_hash: H256,
    pub block_number: U64,
}

/// User operation with all fields being optional
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserOperationPartial {
    pub sender: Option<Address>,
    pub nonce: Option<U256>,
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
            sender: {
                if let Some(sender) = user_operation.sender {
                    sender
                } else {
                    Address::zero()
                }
            },
            nonce: {
                if let Some(nonce) = user_operation.nonce {
                    nonce
                } else {
                    U256::zero()
                }
            },
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
                    U256::zero()
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
                    Bytes::default()
                }
            },
        }
    }
}

/// Gas estimations for user operation (returned from the RPC endpoint eth_estimateUserOperationGas)
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserOperationGasEstimation {
    pub pre_verification_gas: U256,
    pub verification_gas_limit: U256,
    pub call_gas_limit: U256,
}

fn ssz_pack_u256(
    fixed: &mut Vec<Option<Vec<u8>>>,
    fixed_lengths_sum: &mut usize,
    variable_lengths: &mut Vec<usize>,
    value: U256,
) -> Result<(), ssz_rs::SerializeError> {
    let mut element_buffer = Vec::with_capacity(32);
    <[u64; 4] as ssz_rs::Serialize>::serialize(&value.0, &mut element_buffer)?;
    fixed_lengths_sum.add_assign(32);
    fixed.push(Some(element_buffer));
    variable_lengths.push(0);
    Ok(())
}

fn ssz_pack_bytes(
    fixed: &mut Vec<Option<Vec<u8>>>,
    fixed_lengths_sum: &mut usize,
    variable: &mut Vec<Vec<u8>>,
    variable_lengths: &mut Vec<usize>,
    value: Bytes,
) {
    let size = value.len();
    let mut element: Vec<u8> = Vec::with_capacity(size);
    element.extend(value.iter());
    fixed.push(None);
    fixed_lengths_sum.add_assign(4);
    variable_lengths.push(size);
    variable.push(element);
}

impl ssz_rs::Sized for UserOperation {
    fn is_variable_size() -> bool {
        true
    }
    fn size_hint() -> usize {
        0
    }
}

impl ssz_rs::Serialize for UserOperation {
    fn serialize(&self, buffer: &mut Vec<u8>) -> Result<usize, ssz_rs::SerializeError> {
        let mut fixed = Vec::new();
        let mut variable = Vec::new();
        let mut variable_lengths = Vec::new();
        let mut fixed_lengths_sum = 0usize;

        // sender
        let mut element_buffer = Vec::with_capacity(20);
        <[u8; 20] as ssz_rs::Serialize>::serialize(&self.sender.0, &mut element_buffer)?;
        fixed_lengths_sum += element_buffer.len();
        fixed.push(Some(element_buffer));
        variable_lengths.push(0);

        ssz_pack_u256(
            &mut fixed,
            &mut fixed_lengths_sum,
            &mut variable_lengths,
            self.nonce,
        )?;
        ssz_pack_bytes(
            &mut fixed,
            &mut fixed_lengths_sum,
            &mut variable,
            &mut variable_lengths,
            self.init_code.clone(),
        );
        ssz_pack_bytes(
            &mut fixed,
            &mut fixed_lengths_sum,
            &mut variable,
            &mut variable_lengths,
            self.call_data.clone(),
        );
        ssz_pack_u256(
            &mut fixed,
            &mut fixed_lengths_sum,
            &mut variable_lengths,
            self.call_gas_limit,
        )?;
        ssz_pack_u256(
            &mut fixed,
            &mut fixed_lengths_sum,
            &mut variable_lengths,
            self.verification_gas_limit,
        )?;
        ssz_pack_u256(
            &mut fixed,
            &mut fixed_lengths_sum,
            &mut variable_lengths,
            self.pre_verification_gas,
        )?;
        ssz_pack_u256(
            &mut fixed,
            &mut fixed_lengths_sum,
            &mut variable_lengths,
            self.max_fee_per_gas,
        )?;
        ssz_pack_u256(
            &mut fixed,
            &mut fixed_lengths_sum,
            &mut variable_lengths,
            self.max_priority_fee_per_gas,
        )?;
        ssz_pack_bytes(
            &mut fixed,
            &mut fixed_lengths_sum,
            &mut variable,
            &mut variable_lengths,
            self.paymaster_and_data.clone(),
        );
        ssz_pack_bytes(
            &mut fixed,
            &mut fixed_lengths_sum,
            &mut variable,
            &mut variable_lengths,
            self.signature.clone(),
        );

        ssz_rs::__internal::serialize_composite_from_components(
            fixed,
            variable,
            variable_lengths,
            fixed_lengths_sum,
            buffer,
        )
    }
}

fn ssz_unpack_bytes_length(
    start: usize,
    encoding: &[u8],
    offsets: &mut Vec<usize>,
) -> Result<(), ssz_rs::DeserializeError> {
    let end = start + 4usize;
    let next_offset = <u32 as ssz_rs::Deserialize>::deserialize(&encoding[start..end])?;
    offsets.push(next_offset as usize);
    Ok(())
}

fn ssz_unpack_u256(
    start: usize,
    encoding: &[u8],
) -> Result<(U256, usize), ssz_rs::DeserializeError> {
    let encoded_length = 32usize;
    let end = start + encoded_length;
    let result = <[u64; 4] as ssz_rs::Deserialize>::deserialize(&encoding[start..end])?;
    Ok((U256(result), encoded_length))
}

fn ssz_unpack_bytes(
    bytes_zone: &mut Windows<'_, usize>,
    encoding: &[u8],
    total_bytes_read: usize,
) -> Result<(Bytes, usize), ssz_rs::DeserializeError> {
    let range = bytes_zone
        .next()
        .ok_or(ssz_rs::DeserializeError::AdditionalInput {
            provided: encoding.len(),
            expected: total_bytes_read,
        })?;
    let start = range[0];
    let end = range[1];
    let bytes_data = Bytes::from_iter(encoding[start..end].iter());
    Ok((bytes_data, end - start))
}
impl ssz_rs::Deserialize for UserOperation {
    fn deserialize(encoding: &[u8]) -> Result<Self, ssz_rs::DeserializeError>
    where
        Self: Sized,
    {
        let mut start = 0;
        let mut offsets: Vec<usize> = Vec::new();
        let mut container = Self::default();

        let byte_read = {
            let encoded_length = <[u8; 20] as ssz_rs::Sized>::size_hint();
            let end = start + encoded_length;
            let target =
                encoding
                    .get(start..end)
                    .ok_or(ssz_rs::DeserializeError::ExpectedFurtherInput {
                        provided: encoding.len() - start,
                        expected: encoded_length,
                    })?;
            let result = <[u8; 20] as ssz_rs::Deserialize>::deserialize(target)?;
            container.sender = Address::from_slice(&result);
            encoded_length
        };
        start += byte_read;

        let (value, byte_read) = ssz_unpack_u256(start, encoding)?;
        container.nonce = value;
        start += byte_read;

        // init code
        ssz_unpack_bytes_length(start, encoding, &mut offsets)?;
        start += 4usize;

        // cal data
        ssz_unpack_bytes_length(start, encoding, &mut offsets)?;
        start += 4usize;

        let (value, byte_read) = ssz_unpack_u256(start, encoding)?;
        container.call_gas_limit = value;
        start += byte_read;

        let (value, byte_read) = ssz_unpack_u256(start, encoding)?;
        container.verification_gas_limit = value;
        start += byte_read;

        let (value, byte_read) = ssz_unpack_u256(start, encoding)?;
        container.pre_verification_gas = value;
        start += byte_read;

        let (value, byte_read) = ssz_unpack_u256(start, encoding)?;
        container.max_fee_per_gas = value;
        start += byte_read;

        let (value, byte_read) = ssz_unpack_u256(start, encoding)?;
        container.max_priority_fee_per_gas = value;
        start += byte_read;

        // paymaster and data
        ssz_unpack_bytes_length(start, encoding, &mut offsets)?;
        start += 4usize;

        // signature
        ssz_unpack_bytes_length(start, encoding, &mut offsets)?;
        start += 4usize;

        let mut total_bytes_read = start;
        offsets.push(encoding.len());
        let mut bytes_zone = offsets.windows(2);

        // init code
        let (init_code, length) = ssz_unpack_bytes(&mut bytes_zone, encoding, total_bytes_read)?;
        total_bytes_read += length;
        container.init_code = init_code;

        let (call_data, length) = ssz_unpack_bytes(&mut bytes_zone, encoding, total_bytes_read)?;
        total_bytes_read += length;
        container.call_data = call_data;

        let (paymaster_data, length) =
            ssz_unpack_bytes(&mut bytes_zone, encoding, total_bytes_read)?;
        total_bytes_read += length;
        container.paymaster_and_data = paymaster_data;

        let (signature, length) = ssz_unpack_bytes(&mut bytes_zone, encoding, total_bytes_read)?;
        total_bytes_read += length;
        container.signature = signature;

        if total_bytes_read > encoding.len() {
            return Err(ssz_rs::DeserializeError::ExpectedFurtherInput {
                provided: encoding.len(),
                expected: total_bytes_read,
            });
        }
        if total_bytes_read < encoding.len() {
            return Err(ssz_rs::DeserializeError::AdditionalInput {
                provided: encoding.len(),
                expected: total_bytes_read,
            });
        }
        Ok(container)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn user_operation_pack() {
        let uos =  vec![
            UserOperation::default().verification_gas_limit(100_000.into()).pre_verification_gas(21_000.into()).max_priority_fee_per_gas(1_000_000_000.into()),
            UserOperation::default().sender("0x9c5754De1443984659E1b3a8d1931D83475ba29C".parse().unwrap()).call_gas_limit(200_000.into()).verification_gas_limit(100_000.into()).pre_verification_gas(21_000.into()).max_fee_per_gas(3_000_000_000_u64.into()).max_priority_fee_per_gas(1_000_000_000.into()).signature("0x7cb39607585dee8e297d0d7a669ad8c5e43975220b6773c10a138deadbc8ec864981de4b9b3c735288a217115fb33f8326a61ddabc60a534e3b5536515c70f931c".parse().unwrap()),
        ];
        assert_eq!(uos[0].pack(), "0x0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001600000000000000000000000000000000000000000000000000000000000000180000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000186a000000000000000000000000000000000000000000000000000000000000052080000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000003b9aca0000000000000000000000000000000000000000000000000000000000000001a000000000000000000000000000000000000000000000000000000000000001c00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000".parse::<Bytes>().unwrap());
        assert_eq!(uos[1].pack(), "0x0000000000000000000000009c5754de1443984659e1b3a8d1931d83475ba29c0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000016000000000000000000000000000000000000000000000000000000000000001800000000000000000000000000000000000000000000000000000000000030d4000000000000000000000000000000000000000000000000000000000000186a0000000000000000000000000000000000000000000000000000000000000520800000000000000000000000000000000000000000000000000000000b2d05e00000000000000000000000000000000000000000000000000000000003b9aca0000000000000000000000000000000000000000000000000000000000000001a000000000000000000000000000000000000000000000000000000000000001c000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000417cb39607585dee8e297d0d7a669ad8c5e43975220b6773c10a138deadbc8ec864981de4b9b3c735288a217115fb33f8326a61ddabc60a534e3b5536515c70f931c00000000000000000000000000000000000000000000000000000000000000".parse::<Bytes>().unwrap());
    }

    #[test]
    fn user_operation_pack_without_signature() {
        let uos =  vec![
            UserOperation::default().verification_gas_limit(100_000.into()).pre_verification_gas(21_000.into()).max_priority_fee_per_gas(1_000_000_000.into()),
            UserOperation {
                sender: "0x9c5754De1443984659E1b3a8d1931D83475ba29C".parse().unwrap(),
                nonce: 1.into(),
                init_code: Bytes::default(),
                call_data: "0xb61d27f60000000000000000000000009c5754de1443984659e1b3a8d1931d83475ba29c00000000000000000000000000000000000000000000000000005af3107a400000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000000".parse().unwrap(),
                call_gas_limit: 33_100.into(),
                verification_gas_limit: 60_624.into(),
                pre_verification_gas: 44_056.into(),
                max_fee_per_gas: 1_695_000_030_u64.into(),
                max_priority_fee_per_gas: 1_695_000_000.into(),
                paymaster_and_data: Bytes::default(),
                signature: "0x37540ca4f91a9f08993ba4ebd4b7473902f69864c98951f9db8cb47b78764c1a13ad46894a96dc0cad68f9207e49b4dbb897f25f47f040cec2a636a8201c1cd71b".parse().unwrap(),
            },
        ];
        assert_eq!(uos[0].pack_without_signature(), "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000186a000000000000000000000000000000000000000000000000000000000000052080000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000003b9aca00c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470".parse::<Bytes>().unwrap());
        assert_eq!(uos[1].pack_without_signature(), "0x0000000000000000000000009c5754de1443984659e1b3a8d1931d83475ba29c0000000000000000000000000000000000000000000000000000000000000001c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470f7def7aeb687d6992b466243b713223689982cefca0f91a1f5c5f60adb532b93000000000000000000000000000000000000000000000000000000000000814c000000000000000000000000000000000000000000000000000000000000ecd0000000000000000000000000000000000000000000000000000000000000ac18000000000000000000000000000000000000000000000000000000006507a5de000000000000000000000000000000000000000000000000000000006507a5c0c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470".parse::<Bytes>().unwrap());
    }

    #[test]
    fn user_operation_hash() {
        let uos =  vec![
            UserOperation::default().verification_gas_limit(100_000.into()).pre_verification_gas(21_000.into()).max_priority_fee_per_gas(1_000_000_000.into()),
            UserOperation {
                sender: "0x9c5754De1443984659E1b3a8d1931D83475ba29C".parse().unwrap(),
                nonce: U256::zero(),
                init_code: "0x9406cc6185a346906296840746125a0e449764545fbfb9cf000000000000000000000000ce0fefa6f7979c4c9b5373e0f5105b7259092c6d0000000000000000000000000000000000000000000000000000000000000000".parse().unwrap(),
                call_data: "0xb61d27f60000000000000000000000009c5754de1443984659e1b3a8d1931d83475ba29c00000000000000000000000000000000000000000000000000005af3107a400000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000000".parse().unwrap(),
                call_gas_limit: 33_100.into(),
                verification_gas_limit: 361_460.into(),
                pre_verification_gas: 44_980.into(),
                max_fee_per_gas: 1_695_000_030_u64.into(),
                max_priority_fee_per_gas: 1_695_000_000.into(),
                paymaster_and_data: Bytes::default(),
                signature: "0xebfd4657afe1f1c05c1ec65f3f9cc992a3ac083c424454ba61eab93152195e1400d74df01fc9fa53caadcb83a891d478b713016bcc0c64307c1ad3d7ea2e2d921b".parse().unwrap(),
            },
        ];
        assert_eq!(
            uos[0].hash(
                &"0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789"
                    .parse()
                    .unwrap(),
                &80_001.into()
            ),
            "0x95418c07086df02ff6bc9e8bdc150b380cb761beecc098630440bcec6e862702"
                .parse::<H256>()
                .unwrap()
                .into()
        );
        assert_eq!(
            uos[1].hash(
                &"0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789"
                    .parse()
                    .unwrap(),
                &80_001.into()
            ),
            "0x7c1b8c9df49a9e09ecef0f0fe6841d895850d29820f9a4b494097764085dcd7e"
                .parse::<H256>()
                .unwrap()
                .into()
        );
    }

    #[test]
    fn user_operation_ssz() {
        let uo = UserOperation {
            sender: "0x1F9090AAE28B8A3DCEADF281B0F12828E676C326".parse().unwrap(),
            nonce: 100.into(),
            init_code: "0x9406cc6185a346906296840746125a0e449764545fbfb9cf000000000000000000000000ce0fefa6f7979c4c9b5373e0f5105b7259092c6d0000000000000000000000000000000000000000000000000000000000000000".parse().unwrap(),
            call_data: "0xb61d27f60000000000000000000000009c5754de1443984659e1b3a8d1931d83475ba29c00000000000000000000000000000000000000000000000000005af3107a400000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000000".parse().unwrap(),
            call_gas_limit: 100000.into(),
            verification_gas_limit: 361_460.into(),
            pre_verification_gas: 44_980.into(),
            max_fee_per_gas: 1_695_000_030.into(),
            max_priority_fee_per_gas: 1_695_000_000.into(),
            paymaster_and_data: "0x1f".parse().unwrap(),
            signature: "0xebfd4657afe1f1c05c1ec65f3f9cc992a3ac083c424454ba61eab93152195e1400d74df01fc9fa53caadcb83a891d478b713016bcc0c64307c1ad3d7ea2e2d921b".parse().unwrap(),
        };
        let mut encoded = Vec::new();
        ssz_rs::Serialize::serialize(&uo, &mut encoded).unwrap();
        // generated by python codes
        let expected_encode =Bytes::from_str("1f9090aae28b8a3dceadf281b0f12828e676c3266400000000000000000000000000000000000000000000000000000000000000e40000003c010000a086010000000000000000000000000000000000000000000000000000000000f483050000000000000000000000000000000000000000000000000000000000b4af000000000000000000000000000000000000000000000000000000000000dea5076500000000000000000000000000000000000000000000000000000000c0a5076500000000000000000000000000000000000000000000000000000000c0010000c10100009406cc6185a346906296840746125a0e449764545fbfb9cf000000000000000000000000ce0fefa6f7979c4c9b5373e0f5105b7259092c6d0000000000000000000000000000000000000000000000000000000000000000b61d27f60000000000000000000000009c5754de1443984659e1b3a8d1931d83475ba29c00000000000000000000000000000000000000000000000000005af3107a4000000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000001febfd4657afe1f1c05c1ec65f3f9cc992a3ac083c424454ba61eab93152195e1400d74df01fc9fa53caadcb83a891d478b713016bcc0c64307c1ad3d7ea2e2d921b").unwrap().to_vec();
        assert_eq!(encoded, expected_encode);

        let uo_decode =
            <UserOperation as ssz_rs::Deserialize>::deserialize(&expected_encode).unwrap();

        assert_eq!(uo_decode.sender, uo.sender);
        assert_eq!(uo_decode.nonce, uo.nonce);
        assert_eq!(uo_decode.init_code, uo.init_code);
        assert_eq!(uo_decode.call_data, uo.call_data);
        assert_eq!(uo_decode.call_gas_limit, uo.call_gas_limit);
        assert_eq!(uo_decode.verification_gas_limit, uo.verification_gas_limit);
        assert_eq!(uo_decode.pre_verification_gas, uo.pre_verification_gas);
        assert_eq!(uo_decode.max_fee_per_gas, uo.max_fee_per_gas);
        assert_eq!(
            uo_decode.max_priority_fee_per_gas,
            uo.max_priority_fee_per_gas
        );
        assert_eq!(uo_decode.paymaster_and_data, uo.paymaster_and_data);
        assert_eq!(uo_decode.signature, uo.signature);
    }
}

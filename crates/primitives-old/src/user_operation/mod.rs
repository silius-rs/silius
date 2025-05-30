//! Basic transaction type for account abstraction (ERC-4337)

mod hash;
mod request;

use crate::{get_address, utils::as_checksum_addr};
use derive_more::{AsRef, Deref};
use ethers::{
    abi::AbiEncode,
    contract::{EthAbiCodec, EthAbiType},
    types::{Address, Bytes, Log, TransactionReceipt, H256, U256, U64},
    utils::keccak256,
};
pub use hash::UserOperationHash;
pub use request::UserOperationRequest;
use serde::{Deserialize, Serialize};
use ssz_rs::List;
use std::{cmp::Ord, ops::Deref, slice::Windows};

/// User operation with hash
#[derive(AsRef, Deref, Debug, Clone, Serialize, Deserialize)]
pub struct UserOperation {
    /// Hash of the user operation
    pub hash: UserOperationHash,

    /// Raw user operation
    #[deref]
    #[as_ref]
    pub user_operation: UserOperationSigned,
}

impl UserOperation {
    pub fn from_user_operation_signed(
        hash: UserOperationHash,
        user_operation: UserOperationSigned,
    ) -> Self {
        Self { hash, user_operation }
    }
}

impl From<UserOperation> for UserOperationSigned {
    fn from(value: UserOperation) -> Self {
        value.user_operation
    }
}

/// User operation
#[derive(
    Default,
    Clone,
    Debug,
    Ord,
    PartialOrd,
    PartialEq,
    Eq,
    EthAbiCodec,
    EthAbiType,
    Serialize,
    Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub struct UserOperationSigned {
    /// Sender of the user operation
    #[serde(serialize_with = "as_checksum_addr")]
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

    /// The amount of gas to pay bundler to compensate for the pre-verification execution and
    /// calldata
    pub pre_verification_gas: U256,

    /// Maximum fee per gas (similar to EIP-1559)
    pub max_fee_per_gas: U256,

    /// Maximum priority fee per gas (similar to EIP-1559)
    pub max_priority_fee_per_gas: U256,

    /// Address of paymaster sponsoring the user operation, followed by extra data to send to the
    /// paymaster (can be empty)
    pub paymaster_and_data: Bytes,

    /// Data passed to the account along with the nonce during the verification step
    pub signature: Bytes,
}

/// User operation without signature (helper for packing user operation)
#[derive(EthAbiCodec, EthAbiType)]
struct UserOperationNoSignature {
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

impl From<UserOperationSigned> for UserOperationNoSignature {
    fn from(value: UserOperationSigned) -> Self {
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

impl UserOperationSigned {
    /// Packs the user operation into bytes
    pub fn pack(&self) -> Bytes {
        self.clone().encode().into()
    }

    /// Packs the user operation without signature to bytes (used for calculating the hash)
    pub fn pack_without_signature(&self) -> Bytes {
        let user_operation_packed = UserOperationNoSignature::from(self.clone());
        user_operation_packed.encode().into()
    }

    /// Calculates the hash of the user operation
    pub fn hash(&self, entry_point: &Address, chain_id: u64) -> UserOperationHash {
        H256::from_slice(
            keccak256(
                [
                    keccak256(self.pack_without_signature().deref()).to_vec(),
                    entry_point.encode(),
                    U256::from(chain_id).encode(),
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

    /// Gets the entities (optionally if present) involved in the user operation
    pub fn get_entities(&self) -> (Address, Option<Address>, Option<Address>) {
        let sender = self.sender;
        let factory = get_address(&self.init_code);
        let paymaster = get_address(&self.paymaster_and_data);
        (sender, factory, paymaster)
    }

    /// Creates random user operation (for testing purposes)
    #[cfg(feature = "test-utils")]
    pub fn random() -> Self {
        UserOperationSigned::default()
            .sender(Address::random())
            .verification_gas_limit(100_000.into())
            .pre_verification_gas(21_000.into())
            .max_priority_fee_per_gas(1_000_000_000.into())
    }
}

/// This could be increased if we found bigger bytes, not sure about the proper value right now.
const MAXIMUM_SSZ_BYTES_LENGTH: usize = 1024;

fn btyes_to_list(
    value: &Bytes,
) -> Result<List<u8, MAXIMUM_SSZ_BYTES_LENGTH>, ssz_rs::SerializeError> {
    let data = value.to_vec();
    List::<u8, MAXIMUM_SSZ_BYTES_LENGTH>::try_from(data)
        .map_err(|(data, _)| ssz_rs::SerializeError::MaximumEncodedLengthReached(data.len()))
}

impl ssz_rs::Serialize for UserOperationSigned {
    fn serialize(&self, buffer: &mut Vec<u8>) -> Result<usize, ssz_rs::SerializeError> {
        let mut serializer = ssz_rs::__internal::Serializer::default();
        serializer.with_element(&self.sender.0)?;
        serializer.with_element(&self.nonce.0)?;
        serializer.with_element(&btyes_to_list(&self.init_code)?)?;
        serializer.with_element(&btyes_to_list(&self.call_data)?)?;
        serializer.with_element(&self.call_gas_limit.0)?;
        serializer.with_element(&self.verification_gas_limit.0)?;
        serializer.with_element(&self.pre_verification_gas.0)?;
        serializer.with_element(&self.max_fee_per_gas.0)?;
        serializer.with_element(&self.max_priority_fee_per_gas.0)?;
        serializer.with_element(&btyes_to_list(&self.paymaster_and_data)?)?;
        serializer.with_element(&btyes_to_list(&self.signature)?)?;
        serializer.serialize(buffer)
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
    let range = bytes_zone.next().ok_or(ssz_rs::DeserializeError::AdditionalInput {
        provided: encoding.len(),
        expected: total_bytes_read,
    })?;
    let start = range[0];
    let end = range[1];
    let bytes_data = Bytes::from_iter(encoding[start..end].iter());
    Ok((bytes_data, end - start))
}

impl ssz_rs::Deserialize for UserOperationSigned {
    fn deserialize(encoding: &[u8]) -> Result<Self, ssz_rs::DeserializeError>
    where
        Self: Sized,
    {
        let mut start = 0;
        let mut offsets: Vec<usize> = Vec::new();
        let mut container = Self::default();

        let byte_read = {
            let encoded_length = <[u8; 20] as ssz_rs::Serializable>::size_hint();
            let end = start + encoded_length;
            let target =
                encoding.get(start..end).ok_or(ssz_rs::DeserializeError::ExpectedFurtherInput {
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

impl ssz_rs::Serializable for UserOperationSigned {
    fn is_variable_size() -> bool {
        true
    }

    fn size_hint() -> usize {
        0
    }
}

/// Receipt of the user operation (returned from the RPC endpoint eth_getUserOperationReceipt)
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserOperationReceipt {
    #[serde(rename = "userOpHash")]
    pub user_operation_hash: UserOperationHash,
    #[serde(serialize_with = "as_checksum_addr")]
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
    pub user_operation: UserOperationSigned,
    #[serde(serialize_with = "as_checksum_addr")]
    pub entry_point: Address,
    pub transaction_hash: H256,
    pub block_hash: H256,
    pub block_number: U64,
}

/// Gas estimations for user operation (returned from the RPC endpoint eth_estimateUserOperationGas)
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserOperationGasEstimation {
    pub pre_verification_gas: U256,
    pub verification_gas_limit: U256,
    pub call_gas_limit: U256,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn user_operation_signed_pack() {
        let uos =  vec![
            UserOperationSigned::default().verification_gas_limit(100_000.into()).pre_verification_gas(21_000.into()).max_priority_fee_per_gas(1_000_000_000.into()),
            UserOperationSigned::default().sender("0x9c5754De1443984659E1b3a8d1931D83475ba29C".parse().unwrap()).call_gas_limit(200_000.into()).verification_gas_limit(100_000.into()).pre_verification_gas(21_000.into()).max_fee_per_gas(3_000_000_000_u64.into()).max_priority_fee_per_gas(1_000_000_000.into()).signature("0x7cb39607585dee8e297d0d7a669ad8c5e43975220b6773c10a138deadbc8ec864981de4b9b3c735288a217115fb33f8326a61ddabc60a534e3b5536515c70f931c".parse().unwrap()),
        ];
        assert_eq!(uos[0].pack(), "0x0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001600000000000000000000000000000000000000000000000000000000000000180000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000186a000000000000000000000000000000000000000000000000000000000000052080000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000003b9aca0000000000000000000000000000000000000000000000000000000000000001a000000000000000000000000000000000000000000000000000000000000001c00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000".parse::<Bytes>().unwrap());
        assert_eq!(uos[1].pack(), "0x0000000000000000000000009c5754de1443984659e1b3a8d1931d83475ba29c0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000016000000000000000000000000000000000000000000000000000000000000001800000000000000000000000000000000000000000000000000000000000030d4000000000000000000000000000000000000000000000000000000000000186a0000000000000000000000000000000000000000000000000000000000000520800000000000000000000000000000000000000000000000000000000b2d05e00000000000000000000000000000000000000000000000000000000003b9aca0000000000000000000000000000000000000000000000000000000000000001a000000000000000000000000000000000000000000000000000000000000001c000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000417cb39607585dee8e297d0d7a669ad8c5e43975220b6773c10a138deadbc8ec864981de4b9b3c735288a217115fb33f8326a61ddabc60a534e3b5536515c70f931c00000000000000000000000000000000000000000000000000000000000000".parse::<Bytes>().unwrap());
    }

    #[test]
    fn user_operation_signed_pack_without_signature() {
        let uos =  vec![
            UserOperationSigned::default().verification_gas_limit(100_000.into()).pre_verification_gas(21_000.into()).max_priority_fee_per_gas(1_000_000_000.into()),
            UserOperationSigned {
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
    fn user_operation_signed_hash() {
        let uos =  vec![
            UserOperationSigned::default().verification_gas_limit(100_000.into()).pre_verification_gas(21_000.into()).max_priority_fee_per_gas(1_000_000_000.into()),
            UserOperationSigned {
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
            uos[0].hash(&"0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789".parse().unwrap(), 80_001),
            "0x95418c07086df02ff6bc9e8bdc150b380cb761beecc098630440bcec6e862702"
                .parse::<H256>()
                .unwrap()
                .into()
        );
        assert_eq!(
            uos[1].hash(&"0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789".parse().unwrap(), 80_001),
            "0x7c1b8c9df49a9e09ecef0f0fe6841d895850d29820f9a4b494097764085dcd7e"
                .parse::<H256>()
                .unwrap()
                .into()
        );
    }

    #[test]
    fn user_operation_signed_ssz() {
        let uo = UserOperationSigned {
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
        let expected_encode = Bytes::from_str("1f9090aae28b8a3dceadf281b0f12828e676c3266400000000000000000000000000000000000000000000000000000000000000e40000003c010000a086010000000000000000000000000000000000000000000000000000000000f483050000000000000000000000000000000000000000000000000000000000b4af000000000000000000000000000000000000000000000000000000000000dea5076500000000000000000000000000000000000000000000000000000000c0a5076500000000000000000000000000000000000000000000000000000000c0010000c10100009406cc6185a346906296840746125a0e449764545fbfb9cf000000000000000000000000ce0fefa6f7979c4c9b5373e0f5105b7259092c6d0000000000000000000000000000000000000000000000000000000000000000b61d27f60000000000000000000000009c5754de1443984659e1b3a8d1931d83475ba29c00000000000000000000000000000000000000000000000000005af3107a4000000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000001febfd4657afe1f1c05c1ec65f3f9cc992a3ac083c424454ba61eab93152195e1400d74df01fc9fa53caadcb83a891d478b713016bcc0c64307c1ad3d7ea2e2d921b").unwrap().to_vec();
        assert_eq!(encoded, expected_encode);

        let uo_decode =
            <UserOperationSigned as ssz_rs::Deserialize>::deserialize(&expected_encode).unwrap();

        assert_eq!(uo_decode.sender, uo.sender);
        assert_eq!(uo_decode.nonce, uo.nonce);
        assert_eq!(uo_decode.init_code, uo.init_code);
        assert_eq!(uo_decode.call_data, uo.call_data);
        assert_eq!(uo_decode.call_gas_limit, uo.call_gas_limit);
        assert_eq!(uo_decode.verification_gas_limit, uo.verification_gas_limit);
        assert_eq!(uo_decode.pre_verification_gas, uo.pre_verification_gas);
        assert_eq!(uo_decode.max_fee_per_gas, uo.max_fee_per_gas);
        assert_eq!(uo_decode.max_priority_fee_per_gas, uo.max_priority_fee_per_gas);
        assert_eq!(uo_decode.paymaster_and_data, uo.paymaster_and_data);
        assert_eq!(uo_decode.signature, uo.signature);
    }
}

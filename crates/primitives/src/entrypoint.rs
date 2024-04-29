use ethers::{
    abi::{AbiDecode, AbiError},
    contract::{EthAbiCodec, EthAbiType},
    types::{Address, Bytes, U256},
};

#[derive(EthAbiCodec, EthAbiType, Debug, Clone, PartialEq, Eq)]
pub struct StakeInfo {
    pub stake: U256,
    pub unstake_delay_sec: U256,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnInfo {
    pub pre_op_gas: U256,
    pub prefund: U256,
    pub account_validation_data: U256,
    pub paymaster_validation_data: U256,
    pub paymaster_context: Bytes,
}

impl ReturnInfo {
    pub fn decode(data: &[u8]) -> Result<Self, AbiError> {
        let pre_op_gas = <U256 as AbiDecode>::decode(&data[..32])?;
        let prefund = <U256 as AbiDecode>::decode(&data[32..64])?;
        let account_validation_data = <U256 as AbiDecode>::decode(&data[64..96])?;
        let paymaster_validation_data = <U256 as AbiDecode>::decode(&data[96..128])?;
        let bytes_length = <U256 as AbiDecode>::decode(&data[160..192])?;
        let paymaster_context = Bytes::from_iter(&data[192..192 + bytes_length.as_usize()]);
        Ok(Self {
            pre_op_gas,
            prefund,
            account_validation_data,
            paymaster_validation_data,
            paymaster_context,
        })
    }
}

#[derive(EthAbiCodec, EthAbiType, Debug, Clone, PartialEq, Eq)]
pub struct AggregatorStakeInfo {
    pub aggregator: Address,
    pub stake_info: StakeInfo,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationResult {
    pub sender_info: StakeInfo,
    pub factory_info: StakeInfo,
    pub paymaster_info: StakeInfo,
    pub aggresgator_info: AggregatorStakeInfo,
    pub return_info: ReturnInfo,
}

impl AbiDecode for ValidationResult {
    fn decode(data: impl AsRef<[u8]>) -> Result<Self, AbiError> {
        let data = data.as_ref();
        let sender_info = <StakeInfo as AbiDecode>::decode(&data[64..128])?;
        let factory_info = <StakeInfo as AbiDecode>::decode(&data[128..192])?;
        let paymaster_info = <StakeInfo as AbiDecode>::decode(&data[192..256])?;
        let aggresgator_info = <AggregatorStakeInfo as AbiDecode>::decode(&data[256..352])?;
        let return_info = ReturnInfo::decode(&data[352..])?;
        Ok(Self { sender_info, factory_info, paymaster_info, aggresgator_info, return_info })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SimulateValidationResult {
    ValidationResult(ValidationResult),
}

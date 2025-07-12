use alloy_sol_types::sol;
use silius_primitives::{reputation::DepositInfo, user_operation::PackedUserOperation};

sol!(
    #[sol(rpc)]
    IEntryPoint,
    "resources/entry_point_v08.json"
);

impl From<PackedUserOperation> for IEntryPoint::PackedUserOperation {
    fn from(packed_user_operation: PackedUserOperation) -> Self {
        Self {
            sender: packed_user_operation.sender,
            nonce: packed_user_operation.nonce,
            initCode: packed_user_operation.init_code,
            callData: packed_user_operation.call_data,
            accountGasLimits: packed_user_operation.account_gas_limit,
            preVerificationGas: packed_user_operation.pre_verification_gas,
            gasFees: packed_user_operation.gas_fees,
            paymasterAndData: packed_user_operation.paymaster_and_data,
            signature: packed_user_operation.signature,
        }
    }
}

impl From<IStakeManager::DepositInfo> for DepositInfo {
    fn from(deposit_info: IStakeManager::DepositInfo) -> Self {
        Self {
            deposit: deposit_info.deposit,
            staked: deposit_info.staked,
            stake: deposit_info.stake,
            unstake_delay_sec: deposit_info.unstakeDelaySec,
            withdraw_time: deposit_info.withdrawTime,
        }
    }
}

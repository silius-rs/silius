use ethers::{
    contract::{abigen, EthCall},
    types::Selector,
};
use lazy_static::lazy_static;
use std::collections::HashMap;

abigen!(
    EntryPointAPI,
    r#"[
        struct PackedUserOperation {address sender;uint256 nonce;bytes initCode;bytes callData;bytes32 accountGasLimits;uint256 preVerificationGas;bytes32 gasFees;bytes paymasterAndData;bytes signature;}
        struct ValidationResult {StakeInfo senderInfo;StakeInfo factoryInfo;StakeInfo paymasterInfo;AggregatorStakeInfo aggregatorInfo;ReturnInfo returnInfo;}
        struct UserOpsPerAggregator {PackedUserOperation[] userOps;address aggregator;bytes signature;}
        struct DepositInfo {uint256 deposit;bool staked;uint112 stake;uint32 unstakeDelaySec;uint48 withdrawTime;}
        struct ReturnInfo {uint256 preOpGas;uint256 prefund;uint256 accountValidationData;uint256 paymasterValidationData;bytes paymasterContext;}
        struct StakeInfo {uint256 stake;uint256 unstakeDelaySec;}
        struct AggregatorStakeInfo {address aggregator;StakeInfo stakeInfo;}
        struct ExecutionResult {uint256 preOpGas;uint256 paid;uint256 accountValidationData;uint256 paymasterValidationData;bool targetSuccess;bytes targetResult;}
        function handleOps(PackedUserOperation[] calldata ops,address payable beneficiary) external;
        function handleAggregatedOps(UserOpsPerAggregator[] calldata opsPerAggregator,address payable beneficiary) external;
        function getDepositInfo(address account) external view returns (DepositInfo memory info)
        function balanceOf(address account) external view returns (uint256)
        function depositTo(address account) external payable
        function addStake(uint32 _unstakeDelaySec) external payable
        function getSenderAddress(bytes memory initCode) external
        function getUserOpHash(PackedUserOperation calldata userOp) external view returns (bytes32)
        function simulateValidation(PackedUserOperation calldata userOp) external returns (ValidationResult memory)
        function simulateHandleOp(PackedUserOperation calldata op,address target,bytes calldata targetCallData)external returns (ExecutionResult memory)
        function unlockStake() external
        function withdrawStake(address payable withdrawAddress) external
        function withdrawTo(address payable withdrawAddress,uint256 withdrawAmount) external
        function createSender(bytes calldata initCode) external returns (address sender)
        function validateUserOp(PackedUserOperation calldata userOp,bytes32 userOpHash,uint256 missingAccountFunds) external returns (uint256 validationData)
        function validatePaymasterUserOp(PackedUserOperation calldata userOp,bytes32 userOpHash,uint256 maxCost) external returns (bytes memory context, uint256 validationData)
        function getNonce(address sender, uint192 key) public view override returns (uint256 nonce)
        function check() external returns (uint256 result)
        error FailedOp(uint256 opIndex, string reason)
        error FailedOpWithRevert(uint256 opIndex, string reason, bytes inner)
        error PostOpReverted(bytes returnData)
        error SenderAddressResult(address sender)
        event UserOperationRevertReason(bytes32 indexed userOpHash,address indexed sender,uint256 nonce,bytes revertReason)
        event UserOperationEvent(bytes32 indexed userOpHash,address indexed sender,address indexed paymaster,uint256 nonce,bool success,uint256 actualGasCost,uint256 actualGasUsed)
    ]"#
);

lazy_static! {
    pub static ref SELECTORS_NAMES: HashMap<Selector, String> = {
        let mut map = HashMap::new();
        // entry point
        map.insert(entry_point_api::AddStakeCall::selector(), entry_point_api::AddStakeCall::function_name().into());
        map.insert(entry_point_api::BalanceOfCall::selector(), entry_point_api::BalanceOfCall::function_name().into());
        map.insert(entry_point_api::DepositToCall::selector(), entry_point_api::DepositToCall::function_name().into());
        map.insert(entry_point_api::GetDepositInfoCall::selector(), entry_point_api::GetDepositInfoCall::function_name().into());
        map.insert(entry_point_api::GetSenderAddressCall::selector(), entry_point_api::GetSenderAddressCall::function_name().into());
        map.insert(entry_point_api::GetUserOpHashCall::selector(), entry_point_api::GetUserOpHashCall::function_name().into());
        map.insert(entry_point_api::HandleAggregatedOpsCall::selector(), entry_point_api::HandleAggregatedOpsCall::function_name().into());
        map.insert(entry_point_api::HandleOpsCall::selector(), entry_point_api::HandleOpsCall::function_name().into());
        map.insert(entry_point_api::SimulateHandleOpCall::selector(), entry_point_api::SimulateHandleOpCall::function_name().into());
        map.insert(entry_point_api::SimulateValidationCall::selector(), entry_point_api::SimulateValidationCall::function_name().into());
        map.insert(entry_point_api::UnlockStakeCall::selector(), entry_point_api::UnlockStakeCall::function_name().into());
        map.insert(entry_point_api::WithdrawStakeCall::selector(), entry_point_api::WithdrawStakeCall::function_name().into());
        map.insert(entry_point_api::WithdrawToCall::selector(), entry_point_api::WithdrawToCall::function_name().into());
        // sender creator
        map.insert(entry_point_api::CreateSenderCall::selector(), entry_point_api::CreateSenderCall::function_name().into());
        // account
        map.insert(entry_point_api::ValidateUserOpCall::selector(), entry_point_api::ValidateUserOpCall::function_name().into());
        // paymaster
        map.insert(entry_point_api::ValidatePaymasterUserOpCall::selector(), entry_point_api::ValidatePaymasterUserOpCall::function_name().into());

        map
    };
    pub static ref SELECTORS_INDICES: HashMap<Selector, usize> = {
        let mut map = HashMap::new();
        // factory
        map.insert(entry_point_api::CreateSenderCall::selector(), 0);
        // sender/account
        map.insert(entry_point_api::ValidateUserOpCall::selector(), 1);
        // paymaster
        map.insert(entry_point_api::ValidatePaymasterUserOpCall::selector(), 2);
        map
    };


}

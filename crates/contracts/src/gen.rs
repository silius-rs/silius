use ethers::{
    contract::{abigen, EthCall},
    types::Selector,
};
use lazy_static::lazy_static;
use std::collections::HashMap;

abigen!(EntryPointAPI, "$OUT_DIR/IEntryPoint.sol/IEntryPoint.json");
abigen!(
    StakeManagerAPI,
    "$OUT_DIR/IStakeManager.sol/IStakeManager.json"
);
abigen!(PaymasterAPI, "$OUT_DIR/IPaymaster.sol/IPaymaster.json");

lazy_static! {
    pub static ref CONTRACTS_FUNCTIONS: HashMap<Selector, String> = {
        let mut map = HashMap::new();
        // entry point
        map.insert(entry_point_api::AddStakeCall::selector(), entry_point_api::AddStakeCall::function_name().to_string());
        map.insert(entry_point_api::BalanceOfCall::selector(), entry_point_api::BalanceOfCall::function_name().to_string());
        map.insert(entry_point_api::DepositToCall::selector(), entry_point_api::DepositToCall::function_name().to_string());
        map.insert(entry_point_api::GetDepositInfoCall::selector(), entry_point_api::GetDepositInfoCall::function_name().to_string());
        map.insert(entry_point_api::GetSenderAddressCall::selector(), entry_point_api::GetSenderAddressCall::function_name().to_string());
        map.insert(entry_point_api::GetUserOpHashCall::selector(), entry_point_api::GetUserOpHashCall::function_name().to_string());
        map.insert(entry_point_api::HandleAggregatedOpsCall::selector(), entry_point_api::HandleAggregatedOpsCall::function_name().to_string());
        map.insert(entry_point_api::HandleOpsCall::selector(), entry_point_api::HandleOpsCall::function_name().to_string());
        map.insert(entry_point_api::SimulateHandleOpCall::selector(), entry_point_api::SimulateHandleOpCall::function_name().to_string());
        map.insert(entry_point_api::SimulateValidationCall::selector(), entry_point_api::SimulateValidationCall::function_name().to_string());
        map.insert(entry_point_api::UnlockStakeCall::selector(), entry_point_api::UnlockStakeCall::function_name().to_string());
        map.insert(entry_point_api::WithdrawStakeCall::selector(), entry_point_api::WithdrawStakeCall::function_name().to_string());
        map.insert(entry_point_api::WithdrawToCall::selector(), entry_point_api::WithdrawToCall::function_name().to_string());
        // paymaster
        map.insert(paymaster_api::PostOpCall::selector(), paymaster_api::PostOpCall::function_name().to_string());
        map.insert(paymaster_api::ValidatePaymasterUserOpCall::selector(), paymaster_api::ValidatePaymasterUserOpCall::function_name().to_string());
        map
    };
}

// The below generations are not used now. So we comment them out for now.
// abigen!(
//     AggregatedAccount,
//     "$OUT_DIR/IAggregatedAccount.sol/IAggregatedAccount.json"
// );
// abigen!(Aggregator, "$OUT_DIR/IAggregator.sol/IAggregator.json");
// abigen!(
//     Create2Deployer,
//     "$OUT_DIR/ICreate2Deployer.sol/ICreate2Deployer.json"
// );
// abigen!(Account, "$OUT_DIR/IAccount.sol/IAccount.json");
// abigen!(
//     UserOperation,
//     "$OUT_DIR/UserOperation.sol/UserOperationLib.json"
// );

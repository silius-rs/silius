use ethers::{
    contract::{abigen, EthCall},
    types::Selector,
};
use lazy_static::lazy_static;
use std::collections::HashMap;

abigen!(AccountAPI, "$OUT_DIR/IAccount.sol/IAccount.json");
abigen!(EntryPointAPI, "$OUT_DIR/IEntryPoint.sol/IEntryPoint.json");
abigen!(PaymasterAPI, "$OUT_DIR/IPaymaster.sol/IPaymaster.json");
abigen!(SenderCreatorAPI, "$OUT_DIR/SenderCreator.sol/SenderCreator.json");
abigen!(StakeManagerAPI, "$OUT_DIR/IStakeManager.sol/IStakeManager.json");

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
        map.insert(sender_creator_api::CreateSenderCall::selector(), sender_creator_api::CreateSenderCall::function_name().into());
        // account
        map.insert(account_api::ValidateUserOpCall::selector(), account_api::ValidateUserOpCall::function_name().into());
        // paymaster
        map.insert(paymaster_api::ValidatePaymasterUserOpCall::selector(), paymaster_api::ValidatePaymasterUserOpCall::function_name().into());
        map
    };
    pub static ref SELECTORS_INDICES: HashMap<Selector, usize> = {
        let mut map = HashMap::new();
        // factory
        map.insert(sender_creator_api::CreateSenderCall::selector(), 0);
        // sender/account
        map.insert(account_api::ValidateUserOpCall::selector(), 1);
        // paymaster
        map.insert(paymaster_api::ValidatePaymasterUserOpCall::selector(), 2);
        map
    };
}

use ethers::contract::abigen;

abigen!(EntryPointAPI, "$OUT_DIR/IEntryPoint.sol/IEntryPoint.json");

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
// abigen!(Paymaster, "$OUT_DIR/IPaymaster.sol/IPaymaster.json");
// abigen!(
//     StakeManager,
//     "$OUT_DIR/IStakeManager.sol/IStakeManager.json"
// );
// abigen!(Account, "$OUT_DIR/IAccount.sol/IAccount.json");
// abigen!(
//     UserOperation,
//     "$OUT_DIR/UserOperation.sol/UserOperationLib.json"
// );

// impl From<UserOp> for entry_point_api::UserOperation {
//     fn from(value: UserOp) -> Self {
//         Self {
//             sender: value.sender,
//             nonce: value.nonce,
//             init_code: value.init_code,
//             call_data: value.call_data,
//             call_gas_limit: value.call_gas_limit,
//             verification_gas_limit: value.verification_gas_limit,
//             pre_verification_gas: value.pre_verification_gas,
//             max_fee_per_gas: value.max_fee_per_gas,
//             max_priority_fee_per_gas: value.max_priority_fee_per_gas,
//             paymaster_and_data: value.paymaster_and_data,
//             signature: value.signature,
//         }
//     }
// }

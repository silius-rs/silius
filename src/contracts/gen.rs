use ethers::contract::abigen;

abigen!(EntryPointAPI, "$OUT_DIR/IEntryPoint.sol/IEntryPoint.json");
abigen!(
    StakeManagerAPI,
    "$OUT_DIR/IStakeManager.sol/IStakeManager.json"
);

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
// abigen!(Account, "$OUT_DIR/IAccount.sol/IAccount.json");
// abigen!(
//     UserOperation,
//     "$OUT_DIR/UserOperation.sol/UserOperationLib.json"
// );

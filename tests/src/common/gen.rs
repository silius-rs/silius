use ethers::prelude::abigen;

abigen!(SimpleAccountFactory,
    "$CARGO_WORKSPACE_DIR/crates/contracts/thirdparty/account-abstraction/artifacts/contracts/samples/SimpleAccountFactory.sol/SimpleAccountFactory.json");

abigen!(SimpleAccount,
    "$CARGO_WORKSPACE_DIR/crates/contracts/thirdparty/account-abstraction/artifacts/contracts/samples/SimpleAccount.sol/SimpleAccount.json");

abigen!(
    EntryPointContract,
    "$CARGO_WORKSPACE_DIR/crates/contracts/thirdparty/account-abstraction/artifacts/contracts/core/EntryPoint.sol/EntryPoint.json"
);
abigen!(
    TestOpcodesAccountFactory,
    "$CARGO_WORKSPACE_DIR/tests/thirdparty/bundler/packages/bundler/artifacts/contracts/tests/TestOpcodesAccount.sol/TestOpcodesAccountFactory.json"
);
abigen!(
    TestOpcodesAccount,
    "$CARGO_WORKSPACE_DIR/tests/thirdparty/bundler/packages/bundler/artifacts/contracts/tests/TestOpcodesAccount.sol/TestOpcodesAccount.json"
);
abigen!(
    TestStorageAccount,
    "$CARGO_WORKSPACE_DIR/tests/thirdparty/bundler/packages/bundler/artifacts/contracts/tests/TestStorageAccount.sol/TestStorageAccount.json"
);
abigen!(
    TestRecursionAccount,
    "$CARGO_WORKSPACE_DIR/tests/thirdparty/bundler/packages/bundler/artifacts/contracts/tests/TestRecursionAccount.sol/TestRecursionAccount.json"
);
abigen!(
    TestStorageAccountFactory,
    "$CARGO_WORKSPACE_DIR/tests/thirdparty/bundler/packages/bundler/artifacts/contracts/tests/TestStorageAccount.sol/TestStorageAccountFactory.json"
);
abigen!(
    TestRulesAccount,
    "$CARGO_WORKSPACE_DIR/tests/thirdparty/bundler/packages/bundler/artifacts/contracts/tests/TestRulesAccount.sol/TestRulesAccount.json"
);
abigen!(
    TestRulesAccountFactory,
    "$CARGO_WORKSPACE_DIR/tests/thirdparty/bundler/packages/bundler/artifacts/contracts/tests/TestRulesAccount.sol/TestRulesAccountFactory.json"
);
abigen!(
    TracerTest,
    "$CARGO_WORKSPACE_DIR/tests/thirdparty/bundler/packages/bundler/artifacts/contracts/tests/TracerTest.sol/TracerTest.json"
);
abigen!(
    TestCoin,
    "$CARGO_WORKSPACE_DIR/tests/thirdparty/bundler/packages/bundler/artifacts/contracts/tests/TestCoin.sol/TestCoin.json"
);

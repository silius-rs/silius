use ethers::prelude::abigen;

abigen!(SimpleAccountFactory,
    "$CARGO_MANIFEST_DIR/thirdparty/account-abstraction/artifacts/contracts/samples/SimpleAccountFactory.sol/SimpleAccountFactory.json");

abigen!(SimpleAccount,
    "$CARGO_MANIFEST_DIR/thirdparty/account-abstraction/artifacts/contracts/samples/SimpleAccount.sol/SimpleAccount.json");

abigen!(
    EntryPointContract,
    "$CARGO_MANIFEST_DIR/thirdparty/account-abstraction/artifacts/contracts/core/EntryPoint.sol/EntryPoint.json"
);
abigen!(
    TestOpcodesAccountFactory,
    "$CARGO_MANIFEST_DIR/thirdparty/bundler/packages/bundler/artifacts/contracts/tests/TestOpcodesAccount.sol/TestOpcodesAccountFactory.json"
);
abigen!(
    TestOpcodesAccount,
    "$CARGO_MANIFEST_DIR/thirdparty/bundler/packages/bundler/artifacts/contracts/tests/TestOpcodesAccount.sol/TestOpcodesAccount.json"
);
abigen!(
    TestStorageAccount,
    "$CARGO_MANIFEST_DIR/thirdparty/bundler/packages/bundler/artifacts/contracts/tests/TestStorageAccount.sol/TestStorageAccount.json"
);
abigen!(
    TestRecursionAccount,
    "$CARGO_MANIFEST_DIR/thirdparty/bundler/packages/bundler/artifacts/contracts/tests/TestRecursionAccount.sol/TestRecursionAccount.json"
);
abigen!(
    TestStorageAccountFactory,
    "$CARGO_MANIFEST_DIR/thirdparty/bundler/packages/bundler/artifacts/contracts/tests/TestStorageAccount.sol/TestStorageAccountFactory.json"
);
abigen!(
    TestRulesAccount,
    "$CARGO_MANIFEST_DIR/thirdparty/bundler/packages/bundler/artifacts/contracts/tests/TestRulesAccount.sol/TestRulesAccount.json"
);
abigen!(
    TestRulesAccountFactory,
    "$CARGO_MANIFEST_DIR/thirdparty/bundler/packages/bundler/artifacts/contracts/tests/TestRulesAccount.sol/TestRulesAccountFactory.json"
);

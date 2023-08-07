/// Entry point
pub mod entry_point {
    pub const ADDRESS: &str = "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789";
    pub const VERSION: &str = "0.6.0";
}

/// RPC error codes
pub mod rpc_error_codes {
    pub const VALIDATION: i32 = -32500;
    pub const PAYMASTER: i32 = -32501;
    pub const OPCODE: i32 = -32502;
    pub const EXPIRATION: i32 = -32503;
    pub const ENTITY_BANNED: i32 = -32504;
    pub const STAKE_TOO_LOW: i32 = -32505;
    pub const SIGNATURE_AGGREGATOR: i32 = -32506;
    pub const SIGNATURE: i32 = -32507;
    pub const EXECUTION: i32 = -32521;
    pub const USER_OPERATION_HASH: i32 = -32601;
    pub const SANITY_CHECK: i32 = -32602;
}

/// Entities
pub mod entities {
    /// Markers used in entry point smart contracts to differentiate between parts of user operation
    // https://github.com/eth-infinitism/account-abstraction/blob/develop/contracts/core/EntryPoint.sol#L514
    // 0 - factory, 1 - sender/account, 2 - paymaster
    // opcode NUMBER is marker between levels
    pub const NUMBER_LEVELS: usize = 3;

    pub const FACTORY: &str = "factory";
    pub const ACCOUNT: &str = "account";
    pub const PAYMASTER: &str = "paymaster";

    pub const FACTORY_INDEX: usize = 0;
    pub const ACCOUNT_INDEX: usize = 1;
    pub const PAYMASTER_INDEX: usize = 2;

    pub const LEVEL_TO_ENTITY: [&str; NUMBER_LEVELS] = [FACTORY, ACCOUNT, PAYMASTER];
}

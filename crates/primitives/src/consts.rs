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
    pub const ENTITY_BANNED_OR_THROTTLED: i32 = -32504;
    pub const STAKE_TOO_LOW: i32 = -32505;
    pub const SIGNATURE_AGGREGATOR: i32 = -32506;
    pub const SIGNATURE: i32 = -32507;
    pub const EXECUTION: i32 = -32521;
    pub const USER_OPERATION_HASH: i32 = -32601;
    pub const SANITY_CHECK: i32 = -32602;
}

/// Entities
pub mod entities {
    // 0 - factory, 1 - sender/account, 2 - paymaster
    pub const NUMBER_LEVELS: usize = 3;

    pub const FACTORY: &str = "factory";
    pub const SENDER: &str = "account";
    pub const PAYMASTER: &str = "paymaster";

    pub const FACTORY_LEVEL: usize = 0;
    pub const SENDER_LEVEL: usize = 1;
    pub const PAYMASTER_LEVEL: usize = 2;

    pub const LEVEL_TO_ENTITY: [&str; NUMBER_LEVELS] = [FACTORY, SENDER, PAYMASTER];
}

/// Reputation
/// https://github.com/eth-infinitism/account-abstraction/blob/develop/eip/EIPS/eip-aa-rules.md#constants
pub mod reputation {
    pub const MIN_UNSTAKE_DELAY: u64 = 86400;
    // pub const MIN_STAKE_VALUE - Adjustable per chain value, Equivalent to ~$1000 in native tokens
    pub const SAME_SENDER_MEMPOOL_COUNT: usize = 4;
    pub const SAME_UNSTAKED_ENTITY_MEMPOOL_COUNT: usize = 10;
    pub const THROTTLED_ENTITY_MEMPOOL_COUNT: usize = 4;
    pub const THROTTLED_ENTITY_LIVE_BLOCKS: usize = 4;
    pub const THROTTLED_ENTITY_BUNDLE_COUNT: usize = 4;
    pub const MIN_INCLUSION_RATE_DENOMINATOR: u64 = 10;
    pub const INCLUSION_RATE_FACTOR: u64 = 10;
    pub const THROTTLING_SLACK: u64 = 10;
    pub const BAN_SLACK: u64 = 50;
}

/// User opereation mempool
pub mod uopool {
    pub const GAS_INCREASE_PERC: u64 = 10;
    pub const LATEST_SCAN_DEPTH: u64 = 1000;
}

/// Builder JSON-RPC Endpoints
pub mod flashbots_relay_endpoints {
    pub const FLASHBOTS: &str = "https://relay.flashbots.net";
    pub const FLASHBOTS_GOERLI: &str = "https://relay-goerli.flashbots.net";
    pub const BUILDER0X69: &str = "http://builder0x69.io/";
    pub const EDENNETWORK: &str = "https://api.edennetwork.io/v1/bundle";
    pub const BEAVERBUILD: &str = "https://rpc.beaverbuild.org/";
    pub const LIGHTSPEEDBUILDER: &str = "https://rpc.lightspeedbuilder.info/";
    pub const ETH_BUILDER: &str = "https://eth-builder.com/";
    pub const ULTRASOUND: &str = "https://relay.ultrasound.money/";
    pub const AGNOSTIC_RELAY: &str = "https://agnostic-relay.net/";
    pub const RELAYOOR_WTF: &str = "https://relayooor.wtf/";
    pub const RSYNC_BUILDER: &str = "https://rsync-builder.xyz/";
    pub const LOKI_BUILDER: &str = "https://rpc.lokibuilder.xyz/";
}

/// Supported networks' IDs
pub mod supported_networks {
    pub const MAINNET: u64 = 1;
    pub const GOERLI: u64 = 5;
    pub const SEPOLIA: u64 = 11155111;
    pub const DEV: u64 = 1337;
    pub const MUMBAI: u64 = 80001;
}

/// Networking related constants
pub mod networking {
    pub const DEFAULT_HTTP_RPC_PORT: u16 = 3000;
    pub const DEFAULT_WS_RPC_PORT: u16 = 3001;
    pub const DEFAULT_UOPOOL_GRPC_PORT: u16 = 3002;
    pub const DEFAULT_BUNDLER_GRPC_PORT: u16 = 3003;
}

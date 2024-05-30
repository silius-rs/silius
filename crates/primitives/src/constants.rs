//! Account abstraction (ERC-4337)-related constants

/// Entry point smart contract
pub mod entry_point {
    /// Address of the entry point smart contract
    pub const ADDRESS: &str = "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789";
    /// Version of the entry point smart contract
    pub const VERSION: &str = "0.6.0";
}

/// Bundler
pub mod bundler {
    /// Default time interval for auto bundling mode (in seconds)
    pub const BUNDLE_INTERVAL: u64 = 10;
}

/// User operation mempool
pub mod mempool {
    /// Percentage increase of gas price to replace a user operation in the mempool
    pub const GAS_INCREASE_PERC: u64 = 10;
    /// Depth scan when searching for previous user operations
    pub const LATEST_SCAN_DEPTH: u64 = 1000;
}

/// User operation validation
pub mod validation {
    /// Entities (factory, sender/account, paymaster, aggregator)
    pub mod entities {
        // 0 - factory, 1 - sender/account, 2 - paymaster
        pub const NUMBER_OF_LEVELS: usize = 3;

        pub const FACTORY: &str = "factory";
        pub const SENDER: &str = "account";
        pub const PAYMASTER: &str = "paymaster";

        pub const FACTORY_LEVEL: usize = 0;
        pub const SENDER_LEVEL: usize = 1;
        pub const PAYMASTER_LEVEL: usize = 2;

        pub const LEVEL_TO_ENTITY: [&str; NUMBER_OF_LEVELS] = [FACTORY, SENDER, PAYMASTER];
    }

    /// Reputation
    /// <https://github.com/eth-infinitism/account-abstraction/blob/develop/eip/EIPS/eip-aa-rules.md#constants>
    pub mod reputation {
        pub const MIN_UNSTAKE_DELAY: u64 = 86400;
        // pub const MIN_STAKE_VALUE - Adjustable per chain value, Equivalent to ~$1000 in native
        // tokens
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

    /// Simulation
    pub mod simulation {
        pub const MIN_EXTRA_GAS: u64 = 2000;
    }
}

/// Flashbots relay endpoints
pub mod flashbots_relay_endpoints {
    // mainnet
    pub const FLASHBOTS: &str = "https://relay.flashbots.net";
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

    // sepolia
    pub const FLASHBOTS_SEPOLIA: &str = "https://relay-sepolia.flashbots.net";
}

/// Supported chains
pub mod supported_chains {
    use alloy_chains::NamedChain;

    pub const CHAINS: [NamedChain; 10] = [
        NamedChain::Dev,
        NamedChain::Mainnet,
        NamedChain::Sepolia,
        NamedChain::Holesky,
        NamedChain::PolygonMumbai,
        NamedChain::OptimismSepolia,
        NamedChain::ArbitrumSepolia,
        NamedChain::BinanceSmartChainTestnet,
        NamedChain::BaseSepolia,
        NamedChain::AvalancheFuji,
    ];
}

/// RPC
pub mod rpc {
    /// The default port for HTTP
    pub const HTTP_PORT: u16 = 3000;
    /// The default port for WS
    pub const WS_PORT: u16 = 3001;
}

/// gRPC
pub mod grpc {
    /// The default port for user operation mempool
    pub const MEMPOOL_PORT: u16 = 3002;
    /// The default port for bundler
    pub const BUNDLER_PORT: u16 = 3003;
}

/// Storage
pub mod storage {
    /// The default path for database
    pub const DATABASE_FOLDER_NAME: &str = "db";
}

/// P2P
pub mod p2p {
    use std::net::Ipv4Addr;

    /// The default IP address.
    pub const IPV4_ADDRESS: Ipv4Addr = Ipv4Addr::UNSPECIFIED;
    /// The default UDP port for discovery.
    pub const UDP_PORT: u16 = 9000;
    /// The default TCP port for libp2p services.
    pub const TCP_PORT: u16 = 9000;
    /// The default path for storing the node p2p key
    pub const NODE_KEY_FILE_NAME: &str = "p2p/node-key";
    /// The default path for storing the node enr
    pub const NODE_ENR_FILE_NAME: &str = "p2p/node-enr";
    /// Default number of peers to find on the initial discovery.
    pub const FIND_NODE_QUERY_CLOSEST_PEERS: usize = 16;
    /// Default target peers.
    pub const TARGET_PEERS: usize = 50;
    /// Default heartbeat interval (how often we perform discovery and peer management).
    pub const HEARTBEAT_INTERVAL: u64 = 30;
    /// Default outbound ping interval.
    pub const PING_INTERVAL_OUTBOUND: u64 = 15;
    /// Default inbound ping interval.
    pub const PING_INTERVAL_INBOUND: u64 = 20;
    /// Request message size maximum
    pub const REQUEST_SIZE_MAXIMUM: u64 = 1024 * 1024; // bytes
    /// Response message size maximum
    pub const RESPONSE_SIZE_MAXIMUM: u64 = 10 * 1024 * 1024; // bytes
    /// The protocol prefix for request-response protocol
    pub const PROTOCOL_PREFIX: &str = "/account_abstraction/req";
    /// The topic hash prefix for gossisub protocol
    pub const TOPIC_PREFIX: &str = "account_abstraction";
    /// User operations topic for gossipsub protocol
    pub const USER_OPERATIONS_TOPIC: &str = "user_operations";
    /// The snappy encoding for gossipsub protocol
    pub const SSZ_SNAPPY_ENCODING: &str = "ssz_snappy";
    /// The maximum size of a uncompressed gossipsub message
    pub const GOSSIP_MAX_SIZE: usize = 1048576; // bytes
    /// The maximum number of user operations in a single request
    pub const MAX_OPS_PER_REQUEST: usize = 4096;
    /// The maximum time for complete response transfer
    pub const RESP_TIMEOUT: u64 = 10; // seconds
    /// The maximum time to wait for first byte of request response (time-to-first-byte)
    pub const TTFB_TIMEOUT: usize = 5; // seconds
    /// The amount of time to maintain a request context of pooled hashes
    pub const POOLED_HASHES_CONTEXT_TIMEOUT: usize = 10; // seconds
    /// The maximum amount of supported mempools
    pub const MAX_SUPPORTED_MEMPOOLS: usize = 1024;
    /// 4-byte domain for gossip message-id isolation of *invalid* snappy messages
    pub const MESSAGE_DOMAIN_INVALID_SNAPPY: [u8; 4] = [0, 0, 0, 0];
    /// 4-byte domain for gossip message-id isolation of *valid* snappy messages
    pub const MESSAGE_DOMAIN_VALID_SNAPPY: [u8; 4] = [1, 0, 0, 0];
    /// The maximum length for the IPFS CID string.
    pub const MAX_IPFS_CID_LENGTH: usize = 256;
}

use crate::utils::{parse_address, parse_u256, parse_uopool_mode};
use clap::Parser;
use ethers::types::{Address, U256};
use silius_primitives::UoPoolMode;
use std::net::SocketAddr;

#[derive(Clone, Debug, Parser, PartialEq)]
pub struct UoPoolServiceOpts {
    #[clap(long, default_value = "127.0.0.1:3001")]
    pub uopool_grpc_listen_address: SocketAddr,

    #[clap(long, value_parser=parse_u256, default_value = "1")]
    pub min_stake: U256,

    #[clap(long, value_parser=parse_u256, default_value = "0")]
    pub min_unstake_delay: U256,

    #[clap(long, value_parser=parse_u256, default_value = "0")]
    pub min_priority_fee_per_gas: U256,

    #[clap(long, value_delimiter=',', value_parser = parse_address)]
    pub whitelist: Vec<Address>,

    #[clap(long, default_value = "standard", value_parser=parse_uopool_mode)]
    pub uo_pool_mode: UoPoolMode,
}

#[derive(Clone, Debug, Parser, PartialEq)]
pub struct BundlerServiceOpts {
    /// The bundler beneficiary address.
    #[clap(long, value_parser=parse_address)]
    pub beneficiary: Address,

    /// The minimum balance required for the beneficiary address.
    ///
    /// By default, this option is set to `100000000000000000`.
    #[clap(long, default_value = "100000000000000000", value_parser=parse_u256)]
    pub min_balance: U256,

    /// The gRPC listen address for the bundler service.
    ///
    /// By default, this option is set to `127.0.0.1:3002`
    #[clap(long, default_value = "127.0.0.1:3002")]
    pub bundler_grpc_listen_address: SocketAddr,

    /// The bundle interval in seconds.
    ///
    /// By default the interval time is set to 10
    #[clap(long, default_value = "10")]
    pub bundle_interval: u64,

    /// Enables or disables the creation of Flashbots bundler signer.
    ///
    /// By default, this option is set to false.
    #[clap(long)]
    pub build_fb_signer: Option<bool>,

    /// Sets the send bundle mode.
    ///
    /// By default, this option is set to `eth-client`.
    #[clap(long)]
    pub send_bundle_mode: Option<String>,
}

#[derive(Clone, Debug, Parser, PartialEq)]
pub struct RpcServiceOpts {
    /// Sets the RPC listen address.
    ///
    /// By default, this option is set to `127.0.0.1:3000
    #[clap(long, default_value = "127.0.0.1:3000")]
    pub rpc_listen_address: String,

    #[clap(long, value_delimiter=',', default_value = "eth", value_parser = ["eth", "debug", "web3"])]
    pub rpc_api: Vec<String>,

    /// Enables or disables the HTTP RPC.
    ///
    /// By default, this option is set to false.
    /// - To enable: `--http`.
    /// - To disable: no `--http` flag.
    #[clap(long)]
    pub http: bool,

    /// Enables or disables the WebSocket RPC.
    ///
    /// By default, this option is set to false.
    /// - To enable: `--ws`
    /// - To disable: no `--ws` flag.
    #[clap(long)]
    pub ws: bool,

    /// Configures the CORS filter.
    ///
    /// By default, this option is set to `*`.
    #[clap(long, value_delimiter = ',', default_value = "*")]
    pub cors_domain: Vec<String>,
}

impl RpcServiceOpts {
    /// Checks if either HTTP or WebSocket RPC is enabled.
    ///
    /// # Returns
    /// * `bool` - Returns `true` if either HTTP or WebSocket RPC is enabled, otherwise `false`.
    pub fn is_enabled(&self) -> bool {
        self.http || self.ws
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        net::{IpAddr, Ipv4Addr},
        str::FromStr,
    };

    #[test]
    fn bundler_opts() {
        let args = vec![
            "bundleropts",
            "--beneficiary",
            "0x690B9A9E9aa1C9dB991C7721a92d351Db4FaC990",
            "--min-balance",
            "100000000000000000",
            "--bundler-grpc-listen-address",
            "127.0.0.1:3002",
            "--bundle-interval",
            "10",
        ];
        assert_eq!(
            BundlerServiceOpts {
                beneficiary: Address::from_str("0x690B9A9E9aa1C9dB991C7721a92d351Db4FaC990")
                    .unwrap(),
                min_balance: U256::from(100000000000000000_u64),
                bundler_grpc_listen_address: SocketAddr::new(
                    IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                    3002
                ),
                bundle_interval: 10,
                build_fb_signer: None,
                send_bundle_mode: None,
            },
            BundlerServiceOpts::try_parse_from(args).unwrap()
        );
    }

    #[test]
    fn rpc_service_opts_when_http_and_ws_flag() {
        let args = vec![
            "rpcserviceopts",
            "--rpc-listen-address",
            "127.0.0.1:1234",
            "--rpc-api",
            "eth,debug,web3",
            "--http",
            "--ws",
            "--cors-domain",
            "127.0.0.1:4321",
        ];
        assert_eq!(
            RpcServiceOpts {
                rpc_listen_address: String::from("127.0.0.1:1234"),
                rpc_api: vec![
                    String::from("eth"),
                    String::from("debug"),
                    String::from("web3")
                ],
                http: true,
                ws: true,
                cors_domain: vec![String::from("127.0.0.1:4321")],
            },
            RpcServiceOpts::try_parse_from(args).unwrap()
        );
    }

    #[test]
    fn rpc_service_opts_when_http_is_true_ws_is_false() {
        let args = vec![
            "rpcserviceopts",
            "--rpc-listen-address",
            "127.0.0.1:1234",
            "--rpc-api",
            "eth,debug,web3",
            "--http",
            "--cors-domain",
            "127.0.0.1:4321",
        ];
        assert_eq!(
            RpcServiceOpts {
                rpc_listen_address: String::from("127.0.0.1:1234"),
                rpc_api: vec![
                    String::from("eth"),
                    String::from("debug"),
                    String::from("web3")
                ],
                http: true,
                ws: false,
                cors_domain: vec![String::from("127.0.0.1:4321")],
            },
            RpcServiceOpts::try_parse_from(args).unwrap()
        );
    }

    #[test]
    fn rpc_service_opts_when_http_is_false_ws_is_true() {
        let args = vec![
            "rpcserviceopts",
            "--rpc-listen-address",
            "127.0.0.1:1234",
            "--rpc-api",
            "eth,debug,web3",
            "--ws",
            "--cors-domain",
            "127.0.0.1:4321",
        ];
        assert_eq!(
            RpcServiceOpts {
                rpc_listen_address: String::from("127.0.0.1:1234"),
                rpc_api: vec![
                    String::from("eth"),
                    String::from("debug"),
                    String::from("web3")
                ],
                http: false,
                ws: true,
                cors_domain: vec![String::from("127.0.0.1:4321")],
            },
            RpcServiceOpts::try_parse_from(args).unwrap()
        );
    }

    #[test]
    fn rpc_service_opts_when_no_http_and_ws_flag() {
        let args = vec![
            "rpcserviceopts",
            "--rpc-listen-address",
            "127.0.0.1:1234",
            "--rpc-api",
            "eth,debug,web3",
            "--cors-domain",
            "127.0.0.1:4321",
        ];
        assert_eq!(
            RpcServiceOpts {
                rpc_listen_address: String::from("127.0.0.1:1234"),
                rpc_api: vec![
                    String::from("eth"),
                    String::from("debug"),
                    String::from("web3")
                ],
                http: false,
                ws: false,
                cors_domain: vec![String::from("127.0.0.1:4321")],
            },
            RpcServiceOpts::try_parse_from(args).unwrap()
        );
    }

    #[test]
    fn is_enabled_return_true_when_only_http() {
        assert_eq!(
            RpcServiceOpts {
                rpc_listen_address: String::from("127.0.0.1:1234"),
                rpc_api: vec![
                    String::from("eth"),
                    String::from("debug"),
                    String::from("web3")
                ],
                http: true,
                ws: false,
                cors_domain: vec![String::from("127.0.0.1:4321")],
            }
            .is_enabled(),
            true
        );
    }

    #[test]
    fn is_enabled_return_true_when_only_ws() {
        assert_eq!(
            RpcServiceOpts {
                rpc_listen_address: String::from("127.0.0.1:1234"),
                rpc_api: vec![
                    String::from("eth"),
                    String::from("debug"),
                    String::from("web3")
                ],
                http: false,
                ws: true,
                cors_domain: vec![String::from("127.0.0.1:4321")],
            }
            .is_enabled(),
            true
        );
    }

    #[test]
    fn is_enabled_return_true_when_http_and_ws_are_true() {
        assert_eq!(
            RpcServiceOpts {
                rpc_listen_address: String::from("127.0.0.1:1234"),
                rpc_api: vec![
                    String::from("eth"),
                    String::from("debug"),
                    String::from("web3")
                ],
                http: true,
                ws: true,
                cors_domain: vec![String::from("127.0.0.1:4321")],
            }
            .is_enabled(),
            true
        );
    }

    #[test]
    fn is_enabled_return_false_when_http_and_ws_are_false() {
        assert_eq!(
            RpcServiceOpts {
                rpc_listen_address: String::from("127.0.0.1:1234"),
                rpc_api: vec![
                    String::from("eth"),
                    String::from("debug"),
                    String::from("web3")
                ],
                http: false,
                ws: false,
                cors_domain: vec![String::from("127.0.0.1:4321")],
            }
            .is_enabled(),
            false
        );
    }
}

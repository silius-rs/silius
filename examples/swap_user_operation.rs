use aa_bundler_primitives::{UserOperation, Wallet};
use ethers::{
    types::Address,
    signers::Signer,
};
use jsonrpsee::rpc_params;
use jsonrpsee::http_client::HttpClientBuilder;
use jsonrpsee::core::client::ClientT;
use tracing;
use tracing_subscriber::fmt;
use std::str::FromStr;



pub const ANVIL_WALLET_KEY: &str = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
pub const CHAIN_ID: u64 = 1;
pub const ENTRY_POINT: &str = "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    fmt::Subscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .init();

    let url = "http://127.0.0.1:3000";
    let client = HttpClientBuilder::default().build(url)?;
	let params = rpc_params![];

	let response: Result<String, _> = client.request("eth_chainId", params).await;
	tracing::info!("Chain Id: {:?}", response.unwrap());

    let wallet = Wallet::from_key(ANVIL_WALLET_KEY, &CHAIN_ID.into())?;
                // call_gas_limit: 33_100.into(),
                // verification_gas_limit: 60_624.into(),
                // pre_verification_gas: 44_056.into(),
                // max_fee_per_gas: 1_695_000_030_u64.into(),
                // max_priority_fee_per_gas: 1_695_000_000.into(),
                // paymaster_and_data: Bytes::default(),

    let uo_partial = UserOperation::default()
        .sender(wallet.signer.address().into())
        .verification_gas_limit(50_000.into())
        .init_code(
            "0x9406cc6185a346906296840746125a0e449764545fbfb9cf000000000000000000000000ce0fefa6f7979c4c9b5373e0f5105b7259092c6d0000000000000000000000000000000000000000000000000000000000000000".parse().unwrap()
        )
        .call_data("0xb61d27f60000000000000000000000009c5754de1443984659e1b3a8d1931d83475ba29c00000000000000000000000000000000000000000000000000005af3107a400000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000000".parse().unwrap())
        .verification_gas_limit(100_000.into()).pre_verification_gas(50_000.into()).max_fee_per_gas(3_000_000_000_000u64.into());
    let uo = UserOperation::from(uo_partial.clone());

    let ep_address = Address::from_str(ENTRY_POINT).unwrap();

    // sign user operation
    let uo_signed = wallet
        .sign_uo(
            &uo,
            &ep_address,
            &CHAIN_ID.into(),
        )
        .await?;

	let params = rpc_params![
        uo_signed,
        ep_address
    ];

	let response: Result<String, _> = client.request("eth_sendUserOperation", params).await;
	tracing::info!("Chain Id: {:?}", response.unwrap());

    Ok(())
}

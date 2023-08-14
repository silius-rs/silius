use std::{env, str::FromStr, sync::Arc, time::Duration};

use ethers::{
    prelude::{MiddlewareBuilder, SignerMiddleware},
    providers::{Http, Middleware, Provider},
    signers::{coins_bip39::English, MnemonicBuilder, Signer},
    types::{Address, Bytes, U256},
};
use reqwest;
use silius_contracts::EntryPoint;
use silius_examples::{simple_account::SimpleAccountExecute, EstimateResult, Request, Response};
use silius_primitives::UserOperation;
use silius_primitives::Wallet as UoWallet;
use silius_tests::common::gen::SimpleAccountFactory;

// stackup simple account factory
const SIMPLE_ACCOUNT_FACTORY: &str = "0x9406Cc6185a346906296840746125a0E44976454";
// 0.6.0 entrypoint address
const ENTRYPOINT_ADDRESS: &str = "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789";
const CREATE_INDEX: u64 = 1;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let key_phrase = env::var("KEY_PHRASE").unwrap();
    let bundler_url = env::var("BUNDLER_URL").unwrap();
    let wallet = MnemonicBuilder::<English>::default()
        .phrase(key_phrase.as_str())
        .build()?;
    let provider =
        Provider::<Http>::try_from(bundler_url.as_str())?.interval(Duration::from_millis(10u64));
    let client = SignerMiddleware::new(provider.clone(), wallet.clone().with_chain_id(5u64))
        .nonce_manager(wallet.address());
    let provider = Arc::new(client);

    let simple_account_factory_address = SIMPLE_ACCOUNT_FACTORY.to_string().parse::<Address>()?;
    let simple_account_factory =
        SimpleAccountFactory::new(simple_account_factory_address, provider.clone());

    let owner_address = wallet.address();
    println!("simple_account_factory: {:?}", simple_account_factory);
    println!("Signer address: {:x}", owner_address);
    let address = simple_account_factory
        .get_address(owner_address, U256::from(CREATE_INDEX))
        .call()
        .await?;
    println!("Smart account addresss: {:?}", address);

    let nonce = EntryPoint::new(provider.clone(), ENTRYPOINT_ADDRESS.parse::<Address>()?)
        .get_nonce(&address, U256::zero())
        .await?;
    println!("nonce: {:?}", nonce);
    let (gas_price, priority_fee) = provider.estimate_eip1559_fees(None).await?;
    println!(
        "gas_price: {:?}, priority_fee: {:?}",
        gas_price, priority_fee
    );

    let execution = SimpleAccountExecute::new(
        Address::from_str("0x21A14e061fe67A3060ef238DDD8Bf90c81829F7d")?,
        U256::from(1),
        Bytes::default(),
    );

    let user_op = UserOperation {
        sender: address,
        nonce,
        init_code: Bytes::default(),
        call_data: Bytes::from(execution.encode()),
        call_gas_limit: U256::from(10000u64),
        verification_gas_limit: U256::from(10000u64),
        pre_verification_gas: U256::from(1u64),
        max_fee_per_gas: U256::from(1),
        max_priority_fee_per_gas: priority_fee,
        paymaster_and_data: Bytes::new(),
        signature: Bytes::default(),
    };
    let uo_wallet = UoWallet::from_phrase(key_phrase.as_str(), &U256::from(5))?;
    let user_op = uo_wallet
        .sign_uo(
            &user_op,
            &ENTRYPOINT_ADDRESS.to_string().parse::<Address>()?,
            &U256::from(5),
        )
        .await?;

    let value = serde_json::to_value(&user_op).unwrap();

    let req_body = Request {
        jsonrpc: "2.0".to_string(),
        id: 1,
        method: "eth_estimateUserOperationGas".to_string(),
        params: vec![value, ENTRYPOINT_ADDRESS.to_string().into()],
    };
    println!("req_body: {:?}", serde_json::to_string(&req_body)?);
    let post = reqwest::Client::builder()
        .build()?
        .post(bundler_url.as_str())
        .json(&req_body)
        .send()
        .await?;
    println!("post: {:?}", post);
    let res = post.text().await?;
    println!("res: {:?}", res);
    let v = serde_json::from_str::<Response<EstimateResult>>(&res)?;
    println!("json: {:?}", v);

    let user_op = UserOperation {
        pre_verification_gas: v
            .result
            .pre_verification_gas
            .saturating_add(U256::from(1000)),
        verification_gas_limit: v.result.verification_gas_limit,
        call_gas_limit: v.result.call_gas_limit.saturating_mul(U256::from(2)),
        max_priority_fee_per_gas: priority_fee,
        max_fee_per_gas: gas_price,
        ..user_op
    };
    let user_op = uo_wallet
        .sign_uo(
            &user_op,
            &ENTRYPOINT_ADDRESS.to_string().parse::<Address>()?,
            &U256::from(5),
        )
        .await?;

    let send_body = Request {
        jsonrpc: "2.0".to_string(),
        id: 1,
        method: "eth_sendUserOperation".to_string(),
        params: vec![
            serde_json::to_value(&user_op).unwrap(),
            ENTRYPOINT_ADDRESS.to_string().into(),
        ],
    };
    let post = reqwest::Client::builder()
        .build()?
        .post(bundler_url)
        .json(&send_body)
        .send()
        .await?;

    println!("post: {:?}", post);
    let res = post.text().await?;
    println!("res: {:?}", res);

    Ok(())
}

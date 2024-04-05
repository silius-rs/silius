use ethers::{
    prelude::{MiddlewareBuilder, SignerMiddleware},
    providers::{Http, Middleware, Provider},
    signers::{coins_bip39::English, MnemonicBuilder, Signer},
    types::{transaction::eip2718::TypedTransaction, Address, Bytes, U256},
};
use examples_simple_account::{
    simple_account::SimpleAccountExecute, EstimateResult, Request, Response,
};
use reqwest;
use silius_primitives::{constants::entry_point::ADDRESS, UserOperationSigned, Wallet as UoWallet};
use silius_tests::common::gen::SimpleAccountFactory;
use std::{env, sync::Arc, time::Duration};

// stackup simple account factory
const SIMPLE_ACCOUNT_FACTORY: &str = "0x9406Cc6185a346906296840746125a0E44976454";
const CREATE_INDEX: u64 = 2;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    if let Ok(bundler_url) = env::var("BUNDLER_URL") {
        let seed_phrase = env::var("SEED_PHRASE").unwrap();

        let provider = Provider::<Http>::try_from(bundler_url.as_str())?
            .interval(Duration::from_millis(10u64));
        let chain_id = provider.get_chainid().await?.as_u64();

        let wallet = MnemonicBuilder::<English>::default().phrase(seed_phrase.as_str()).build()?;
        let client =
            SignerMiddleware::new(provider.clone(), wallet.clone().with_chain_id(chain_id))
                .nonce_manager(wallet.address());
        let provider = Arc::new(client);

        let simple_account_factory_address =
            SIMPLE_ACCOUNT_FACTORY.to_string().parse::<Address>()?;
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

        let nonce = provider.get_transaction_count(address, None).await?;
        let call = simple_account_factory.create_account(owner_address, U256::from(CREATE_INDEX));
        let tx: TypedTransaction = call.tx;
        println!("tx: {:?}", tx);
        let mut init_code = Vec::new();
        init_code.extend_from_slice(simple_account_factory_address.as_bytes());
        init_code.extend_from_slice(tx.data().unwrap().to_vec().as_slice());
        println!("init_code: {:?}", init_code);

        let (gas_price, priority_fee) = provider.estimate_eip1559_fees(None).await?;
        println!("gas_price: {:?}, priority_fee: {:?}", gas_price, priority_fee);

        let execution = SimpleAccountExecute::new(Address::zero(), U256::from(0), Bytes::default());
        println!("{:}", Bytes::from(execution.encode()));
        let user_op = UserOperationSigned {
            sender: address,
            nonce,
            init_code: Bytes::from(init_code),
            call_data: Bytes::from(execution.encode()),
            call_gas_limit: U256::from(1),
            verification_gas_limit: U256::from(1000000u64),
            pre_verification_gas: U256::from(1),
            max_fee_per_gas: gas_price,
            max_priority_fee_per_gas: priority_fee,
            paymaster_and_data: Bytes::new(),
            signature: Bytes::default(),
        };
        let uo_wallet = UoWallet::from_phrase(seed_phrase.as_str(), chain_id, false)?;
        let user_op = uo_wallet
            .sign_user_operation(&user_op, &ADDRESS.to_string().parse::<Address>()?, chain_id)
            .await?;

        let value = serde_json::to_value(&user_op.user_operation).unwrap();

        let req_body = Request {
            jsonrpc: "2.0".into(),
            id: 1,
            method: "eth_estimateUserOperationGas".into(),
            params: vec![value, ADDRESS.to_string().into()],
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

        let user_op = UserOperationSigned {
            pre_verification_gas: v.result.pre_verification_gas.saturating_add(U256::from(1000)),
            verification_gas_limit: v.result.verification_gas_limit,
            call_gas_limit: v.result.call_gas_limit.saturating_add(U256::from(2000)),
            max_priority_fee_per_gas: priority_fee,
            max_fee_per_gas: gas_price,
            ..user_op.user_operation
        };
        let user_op = uo_wallet
            .sign_user_operation(&user_op, &ADDRESS.to_string().parse::<Address>()?, chain_id)
            .await?;

        let send_body = Request {
            jsonrpc: "2.0".into(),
            id: 1,
            method: "eth_sendUserOperation".into(),
            params: vec![
                serde_json::to_value(&user_op.user_operation).unwrap(),
                ADDRESS.to_string().into(),
            ],
        };
        let post = reqwest::Client::builder()
            .build()?
            .post(bundler_url.as_str())
            .json(&send_body)
            .send()
            .await?;

        println!("post: {:?}", post);
        let res = post.text().await?;
        println!("res: {:?}", res);
    }

    Ok(())
}

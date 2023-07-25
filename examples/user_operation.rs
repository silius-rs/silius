use aa_bundler_primitives::{UserOperation, Wallet};
use ethers::types::Address;
use std::str::FromStr;

pub const MNEMONIC_PHRASE: &str = "test test test test test test test test test test test junk";
pub const CHAIN_ID: u64 = 1337;
pub const ENTRY_POINT: &str = "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // create wallet
    let wallet = Wallet::from_phrase(MNEMONIC_PHRASE, &CHAIN_ID.into(), None)?;
    println!("Wallet address: {:?}", wallet.signer);

    // create simple user operation
    let uo = UserOperation::default().verification_gas_limit(50_000.into());
    println!("User operation: {:?}", uo);

    // calculate user operation hash
    let uo_hash = uo.hash(&Address::from_str(ENTRY_POINT).unwrap(), &CHAIN_ID.into());
    println!("User operation hash: {:?}", uo_hash);

    // sign user operation
    let uo_signed = wallet
        .sign_uo(
            &uo,
            &Address::from_str(ENTRY_POINT).unwrap(),
            &CHAIN_ID.into(),
        )
        .await?;
    println!("User operation signed: {:?}", uo_signed);

    Ok(())
}

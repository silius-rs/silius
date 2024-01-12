use ethers::types::Address;
use silius_primitives::{UserOperationSigned, Wallet};
use std::str::FromStr;

pub const MNEMONIC_PHRASE: &str = "test test test test test test test test test test test junk";
pub const CHAIN_ID: u64 = 1337;
pub const ENTRY_POINT: &str = "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789";

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // create wallet
    let wallet = Wallet::from_phrase(MNEMONIC_PHRASE, CHAIN_ID, false)?;
    println!("Wallet address: {:?}", wallet.signer);

    // create simple user operation
    let uo = UserOperationSigned::default().verification_gas_limit(50_000.into());
    println!("User operation: {:?}", uo);

    // calculate user operation hash
    let uo_hash = uo.hash(&Address::from_str(ENTRY_POINT).unwrap(), CHAIN_ID);
    println!("User operation hash: {:?}", uo_hash);

    // sign user operation
    let uo_signed =
        wallet.sign_user_operation(&uo, &Address::from_str(ENTRY_POINT).unwrap(), CHAIN_ID).await?;
    println!("User operation signed: {:?}", uo_signed);

    Ok(())
}

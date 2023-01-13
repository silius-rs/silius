use std::sync::Arc;

use aa_bundler::types::user_operation::UserOperation;
use ethers::{
    prelude::k256::ecdsa::SigningKey,
    providers::Middleware,
    signers::{Signer, Wallet},
    types::{Address, Bytes, U256},
};

use self::gen::{
    EntryPointContract, TestOpcodesAccount, TestOpcodesAccountFactory, TestRecursionAccount,
    TestRulesAccountFactory, TestStorageAccount, TestStorageAccountFactory,
};
pub mod gen;

pub const ANVIL_TEST_KEY_PHRASE: &str =
    "test test test test test test test test test test test junk";

pub struct DeployedContract<C> {
    contract: C,
    pub address: Address,
}
impl<C> DeployedContract<C> {
    pub fn new(contract: C, address: Address) -> Self {
        Self { contract, address }
    }

    pub fn contract(&self) -> &C {
        &self.contract
    }
}

pub async fn deploy_entry_point<M: Middleware + 'static>(
    client: Arc<M>,
) -> anyhow::Result<DeployedContract<EntryPointContract<M>>> {
    let (entry_point, receipt) = EntryPointContract::deploy(client, ())?
        .send_with_receipt()
        .await?;
    let address = receipt.contract_address.unwrap();
    Ok(DeployedContract::new(entry_point, address))
}

pub async fn deploy_test_opcode_account<M: Middleware + 'static>(
    client: Arc<M>,
) -> anyhow::Result<DeployedContract<TestOpcodesAccount<M>>> {
    let (account, receipt) = TestOpcodesAccount::deploy(client, ())?
        .send_with_receipt()
        .await?;
    let address = receipt.contract_address.unwrap();
    Ok(DeployedContract::new(account, address))
}

pub async fn deploy_test_opcode_account_factory<M: Middleware + 'static>(
    client: Arc<M>,
) -> anyhow::Result<DeployedContract<TestOpcodesAccountFactory<M>>> {
    let (factory, receipt) = TestOpcodesAccountFactory::deploy(client, ())?
        .send_with_receipt()
        .await?;
    let address = receipt.contract_address.unwrap();
    Ok(DeployedContract::new(factory, address))
}

pub async fn deploy_test_storage_account_factory<M: Middleware + 'static>(
    client: Arc<M>,
) -> anyhow::Result<DeployedContract<TestStorageAccountFactory<M>>> {
    let (factory, receipt) = TestStorageAccountFactory::deploy(client, ())?
        .send_with_receipt()
        .await?;
    let address = receipt.contract_address.unwrap();
    Ok(DeployedContract::new(factory, address))
}

pub async fn deploy_test_storage_account<M: Middleware + 'static>(
    client: Arc<M>,
) -> anyhow::Result<DeployedContract<TestStorageAccount<M>>> {
    let (account, receipt) = TestStorageAccount::deploy(client, ())?
        .send_with_receipt()
        .await?;
    let address = receipt.contract_address.unwrap();
    Ok(DeployedContract::new(account, address))
}

pub async fn deploy_test_recursion_account<M: Middleware + 'static>(
    client: Arc<M>,
    entry_point_address: Address,
) -> anyhow::Result<DeployedContract<TestRecursionAccount<M>>> {
    let (account, receipt) = TestRecursionAccount::deploy(client, entry_point_address)?
        .send_with_receipt()
        .await?;
    let address = receipt.contract_address.unwrap();
    Ok(DeployedContract::new(account, address))
}

pub async fn deploy_test_rules_account_factory<M: Middleware + 'static>(
    client: Arc<M>,
) -> anyhow::Result<DeployedContract<TestRulesAccountFactory<M>>> {
    let (factory, receipt) = TestRulesAccountFactory::deploy(client, ())?
        .send_with_receipt()
        .await?;
    let address = receipt.contract_address.unwrap();
    Ok(DeployedContract::new(factory, address))
}

pub async fn sign(
    user_op: &mut UserOperation,
    entry_point_address: Address,
    chain_id: U256,
    key: Wallet<SigningKey>,
) -> anyhow::Result<()> {
    let user_op_hash = user_op.hash(entry_point_address, chain_id);
    let signature = key.sign_message(user_op_hash.as_bytes()).await?;
    user_op.signature = Bytes::from(signature.to_vec());
    Ok(())
}

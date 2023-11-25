use crate::{
    mempool::{Mempool, UserOperationAct, UserOperationAddrAct, UserOperationCodeHashAct},
    reputation::{HashSetOp, ReputationEntryOp},
    utils::equal_code_hashes,
    validate::{SimulationTraceCheck, SimulationTraceHelper},
    Reputation,
};
use ethers::{
    providers::Middleware,
    types::{Address, H256},
    utils::keccak256,
};
use silius_primitives::{
    simulation::{CodeHash, SimulationCheckError},
    UserOperation,
};
use std::sync::Arc;
use tokio::task::JoinSet;

#[derive(Clone)]
pub struct CodeHashes;

impl CodeHashes {
    /// The helper function to retrieve code hashes given a list of addresses
    ///
    /// # Arguments
    /// `addrs` - The list of addresses
    /// `hashes` - The list of code hashes
    /// `eth_client` - The Ethereum client
    ///
    /// # Returns
    /// None if code hash is available, otherwise [SimulationCheckError](SimulationCheckError).
    async fn get_code_hashes<M: Middleware + 'static>(
        &self,
        addrs: Vec<Address>,
        hashes: &mut Vec<CodeHash>,
        eth_client: &Arc<M>,
    ) -> Result<(), SimulationCheckError> {
        let mut ts: JoinSet<Option<(Address, H256)>> = JoinSet::new();

        for addr in addrs {
            let eth_client = eth_client.clone();

            ts.spawn(async move {
                match eth_client.get_code(addr, None).await {
                    Ok(code) => Some((addr, keccak256(&code).into())),
                    Err(_) => None,
                }
            });
        }

        while let Some(res) = ts.join_next().await {
            match res {
                Ok(Some(h)) => hashes.push(CodeHash {
                    address: h.0,
                    hash: h.1,
                }),
                Ok(None) | Err(_) => {
                    return Err(SimulationCheckError::UnknownError {
                        message: "Failed to retrieve code hashes".to_string(),
                    });
                }
            }
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl<M: Middleware> SimulationTraceCheck<M> for CodeHashes {
    /// The [check_user_operation] method implementation that checks the code hashes
    ///
    /// # Arguments
    /// `uo` - The user operation to check
    /// `helper` - The [SimulationTraceHelper](SimulationTraceHelper)
    ///
    /// # Returns
    /// None if the check passes, otherwise a [SimulationCheckError] error.
    async fn check_user_operation<T, Y, X, Z, H, R>(
        &self,
        uo: &UserOperation,
        mempool: &Mempool<T, Y, X, Z>,
        _reputation: &Reputation<H, R>,
        helper: &mut SimulationTraceHelper<M>,
    ) -> Result<(), SimulationCheckError>
    where
        T: UserOperationAct,
        Y: UserOperationAddrAct,
        X: UserOperationAddrAct,
        Z: UserOperationCodeHashAct,
        H: HashSetOp,
        R: ReputationEntryOp,
    {
        // [COD-010] - between the first and the second validations, the EXTCODEHASH value of any visited address, entity or referenced library, may not be changed

        let addrs = helper
            .js_trace
            .calls_from_entry_point
            .iter()
            .flat_map(|l| l.contract_size.keys().copied().collect::<Vec<Address>>())
            .collect::<Vec<Address>>();

        let hashes: &mut Vec<CodeHash> = &mut vec![];
        self.get_code_hashes(addrs, hashes, &helper.entry_point.eth_client())
            .await?;

        let uo_hash = uo.hash(&helper.entry_point.address(), &helper.chain.id().into());

        match mempool.has_code_hashes(&uo_hash) {
            Ok(true) => {
                // 2nd simulation
                let hashes_prev = mempool.get_code_hashes(&uo_hash).map_err(|err| {
                    SimulationCheckError::UnknownError {
                        message: format!("{err:?}"),
                    }
                })?;
                if !equal_code_hashes(hashes, &hashes_prev) {
                    return Err(SimulationCheckError::CodeHashes {
                        message: "Modified code hashes after 1st simulation".to_string(),
                    });
                } else {
                    helper.code_hashes = Some(hashes.to_vec());
                }
            }
            Ok(false) => {
                // 1st simulation
                helper.code_hashes = Some(hashes.to_vec());
            }
            Err(err) => {
                return Err(SimulationCheckError::UnknownError {
                    message: format!("{err:?}"),
                })
            }
        }

        Ok(())
    }
}

use crate::{
    mempool::Mempool,
    utils::equal_code_hashes,
    validate::{SimulationTraceCheck, SimulationTraceHelper},
    Reputation, SimulationError,
};
use ethers::{
    providers::Middleware,
    types::{Address, H256},
    utils::keccak256,
};
use silius_primitives::{simulation::CodeHash, UserOperation};
use std::sync::Arc;
use tokio::task::JoinSet;
use tracing::debug;

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
    /// None if code hash is available, otherwise [SimulationError](SimulationError).
    async fn get_code_hashes<M: Middleware + 'static>(
        &self,
        addrs: Vec<Address>,
        hashes: &mut Vec<CodeHash>,
        eth_client: &Arc<M>,
    ) -> Result<(), SimulationError> {
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
                Ok(Some(h)) => hashes.push(CodeHash { address: h.0, hash: h.1 }),
                Ok(None) | Err(_) => {
                    return Err(SimulationError::Other {
                        inner: "Failed to retrieve code hashes".into(),
                    });
                }
            }
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl<M: Middleware> SimulationTraceCheck<M> for CodeHashes {
    /// The method implementation that checks the code hashes.
    ///
    /// # Arguments
    /// `uo` - The user operation to check
    /// `helper` - The [SimulationTraceHelper](SimulationTraceHelper)
    ///
    /// # Returns
    /// None if the check passes, otherwise a [SimulationError] error.
    async fn check_user_operation(
        &self,
        uo: &UserOperation,
        mempool: &Mempool,
        _reputation: &Reputation,
        helper: &mut SimulationTraceHelper<M>,
    ) -> Result<(), SimulationError> {
        // [COD-010] - between the first and the second validations, the EXTCODEHASH value of any
        // visited address, entity or referenced library, may not be changed

        let addrs = helper
            .js_trace
            .calls_from_entry_point
            .iter()
            .flat_map(|l| l.contract_size.keys().copied().collect::<Vec<Address>>())
            .collect::<Vec<Address>>();

        let hashes: &mut Vec<CodeHash> = &mut vec![];
        self.get_code_hashes(addrs, hashes, &helper.entry_point.eth_client()).await?;

        match mempool.has_code_hashes(&uo.hash) {
            Ok(true) => {
                // 2nd simulation
                let hashes_prev = mempool
                    .get_code_hashes(&uo.hash)
                    .map_err(|err| SimulationError::Other { inner: err.to_string() })?;
                debug!(
                    "Veryfing {:?} code hashes in 2nd simulation: {:?} vs {:?}",
                    uo.hash, hashes, hashes_prev
                );
                if !equal_code_hashes(hashes, &hashes_prev) {
                    return Err(SimulationError::CodeHashes {});
                } else {
                    helper.code_hashes = Some(hashes.to_vec());
                }
            }
            Ok(false) => {
                // 1st simulation
                debug!("Setting code hashes in 1st simulation: {:?}", hashes);
                helper.code_hashes = Some(hashes.to_vec());
            }
            Err(err) => return Err(SimulationError::Other { inner: err.to_string() }),
        }

        Ok(())
    }
}

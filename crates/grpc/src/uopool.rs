use crate::proto::uopool::*;
use crate::{
    proto::types::{GetChainIdResponse, GetSupportedEntryPointsResponse},
    utils::{parse_addr, parse_hash, parse_uo},
};
use async_trait::async_trait;
use dashmap::DashMap;
use discv5::Enr;
use ethers::{
    providers::Middleware,
    types::{Address, U256},
};
use expanded_pathbuf::ExpandedPathBuf;
use eyre::Result;
use futures::channel::mpsc::unbounded;
use futures::StreamExt;
use libp2p_identity::Keypair;
use silius_p2p::config::Config;
use silius_p2p::network::{EntrypointChannels, Network};
use silius_primitives::consts::p2p::DB_FOLDER_NAME;
use silius_primitives::provider::BlockStream;
use silius_primitives::reputation::ReputationEntry;
use silius_primitives::UserOperation;
use silius_primitives::{uopool::AddError, Chain, UoPoolMode};
use silius_uopool::{
    init_env, DBError, DatabaseMempool, DatabaseReputation, Mempool, VecCh, VecUo, WriteMap,
};
use silius_uopool::{
    mempool_id, validate::validator::StandardUserOperationValidator, MempoolId, Reputation,
    UoPool as UserOperationPool, UoPoolBuilder,
};
use std::fmt::{Debug, Display};
use std::os::unix::prelude::PermissionsExt;
use std::path::PathBuf;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tonic::{Code, Request, Response, Status};
use tracing::{error, info};

type StandardUserPool<M, P, R, E> =
    UserOperationPool<M, StandardUserOperationValidator<M, P, R, E>, P, R, E>;

pub struct UoPoolService<M, P, R, E>
where
    M: Middleware + Clone + 'static,
    P: Mempool<UserOperations = VecUo, CodeHashes = VecCh, Error = E> + Send + Sync,
    R: Reputation<ReputationEntries = Vec<ReputationEntry>, Error = E> + Send + Sync,
{
    pub uopools: Arc<DashMap<MempoolId, UoPoolBuilder<M, P, R, E>>>,
    pub chain: Chain,
}

impl<M, P, R, E> UoPoolService<M, P, R, E>
where
    M: Middleware + Clone + 'static,
    P: Mempool<UserOperations = VecUo, CodeHashes = VecCh, Error = E> + Send + Sync + 'static,
    R: Reputation<ReputationEntries = Vec<ReputationEntry>, Error = E> + Send + Sync + 'static,
    E: Debug + Display + 'static,
{
    pub fn new(uopools: Arc<DashMap<MempoolId, UoPoolBuilder<M, P, R, E>>>, chain: Chain) -> Self {
        Self { uopools, chain }
    }

    fn get_uopool(&self, ep: &Address) -> tonic::Result<StandardUserPool<M, P, R, E>> {
        let m_id = mempool_id(ep, &U256::from(self.chain.id()));
        self.uopools
            .get(&m_id)
            .map(|b| b.uopool())
            .ok_or(Status::new(
                Code::Unavailable,
                "User operation pool is not available",
            ))
    }
}

#[async_trait]
impl<M, P, R, E> uo_pool_server::UoPool for UoPoolService<M, P, R, E>
where
    M: Middleware + Clone + 'static,
    P: Mempool<UserOperations = VecUo, CodeHashes = VecCh, Error = E> + Send + Sync + 'static,
    R: Reputation<ReputationEntries = Vec<ReputationEntry>, Error = E> + Send + Sync + 'static,
    E: Debug + Display + 'static,
{
    async fn add(&self, req: Request<AddRequest>) -> Result<Response<AddResponse>, Status> {
        let req = req.into_inner();

        let uo = parse_uo(req.uo)?;
        let ep = parse_addr(req.ep)?;

        let res = {
            let uopool = self.get_uopool(&ep)?;
            uopool.validate_user_operation(&uo).await
        };

        let mut uopool = self.get_uopool(&ep)?;

        match uopool.add_user_operation(uo, res).await {
            Ok(uo_hash) => Ok(Response::new(AddResponse {
                res: AddResult::Added as i32,
                data: serde_json::to_string(&uo_hash)
                    .map_err(|err| Status::internal(format!("Failed to serialize hash: {err}")))?,
            })),
            Err(err) => match err {
                AddError::Verification(err) => Ok(Response::new(AddResponse {
                    res: AddResult::NotAdded as i32,
                    data: serde_json::to_string(&err).map_err(|err| {
                        Status::internal(format!("Failed to serialize error: {err}"))
                    })?,
                })),
                AddError::MempoolError { message } => {
                    Err(Status::internal(format!("Internal error: {message}")))
                }
            },
        }
    }

    async fn remove(&self, req: Request<RemoveRequest>) -> Result<Response<()>, Status> {
        let req = req.into_inner();

        let ep = parse_addr(req.ep)?;
        let mut uopool = self.get_uopool(&ep)?;

        uopool.remove_user_operations(req.hashes.into_iter().map(Into::into).collect());

        Ok(Response::new(()))
    }

    async fn get_chain_id(
        &self,
        _req: Request<()>,
    ) -> Result<Response<GetChainIdResponse>, Status> {
        Ok(Response::new(GetChainIdResponse {
            chain_id: self.chain.id(),
        }))
    }

    async fn get_supported_entry_points(
        &self,
        _req: Request<()>,
    ) -> Result<Response<GetSupportedEntryPointsResponse>, Status> {
        Ok(Response::new(GetSupportedEntryPointsResponse {
            eps: self
                .uopools
                .iter()
                .map(|mempool| mempool.uopool().entry_point_address().into())
                .collect(),
        }))
    }

    async fn estimate_user_operation_gas(
        &self,
        req: Request<EstimateUserOperationGasRequest>,
    ) -> Result<Response<EstimateUserOperationGasResponse>, Status> {
        let req = req.into_inner();

        let uo = parse_uo(req.uo)?;
        let ep = parse_addr(req.ep)?;

        let uopool = self.get_uopool(&ep)?;

        Ok(Response::new(
            match uopool.estimate_user_operation_gas(&uo).await {
                Ok(gas) => EstimateUserOperationGasResponse {
                    res: EstimateUserOperationGasResult::Estimated as i32,
                    data: serde_json::to_string(&gas).map_err(|err| {
                        Status::internal(format!("Failed to serialize gas: {err}"))
                    })?,
                },
                Err(err) => EstimateUserOperationGasResponse {
                    res: EstimateUserOperationGasResult::NotEstimated as i32,
                    data: serde_json::to_string(&err).map_err(|err| {
                        Status::internal(format!("Failed to serialize error: {err}"))
                    })?,
                },
            },
        ))
    }

    async fn get_sorted_user_operations(
        &self,
        req: Request<GetSortedRequest>,
    ) -> Result<Response<GetSortedResponse>, Status> {
        let req = req.into_inner();

        let ep = parse_addr(req.ep)?;

        let uos = {
            let uopool = self.get_uopool(&ep)?;
            uopool.get_sorted_user_operations().map_err(|e| {
                tonic::Status::internal(format!("Get sorted uos internal error: {e}"))
            })?
        };

        let uos_valid = {
            let mut uopool = self.get_uopool(&ep)?;
            uopool
                .bundle_user_operations(uos)
                .await
                .map_err(|e| tonic::Status::internal(format!("Bundle uos internal error: {e}")))?
        };

        Ok(Response::new(GetSortedResponse {
            uos: uos_valid.into_iter().map(Into::into).collect(),
        }))
    }

    async fn get_user_operation_by_hash(
        &self,
        req: Request<UserOperationHashRequest>,
    ) -> Result<Response<GetUserOperationByHashResponse>, Status> {
        let req = req.into_inner();

        let uo_hash = parse_hash(req.hash)?;

        for uopool in self.uopools.iter() {
            if let Ok(uo_by_hash) = uopool
                .uopool()
                .get_user_operation_by_hash(&uo_hash.into())
                .await
            {
                return Ok(Response::new(GetUserOperationByHashResponse {
                    user_operation: Some(uo_by_hash.user_operation.into()),
                    entry_point: Some(uo_by_hash.entry_point.into()),
                    transaction_hash: Some(uo_by_hash.transaction_hash.into()),
                    block_hash: Some(uo_by_hash.block_hash.into()),
                    block_number: uo_by_hash.block_number.as_u64(),
                }));
            }
        }

        Err(tonic::Status::not_found("User operation not found"))
    }

    async fn get_user_operation_receipt(
        &self,
        req: Request<UserOperationHashRequest>,
    ) -> Result<Response<GetUserOperationReceiptResponse>, Status> {
        let req = req.into_inner();

        let uo_hash = parse_hash(req.hash)?;

        for uopool in self.uopools.iter() {
            if let Ok(uo_receipt) = uopool
                .uopool()
                .get_user_operation_receipt(&uo_hash.into())
                .await
            {
                return Ok(Response::new(GetUserOperationReceiptResponse {
                    user_operation_hash: Some(uo_receipt.user_operation_hash.into()),
                    sender: Some(uo_receipt.sender.into()),
                    nonce: Some(uo_receipt.nonce.into()),
                    actual_gas_cost: Some(uo_receipt.actual_gas_cost.into()),
                    actual_gas_used: Some(uo_receipt.actual_gas_used.into()),
                    success: uo_receipt.success,
                    tx_receipt: Some(uo_receipt.tx_receipt.into()),
                    logs: uo_receipt.logs.into_iter().map(|log| log.into()).collect(),
                    paymaster: uo_receipt.paymaster.map(|p| p.into()),
                    reason: uo_receipt.reason,
                }));
            }
        }

        Err(tonic::Status::not_found("User operation receipt not found"))
    }

    async fn get_all(
        &self,
        req: Request<GetAllRequest>,
    ) -> Result<Response<GetAllResponse>, Status> {
        let req = req.into_inner();

        let ep = parse_addr(req.ep)?;
        let uopool = self.get_uopool(&ep)?;

        Ok(Response::new(GetAllResponse {
            uos: uopool.get_all().into_iter().map(Into::into).collect(),
        }))
    }

    async fn clear_mempool(&self, _req: Request<()>) -> Result<Response<()>, Status> {
        self.uopools.iter_mut().for_each(|uopool| {
            uopool.uopool().clear_mempool();
        });
        Ok(Response::new(()))
    }

    async fn clear_reputation(&self, _req: Request<()>) -> Result<Response<()>, Status> {
        self.uopools.iter_mut().for_each(|uopool| {
            uopool.uopool().clear_reputation();
        });
        Ok(Response::new(()))
    }

    async fn clear(&self, _req: Request<()>) -> Result<Response<()>, Status> {
        self.uopools.iter_mut().for_each(|uopool| {
            uopool.uopool().clear();
        });
        Ok(Response::new(()))
    }

    async fn get_all_reputation(
        &self,
        req: Request<GetAllReputationRequest>,
    ) -> Result<Response<GetAllReputationResponse>, Status> {
        let req = req.into_inner();

        let ep = parse_addr(req.ep)?;
        let uopool = self.get_uopool(&ep)?;

        Ok(Response::new(GetAllReputationResponse {
            rep: uopool
                .get_reputation()
                .into_iter()
                .map(Into::into)
                .collect(),
        }))
    }

    async fn set_reputation(
        &self,
        req: Request<SetReputationRequest>,
    ) -> Result<Response<SetReputationResponse>, Status> {
        let req = req.into_inner();

        let ep = parse_addr(req.ep)?;
        let mut uopool = self.get_uopool(&ep)?;

        let res = Response::new(SetReputationResponse {
            res: match uopool.set_reputation(req.rep.iter().map(|re| re.clone().into()).collect()) {
                Ok(_) => SetReputationResult::SetReputation as i32,
                Err(_) => SetReputationResult::NotSetReputation as i32,
            },
        });

        Ok(res)
    }

    async fn get_stake_info(
        &self,
        req: Request<GetStakeInfoRequest>,
    ) -> Result<Response<GetStakeInfoResponse>, Status> {
        let req = req.into_inner();

        let ep = parse_addr(req.ep)?;
        let addr = parse_addr(req.addr)?;
        let uopool = self.get_uopool(&ep)?;

        let res = uopool
            .get_stake_info(&addr)
            .await
            .map_err(|e| tonic::Status::internal(format!("Get stake info internal error: {e}")))?;
        Ok(Response::new(GetStakeInfoResponse {
            info: Some(res.stake_info.into()),
            is_staked: res.is_staked,
        }))
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn uopool_service_run<M>(
    addr: SocketAddr,
    datadir: ExpandedPathBuf,
    eps: Vec<Address>,
    eth_client: Arc<M>,
    block_streams: Vec<BlockStream>,
    chain: Chain,
    max_verification_gas: U256,
    min_stake: U256,
    min_priority_fee_per_gas: U256,
    whitelist: Vec<Address>,
    upool_mode: UoPoolMode,
    p2p_enabled: bool,
    node_key_file: PathBuf,
    config: Config,
    bootnodes: Vec<Enr>,
) -> Result<()>
where
    M: Middleware + Clone + 'static,
{
    tokio::spawn(async move {
        let mut builder = tonic::transport::Server::builder();

        let m_map = Arc::new(DashMap::<
            MempoolId,
            UoPoolBuilder<M, DatabaseMempool<WriteMap>, DatabaseReputation<WriteMap>, DBError>,
        >::new());

        let env =
            Arc::new(init_env::<WriteMap>(datadir.join(DB_FOLDER_NAME)).expect("Init mdbx failed"));
        env.create_tables()
            .expect("Create mdbx database tables failed");

        if p2p_enabled {
            let mut entrypoint_channels: EntrypointChannels = Vec::new();

            for (ep, block_stream) in eps.into_iter().zip(block_streams.into_iter()) {
                let id = mempool_id(&ep, &U256::from(chain.id()));
                let (waiting_to_pub_sd, waiting_to_pub_rv) = unbounded::<(UserOperation, U256)>();
                let uo_builder = UoPoolBuilder::new(
                    upool_mode == UoPoolMode::Unsafe,
                    eth_client.clone(),
                    ep,
                    chain,
                    max_verification_gas,
                    min_stake,
                    min_priority_fee_per_gas,
                    whitelist.clone(),
                    DatabaseMempool::new(env.clone()),
                    DatabaseReputation::new(env.clone()),
                    Some(waiting_to_pub_sd),
                );
                uo_builder.register_block_updates(block_stream);
                uo_builder.register_reputation_updates();

                let (p2p_userop_sd, mut p2p_userop_rv) = unbounded::<UserOperation>();
                let mut uo_pool = uo_builder.uopool();
                // spawn a task which would consume the userop received from p2p network
                tokio::spawn(async move {
                    while let Some(user_op) = p2p_userop_rv.next().await {
                        let res = uo_pool.validate_user_operation(&user_op).await;
                        match uo_pool.add_user_operation(user_op, res).await {
                            Ok(_) => {}
                            Err(e) => error!("Failed to add user operation: {:?} from p2p", e),
                        }
                    }
                });
                m_map.insert(id, uo_builder);
                entrypoint_channels.push((chain, ep, waiting_to_pub_rv, p2p_userop_sd))
            }

            let discovery_secret = if node_key_file.exists() {
                let content =
                    std::fs::read(node_key_file).expect("discovery secret file currupted");
                Keypair::from_protobuf_encoding(&content).expect("discovery secret file currupted")
            } else {
                info!("The p2p spec private key is not exist. Creating one now!");
                let keypair = Keypair::generate_secp256k1();
                std::fs::write(
                    node_key_file.clone(),
                    keypair
                        .to_protobuf_encoding()
                        .expect("discovery secret encode failed"),
                )
                .expect("write discoveray secret file failed");
                std::fs::set_permissions(node_key_file, std::fs::Permissions::from_mode(0o600))
                    .expect("Setting key file permission failed");
                keypair
            };
            let listen_addrs = config.listen_addr.to_multi_addr();
            let mut p2p_network = Network::new(
                discovery_secret,
                config,
                entrypoint_channels,
                Duration::from_secs(10),
                30,
            )
            .expect("p2p network init failed");
            info!("Enr: {}", p2p_network.local_enr().to_base64());
            for listen_addr in listen_addrs.into_iter() {
                info!("P2P node listened on {}", listen_addr);
                p2p_network
                    .listen_on(listen_addr)
                    .expect("Listen on p2p network failed");
            }

            if bootnodes.is_empty() {
                info!("Start p2p mode without bootnodes");
            }
            for enr in bootnodes {
                info!("Trying to dial p2p node {enr:}");
                p2p_network.dial(enr).expect("Dial bootnode failed");
            }

            tokio::spawn(async move {
                loop {
                    p2p_network.next_event().await;
                }
            });
        } else {
            for (ep, block_stream) in eps.into_iter().zip(block_streams.into_iter()) {
                let id = mempool_id(&ep, &U256::from(chain.id()));
                let uo_builder = UoPoolBuilder::new(
                    upool_mode == UoPoolMode::Unsafe,
                    eth_client.clone(),
                    ep,
                    chain,
                    max_verification_gas,
                    min_stake,
                    min_priority_fee_per_gas,
                    whitelist.clone(),
                    DatabaseMempool::new(env.clone()),
                    DatabaseReputation::new(env.clone()),
                    None,
                );
                uo_builder.register_block_updates(block_stream);
                uo_builder.register_reputation_updates();
                m_map.insert(id, uo_builder);
            }
        };
        let svc = uo_pool_server::UoPoolServer::new(UoPoolService::<
            M,
            DatabaseMempool<WriteMap>,
            DatabaseReputation<WriteMap>,
            DBError,
        >::new(m_map.clone(), chain));

        builder.add_service(svc).serve(addr).await
    });

    tokio::time::sleep(Duration::from_secs(1)).await;

    Ok(())
}

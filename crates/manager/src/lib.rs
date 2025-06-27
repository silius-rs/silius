use std::{sync::Arc, time::Duration};

use alloy_primitives::TxHash;
use alloy_provider::Provider;
use silius_builder::Builder;
use silius_chain::Chain;
use silius_mempool::Mempool;
use tokio::{
    sync::{Mutex, mpsc},
    time::interval,
};
use tracing::{info, trace};

pub struct SiliusManager<P: Provider + 'static> {
    pub builder: Mutex<Builder<P>>,
    pub mempool: Arc<Mempool<P>>,
    pub chain: Arc<Chain<P>>,
    network_sender: mpsc::UnboundedSender<()>,
}

impl<P: Provider + 'static> SiliusManager<P> {
    pub async fn new(
        builder: Builder<P>,
        mempool: Arc<Mempool<P>>,
        chain: Arc<Chain<P>>,
        network_sender: mpsc::UnboundedSender<()>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            builder: Mutex::new(builder),
            mempool,
            chain,
            network_sender,
        })
    }

    pub async fn start_builder(&self) -> anyhow::Result<()> {
        let mut builder = self.builder.lock().await;
        builder.start();
        Ok(())
    }

    pub async fn stop_builder(&self) -> anyhow::Result<()> {
        let mut builder = self.builder.lock().await;
        builder.stop();
        Ok(())
    }

    pub async fn submit_bundle(&self) -> anyhow::Result<TxHash> {
        let builder = self.builder.lock().await;
        if builder.is_active {
            let user_operations = self.mempool.get_user_operations()?;
            return if !user_operations.is_empty() {
                let bundle = builder.create_bundle(&user_operations).await?;
                Ok(builder.submit_bundle(bundle).await?)
            } else {
                Err(anyhow::anyhow!("No user operations to submit"))
            };
        }
        Err(anyhow::anyhow!("Builder is not active"))
    }

    pub async fn start(&self, submit_interval: u64, network_receiver: mpsc::UnboundedReceiver<()>) {
        let mut submit_interval = interval(Duration::from_millis(submit_interval));
        let _ = self.start_builder().await;
        loop {
            tokio::select! {
                _ = submit_interval.tick() => {
                    match self.submit_bundle().await {
                        Ok(tx_hash) => {
                            info!("Submitted bundle: {:?}", tx_hash);
                        }
                        Err(e) => {
                            trace!("Failed to submit bundle: {}", e);
                        }
                    }
                }
            }
        }
    }
}

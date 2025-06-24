use std::sync::Arc;

use alloy_provider::Provider;
use silius_builder::Builder;
use silius_chain::Chain;
use silius_mempool::Mempool;
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct SiliusManager<P: Provider + 'static> {
    pub builder: Builder<P>,
    pub mempool: Arc<Mempool>,
    pub chain: Arc<Chain<P>>,
    network_sender: mpsc::UnboundedSender<()>,
}

impl<P: Provider + 'static> SiliusManager<P> {
    pub async fn new(
        builder: Builder<P>,
        mempool: Arc<Mempool>,
        chain: Arc<Chain<P>>,
        network_sender: mpsc::UnboundedSender<()>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            builder,
            mempool,
            chain,
            network_sender,
        })
    }

    pub async fn start(&self, network_receiver: mpsc::UnboundedReceiver<()>) {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        }
    }
}

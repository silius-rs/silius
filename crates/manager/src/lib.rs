use std::sync::Arc;

use alloy_provider::Provider;
use silius_builder::Builder;
use silius_chain::Chain;
use silius_mempool::Mempool;
use tokio::sync::mpsc;

pub struct SiliusManager<P: Provider + 'static> {
    pub builder: Arc<dyn Builder>,
    pub mempool: Arc<Mempool>,
    pub chain: Arc<Chain<P>>,
    network_sender: mpsc::UnboundedSender<()>,
    network_receiver: mpsc::UnboundedReceiver<()>,
}

impl<P: Provider + 'static> SiliusManager<P> {
    pub async fn new(
        builder: Arc<dyn Builder>,
        mempool: Arc<Mempool>,
        chain: Arc<Chain<P>>,
        network_sender: mpsc::UnboundedSender<()>,
        network_receiver: mpsc::UnboundedReceiver<()>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            builder,
            mempool,
            chain,
            network_sender,
            network_receiver,
        })
    }

    pub async fn start(&self) {}
}

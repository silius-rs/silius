use std::sync::{Arc, mpsc};

use silius_builder::Builder;
use silius_mempool::Mempool;

pub struct SiliusManager<B: Builder> {
    pub builder: Arc<B>,
    pub mempool: Arc<Mempool>,
    network_sender: mpsc::Sender<()>,
    network_receiver: mpsc::Receiver<()>,
}

impl<B: Builder> SiliusManager<B> {
    pub async fn new(
        builder: Arc<B>,
        mempool: Arc<Mempool>,
        network_sender: mpsc::Sender<()>,
        network_receiver: mpsc::Receiver<()>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            builder,
            mempool,
            network_sender,
            network_receiver,
        })
    }
}

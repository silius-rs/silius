//! Utils for creating ethers providers

use async_stream::stream;
use ethers::{
    providers::{Http, Middleware, Provider, Ws},
    types::{Chain, H256},
};
use futures_util::{Stream, StreamExt};
use std::{pin::Pin, sync::Arc, time::Duration};

pub type BlockStream = Pin<Box<dyn Stream<Item = eyre::Result<H256>> + Send>>;

/// Creates ethers provider with HTTP connection
pub async fn create_http_provider(addr: &str) -> eyre::Result<Provider<Http>> {
    let provider = Provider::<Http>::try_from(addr)?;

    let chain_id = provider.get_chainid().await?;

    Ok(provider.interval(if chain_id == Chain::Dev.into() {
        Duration::from_millis(5u64)
    } else {
        Duration::from_millis(500u64)
    }))
}

/// Creates ethers provider with WebSockets connection
pub async fn create_ws_provider(addr: &str) -> eyre::Result<Provider<Ws>> {
    let provider = Provider::<Ws>::connect_with_reconnects(addr, usize::MAX).await?;
    Ok(provider)
}

/// Listens for new blocks over HTTP connection
pub async fn create_http_block_stream(provider: Arc<Provider<Http>>) -> BlockStream {
    Box::pin(stream! {
        let mut stream = provider.watch_blocks().await?.stream();
        while let Some(hash) = stream.next().await {
            yield Ok(hash);
        }
    })
}

/// Create multiple HTTP block streams
pub async fn create_http_block_streams(
    provider: Arc<Provider<Http>>,
    n: usize,
) -> Vec<BlockStream> {
    let mut streams = Vec::new();
    for _ in 0..n {
        streams.push(create_http_block_stream(provider.clone()).await);
    }
    streams
}

/// Listens for new block over WS connection
pub async fn create_ws_block_stream(provider: Arc<Provider<Ws>>) -> BlockStream {
    Box::pin(stream! {
        let mut stream = provider.subscribe_blocks().await?;
        while let Some(block) = stream.next().await {
            if let Some(hash) = block.hash {
                yield Ok(hash);
            }
        }
    })
}

/// Creates multiple WS block streams
pub async fn create_ws_block_streams(provider: Arc<Provider<Ws>>, n: usize) -> Vec<BlockStream> {
    let mut streams = Vec::new();
    for _ in 0..n {
        streams.push(create_ws_block_stream(provider.clone()).await);
    }
    streams
}

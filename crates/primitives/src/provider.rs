//! Utils for creating ethers providers

use async_stream::stream;
use ethers::{
    providers::{Http, Middleware, Provider, PubsubClient, Ws},
    types::H256,
};
use futures_util::{Stream, StreamExt};
use std::{pin::Pin, sync::Arc, time::Duration};

pub type BlockStream = Pin<Box<dyn Stream<Item = eyre::Result<H256>> + Send>>;

/// Creates ethers provider with HTTP connection
pub async fn create_http_provider(
    addr: &str,
    poll_interval: Duration,
) -> eyre::Result<Provider<Http>> {
    let provider = Provider::<Http>::try_from(addr)?;

    Ok(provider.interval(poll_interval))
}

/// Creates ethers provider with WebSockets connection
pub async fn create_ws_provider(addr: &str) -> eyre::Result<Provider<Ws>> {
    let provider = Provider::<Ws>::connect_with_reconnects(addr, usize::MAX).await?;
    Ok(provider)
}

/// Listens for new blocks over HTTP connection
pub async fn create_http_block_stream<M: Middleware + 'static>(provider: Arc<M>) -> BlockStream {
    Box::pin(stream! {
        let mut stream = provider.watch_blocks().await?.stream();
        while let Some(hash) = stream.next().await {
            yield Ok(hash);
        }
    })
}

/// Create multiple HTTP block streams
pub async fn create_http_block_streams<M: Middleware + 'static>(
    provider: Arc<M>,
    n: usize,
) -> Vec<BlockStream> {
    let mut streams = Vec::new();
    for _ in 0..n {
        streams.push(create_http_block_stream(provider.clone()).await);
    }
    streams
}

/// Listens for new block over WS connection
pub async fn create_ws_block_stream<M: Middleware + 'static>(provider: Arc<M>) -> BlockStream
where
    <M as Middleware>::Provider: PubsubClient,
{
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
pub async fn create_ws_block_streams<M: Middleware + 'static>(
    provider: Arc<M>,
    n: usize,
) -> Vec<BlockStream>
where
    <M as Middleware>::Provider: PubsubClient,
{
    let mut streams = Vec::new();
    for _ in 0..n {
        streams.push(create_ws_block_stream(provider.clone()).await);
    }
    streams
}

use std::sync::Arc;

use alloy::{primitives::Address, providers::RootProvider, transports::BoxTransport};

#[derive(Debug, Clone)]
pub(crate) struct ChainState {
    pub(crate) multicall_address: Address,
    pub(crate) provider: Arc<RootProvider<BoxTransport>>,
}

#[derive(Debug, Clone)]
pub(crate) struct AppState {
    // pub(crate) mainnet: Arc<ChainState>,
    // pub(crate) base: Arc<ChainState>,
    // pub(crate) arbitrum: Arc<ChainState>,
    // pub(crate) optimism: Arc<ChainState>,
    // pub(crate) polygon: Arc<ChainState>,
    // pub(crate) bsc: Arc<ChainState>,
    pub(crate) mainnet: ChainState,
    pub(crate) base: ChainState,
    pub(crate) arbitrum: ChainState,
    pub(crate) optimism: ChainState,
    pub(crate) polygon: ChainState,
    pub(crate) bsc: ChainState,
}

use std::sync::Arc;

use alloy::{providers::RootProvider, transports::BoxTransport};

#[derive(Debug, Clone)]
pub(crate) struct AppState {
    pub(crate) mainnet: Arc<RootProvider<BoxTransport>>,
    pub(crate) base: Arc<RootProvider<BoxTransport>>,
    pub(crate) arbitrum: Arc<RootProvider<BoxTransport>>,
    pub(crate) optimism: Arc<RootProvider<BoxTransport>>,
    pub(crate) polygon: Arc<RootProvider<BoxTransport>>,
    pub(crate) bsc: Arc<RootProvider<BoxTransport>>,
}

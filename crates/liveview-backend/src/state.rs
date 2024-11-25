use alloy::{providers::RootProvider, transports::BoxTransport};

#[derive(Debug, Clone)]
pub(crate) struct AppState {
    pub(crate) provider: RootProvider<BoxTransport>,
}

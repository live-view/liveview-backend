use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct Data {
    pub(crate) mainnet: String,
    pub(crate) base: String,
    pub(crate) arbitrum: String,
    pub(crate) optimism: String,
    pub(crate) polygon: String,
    pub(crate) bsc: String,
}

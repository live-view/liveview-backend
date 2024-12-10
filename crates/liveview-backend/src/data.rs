use alloy::primitives::Address;
use serde::Deserialize;
use url::Url;

#[derive(Deserialize)]
pub(crate) enum ChainType {
    Mainnet,
    Base,
    Arbitrum,
    Optimism,
    Polygon,
    Bsc,
}

#[derive(Deserialize)]
pub(crate) struct Chain {
    pub(crate) rpc_url: Url,
    pub(crate) multicall_address: Address,
}

#[derive(Deserialize)]
pub(crate) struct Data {
    pub(crate) mainnet: Chain,
    pub(crate) base: Chain,
    pub(crate) arbitrum: Chain,
    pub(crate) optimism: Chain,
    pub(crate) polygon: Chain,
    pub(crate) bsc: Chain,
}

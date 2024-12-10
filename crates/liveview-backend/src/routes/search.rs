use std::sync::Arc;

use alloy::{
    primitives::{Address, FixedBytes, U256},
    sol_types::SolCall,
};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{ErrorResponse, Result},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{
    data::ChainType,
    interfaces::{Multicall, ERC721},
    state::AppState,
};

#[derive(Deserialize)]
pub(crate) struct SearchQuery {
    pub(crate) chain: ChainType,
    pub(crate) address: Address,
}

#[derive(Serialize)]
pub(crate) struct SuccessData {
    pub(crate) name: String,
    pub(crate) symbol: String,
}

// #[derive(Serialize)]
// struct ErrorData {
//     message: String,
// }

#[axum::debug_handler]
pub(crate) async fn search(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<SuccessData>> {
    let chain_state = match query.chain {
        ChainType::Mainnet => &state.mainnet,
        ChainType::Base => &state.base,
        ChainType::Arbitrum => &state.arbitrum,
        ChainType::Optimism => &state.optimism,
        ChainType::Polygon => &state.polygon,
        ChainType::Bsc => &state.bsc,
    };

    let erc721 = ERC721::new(query.address, Arc::clone(&chain_state.provider));

    let supports_interface = match erc721
        .supportsInterface(FixedBytes(
            [0x80, 0xac, 0x58, 0xcd], /* ERC721.supportsInterface */
        ))
        .call()
        .await
    {
        Ok(res) => res._0,
        Err(_) => false, /* Error means that the address doesn't support the interface */
    };
    if !supports_interface {
        return Err(ErrorResponse::from((
            StatusCode::BAD_REQUEST,
            "Invalid address".to_owned(),
        )));
    }

    let multicall = Multicall::new(
        chain_state.multicall_address,
        Arc::clone(&chain_state.provider),
    );

    let calls = vec![
        Multicall::Call {
            target: query.address,
            gasLimit: U256::MAX,
            callData: erc721.name().calldata().to_owned(),
        },
        Multicall::Call {
            target: query.address,
            gasLimit: U256::MAX,
            callData: erc721.symbol().calldata().to_owned(),
        },
    ];

    let res = match multicall.multicall(calls).call().await {
        Ok(res) => res.returnData,
        Err(_) => {
            return Err(ErrorResponse::from((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to call fetch data".to_owned(),
            )));
        }
    };

    let name = match ERC721::nameCall::abi_decode_returns(&res[0].returnData, false) {
        Ok(decode_res) => decode_res._0,
        Err(_) => {
            return Err(ErrorResponse::from((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to decode name".to_owned(),
            )));
        }
    };
    let symbol = match ERC721::symbolCall::abi_decode_returns(&res[1].returnData, false) {
        Ok(decode_res) => decode_res._0,
        Err(_) => {
            return Err(ErrorResponse::from((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to decode symbol".to_owned(),
            )));
        }
    };

    Ok(Json(SuccessData { name, symbol }))
}

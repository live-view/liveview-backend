use std::sync::Arc;

use alloy::{
    primitives::{Address, FixedBytes, U256},
    providers::Provider,
    sol_types::SolCall,
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use socketioxide::{
    extract::{Data as SocketData, SocketRef, State as SocketState},
    socket::Sid as SocketSid,
};
use tokio::sync::watch;
use tracing::debug;

use crate::{
    interfaces::{Multicall, ERC721},
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub(crate) enum Chain {
    Mainnet,
    Base,
    Arbitrum,
    Optimism,
    Polygon,
    Bsc,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RequestData {
    pub(crate) chain: Chain,
    pub(crate) addresses: Vec<Address>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ResponseData {
    pub(crate) id: SocketSid,
    pub(crate) block_number: u64,
    pub(crate) timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ErrorData {
    pub(crate) id: SocketSid,
    pub(crate) message: String,
}

pub(crate) async fn ws(socket: SocketRef, state: SocketState<Arc<AppState>>) {
    debug!(ns = socket.ns(), ?socket.id, "Socket.IO connected");

    let state = Arc::clone(&state);

    socket.on(
        "request",
        |socket: SocketRef, SocketData::<RequestData>(data)| async move {
            debug!(?data, "Received event");

            // Watch channel for disconnection
            let (tx, mut rx) = watch::channel(());
            let socket_id = socket.id;
            socket.on_disconnect(move || {
                debug!(?socket_id, "Socket disconnected");

                tx.send(()).ok();
            });

            // If there's no addresses
            if data.addresses.is_empty() {
                socket
                    .emit(
                        "error",
                        &ErrorData {
                            id: socket.id,
                            message: "No addresses provided".to_owned(),
                        },
                    )
                    .ok();

                return;
            }

            let chain_state = match data.chain {
                Chain::Mainnet => &state.mainnet,
                Chain::Base => &state.base,
                Chain::Arbitrum => &state.arbitrum,
                Chain::Optimism => &state.optimism,
                Chain::Polygon => &state.polygon,
                Chain::Bsc => &state.bsc,
            };

            // Check if all addresses are correct
            let multicall = Multicall::new(
                chain_state.multicall_address,
                Arc::clone(&chain_state.provider),
            );
            let erc721 = ERC721::new(
                chain_state.multicall_address,
                Arc::clone(&chain_state.provider),
            );

            let calls = data
                .addresses
                .into_iter()
                .map(|address| Multicall::Call {
                    target: address,
                    gasLimit: U256::MAX,
                    callData: erc721
                        .supportsInterface(FixedBytes(
                            [0x80, 0xac, 0x58, 0xcd], /* ERC721.supportsInterface */
                        ))
                        .calldata()
                        .to_owned(),
                })
                .collect::<Vec<_>>();

            // Check all addresses for support of ERC721.supportsInterface in multicall
            let multicall_res = match multicall.multicall(calls).call().await {
                Ok(res) => res.returnData,
                Err(_) => {
                    let message = "Failed to call fetch data".to_string();

                    let response_data = ErrorData {
                        id: socket.id,
                        message,
                    };
                    socket.emit("error", &response_data).ok();

                    return;
                }
            };

            // Check if all addresses support the interface
            for r in multicall_res {
                let decode_res =
                    match ERC721::supportsInterfaceCall::abi_decode_returns(&r.returnData, false) {
                        Ok(res) => res._0,
                        Err(_) => false, // Error means that the address doesn't support the interface
                    };

                if !decode_res {
                    socket
                        .emit(
                            "error",
                            &ErrorData {
                                id: socket.id,
                                message: "Invalid address provided".to_owned(),
                            },
                        )
                        .ok();

                    return;
                }
            }

            // Create a subscription to blocks
            let sub = match chain_state.provider.subscribe_blocks().await {
                Ok(sub) => sub,
                Err(_) => {
                    let message = "Failed to subscribe to blocks".to_owned();

                    let response_data = ErrorData {
                        id: socket.id,
                        message,
                    };
                    socket.emit("error", &response_data).ok();

                    return;
                }
            };

            // Convert the subscription into a stream
            let mut stream = sub.into_stream();

            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        biased; // Check for task cancellation first

                        _ = rx.changed() => {
                            debug!(?socket.id, "Task cancelled");

                            // Break the loop when the task is cancelled
                            break;
                        },
                        Some(block) = stream.next() => {
                             let response_data = ResponseData {
                                id: socket.id,
                                block_number: block.number,
                                timestamp: chrono::Utc::now(),
                            };

                           socket.emit("response", &response_data).ok();
                        },
                        else => break, // Break the loop when the stream is closed
                    }
                }
            });
        },
    );
}

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use alloy::{
    primitives::{Address, FixedBytes, U256},
    providers::Provider,
    rpc::types::Filter,
    sol_types::{SolCall, SolEvent},
};
use chrono::{NaiveDateTime, Utc};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use socketioxide::{
    extract::{Data as SocketData, SocketRef, State as SocketState},
    socket::Sid as SocketSid,
};
use tokio::sync::watch;
use tracing::{debug, instrument};

use crate::{
    interfaces::{Multicall, ERC721},
    state::AppState,
};

#[derive(Debug, Deserialize)]
enum Chain {
    Mainnet,
    Base,
    Arbitrum,
    Optimism,
    Polygon,
    Bsc,
}

#[derive(Debug, Deserialize)]
struct RequestData {
    chain: Chain,
    addresses: Vec<Address>,
}

#[derive(Debug, Serialize)]
struct ResponseData {
    id: SocketSid,
    address: Address,
    name: String,
    symbol: String,
    from: Address,
    to: Address,
    token_id: U256,
    block_number: u64,
    transaction_hash: FixedBytes<32>,
    timestamp: NaiveDateTime,
}

#[derive(Debug, Serialize)]
struct ErrorData {
    id: SocketSid,
    message: String,
}

#[derive(Debug)]
struct TokenData {
    name: String,
    symbol: String,
}

#[instrument(skip(state))]
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

            // Remove duplicates
            let data_addresses = data
                .addresses
                .to_vec()
                .drain(..)
                .collect::<HashSet<_>>()
                .drain()
                .collect::<Vec<_>>();

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

            let mut calls = vec![];
            for addr in &data_addresses {
                calls.push(Multicall::Call {
                    target: addr.to_owned(),
                    gasLimit: U256::MAX,
                    callData: erc721
                        .supportsInterface(FixedBytes(
                            [0x80, 0xac, 0x58, 0xcd], /* ERC721.supportsInterface */
                        ))
                        .calldata()
                        .to_owned(),
                });
                calls.push(Multicall::Call {
                    target: addr.to_owned(),
                    gasLimit: U256::MAX,
                    callData: erc721.name().calldata().to_owned(),
                });
                calls.push(Multicall::Call {
                    target: addr.to_owned(),
                    gasLimit: U256::MAX,
                    callData: erc721.symbol().calldata().to_owned(),
                });
            }

            // Check all addresses for support of ERC721.supportsInterface in multicall
            let multicall_res = match multicall.multicall(calls).call().await {
                Ok(res) => res.returnData,
                Err(_) => {
                    socket
                        .emit(
                            "error",
                            &ErrorData {
                                id: socket.id,
                                message: "Failed to call fetch data".to_owned(),
                            },
                        )
                        .ok();

                    return;
                }
            };

            let mut token_data = HashMap::new();

            // Check if all addresses support the interface
            for (i, res) in multicall_res
                /* 1 for supportsInterface, 1 for name, 1 for symbol */
                .chunks(3)
                .enumerate()
            {
                // First index is for supportsInterface call
                let interface_data = res[0].returnData.to_owned();
                let interface_res =
                    match ERC721::supportsInterfaceCall::abi_decode_returns(&interface_data, false)
                    {
                        Ok(res) => res._0,
                        Err(_) => false, // Error means that the address doesn't support the interface
                    };

                if !interface_res {
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

                // Second index in for name
                let name_data = res[1].returnData.to_owned();
                let name_res = match ERC721::nameCall::abi_decode_returns(&name_data, false) {
                    Ok(decode_res) => decode_res._0,
                    Err(_) => {
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
                };

                // Third index in for symbol
                let symbol_data = res[2].returnData.to_owned();
                let symbol_res = match ERC721::symbolCall::abi_decode_returns(&symbol_data, false) {
                    Ok(decode_res) => decode_res._0,
                    Err(_) => {
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
                };

                token_data.insert(
                    data_addresses.to_owned()[i],
                    TokenData {
                        name: name_res,
                        symbol: symbol_res,
                    },
                );
            }

            // Create a subscription to blocks
            // let sub = match chain_state.provider.subscribe_blocks().await {
            let filter = Filter::new()
                .address(data.addresses)
                .event(ERC721::Transfer::SIGNATURE);

            let sub = match chain_state.provider.subscribe_logs(&filter).await {
                Ok(sub) => sub,
                Err(_) => {
                    socket
                        .emit(
                            "error",
                            &ErrorData {
                                id: socket.id,
                                message: "Failed to subscribe to blocks".to_owned(),
                            },
                        )
                        .ok();

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
                        Some(log) = stream.next() => {
                            let event = match log.log_decode::<ERC721::Transfer>() {
                                Ok(event) => event,
                                Err(_) => continue, // Skip if errors occurs while decoding the event
                            };
                            let event_data = event.data();

                            let token = match token_data.get(&event.address()) {
                                Some(data) => data,
                                None => unreachable!(),
                            };

                            let response_data = ResponseData {
                                id: socket.id,
                                address: event.address(),
                                name: token.name.to_owned(),
                                symbol: token.symbol.to_owned(),
                                from: event_data.from,
                                to: event_data.to,
                                token_id: event_data.tokenId,
                                block_number: log.block_number.unwrap_or_default(),
                                transaction_hash: log.transaction_hash.unwrap_or_default(),
                                timestamp: Utc::now().naive_utc(),
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

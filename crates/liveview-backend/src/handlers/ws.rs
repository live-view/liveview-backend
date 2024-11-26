use std::sync::Arc;

use alloy::{primitives::Address, providers::Provider};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use socketioxide::{
    extract::{Data as SocketData, SocketRef, State as SocketState},
    socket::Sid as SocketSid,
};
use tokio::sync::watch;
use tracing::debug;

use crate::state::AppState;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) enum Chain {
    Mainnet,
    Base,
    Arbitrum,
    Optimism,
    Polygon,
    BSC,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RequestData {
    pub(crate) chain: Chain,
    pub(crate) addresses: Vec<Address>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ResponseData {
    pub(crate) id: SocketSid,
    pub(crate) chain: Chain,
    pub(crate) block_number: u64,
    pub(crate) timestamp: chrono::DateTime<chrono::Utc>,
    pub(crate) addresses: Vec<Address>,
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

            // If there's no addresses
            if data.addresses.is_empty() {
                let message = "No addresses provided".to_owned();

                debug!(?socket.id, ?message);

                let response_data = ErrorData {
                    id: socket.id,
                    message,
                };
                socket.emit("error", &response_data).ok();

                return;
            }

            // Use a watch channel for graceful task cancellation
            let (tx, mut rx) = watch::channel(());

            // Send disconnect event when the task is cancelled
            socket.on_disconnect(move || {
                debug!("Socket disconnected");

                tx.send(()).ok();
            });

            let provider = match data.chain {
                Chain::Mainnet => Arc::clone(&state.mainnet),
                Chain::Base => Arc::clone(&state.base),
                Chain::Arbitrum => Arc::clone(&state.arbitrum),
                Chain::Optimism => Arc::clone(&state.optimism),
                Chain::Polygon => Arc::clone(&state.polygon),
                Chain::BSC => Arc::clone(&state.bsc),
            };

            // Create a subscription to blocks
            let sub = match provider.subscribe_blocks().await {
                Ok(sub) => sub,
                Err(_) => {
                    let message = "Failed to subscribe to blocks".to_owned();

                    debug!(?socket.id, ?message);

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
                                chain: data.chain.to_owned(),
                                timestamp: chrono::Utc::now(),
                                addresses: data.addresses.to_owned(),
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

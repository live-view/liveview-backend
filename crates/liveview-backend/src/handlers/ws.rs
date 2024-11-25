use std::sync::Arc;

use alloy::providers::Provider;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use socketioxide::{
    extract::{Data as SocketData, SocketRef, State as SocketState},
    socket::Sid as SocketSid,
};
use tokio::sync::watch;

use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) enum Chain {
    Mainnet,
    Base,
    Arbitrum,
    Optimism,
    Polygon,
    BSC,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct RequestData {
    pub(crate) chain: Chain,
    pub(crate) addresses: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
pub(crate) struct ResponseData {
    pub(crate) id: SocketSid,
    pub(crate) timestamp: chrono::DateTime<chrono::Utc>,
    pub(crate) chain: Chain,
    pub(crate) addresses: Vec<String>,
    pub(crate) block_number: u64,
}

pub(crate) async fn ws(socket: SocketRef, state: SocketState<Arc<AppState>>) {
    tracing::debug!(ns = socket.ns(), ?socket.id, "Socket.IO connected");

    let state = Arc::clone(&state);

    socket.on(
        "request",
        |socket: SocketRef, SocketData::<RequestData>(data)| async move {
            tracing::debug!(?data, "Received event");

            // Use a watch channel for graceful task cancellation
            let (tx, mut rx) = watch::channel(());

            // Send disconnect event when the task is cancelled
            socket.on_disconnect(move || {
                tracing::debug!("Socket disconnected");

                tx.send(()).ok();
            });

            // Create a subscription to blocks
            let sub = match state.provider.subscribe_blocks().await {
                Ok(sub) => sub,
                Err(e) => {
                    tracing::error!(error = ?e, ?socket.id, "Failed to subscribe to blocks");
                    return;
                }
            };

            // Convert the subscription into a stream
            let mut stream = sub.into_stream();

            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = rx.changed() => {
                            tracing::debug!(?socket.id, "Task cancelled");

                            // Break the loop when the task is cancelled
                            break;
                        },
                        Some(block) = stream.next() => {
                             let response_data = ResponseData {
                                id: socket.id,
                                timestamp: chrono::Utc::now(),
                                chain: data.chain.to_owned(),
                                addresses: data.addresses.to_owned(),
                                block_number: block.number,
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

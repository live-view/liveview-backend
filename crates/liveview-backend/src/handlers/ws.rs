use std::sync::Arc;

use serde_json::Value;
use socketioxide::extract::{AckSender, Data as SocketData, SocketRef, State as SocketState};
use tokio::sync::Mutex;

use crate::state::AppState;

pub(crate) async fn ws(
    socket: SocketRef,
    SocketData(data): SocketData<Value>,
    state: SocketState<Arc<Mutex<AppState>>>,
) {
    tracing::info!(ns = socket.ns(), ?socket.id, "Socket.IO connected");

    let state = Arc::clone(&state);
    let mut state = state.lock().await;

    tracing::info!("count: {}", state.count);

    state.count += 1;

    socket.emit("auth", &data).ok();

    socket.on("message", |socket: SocketRef, SocketData::<Value>(data)| {
        tracing::info!(?data, "Received event:");
        socket.emit("message-back", &data).ok();
    });

    socket.on(
        "message-with-ack",
        |SocketData::<Value>(data), ack: AckSender| {
            tracing::info!(?data, "Received event");
            ack.send(&data).ok();
        },
    );
}

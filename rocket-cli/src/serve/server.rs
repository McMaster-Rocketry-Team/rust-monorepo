use std::sync::Arc;
use std::sync::RwLock;
use anyhow::Result;
use firmware_common_new::vlp::packets::VLPDownlinkPacket;
use lora_phy::mod_params::PacketStatus;
use std::time::Duration;

use axum::{
    Json, Router, extract::{WebSocketUpgrade, State, ws::{Message, WebSocket}}, http::StatusCode, response::Response, routing::{any, get, post},
};

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::gs::{config::GroundStationConfig, vlp_client::VLPClientTrait};

pub async fn start_server(
    client: &'static impl VLPClientTrait,
    config: Arc<RwLock<GroundStationConfig>>,
) -> Result<()> {

    let (tx, _rx) = broadcast::channel::<String>(100);

    let state = AppState {tx: tx.clone()};

    // spawn data polling task from client
    tokio::spawn(async move {
        loop {
            if let Some((packet, status)) = client.try_receive() {
                // println!("{:#?}", packet);
                // println!("rssi {}, snr {}", status.rssi, status.snr);
                /* 
                TODO: at the moment just sending with JSON to test the server functionality. 
                Need to implement correct sending method (from NICK/docs/BACKEND_WIRING-4.2, copied here)
                Packet decoding and unit corrections can be seen in the formatting in gs/downlink_packet_display/sections (L259)
                on each received `VLPDownlinkPacket`, decode it and flatten it
                    into `(key, value, timestampMs)` triples using the `gsd_*` dictionary —
                    the flattening rules (booleans as 0/1, enums as their numeric values,
                    fixed-point fields as floats) are exactly what the catalog already
                    declares. Include the link-quality pseudo-signals (RSSI, SNR,
                    `gsd_packet_type`, packet counters) which come from the LoRa
                    `PacketStatus`, not the payload.
                 */
                let pws = PacketWithStatus {packet: packet, rssi: status.rssi, snr: status.snr};
                let json_pws = serde_json::to_string(&pws);
                if let Ok(data) = json_pws {
                    // safe to ignore error, just shows no clients connected
                    let _ = tx.send(data);
                }
            } else {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
    });

    let app = Router::new()
        .route("/ws", any(ws_handler))
        .route("/cmd", post(command_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    Ok(())
}

#[derive(Clone)]
struct AppState {
    tx: broadcast::Sender<String>
}

#[derive(Serialize)]
struct PacketWithStatus {
    packet: VLPDownlinkPacket,
    rssi: i16,
    snr: i16
}

#[derive(Deserialize)]
struct PostCommandRequest {
    command: String,
}

#[derive(Serialize)]
struct PostCommandResponse {
    id: u64,
    response: String,
}

async fn command_handler(Json(payload): Json<PostCommandRequest>) -> (StatusCode, Json<PostCommandResponse>) {
    let res = PostCommandResponse {
        id: 1,
        response: format!("response to {}", payload.command)
    };
    (StatusCode::CREATED, Json(res))
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    let rx = state.tx.subscribe();
    ws.on_upgrade(move |socket| handle_socket(socket, rx))
}

async fn handle_socket(mut socket: WebSocket, mut rx: broadcast::Receiver<String>) {
    while let Ok(data) = rx.recv().await {
        if socket.send(Message::Text(data.into())).await.is_err() {
            // Client disconnected or socket closed
            println!("Breaking");
            break;
        }
    }
}
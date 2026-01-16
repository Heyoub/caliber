//! WebSocket realtime manager with reconnect backoff.

use crate::api_client::WsClient;
use crate::events::TuiEvent;
use caliber_api::events::WsEvent;
use caliber_core::EntityId;
use futures_util::StreamExt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

pub fn spawn_ws_manager(
    ws: WsClient,
    tenant_id: EntityId,
    sender: mpsc::Sender<TuiEvent>,
) {
    tokio::spawn(async move {
        let mut backoff = ws.reconnect_config().initial_ms;
        loop {
            match ws.connect(tenant_id).await {
                Ok(mut stream) => {
                    let _ = sender
                        .send(TuiEvent::Ws(WsEvent::Connected { tenant_id }))
                        .await;
                    backoff = ws.reconnect_config().initial_ms;

                    while let Some(message) = stream.next().await {
                        match message {
                            Ok(Message::Text(text)) => {
                                match serde_json::from_str::<WsEvent>(&text) {
                                    Ok(event) => {
                                        let _ = sender.send(TuiEvent::Ws(event)).await;
                                    }
                                    Err(err) => {
                                        let _ = sender
                                            .send(TuiEvent::ApiError(format!(
                                                "WS decode error: {}",
                                                err
                                            )))
                                            .await;
                                    }
                                }
                            }
                            Ok(Message::Binary(_)) => {}
                            Ok(Message::Close(_)) => break,
                            Ok(_) => {}
                            Err(err) => {
                                let _ = sender
                                    .send(TuiEvent::Ws(WsEvent::Error {
                                        message: err.to_string(),
                                    }))
                                    .await;
                                break;
                            }
                        }
                    }

                    let _ = sender
                        .send(TuiEvent::Ws(WsEvent::Disconnected {
                            reason: "connection closed".to_string(),
                        }))
                        .await;
                }
                Err(err) => {
                    let _ = sender
                        .send(TuiEvent::Ws(WsEvent::Error {
                            message: err.to_string(),
                        }))
                        .await;
                }
            }

            let delay = jittered_backoff(backoff, ws.reconnect_config().jitter_ms);
            tokio::time::sleep(Duration::from_millis(delay)).await;

            let next = (backoff as f64 * ws.reconnect_config().multiplier) as u64;
            backoff = next.min(ws.reconnect_config().max_ms);
        }
    });
}

fn jittered_backoff(base_ms: u64, jitter_ms: u64) -> u64 {
    if jitter_ms == 0 {
        return base_ms;
    }
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_nanos(0))
        .subsec_nanos() as u64;
    let jitter = nanos % jitter_ms;
    base_ms.saturating_add(jitter)
}

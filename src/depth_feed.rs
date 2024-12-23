use std::sync::Arc;
use colored::Colorize;
use futures_util::StreamExt;
use tokio::sync::RwLock;
use tokio_tungstenite::tungstenite::Message;
use crate::{debug_print};
use crate::event_buffer::EventBuffer;
use crate::messages::{OrderBookUpdate, handle_ping_message};
use crate::order_book::{InstrumentState, OrderBook};
use crate::recovery::TimeoutState;

async fn handle_update(
    event_buffer: Arc<RwLock<EventBuffer>>,
    update: OrderBookUpdate,
    order_book: Arc<RwLock<OrderBook>>,
    state: Arc<RwLock<InstrumentState>>,
    timeout_state: Arc<TimeoutState>,
) {
    let mut buffer = event_buffer.write().await;
    buffer
        .buffer_and_process_update(update, Arc::clone(&order_book), Arc::clone(&state), Arc::clone(&timeout_state))
        .await;
}

pub async fn start_depth_feed(
    ws_url: String,
    event_buffer: Arc<RwLock<EventBuffer>>,
    order_book: Arc<RwLock<OrderBook>>,
    state: Arc<RwLock<InstrumentState>>,
    connection_id: usize,
    timeout_state: Arc<TimeoutState>,
) {
    println!("Connection {}: Starting WebSocket connection...", connection_id);

    let (ws_stream, _) = tokio_tungstenite::connect_async(&ws_url)
        .await
        .expect("Failed to connect to WebSocket");

    let (mut write, mut read) = ws_stream.split();

    let connection_name = format!("Depth connection {}", connection_id);

    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(update) = serde_json::from_str::<OrderBookUpdate>(&text) {
                    debug_print!(
                        "Connection {}: Received update first_trade_id = {}, last_trade_id = {}",
                        connection_id, update.first_trade_id, update.last_trade_id
                    );

                    handle_update(Arc::clone(&event_buffer), update, Arc::clone(&order_book), Arc::clone(&state), Arc::clone(&timeout_state)).await;
                }
            }
            Err(err) => {
                eprintln!("{}", format!("Connection {}: Error reading WebSocket: {:?}", connection_id, err).red().bold());
                break;
            }
            Ok(Message::Ping(data)) => {
                handle_ping_message(&connection_name, data, &mut write).await;
            }
            _ => {
                eprintln!("{}", format!("Connection {}: Unknown message received: {:?}", connection_id, msg).red().bold());
            }
        }
    }

    println!("Connection {}: WebSocket connection closed.", connection_id);
}

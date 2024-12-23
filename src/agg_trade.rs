use colored::Colorize;
use futures_util::StreamExt;
use tokio_tungstenite::tungstenite::Message;
use crate::messages::{AggTrade, handle_ping_message};

pub async fn start_aggtrade_feed(ws_url: String) {
    println!("Starting aggTrade feed...");

    let (ws_stream, _) = tokio_tungstenite::connect_async(&ws_url)
        .await
        .expect("Failed to connect to WebSocket");
    let (mut write, mut read) = ws_stream.split();

    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(trade) = serde_json::from_str::<AggTrade>(&text) {
                    println!("{}", trade)
                }
            }
            Err(err) => {
                eprintln!("{}", format!("Error reading WebSocket: {:?}", err).red().bold());
                break;
            }
            Ok(Message::Ping(data)) => {
                handle_ping_message("AggTrade Feed", data, &mut write).await;
            }
            _ => {
                eprintln!("{}", format!("AggTrade - Unknown message received: {:?}", msg).red().bold());
            }
        }
    }

    println!("AggTrade feed stopped.");
}

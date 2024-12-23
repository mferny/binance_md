use colored::Colorize;
use futures_util::StreamExt;
use tokio_tungstenite::tungstenite::Message;
use crate::messages::{BestDeal, handle_ping_message};

pub async fn start_best_deal_feed(ws_url: String) {
    println!("Starting best deal feed...");

    let (ws_stream, _) = tokio_tungstenite::connect_async(&ws_url)
        .await
        .expect("Failed to connect to WebSocket");
    let (mut write, mut read) = ws_stream.split();

    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(depth_update) = serde_json::from_str::<BestDeal>(&text) {
                    println!("{}", depth_update);
                }
            }
            Err(err) => {
                eprintln!("{}", format!("Error reading WebSocket: {:?}", err).red().bold());
                break;
            }
            Ok(Message::Ping(data)) => {
                handle_ping_message("BestDeal feed", data, &mut write).await;
            }
            _ => {
                eprintln!("{}", format!("BestDeal - Unknown message received: {:?}", msg).red().bold());
            }
        }
    }

    println!("Best deal feed stopped.");
}
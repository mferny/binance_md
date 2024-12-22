use colored::Colorize;
use futures_util::SinkExt;
use futures_util::stream::SplitSink;
use serde::Deserialize;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

// Order book price and quantity depth updates from Depth Stream (L2) <symbol>@depth
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct OrderBookUpdate {
    pub e: String,            // Event type
    pub E: u64,               // Event time
    pub s: String,            // Symbol
    pub U: u64,               // First update ID in event
    pub u: u64,               // Final update ID in event
    pub b: Vec<[String; 2]>,  // Bids to be updated [price, qty]
    pub a: Vec<[String; 2]>   // Asks to be updated [price, qty]
}

// Snapshot is used for restoring order book state
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    // last processed sequence number
    pub last_update_id: u64,
    // price + qty
    pub bids: Vec<[String; 2]>,
    pub asks: Vec<[String; 2]>,
}

// Aggregated Trades feed message structure
#[derive(Debug, Deserialize)]
pub struct AggTrade {
    pub e: String,    // Event type
    pub E: u64,       // Event time
    pub s: String,    // Symbol
    pub a: u64,       // Aggregate trade ID
    pub p: String,    // Price
    pub q: String,    // Quantity
    pub f: u64,       // First trade ID
    pub l: u64,       // Last trade ID
    pub T: u64,       // Trade time
    pub m: bool,      // Buyer is the market maker
    pub M: bool,      // Ignore
}

// Best Deal message structure
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BestDeal {
    pub last_update_id: u64,          // Last processed event ID
    pub bids: Vec<[String; 2]>,  // Price and quantity of the best bid
    pub asks: Vec<[String; 2]>,  // Price and quantity of the best ask
}

// ping-pong messaging is required by web socket protocol
pub async fn handle_ping_message(
    connection_name: &str,
    data: Vec<u8>,
    write: &mut SplitSink<WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>, Message>) {
    println!("{}", format!(
        "[{}]: Received Ping message with data: {:?}", connection_name, String::from_utf8_lossy(&data)).green());

    // respond with a Pong
    if let Err(err) = write.send(Message::Pong(data.clone())).await {
        eprintln!("{}", format!("Failed to send Pong: {:?}", err).red().bold());
    } else {
        println!("{}", format!(
            "[{}]: Sent Pong message with data: {:?}", connection_name, String::from_utf8_lossy(&data)).green());
    }
}

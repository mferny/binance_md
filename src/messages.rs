use std::fmt;
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
    #[serde(alias="e")]
    pub event_type: String,
    #[serde(alias="E")]
    pub event_time: u64,
    #[serde(alias="s")]
    pub symbol: String,
    #[serde(alias="U")]
    pub first_trade_id: u64,
    #[serde(alias="u")]
    pub last_trade_id: u64,
    #[serde(alias="b")]
    pub bids: Vec<[String; 2]>,
    #[serde(alias="a")]
    pub asks: Vec<[String; 2]>,
}

// Snapshot is used for restoring order book state
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    // last processed sequence number
    pub last_update_id: u64,
    // price + qty
    pub bids: Vec<[String; 2]>, // depth is quite big so using Vec here
    pub asks: Vec<[String; 2]>,
}

// Aggregated Trades feed message structure
#[derive(Deserialize)]
pub struct AggTrade {
    #[serde(alias="e")]
    pub event_type: String,
    #[serde(alias="E")]
    pub event_time: u64,
    #[serde(alias="s")]
    pub symbol: String,
    #[serde(alias="a")]
    pub agg_trade_id: u64,
    #[serde(alias="p")]
    pub price: String,
    #[serde(alias="q")]
    pub qty: String,
    #[serde(alias="f")]
    pub first_trade_id: u64,
    #[serde(alias="l")]
    pub last_trade_id: u64,
    #[serde(alias="T")]
    pub trade_time: u64,
    #[serde(alias="m")]
    pub is_market_maker: bool,
    #[serde(alias="M")]
    pub ignore: bool,
}

impl fmt::Display for AggTrade {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", format!("AggTrade - Symbol: {}, Price: {}, Quantity: {}, Is Market Maker: {}",
            self.symbol, self.price, self.qty, self.is_market_maker).cyan().bold())
    }
}

// Best Deal message structure
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BestDeal {
    pub last_update_id: u64,          // Last processed event ID
    pub bids: Vec<[String; 2]>,  // Price and quantity of the best bid
    pub asks: Vec<[String; 2]>,  // Price and quantity of the best ask
}

impl fmt::Display for BestDeal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", "Best Deal:".magenta().bold())?;
        writeln!(f, "{}", format!("  Last Update ID: {}", self.last_update_id).magenta().bold())?;

        if let Some(best_bid) = self.bids.first() {
            writeln!(f, "{}", format!("  Best Bid: Price: {}, Quantity: {}",
                best_bid[0], best_bid[1]).magenta().bold())?;
        } else {
            writeln!(f, "{}", "  Best Bid: None".magenta().bold())?;
        }

        if let Some(best_ask) = self.asks.first() {
            writeln!(f, "{}", format!("  Best Ask: Price: {}, Quantity: {}",
                    best_ask[0], best_ask[1]).magenta().bold())?;
        } else {
            writeln!(f, "{}", "  Best Ask: None".magenta().bold())?;
        }

        Ok(())
    }
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

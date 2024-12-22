// connect to Binance Spot API
// https://developers.binance.com/docs/binance-spot-api-docs/web-socket-streams
// print real time book updates (built book) + best deals + trades
// open several connections for one instrument for arbitration for depth feed

mod order_book;
mod messages;
mod recovery;
mod event_buffer;
mod debug;
mod best_deal;
mod agg_trade;
mod depth_feed;

use tokio::sync::{RwLock};
use futures_util::{StreamExt, future, SinkExt};
use reqwest;
use std::sync::Arc;
use tokio::time::Duration;
use colored::Colorize;


use order_book::{OrderBook, InstrumentState};
use recovery::{monitor_and_recover, TimeoutState};
use event_buffer::EventBuffer;
use crate::debug::is_debug_mode;
use crate::best_deal::start_best_deal_feed;
use crate::depth_feed::start_depth_feed;
use crate::agg_trade::start_aggtrade_feed;

#[tokio::main]
async fn main() {
    let debug_enabled = is_debug_mode();
    println!("Debug mode is {}", if debug_enabled { "enabled" } else { "disabled" });

    let symbol = "btcusdt";
    // number of ws connections for the same instrument for incremental feed, took a random value
    let num_connections = 3;
    // full depth (L2) market data ws
    let l2_url = format!("wss://stream.binance.com:9443/ws/{}@depth", symbol);
    // aggregated trades feed
    let aggtrade_url = format!("wss://stream.binance.com:9443/ws/{}@aggTrade", symbol);
    // here we subscribe to 1st level only to get the best deal feed
    let best_deal_url = format!("wss://stream.binance.com:9443/ws/{}@depth5", symbol);

    // snapshot is used for restoring the book state at the beginning and in case of loosing data
    // from incremental feeds
    let snapshot_url = format!(
        "https://api.binance.com/api/v3/depth?symbol={}", symbol.to_uppercase());

    // order book - current state of the market for symbol
    let order_book = Arc::new(RwLock::new(OrderBook::new()));
    // event buffer is used to store updates
    let event_buffer = Arc::new(RwLock::new(EventBuffer::new()));
    // flag to indicate the state of an instrument
    let state = Arc::new(RwLock::new(InstrumentState::JustStarted));
    // timeout state with a 5-second duration for triggering recovery in case of inactivity
    let timeout_state = Arc::new(TimeoutState::new(Duration::from_secs(5)));

    // start monitoring and recovery
    let monitor_handle = tokio::spawn(monitor_and_recover(
        Arc::clone(&event_buffer),
        Arc::clone(&order_book),
        Arc::clone(&state),
        snapshot_url.clone(),
        Arc::clone(&timeout_state),
    ));

    // start publishing trades
    let aggtrade_handle = tokio::spawn(start_aggtrade_feed(aggtrade_url));
    // and best deals
    let best_deal_handle = tokio::spawn(start_best_deal_feed(best_deal_url));

    let mut tasks = vec![];
    for connection_id in 0..num_connections {
        let event_buffer = Arc::clone(&event_buffer);
        let order_book = Arc::clone(&order_book);
        let state = Arc::clone(&state);
        let l2_url = l2_url.clone();
        let timeout_state = Arc::clone(&timeout_state);

        tasks.push(tokio::spawn(async move {
            start_depth_feed(l2_url, event_buffer, order_book, state, connection_id, timeout_state).await;
        }));
    }

    // Wait for all tasks to complete
    tasks.push(monitor_handle);
    tasks.push(aggtrade_handle);
    tasks.push(best_deal_handle);
    future::join_all(tasks).await;
}

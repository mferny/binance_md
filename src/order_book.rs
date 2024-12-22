use std::collections::{BTreeMap};
use std::sync::Arc;
use colored::Colorize;
use ordered_float::OrderedFloat;
use tokio::sync::RwLock;
use crate::messages::{OrderBookUpdate, Snapshot};
use crate::debug_print;

#[derive(Debug, PartialEq)]
pub enum InstrumentState {
    Normal,        // Normal processing of updates
    Recovering,    // Currently fetching and applying a snapshot
    JustRecovered, // Snapshot applied, ready to process buffered updates
    JustStarted,   // Initial state where recovery is always needed
}

// as we don't have level numbers in incremental updates, BTreeMap can be used for inserting
// updates, that are ordered by price
// for simplicity in this task all fields are public
pub struct OrderBook {
    // price + qty
    bids: BTreeMap<OrderedFloat<f64>, f64>,
    asks: BTreeMap<OrderedFloat<f64>, f64>,
    // update ID can be interpreted as a sequence number
    pub last_applied_id: u64,
}

impl OrderBook {
    pub fn new() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            last_applied_id: 0,
        }
    }

    pub async fn apply_snapshot_locked(
        order_book: &Arc<RwLock<Self>>,
        snapshot: &Snapshot,
        state: Arc<RwLock<InstrumentState>>)
    {
        let mut book = order_book.write().await;
        book.apply_snapshot(snapshot);

        let mut state_lock = state.write().await;
        *state_lock = InstrumentState::JustRecovered;
        debug_print!("Instrument state set to JustRecovered.");
    }

    fn apply_snapshot(
        &mut self, snapshot: &Snapshot)
    {
        self.last_applied_id = snapshot.last_update_id;
        self.bids.clear();
        self.asks.clear();
        for bid in &snapshot.bids {
            let price = OrderedFloat(bid[0].parse::<f64>().unwrap());
            let qty: f64 = bid[1].parse().unwrap();
            self.bids.insert(price, qty);
        }
        for ask in &snapshot.asks {
            let price = OrderedFloat(ask[0].parse::<f64>().unwrap());
            let qty: f64 = ask[1].parse().unwrap();
            self.asks.insert(price, qty);
        }

        debug_print!("Applied snapshot");
        self.print();
    }

    pub async fn apply_update_locked(
        order_book: &Arc<RwLock<Self>>,
        update: &OrderBookUpdate,
    ) -> Result<(), String> {
        let mut book = order_book.write().await;
        book.apply_update(update)
    }

    pub fn apply_update(&mut self, update: &OrderBookUpdate) -> Result<(), String> {
        // in this case we already either processed these updates or restored a
        // later state from snapshot
        if update.u <= self.last_applied_id {
            return Ok(());
        }
        if update.U > self.last_applied_id + 1 {
            return Err(format!(
                "Update ID in increment is more then in snapshot by {} \
                Will recover from a newer snapshot and buffer updates till recovered.",
                update.U - self.last_applied_id
            ));
        }

        for bid in &update.b {
            let price = OrderedFloat(bid[0].parse::<f64>().unwrap());
            let qty: f64 = bid[1].parse().unwrap();
            // remove level with zero qty
            if qty == 0.0 {
                self.bids.remove(&price);
            } else {
                self.bids.insert(price, qty);
            }
        }
        for ask in &update.a {
            let price = OrderedFloat(ask[0].parse::<f64>().unwrap());
            let qty: f64 = ask[1].parse().unwrap();
            if qty == 0.0 {
                self.asks.remove(&price);
            } else {
                self.asks.insert(price, qty);
            }
        }

        self.last_applied_id = update.u;

        debug_print!("Applied update");
        self.print();

        Ok(())
    }

    pub fn print(&self) {
        println!("{}", "Order Book:".blue().bold());
        println!("{}", "Bids:".blue().bold());
        // taking top 5 updates for visibility
        for (price, qty) in self.bids.iter().rev().take(5) {
            println!("{}", format!("Price: {:.6}, Qty: {:.6}", price, qty).blue().bold());
        }
        println!("{}", "Asks:".blue().bold());
        for (price, qty) in self.asks.iter().take(5) {
            println!("{}", format!("Price: {:.6}, Qty: {:.6}", price, qty).blue().bold());
        }
    }
}
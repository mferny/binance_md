use std::sync::Arc;
use tokio::sync::RwLock;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use colored::Colorize;

use crate::messages::OrderBookUpdate;
use crate::order_book::{InstrumentState, OrderBook};
use crate::recovery::TimeoutState;
use crate::debug_print;

#[derive(Debug, Eq)]
pub struct PrioritizedOrderBookUpdate(pub OrderBookUpdate);

impl Ord for PrioritizedOrderBookUpdate {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse the comparison for a min-heap
        other.0.U.cmp(&self.0.U)
    }
}

impl PartialOrd for PrioritizedOrderBookUpdate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for PrioritizedOrderBookUpdate {
    fn eq(&self, other: &Self) -> bool {
        self.0.U == other.0.U
    }
}

// Event buffer is used for storing events from the net. The first received consequent event will
// be applied immediately, past updates - ignored and future updates will be buffered for future publishing
pub struct EventBuffer {
    pub buffer: BinaryHeap<PrioritizedOrderBookUpdate>,
}

impl EventBuffer {
    pub fn new() -> Self {
        Self {
            buffer: BinaryHeap::new(),
        }
    }

    // add an update to the buffer
    pub async fn buffer_and_process_update(
        &mut self,
        update: OrderBookUpdate,
        order_book: Arc<RwLock<OrderBook>>,
        state: Arc<RwLock<InstrumentState>>,
        timeout_state: Arc<TimeoutState>,
    ) {
        debug_print!("Buffering update: {}", update.U);
        self.buffer.push(PrioritizedOrderBookUpdate(update));

        self.process_buffered_updates(order_book, state, Arc::clone(&timeout_state)).await;
    }

    pub async fn process_buffered_updates(
        &mut self,
        order_book: Arc<RwLock<OrderBook>>,
        state: Arc<RwLock<InstrumentState>>,
        timeout_state: Arc<TimeoutState>,
    ) {
        loop {
            let mut process_next_update = true;

            let next_update = {
                let mut state_lock = state.write().await;
                let mut book = order_book.write().await;

                if self.buffer.is_empty() {
                    debug_print!("No updates in the buffer.");
                    process_next_update = false; // Exit the loop
                    None
                } else if let Some(PrioritizedOrderBookUpdate(update)) = self.buffer.peek() {
                    if *state_lock == InstrumentState::JustRecovered {
                        // in JustRecovered state, take updates in range
                        if update.U <= book.last_applied_id + 1 && update.u >= book.last_applied_id + 1 {
                            debug_print!("Taking update after recovery: U={}, u={}", update.U, update.u);
                            *state_lock = InstrumentState::Normal;
                            debug_print!("State set to Normal after processing.");
                            self.buffer.pop().map(|entry| entry.0) // remove and process the update
                        } else if update.U > book.last_applied_id + 1 {
                            debug_print!(
                                "Future update detected after recovery: U={}, waiting for prior updates. Last Applied ID={}",
                                update.U, book.last_applied_id
                            );
                            process_next_update = false; // stop processing further updates
                            None
                        } else {
                            debug_print!(
                                "Outdated update after recovery: U={}, removing from buffer. Last Applied ID={}",
                                update.U, book.last_applied_id
                            );
                            self.buffer.pop(); // remove outdated update
                            None
                        }
                    } else {
                        // normal state: take only consecutive updates
                        if update.U == book.last_applied_id + 1 {
                            debug_print!("Taking consecutive update: U={}, u={}", update.U, update.u);
                            self.buffer.pop().map(|entry| entry.0) // remove and process the update
                        } else if update.U > book.last_applied_id + 1 {
                            debug_print!(
                                "Future update detected: U={}, waiting for prior updates. Last Applied ID={}",
                                update.U, book.last_applied_id
                            );
                            process_next_update = false; // stop processing further updates
                            None
                        } else {
                            debug_print!(
                                "Outdated update detected: U={}, removing from buffer. Last Applied ID={}",
                                update.U, book.last_applied_id
                            );
                            self.buffer.pop(); // remove outdated update
                            None
                        }
                    }
                } else {
                    debug_print!("No updates in the buffer.");
                    None
                }
            };

            if let Some(update) = next_update {
                if let Err(err) = OrderBook::apply_update_locked(&order_book, &update).await {
                    eprintln!("{}",
                        format!("Error applying buffered update: {}. Update ID: {:?}",
                        err, update.U).red().bold());
                } else {
                    // reset the inactivity timer on successful update
                    timeout_state.reset().await;
                }
            }

            // break the loop if we encounter a future update or no more updates to process
            if !process_next_update {
                break;
            }
        }
    }

    // check the size of the buffer
    pub async fn len(event_buffer: Arc<RwLock<Self>>) -> usize {
        let buffer = event_buffer.read().await;
        buffer.buffer.len()
    }
}
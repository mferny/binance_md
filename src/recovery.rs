use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::Instant;
use crate::messages::{Snapshot};
use crate::order_book::{OrderBook, InstrumentState};
use crate::event_buffer::EventBuffer;
use crate::debug_print;

// TimeoutState is used for checking for inactivity in publishing updates. If updates
// were not published for 5 secs, recovery from snapshot channel will be triggered
pub struct TimeoutState {
    pub last_activity: Arc<RwLock<Instant>>, // Tracks the last activity time
    pub timeout_duration: Duration,
}

impl TimeoutState {
    pub fn new(timeout_duration: Duration) -> Self {
        TimeoutState {
            last_activity: Arc::new(RwLock::new(Instant::now())),
            timeout_duration,
        }
    }

    pub async fn reset(&self) {
        let mut last_activity = self.last_activity.write().await;
        *last_activity = Instant::now();
        debug_print!("Inactivity timer reset");
    }

    pub async fn is_timed_out(&self) -> bool {
        let last_activity = self.last_activity.read().await;
        last_activity.elapsed() >= self.timeout_duration
    }
}

pub async fn monitor_and_recover(
    event_buffer: Arc<RwLock<EventBuffer>>,
    order_book: Arc<RwLock<OrderBook>>,
    state: Arc<RwLock<InstrumentState>>,
    snapshot_url: String,
    timeout_state: Arc<TimeoutState>,
) {
    let trigger_recovery = || async {
        println!("Triggering recovery...");
        recover_order_book(
            snapshot_url.clone(),
            Arc::clone(&order_book),
            Arc::clone(&event_buffer),
            Arc::clone(&state),
            timeout_state.clone(),
        )
            .await;
    };

    // Perform initial recovery if the instrument is in the `JustStarted` state
    if *state.read().await == InstrumentState::JustStarted {
        debug_print!("Instrument is in JustStarted state, recovery required.");
        trigger_recovery().await;
    }

    loop {
        // Wait for the timeout duration
        tokio::time::sleep(timeout_state.timeout_duration).await;

        // Check inactivity and trigger recovery if needed
        if timeout_state.is_timed_out().await {
            debug_print!("Inactivity timeout reached.");
            trigger_recovery().await;
        } else {
            debug_print!("No inactivity detected. Continuing monitoring...");
        }
    }
}

// here we process snapshot update, apply it to the book and then apply buffered updates
// that we received during recovery process
pub(crate) async fn recover_order_book(
    snapshot_url: String,
    order_book: Arc<RwLock<OrderBook>>,
    event_buffer: Arc<RwLock<EventBuffer>>,
    state: Arc<RwLock<InstrumentState>>,
    timeout_state: Arc<TimeoutState>,
) {
    {
        let mut state_lock = state.write().await;

        if *state_lock == InstrumentState::JustStarted {
            debug_print!("Transitioning from JustStarted to Recovering.");
        } else {
            debug_print!("Transitioning from Normal to Recovering.");
        }

        *state_lock = InstrumentState::Recovering;
    }

    debug_print!("Starting recovery...");

    match fetch_snapshot(&snapshot_url).await {
        Ok(snapshot) => {
            debug_print!("Snapshot fetched successfully. Applying snapshot...");
            // Apply snapshot to the order book
            OrderBook::apply_snapshot_locked(&order_book, &snapshot, Arc::clone(&state)).await;

            timeout_state.reset().await;

            debug_print!("Snapshot applied. Processing buffered updates...");

            // Process buffered updates immediately after recovery
            {
                let mut buffer = event_buffer.write().await;
                buffer.process_buffered_updates(Arc::clone(&order_book), Arc::clone(&state), Arc::clone(&timeout_state)).await;
            }

            {
                let mut state_lock = state.write().await;
                *state_lock = InstrumentState::JustRecovered;
                debug_print!("Instrument state set to JustRecovered.");
            }

            debug_print!("Recovery complete.");
        }
        Err(err) => {
            eprintln!("Failed to recover order book: {}", err);
            let mut state_lock = state.write().await;
            *state_lock = InstrumentState::Normal; // Reset to normal even if recovery fails
        }
    }
}

// receive snapshot for recovery
pub(crate) async fn fetch_snapshot(
    snapshot_url: &str) -> Result<Snapshot, String>
{
    debug_print!("Fetching snapshot from: {}", snapshot_url);

    match reqwest::get(snapshot_url).await {
        Ok(response) => {
            let raw_json = response.text().await.unwrap_or_else(|err| {
                eprintln!("Failed to read response text. Error: {:?}", err);
                "".to_string()
            });

            match serde_json::from_str::<Snapshot>(&raw_json) {
                Ok(snapshot) => {
                    debug_print!("Fetched snapshot with ID: {:?}", snapshot.last_update_id);
                    Ok(snapshot)
                }
                Err(err) => {
                    eprintln!("Failed to parse snapshot JSON. Error: {:?}", err);
                    Err("Failed to parse snapshot JSON".to_string())
                }
            }
        }
        Err(err) => {
            eprintln!("Failed to fetch snapshot: {:?}", err);
            Err("Failed to fetch snapshot".to_string())
        }
    }
}

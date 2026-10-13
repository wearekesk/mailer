//! In-memory CSV-import progress + stop signal.
//!
//! `services::subscribers::start_async_import` spawns a tokio task that
//! ticks `lists.currently_processing` while the worker chews through
//! the CSV. The same task checks an `AtomicBool` here every row so
//! "Stop import" can interrupt a multi-million row run without waiting
//! for the whole file.
//!
//! Per-list state lives in a process-global `Mutex<HashMap<i32,
//! Arc<ImportState>>>`. State is lost on process restart — a crashed
//! import will look "stuck" with stale `currently_processing` until
//! the operator either clicks Stop (which clears the row counters) or
//! starts a new import.
//!
//! Skipped rows are buffered in memory under the same `ImportState` so
//! the operator can download them as a CSV via
//! `/app/lists/:id/skipped-import.csv` after the import finishes.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Debug, Default)]
pub struct ImportState {
    pub stopped: AtomicBool,
    pub started_at: i64,
    pub skipped_rows: Mutex<Vec<SkippedRow>>,
}

#[derive(Debug, Clone)]
pub struct SkippedRow {
    pub email: String,
    pub reason: String,
}

fn registry() -> &'static Mutex<HashMap<i32, Arc<ImportState>>> {
    static REGISTRY: OnceLock<Mutex<HashMap<i32, Arc<ImportState>>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Begin tracking a new import. Returns a fresh `ImportState`; any
/// prior state for `list_id` (e.g. a previous import that finished) is
/// dropped.
pub fn begin(list_id: i32) -> Arc<ImportState> {
    let state = Arc::new(ImportState {
        stopped: AtomicBool::new(false),
        started_at: chrono::Utc::now().timestamp(),
        skipped_rows: Mutex::new(Vec::new()),
    });
    registry().lock().unwrap().insert(list_id, state.clone());
    state
}

/// Flip the stop signal. The worker checks `is_stopped()` every row.
pub fn stop(list_id: i32) {
    if let Some(state) = registry().lock().unwrap().get(&list_id) {
        state.stopped.store(true, Ordering::Release);
    }
}

pub fn is_stopped(state: &ImportState) -> bool {
    state.stopped.load(Ordering::Acquire)
}

/// Push a skipped row into the in-memory buffer for later CSV download.
pub fn record_skipped(state: &ImportState, email: String, reason: &str) {
    if let Ok(mut buf) = state.skipped_rows.lock() {
        // Cap the buffer so a million-row bad import doesn't OOM us.
        if buf.len() < 50_000 {
            buf.push(SkippedRow {
                email,
                reason: reason.to_string(),
            });
        }
    }
}

/// Snapshot the current skipped-row list (clone) for CSV export.
pub fn snapshot_skipped(list_id: i32) -> Vec<SkippedRow> {
    registry()
        .lock()
        .unwrap()
        .get(&list_id)
        .and_then(|s| s.skipped_rows.lock().ok().map(|b| b.clone()))
        .unwrap_or_default()
}

/// Drop the registry entry — call after the worker finishes its run
/// successfully so the next import starts clean. Skipped rows survive
/// for the operator to download until then.
pub fn finish(list_id: i32) {
    // Don't drop the entry — keep it so the skipped CSV stays available.
    // Just clear the stop flag so a re-run doesn't get aborted at row 0.
    if let Some(state) = registry().lock().unwrap().get(&list_id) {
        state.stopped.store(false, Ordering::Release);
    }
}

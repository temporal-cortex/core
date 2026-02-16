//! WASM bindings for truth-engine.
//!
//! Exposes RRULE expansion, conflict detection, and free/busy computation to
//! JavaScript via `wasm-bindgen`. All complex types are passed as JSON strings,
//! matching the pattern established by `toon-wasm`.
//!
//! ## Build process
//!
//! ```sh
//! cargo build -p truth-engine-wasm --target wasm32-unknown-unknown --release
//! wasm-bindgen --target nodejs --out-dir packages/truth-engine-js/wasm/ \
//!   target/wasm32-unknown-unknown/release/truth_engine_wasm.wasm
//! # Rename .js -> .cjs for ESM compatibility
//! mv packages/truth-engine-js/wasm/truth_engine_wasm.js \
//!    packages/truth-engine-js/wasm/truth_engine_wasm.cjs
//! ```

use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use truth_engine::expander::ExpandedEvent;
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// Serde-friendly DTOs for crossing the WASM boundary as JSON
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct ExpandedEventDto {
    start: String,
    end: String,
}

impl From<&ExpandedEvent> for ExpandedEventDto {
    fn from(e: &ExpandedEvent) -> Self {
        Self {
            start: e.start.to_rfc3339(),
            end: e.end.to_rfc3339(),
        }
    }
}

#[derive(Serialize)]
struct ConflictDto {
    event_a: ExpandedEventDto,
    event_b: ExpandedEventDto,
    overlap_minutes: i64,
}

#[derive(Serialize)]
struct FreeSlotDto {
    start: String,
    end: String,
    duration_minutes: i64,
}

/// Input format for events passed from JavaScript.
#[derive(Deserialize)]
struct EventInput {
    start: String,
    end: String,
}

// ---------------------------------------------------------------------------
// Helper: parse an ISO 8601 string into a UTC DateTime
// ---------------------------------------------------------------------------

/// Parse an ISO 8601 datetime string into `DateTime<Utc>`.
///
/// Accepts both RFC 3339 (with timezone offset, e.g., "2026-02-17T14:00:00+00:00")
/// and naive local time (e.g., "2026-02-17T14:00:00"), which is interpreted as UTC.
fn parse_datetime(s: &str) -> Result<DateTime<Utc>, JsValue> {
    // Try RFC 3339 first (has timezone info).
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Utc));
    }
    // Fall back to naive datetime interpreted as UTC.
    NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
        .map(|ndt| ndt.and_utc())
        .map_err(|e| JsValue::from_str(&format!("Invalid datetime '{}': {}", s, e)))
}

/// Convert a JSON array of `{start, end}` event objects into `Vec<ExpandedEvent>`.
fn parse_events_json(json: &str) -> Result<Vec<ExpandedEvent>, JsValue> {
    let inputs: Vec<EventInput> = serde_json::from_str(json)
        .map_err(|e| JsValue::from_str(&format!("Invalid events JSON: {}", e)))?;

    inputs
        .into_iter()
        .map(|input| {
            let start = parse_datetime(&input.start)?;
            let end = parse_datetime(&input.end)?;
            Ok(ExpandedEvent { start, end })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// WASM exports
// ---------------------------------------------------------------------------

/// Expand an RRULE string into concrete datetime instances.
///
/// Returns a JSON string containing an array of `{start, end}` objects with
/// RFC 3339 datetime strings.
///
/// # Arguments
/// - `rrule` -- RFC 5545 RRULE string (e.g., "FREQ=WEEKLY;BYDAY=TU,TH")
/// - `dtstart` -- Local datetime string (e.g., "2026-02-17T14:00:00")
/// - `duration_minutes` -- Duration of each instance in minutes
/// - `timezone` -- IANA timezone (e.g., "America/Los_Angeles")
/// - `until` -- Optional end boundary for expansion (local datetime string)
/// - `max_count` -- Optional maximum number of instances
#[wasm_bindgen(js_name = "expandRRule")]
pub fn expand_rrule(
    rrule: &str,
    dtstart: &str,
    duration_minutes: u32,
    timezone: &str,
    until: Option<String>,
    max_count: Option<u32>,
) -> Result<String, JsValue> {
    let events = truth_engine::expand_rrule(
        rrule,
        dtstart,
        duration_minutes,
        timezone,
        until.as_deref(),
        max_count,
    )
    .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let dtos: Vec<ExpandedEventDto> = events.iter().map(ExpandedEventDto::from).collect();

    serde_json::to_string(&dtos)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}

/// Find all pairwise conflicts (overlapping time ranges) between two event lists.
///
/// Both arguments must be JSON arrays of `{start, end}` objects with ISO 8601
/// datetime strings. Returns a JSON string containing an array of conflict objects,
/// each with `event_a`, `event_b`, and `overlap_minutes`.
#[wasm_bindgen(js_name = "findConflicts")]
pub fn find_conflicts(events_a_json: &str, events_b_json: &str) -> Result<String, JsValue> {
    let events_a = parse_events_json(events_a_json)?;
    let events_b = parse_events_json(events_b_json)?;

    let conflicts = truth_engine::find_conflicts(&events_a, &events_b);

    let dtos: Vec<ConflictDto> = conflicts
        .iter()
        .map(|c| ConflictDto {
            event_a: ExpandedEventDto::from(&c.event_a),
            event_b: ExpandedEventDto::from(&c.event_b),
            overlap_minutes: c.overlap_minutes,
        })
        .collect();

    serde_json::to_string(&dtos)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}

/// Find free time slots within a given time window, given a list of busy events.
///
/// `events_json` must be a JSON array of `{start, end}` objects. `window_start`
/// and `window_end` are ISO 8601 datetime strings. Returns a JSON string containing
/// an array of `{start, end, duration_minutes}` objects.
#[wasm_bindgen(js_name = "findFreeSlots")]
pub fn find_free_slots(
    events_json: &str,
    window_start: &str,
    window_end: &str,
) -> Result<String, JsValue> {
    let events = parse_events_json(events_json)?;
    let ws = parse_datetime(window_start)?;
    let we = parse_datetime(window_end)?;

    let slots = truth_engine::find_free_slots(&events, ws, we);

    let dtos: Vec<FreeSlotDto> = slots
        .iter()
        .map(|s| FreeSlotDto {
            start: s.start.to_rfc3339(),
            end: s.end.to_rfc3339(),
            duration_minutes: s.duration_minutes,
        })
        .collect();

    serde_json::to_string(&dtos)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}

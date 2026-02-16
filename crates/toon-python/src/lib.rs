//! # toon-python
//!
//! Python bindings for the TOON format encoder/decoder and truth-engine,
//! built with PyO3.
//!
//! Exposes the following functions to Python as the `toon_format` module:
//!
//! - `encode(json)` -- JSON string -> TOON string
//! - `decode(toon)` -- TOON string -> JSON string
//! - `filter_and_encode(json, patterns)` -- semantic filter + TOON encode
//! - `expand_rrule(...)` -- RRULE expansion -> JSON string of events

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Encode a JSON string into TOON format.
///
/// Args:
///     json: A valid JSON string.
///
/// Returns:
///     The TOON-encoded string.
///
/// Raises:
///     ValueError: If the input is not valid JSON or encoding fails.
#[pyfunction]
fn encode(json: &str) -> PyResult<String> {
    toon_core::encode(json).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Decode a TOON string back into JSON.
///
/// Args:
///     toon: A valid TOON string.
///
/// Returns:
///     The JSON string.
///
/// Raises:
///     ValueError: If the input is not valid TOON or decoding fails.
#[pyfunction]
fn decode(toon: &str) -> PyResult<String> {
    toon_core::decode(toon).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Filter fields from a JSON string by pattern, then encode to TOON.
///
/// Patterns support dot-separated paths and wildcards:
/// - `"etag"` -- strip the top-level field named "etag"
/// - `"items.etag"` -- strip "etag" inside objects under "items"
/// - `"*.etag"` -- wildcard: strip "etag" at any depth
///
/// Args:
///     json: A valid JSON string.
///     patterns: A list of field patterns to strip.
///
/// Returns:
///     The filtered TOON-encoded string.
///
/// Raises:
///     ValueError: If the input is not valid JSON or encoding fails.
#[pyfunction]
fn filter_and_encode(json: &str, patterns: Vec<String>) -> PyResult<String> {
    let pattern_refs: Vec<&str> = patterns.iter().map(|s| s.as_str()).collect();
    toon_core::filter_and_encode(json, &pattern_refs)
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Expand an RRULE into concrete event instances, returned as a JSON string.
///
/// Each event in the returned JSON array has `start` and `end` fields
/// as ISO 8601 UTC datetime strings.
///
/// Args:
///     rrule: RFC 5545 RRULE string (e.g., "FREQ=DAILY;COUNT=3").
///     dtstart: Local datetime string (e.g., "2026-02-17T14:00:00").
///     duration_minutes: Duration of each event instance in minutes.
///     timezone: IANA timezone identifier (e.g., "America/Los_Angeles").
///     until: Optional end boundary for expansion (local datetime string).
///     max_count: Optional maximum number of instances to generate.
///
/// Returns:
///     A JSON string containing an array of event objects with `start` and `end` fields.
///
/// Raises:
///     ValueError: If the RRULE or timezone is invalid.
#[pyfunction]
#[pyo3(signature = (rrule, dtstart, duration_minutes, timezone, until=None, max_count=None))]
fn expand_rrule(
    rrule: &str,
    dtstart: &str,
    duration_minutes: i64,
    timezone: &str,
    until: Option<&str>,
    max_count: Option<u32>,
) -> PyResult<String> {
    let events = truth_engine::expand_rrule(
        rrule,
        dtstart,
        duration_minutes as u32,
        timezone,
        until,
        max_count,
    )
    .map_err(|e| PyValueError::new_err(e.to_string()))?;

    // Serialize to JSON: [{"start": "...", "end": "..."}, ...]
    let json_events: Vec<serde_json::Value> = events
        .into_iter()
        .map(|evt| {
            serde_json::json!({
                "start": evt.start.to_rfc3339(),
                "end": evt.end.to_rfc3339(),
            })
        })
        .collect();

    serde_json::to_string(&json_events).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// The `toon_format` Python module, implemented in Rust via PyO3.
#[pymodule]
fn toon_format(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(encode, m)?)?;
    m.add_function(wrap_pyfunction!(decode, m)?)?;
    m.add_function(wrap_pyfunction!(filter_and_encode, m)?)?;
    m.add_function(wrap_pyfunction!(expand_rrule, m)?)?;
    Ok(())
}

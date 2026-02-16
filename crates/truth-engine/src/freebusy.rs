//! Compute free time slots from event lists.
//!
//! Sorts events by start time, merges overlapping busy periods, then computes
//! the gaps between merged periods within a given time window.

use crate::expander::ExpandedEvent;
use chrono::{DateTime, Utc};

/// A free time slot.
#[derive(Debug, Clone, PartialEq)]
pub struct FreeSlot {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub duration_minutes: i64,
}

/// Merge overlapping or adjacent busy periods, clipped to the given window.
///
/// Returns a sorted, non-overlapping list of (start, end) intervals.
fn merge_busy_periods(
    events: &[ExpandedEvent],
    window_start: DateTime<Utc>,
    window_end: DateTime<Utc>,
) -> Vec<(DateTime<Utc>, DateTime<Utc>)> {
    // Collect events clipped to the window, discarding events entirely outside.
    let mut intervals: Vec<(DateTime<Utc>, DateTime<Utc>)> = events
        .iter()
        .filter(|e| e.start < window_end && e.end > window_start)
        .map(|e| (e.start.max(window_start), e.end.min(window_end)))
        .collect();

    if intervals.is_empty() {
        return Vec::new();
    }

    // Sort by start time (then by end time for stability).
    intervals.sort_by_key(|&(start, end)| (start, end));

    // Merge overlapping intervals.
    let mut merged: Vec<(DateTime<Utc>, DateTime<Utc>)> = Vec::new();
    for (start, end) in intervals {
        if let Some(last) = merged.last_mut() {
            if start <= last.1 {
                // Overlapping or adjacent — extend the current interval.
                last.1 = last.1.max(end);
                continue;
            }
        }
        merged.push((start, end));
    }

    merged
}

/// Find free time slots within a given time window, given a list of busy events.
///
/// Events may overlap -- overlapping busy periods are merged before computing gaps.
/// Returns free slots sorted by start time.
pub fn find_free_slots(
    events: &[ExpandedEvent],
    window_start: DateTime<Utc>,
    window_end: DateTime<Utc>,
) -> Vec<FreeSlot> {
    let merged = merge_busy_periods(events, window_start, window_end);

    let mut free_slots = Vec::new();
    let mut cursor = window_start;

    for (busy_start, busy_end) in &merged {
        if cursor < *busy_start {
            let duration_minutes = (*busy_start - cursor).num_minutes();
            free_slots.push(FreeSlot {
                start: cursor,
                end: *busy_start,
                duration_minutes,
            });
        }
        cursor = cursor.max(*busy_end);
    }

    // Trailing free slot after the last busy period.
    if cursor < window_end {
        let duration_minutes = (window_end - cursor).num_minutes();
        free_slots.push(FreeSlot {
            start: cursor,
            end: window_end,
            duration_minutes,
        });
    }

    free_slots
}

/// Find the first free slot of at least `min_duration_minutes` within the window.
///
/// Delegates to [`find_free_slots`] and returns the first slot meeting the minimum
/// duration requirement.
pub fn find_first_free_slot(
    events: &[ExpandedEvent],
    window_start: DateTime<Utc>,
    window_end: DateTime<Utc>,
    min_duration_minutes: i64,
) -> Option<FreeSlot> {
    find_free_slots(events, window_start, window_end)
        .into_iter()
        .find(|slot| slot.duration_minutes >= min_duration_minutes)
}

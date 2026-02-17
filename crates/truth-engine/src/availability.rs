//! Multi-stream availability merging with privacy-preserving output.
//!
//! Accepts N event streams (from different calendars/providers), merges them into
//! unified busy/free blocks within a time window. Supports privacy levels to control
//! how much source information is exposed.
//!
//! This module is the core of the "Unified Availability Graph" — it computes the
//! single source of truth for a user's availability across all their calendars.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::expander::ExpandedEvent;
use crate::freebusy::{self, FreeSlot};

/// A named event stream from a single calendar source.
#[derive(Debug, Clone)]
pub struct EventStream {
    /// Opaque identifier for this stream (e.g., "work-google", "personal-icloud").
    pub stream_id: String,
    /// The events in this stream (already expanded from RRULEs if applicable).
    pub events: Vec<ExpandedEvent>,
}

/// Privacy level for availability output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PrivacyLevel {
    /// Show time ranges and source count per busy block.
    Full,
    /// Show only busy/free time ranges — no source details leak through.
    /// `source_count` is set to 0 for all busy blocks.
    #[default]
    Opaque,
}

/// A merged busy block in the unified availability view.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BusyBlock {
    /// Start of the busy period.
    pub start: DateTime<Utc>,
    /// End of the busy period.
    pub end: DateTime<Utc>,
    /// Number of source streams that contributed events to this block.
    /// Set to 0 when privacy is `Opaque`.
    pub source_count: usize,
}

/// Unified availability result after merging N event streams.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedAvailability {
    /// Merged busy blocks (sorted by start, non-overlapping).
    pub busy: Vec<BusyBlock>,
    /// Free slots (gaps between busy blocks within the window).
    pub free: Vec<FreeSlot>,
    /// The analysis window start.
    pub window_start: DateTime<Utc>,
    /// The analysis window end.
    pub window_end: DateTime<Utc>,
    /// Privacy level applied to this result.
    pub privacy: PrivacyLevel,
}

/// Merge N event streams into unified availability within a time window.
///
/// All events from all streams are flattened, clipped to the window, and merged
/// into non-overlapping busy blocks. Free slots are the gaps between busy blocks.
///
/// When `privacy` is `Opaque`, `source_count` is set to 0 on all busy blocks —
/// no information about how many calendars contributed leaks through.
///
/// # Arguments
///
/// * `streams` — The event streams to merge (from different calendars/providers).
/// * `window_start` — Start of the time window to analyze.
/// * `window_end` — End of the time window to analyze.
/// * `privacy` — Controls whether source count is included in busy blocks.
pub fn merge_availability(
    streams: &[EventStream],
    window_start: DateTime<Utc>,
    window_end: DateTime<Utc>,
    privacy: PrivacyLevel,
) -> UnifiedAvailability {
    if streams.is_empty() || window_start >= window_end {
        let free = if window_start < window_end {
            vec![FreeSlot {
                start: window_start,
                end: window_end,
                duration_minutes: (window_end - window_start).num_minutes(),
            }]
        } else {
            vec![]
        };
        return UnifiedAvailability {
            busy: vec![],
            free,
            window_start,
            window_end,
            privacy,
        };
    }

    // Flatten all events from all streams into a single list.
    let all_events: Vec<ExpandedEvent> = streams
        .iter()
        .flat_map(|s| s.events.iter().cloned())
        .collect();

    // Compute merged busy periods using the existing freebusy algorithm.
    let merged_intervals = freebusy::merge_busy_periods(&all_events, window_start, window_end);

    // Build busy blocks with source count tracking.
    let busy: Vec<BusyBlock> = if privacy == PrivacyLevel::Full {
        // For Full privacy, compute source counts via sweep-line.
        compute_busy_blocks_with_sources(streams, &merged_intervals, window_start, window_end)
    } else {
        // For Opaque privacy, source_count is always 0.
        merged_intervals
            .iter()
            .map(|(start, end)| BusyBlock {
                start: *start,
                end: *end,
                source_count: 0,
            })
            .collect()
    };

    // Compute free slots from the merged intervals.
    let free = freebusy::find_free_slots(&all_events, window_start, window_end);

    UnifiedAvailability {
        busy,
        free,
        window_start,
        window_end,
        privacy,
    }
}

/// Find the first free slot of at least `min_duration_minutes` across N merged
/// event streams.
///
/// This is a convenience function that merges all streams and returns the first
/// slot meeting the minimum duration requirement.
pub fn find_first_free_across(
    streams: &[EventStream],
    window_start: DateTime<Utc>,
    window_end: DateTime<Utc>,
    min_duration_minutes: i64,
) -> Option<FreeSlot> {
    let all_events: Vec<ExpandedEvent> = streams
        .iter()
        .flat_map(|s| s.events.iter().cloned())
        .collect();

    freebusy::find_first_free_slot(&all_events, window_start, window_end, min_duration_minutes)
}

/// Compute busy blocks with per-block source counts.
///
/// For each merged interval, count how many distinct streams contributed at least
/// one event that overlaps with that interval.
fn compute_busy_blocks_with_sources(
    streams: &[EventStream],
    merged_intervals: &[(DateTime<Utc>, DateTime<Utc>)],
    window_start: DateTime<Utc>,
    window_end: DateTime<Utc>,
) -> Vec<BusyBlock> {
    merged_intervals
        .iter()
        .map(|(interval_start, interval_end)| {
            // Count how many streams have at least one event overlapping this interval.
            let source_count = streams
                .iter()
                .filter(|stream| {
                    stream.events.iter().any(|event| {
                        // Clip event to window first.
                        let ev_start = event.start.max(window_start);
                        let ev_end = event.end.min(window_end);
                        // Check overlap with the merged interval.
                        ev_start < *interval_end && ev_end > *interval_start
                    })
                })
                .count();
            BusyBlock {
                start: *interval_start,
                end: *interval_end,
                source_count,
            }
        })
        .collect()
}

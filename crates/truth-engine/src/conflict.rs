//! Detect overlapping events in expanded schedules.
//!
//! Performs pairwise comparison between two event lists to find time overlaps.
//! Adjacent events (where one ends exactly when another starts) are NOT conflicts.

use crate::expander::ExpandedEvent;

/// A detected conflict between two events.
#[derive(Debug, Clone, PartialEq)]
pub struct Conflict {
    pub event_a: ExpandedEvent,
    pub event_b: ExpandedEvent,
    pub overlap_minutes: i64,
}

/// Find all pairwise conflicts (overlapping time ranges) between two event lists.
///
/// Two events overlap when `a.start < b.end && b.start < a.end`.
/// The overlap duration is `min(a.end, b.end) - max(a.start, b.start)`.
///
/// Adjacent events where one ends exactly when another starts are NOT conflicts.
pub fn find_conflicts(events_a: &[ExpandedEvent], events_b: &[ExpandedEvent]) -> Vec<Conflict> {
    let mut conflicts = Vec::new();

    for a in events_a {
        for b in events_b {
            // Two intervals overlap iff a.start < b.end AND b.start < a.end.
            // This excludes the adjacent case where a.end == b.start.
            if a.start < b.end && b.start < a.end {
                let overlap_start = a.start.max(b.start);
                let overlap_end = a.end.min(b.end);
                let overlap_minutes = (overlap_end - overlap_start).num_minutes();

                conflicts.push(Conflict {
                    event_a: a.clone(),
                    event_b: b.clone(),
                    overlap_minutes,
                });
            }
        }
    }

    conflicts
}

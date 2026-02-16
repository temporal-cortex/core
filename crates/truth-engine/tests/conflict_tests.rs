//! Tests for conflict detection — TDD RED phase.
//!
//! All tests should compile but fail with `todo!()` panics until implementation.

use chrono::{TimeZone, Utc};
use truth_engine::expander::ExpandedEvent;
use truth_engine::find_conflicts;

/// Helper to create an ExpandedEvent from hour ranges on a given day.
fn event(
    year: i32,
    month: u32,
    day: u32,
    start_hour: u32,
    start_min: u32,
    end_hour: u32,
    end_min: u32,
) -> ExpandedEvent {
    ExpandedEvent {
        start: Utc
            .with_ymd_and_hms(year, month, day, start_hour, start_min, 0)
            .unwrap(),
        end: Utc
            .with_ymd_and_hms(year, month, day, end_hour, end_min, 0)
            .unwrap(),
    }
}

#[test]
fn two_overlapping_events_detected() {
    // Event A: 09:00-10:00, Event B: 09:30-10:30 → 30-min overlap
    let a = vec![event(2026, 3, 1, 9, 0, 10, 0)];
    let b = vec![event(2026, 3, 1, 9, 30, 10, 30)];

    let conflicts = find_conflicts(&a, &b);

    assert_eq!(conflicts.len(), 1, "should detect exactly one conflict");
    assert_eq!(conflicts[0].overlap_minutes, 30);
}

#[test]
fn non_overlapping_events_no_conflict() {
    // Event A: 09:00-10:00, Event B: 11:00-12:00 → no overlap
    let a = vec![event(2026, 3, 1, 9, 0, 10, 0)];
    let b = vec![event(2026, 3, 1, 11, 0, 12, 0)];

    let conflicts = find_conflicts(&a, &b);

    assert!(
        conflicts.is_empty(),
        "non-overlapping events should not be conflicts"
    );
}

#[test]
fn adjacent_events_not_a_conflict() {
    // Event A: 09:00-10:00, Event B: 10:00-11:00 → adjacent, NOT overlapping
    let a = vec![event(2026, 3, 1, 9, 0, 10, 0)];
    let b = vec![event(2026, 3, 1, 10, 0, 11, 0)];

    let conflicts = find_conflicts(&a, &b);

    assert!(
        conflicts.is_empty(),
        "adjacent events (end == start) should not be conflicts"
    );
}

#[test]
fn multiple_conflicts_all_found() {
    // List A has 2 events, List B has 2 events, with some overlaps
    let a = vec![
        event(2026, 3, 1, 9, 0, 10, 0),  // overlaps with b[0]
        event(2026, 3, 1, 14, 0, 15, 0), // overlaps with b[1]
    ];
    let b = vec![
        event(2026, 3, 1, 9, 30, 10, 30),  // overlaps with a[0]
        event(2026, 3, 1, 14, 30, 15, 30), // overlaps with a[1]
    ];

    let conflicts = find_conflicts(&a, &b);

    assert_eq!(conflicts.len(), 2, "should find both conflicts");

    // First conflict: 09:30-10:00 = 30 min
    assert_eq!(conflicts[0].overlap_minutes, 30);
    // Second conflict: 14:30-15:00 = 30 min
    assert_eq!(conflicts[1].overlap_minutes, 30);
}

#[test]
fn fully_contained_event_correct_overlap() {
    // Event A: 09:00-12:00 (3 hours), Event B: 10:00-11:00 (1 hour, fully inside A)
    let a = vec![event(2026, 3, 1, 9, 0, 12, 0)];
    let b = vec![event(2026, 3, 1, 10, 0, 11, 0)];

    let conflicts = find_conflicts(&a, &b);

    assert_eq!(
        conflicts.len(),
        1,
        "fully contained event should be a conflict"
    );
    assert_eq!(
        conflicts[0].overlap_minutes, 60,
        "overlap should be the duration of the smaller event (60 min)"
    );
}

#[test]
fn empty_event_lists_no_conflicts() {
    let conflicts = find_conflicts(&[], &[]);
    assert!(
        conflicts.is_empty(),
        "empty lists should produce no conflicts"
    );
}

#[test]
fn one_empty_list_no_conflicts() {
    let a = vec![event(2026, 3, 1, 9, 0, 10, 0)];
    let conflicts = find_conflicts(&a, &[]);
    assert!(
        conflicts.is_empty(),
        "one empty list should produce no conflicts"
    );
}

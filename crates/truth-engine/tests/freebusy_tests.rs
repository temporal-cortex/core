//! Tests for free/busy slot computation — TDD RED phase.
//!
//! All tests should compile but fail with `todo!()` panics until implementation.

use chrono::{TimeZone, Utc};
use truth_engine::expander::ExpandedEvent;
use truth_engine::freebusy::{find_first_free_slot, find_free_slots};

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
fn single_event_produces_two_free_slots() {
    // Window: 08:00-17:00, Event: 10:00-11:00
    // Expected free: 08:00-10:00 (120 min), 11:00-17:00 (360 min)
    let events = vec![event(2026, 3, 1, 10, 0, 11, 0)];
    let window_start = Utc.with_ymd_and_hms(2026, 3, 1, 8, 0, 0).unwrap();
    let window_end = Utc.with_ymd_and_hms(2026, 3, 1, 17, 0, 0).unwrap();

    let slots = find_free_slots(&events, window_start, window_end);

    assert_eq!(slots.len(), 2, "single event should produce 2 free slots");

    // Before event: 08:00-10:00
    assert_eq!(
        slots[0].start,
        Utc.with_ymd_and_hms(2026, 3, 1, 8, 0, 0).unwrap()
    );
    assert_eq!(
        slots[0].end,
        Utc.with_ymd_and_hms(2026, 3, 1, 10, 0, 0).unwrap()
    );
    assert_eq!(slots[0].duration_minutes, 120);

    // After event: 11:00-17:00
    assert_eq!(
        slots[1].start,
        Utc.with_ymd_and_hms(2026, 3, 1, 11, 0, 0).unwrap()
    );
    assert_eq!(
        slots[1].end,
        Utc.with_ymd_and_hms(2026, 3, 1, 17, 0, 0).unwrap()
    );
    assert_eq!(slots[1].duration_minutes, 360);
}

#[test]
fn overlapping_events_merged_correctly() {
    // Window: 08:00-17:00
    // Event A: 10:00-11:30, Event B: 11:00-12:00 → merged busy: 10:00-12:00
    // Expected free: 08:00-10:00 (120 min), 12:00-17:00 (300 min)
    let events = vec![
        event(2026, 3, 1, 10, 0, 11, 30),
        event(2026, 3, 1, 11, 0, 12, 0),
    ];
    let window_start = Utc.with_ymd_and_hms(2026, 3, 1, 8, 0, 0).unwrap();
    let window_end = Utc.with_ymd_and_hms(2026, 3, 1, 17, 0, 0).unwrap();

    let slots = find_free_slots(&events, window_start, window_end);

    assert_eq!(
        slots.len(),
        2,
        "overlapping events should merge into one busy block"
    );

    assert_eq!(
        slots[0].start,
        Utc.with_ymd_and_hms(2026, 3, 1, 8, 0, 0).unwrap()
    );
    assert_eq!(
        slots[0].end,
        Utc.with_ymd_and_hms(2026, 3, 1, 10, 0, 0).unwrap()
    );
    assert_eq!(slots[0].duration_minutes, 120);

    assert_eq!(
        slots[1].start,
        Utc.with_ymd_and_hms(2026, 3, 1, 12, 0, 0).unwrap()
    );
    assert_eq!(
        slots[1].end,
        Utc.with_ymd_and_hms(2026, 3, 1, 17, 0, 0).unwrap()
    );
    assert_eq!(slots[1].duration_minutes, 300);
}

#[test]
fn no_events_entire_window_is_free() {
    let window_start = Utc.with_ymd_and_hms(2026, 3, 1, 8, 0, 0).unwrap();
    let window_end = Utc.with_ymd_and_hms(2026, 3, 1, 17, 0, 0).unwrap();

    let slots = find_free_slots(&[], window_start, window_end);

    assert_eq!(slots.len(), 1, "no events should produce one free slot");
    assert_eq!(slots[0].start, window_start);
    assert_eq!(slots[0].end, window_end);
    assert_eq!(slots[0].duration_minutes, 540); // 9 hours
}

#[test]
fn find_first_free_slot_with_minimum_duration() {
    // Window: 08:00-17:00
    // Events: 08:00-08:30 (free: 30 min), 09:00-12:00 (gap 08:30-09:00 = 30 min)
    // First gap >= 60 min is 12:00-17:00
    let events = vec![
        event(2026, 3, 1, 8, 0, 8, 30),
        event(2026, 3, 1, 9, 0, 12, 0),
    ];
    let window_start = Utc.with_ymd_and_hms(2026, 3, 1, 8, 0, 0).unwrap();
    let window_end = Utc.with_ymd_and_hms(2026, 3, 1, 17, 0, 0).unwrap();

    let slot = find_first_free_slot(&events, window_start, window_end, 60);

    assert!(slot.is_some(), "should find a free slot of at least 60 min");
    let slot = slot.unwrap();
    assert_eq!(
        slot.start,
        Utc.with_ymd_and_hms(2026, 3, 1, 12, 0, 0).unwrap()
    );
    assert_eq!(
        slot.end,
        Utc.with_ymd_and_hms(2026, 3, 1, 17, 0, 0).unwrap()
    );
    assert_eq!(slot.duration_minutes, 300);
}

#[test]
fn events_filling_entire_window_no_free_slots() {
    // Window: 09:00-12:00, Events: 09:00-12:00 (fills entire window)
    let events = vec![event(2026, 3, 1, 9, 0, 12, 0)];
    let window_start = Utc.with_ymd_and_hms(2026, 3, 1, 9, 0, 0).unwrap();
    let window_end = Utc.with_ymd_and_hms(2026, 3, 1, 12, 0, 0).unwrap();

    let slots = find_free_slots(&events, window_start, window_end);

    assert!(
        slots.is_empty(),
        "events filling entire window should produce no free slots"
    );
}

#[test]
fn find_first_free_slot_no_gap_large_enough() {
    // Window: 09:00-12:00
    // Events: 09:00-10:00, 10:15-12:00
    // Only gap is 10:00-10:15 = 15 min, but we need 60 min
    let events = vec![
        event(2026, 3, 1, 9, 0, 10, 0),
        event(2026, 3, 1, 10, 15, 12, 0),
    ];
    let window_start = Utc.with_ymd_and_hms(2026, 3, 1, 9, 0, 0).unwrap();
    let window_end = Utc.with_ymd_and_hms(2026, 3, 1, 12, 0, 0).unwrap();

    let slot = find_first_free_slot(&events, window_start, window_end, 60);

    assert!(slot.is_none(), "no gap large enough should return None");
}

#[test]
fn multiple_gaps_between_events() {
    // Window: 08:00-18:00
    // Events: 09:00-10:00, 12:00-13:00, 15:00-16:00
    // Free: 08:00-09:00 (60), 10:00-12:00 (120), 13:00-15:00 (120), 16:00-18:00 (120)
    let events = vec![
        event(2026, 3, 1, 9, 0, 10, 0),
        event(2026, 3, 1, 12, 0, 13, 0),
        event(2026, 3, 1, 15, 0, 16, 0),
    ];
    let window_start = Utc.with_ymd_and_hms(2026, 3, 1, 8, 0, 0).unwrap();
    let window_end = Utc.with_ymd_and_hms(2026, 3, 1, 18, 0, 0).unwrap();

    let slots = find_free_slots(&events, window_start, window_end);

    assert_eq!(slots.len(), 4, "should find 4 free slots between 3 events");

    assert_eq!(slots[0].duration_minutes, 60); // 08:00-09:00
    assert_eq!(slots[1].duration_minutes, 120); // 10:00-12:00
    assert_eq!(slots[2].duration_minutes, 120); // 13:00-15:00
    assert_eq!(slots[3].duration_minutes, 120); // 16:00-18:00
}

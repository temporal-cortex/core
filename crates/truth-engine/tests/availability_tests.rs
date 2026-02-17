//! Tests for multi-stream availability merging.
//!
//! Follows TDD: tests were written first (RED), then the implementation (GREEN).

use chrono::{TimeZone, Utc};
use truth_engine::availability::{merge_availability, find_first_free_across, EventStream, PrivacyLevel};
use truth_engine::expander::ExpandedEvent;

// ── Helpers ─────────────────────────────────────────────────────────────────

fn event(start: &str, end: &str) -> ExpandedEvent {
    ExpandedEvent {
        start: start.parse().unwrap(),
        end: end.parse().unwrap(),
    }
}

fn stream(id: &str, events: Vec<ExpandedEvent>) -> EventStream {
    EventStream {
        stream_id: id.to_string(),
        events,
    }
}

// ── Test 1: Single stream matches find_free_slots ───────────────────────────

#[test]
fn single_stream_matches_find_free_slots() {
    let events = vec![
        event("2026-03-16T09:00:00Z", "2026-03-16T10:00:00Z"),
        event("2026-03-16T14:00:00Z", "2026-03-16T15:00:00Z"),
    ];
    let streams = vec![stream("work", events.clone())];

    let window_start = Utc.with_ymd_and_hms(2026, 3, 16, 8, 0, 0).unwrap();
    let window_end = Utc.with_ymd_and_hms(2026, 3, 16, 17, 0, 0).unwrap();

    let result = merge_availability(&streams, window_start, window_end, PrivacyLevel::Full);

    // Should have 2 busy blocks
    assert_eq!(result.busy.len(), 2);
    assert_eq!(result.busy[0].start, events[0].start);
    assert_eq!(result.busy[0].end, events[0].end);
    assert_eq!(result.busy[1].start, events[1].start);
    assert_eq!(result.busy[1].end, events[1].end);

    // Should have 3 free slots: 08-09, 10-14, 15-17
    assert_eq!(result.free.len(), 3);
    assert_eq!(result.free[0].duration_minutes, 60);  // 08:00-09:00
    assert_eq!(result.free[1].duration_minutes, 240); // 10:00-14:00
    assert_eq!(result.free[2].duration_minutes, 120); // 15:00-17:00

    // Compare against direct find_free_slots
    let direct_free = truth_engine::find_free_slots(&events, window_start, window_end);
    assert_eq!(result.free, direct_free);
}

// ── Test 2: Two non-overlapping streams ─────────────────────────────────────

#[test]
fn two_non_overlapping_streams_merge_all_busy_blocks() {
    let stream_a = stream("work", vec![
        event("2026-03-16T09:00:00Z", "2026-03-16T10:00:00Z"),
    ]);
    let stream_b = stream("personal", vec![
        event("2026-03-16T14:00:00Z", "2026-03-16T15:00:00Z"),
    ]);

    let window_start = Utc.with_ymd_and_hms(2026, 3, 16, 8, 0, 0).unwrap();
    let window_end = Utc.with_ymd_and_hms(2026, 3, 16, 17, 0, 0).unwrap();

    let result = merge_availability(
        &[stream_a, stream_b],
        window_start,
        window_end,
        PrivacyLevel::Full,
    );

    // Two separate busy blocks
    assert_eq!(result.busy.len(), 2);
    assert_eq!(result.busy[0].source_count, 1); // Only work stream
    assert_eq!(result.busy[1].source_count, 1); // Only personal stream

    // 3 free slots: 08-09, 10-14, 15-17
    assert_eq!(result.free.len(), 3);
}

// ── Test 3: Two overlapping streams ─────────────────────────────────────────

#[test]
fn two_overlapping_streams_merge_with_source_count_2() {
    let stream_a = stream("work", vec![
        event("2026-03-16T09:00:00Z", "2026-03-16T11:00:00Z"),
    ]);
    let stream_b = stream("personal", vec![
        event("2026-03-16T10:00:00Z", "2026-03-16T12:00:00Z"),
    ]);

    let window_start = Utc.with_ymd_and_hms(2026, 3, 16, 8, 0, 0).unwrap();
    let window_end = Utc.with_ymd_and_hms(2026, 3, 16, 17, 0, 0).unwrap();

    let result = merge_availability(
        &[stream_a, stream_b],
        window_start,
        window_end,
        PrivacyLevel::Full,
    );

    // Should merge into a single busy block 09:00-12:00
    assert_eq!(result.busy.len(), 1);
    assert_eq!(
        result.busy[0].start,
        Utc.with_ymd_and_hms(2026, 3, 16, 9, 0, 0).unwrap()
    );
    assert_eq!(
        result.busy[0].end,
        Utc.with_ymd_and_hms(2026, 3, 16, 12, 0, 0).unwrap()
    );
    // Both streams contributed
    assert_eq!(result.busy[0].source_count, 2);

    // 2 free slots: 08-09 (60 min) and 12-17 (300 min)
    assert_eq!(result.free.len(), 2);
    assert_eq!(result.free[0].duration_minutes, 60);
    assert_eq!(result.free[1].duration_minutes, 300);
}

// ── Test 4: Three streams with cascading overlaps ───────────────────────────

#[test]
fn three_streams_cascading_overlaps() {
    let stream_a = stream("work", vec![
        event("2026-03-16T09:00:00Z", "2026-03-16T10:30:00Z"),
    ]);
    let stream_b = stream("personal", vec![
        event("2026-03-16T10:00:00Z", "2026-03-16T11:30:00Z"),
    ]);
    let stream_c = stream("sideproject", vec![
        event("2026-03-16T11:00:00Z", "2026-03-16T12:00:00Z"),
    ]);

    let window_start = Utc.with_ymd_and_hms(2026, 3, 16, 8, 0, 0).unwrap();
    let window_end = Utc.with_ymd_and_hms(2026, 3, 16, 17, 0, 0).unwrap();

    let result = merge_availability(
        &[stream_a, stream_b, stream_c],
        window_start,
        window_end,
        PrivacyLevel::Full,
    );

    // All three cascade into one merged block: 09:00-12:00
    assert_eq!(result.busy.len(), 1);
    assert_eq!(
        result.busy[0].start,
        Utc.with_ymd_and_hms(2026, 3, 16, 9, 0, 0).unwrap()
    );
    assert_eq!(
        result.busy[0].end,
        Utc.with_ymd_and_hms(2026, 3, 16, 12, 0, 0).unwrap()
    );
    // All three streams contributed
    assert_eq!(result.busy[0].source_count, 3);
}

// ── Test 5: Empty streams → full-window free slot ───────────────────────────

#[test]
fn empty_streams_produce_full_window_free_slot() {
    let window_start = Utc.with_ymd_and_hms(2026, 3, 16, 8, 0, 0).unwrap();
    let window_end = Utc.with_ymd_and_hms(2026, 3, 16, 17, 0, 0).unwrap();

    // No streams at all
    let result = merge_availability(&[], window_start, window_end, PrivacyLevel::Full);
    assert_eq!(result.busy.len(), 0);
    assert_eq!(result.free.len(), 1);
    assert_eq!(result.free[0].duration_minutes, 540); // 9 hours

    // Streams with no events
    let empty_a = stream("work", vec![]);
    let empty_b = stream("personal", vec![]);
    let result = merge_availability(
        &[empty_a, empty_b],
        window_start,
        window_end,
        PrivacyLevel::Full,
    );
    assert_eq!(result.busy.len(), 0);
    assert_eq!(result.free.len(), 1);
    assert_eq!(result.free[0].duration_minutes, 540);
}

// ── Test 6: Privacy Opaque sets source_count to 0 ───────────────────────────

#[test]
fn opaque_privacy_hides_source_count() {
    let stream_a = stream("work", vec![
        event("2026-03-16T09:00:00Z", "2026-03-16T10:00:00Z"),
    ]);
    let stream_b = stream("personal", vec![
        event("2026-03-16T09:30:00Z", "2026-03-16T10:30:00Z"),
    ]);

    let window_start = Utc.with_ymd_and_hms(2026, 3, 16, 8, 0, 0).unwrap();
    let window_end = Utc.with_ymd_and_hms(2026, 3, 16, 17, 0, 0).unwrap();

    let result = merge_availability(
        &[stream_a, stream_b],
        window_start,
        window_end,
        PrivacyLevel::Opaque,
    );

    assert_eq!(result.privacy, PrivacyLevel::Opaque);
    // source_count should be 0 for all blocks in Opaque mode
    for block in &result.busy {
        assert_eq!(block.source_count, 0, "Opaque mode must hide source count");
    }
}

// ── Test 7: find_first_free_across respects min_duration ────────────────────

#[test]
fn find_first_free_across_respects_min_duration() {
    // Create events leaving only short gaps:
    // Stream A: 09:00-09:45 (leaves 15 min gap)
    // Stream B: 10:00-12:00
    let stream_a = stream("work", vec![
        event("2026-03-16T09:00:00Z", "2026-03-16T09:45:00Z"),
    ]);
    let stream_b = stream("personal", vec![
        event("2026-03-16T10:00:00Z", "2026-03-16T12:00:00Z"),
    ]);

    let window_start = Utc.with_ymd_and_hms(2026, 3, 16, 9, 0, 0).unwrap();
    let window_end = Utc.with_ymd_and_hms(2026, 3, 16, 17, 0, 0).unwrap();

    // Looking for 60 min slot — the 15 min gap should be skipped
    let slot = find_first_free_across(
        &[stream_a, stream_b],
        window_start,
        window_end,
        60,
    );
    assert!(slot.is_some());
    let slot = slot.unwrap();
    assert_eq!(
        slot.start,
        Utc.with_ymd_and_hms(2026, 3, 16, 12, 0, 0).unwrap()
    );
    assert_eq!(slot.duration_minutes, 300); // 12:00-17:00
}

// ── Test 8: Events outside window are clipped/ignored ───────────────────────

#[test]
fn events_outside_window_are_clipped() {
    let stream_a = stream("work", vec![
        // Starts before window, ends inside
        event("2026-03-16T07:00:00Z", "2026-03-16T09:30:00Z"),
        // Entirely inside window
        event("2026-03-16T14:00:00Z", "2026-03-16T15:00:00Z"),
        // Starts inside window, ends after
        event("2026-03-16T16:30:00Z", "2026-03-16T18:00:00Z"),
        // Entirely outside window — should be ignored
        event("2026-03-16T20:00:00Z", "2026-03-16T21:00:00Z"),
    ]);

    let window_start = Utc.with_ymd_and_hms(2026, 3, 16, 8, 0, 0).unwrap();
    let window_end = Utc.with_ymd_and_hms(2026, 3, 16, 17, 0, 0).unwrap();

    let result = merge_availability(&[stream_a], window_start, window_end, PrivacyLevel::Full);

    // 3 busy blocks (the one outside window is ignored)
    assert_eq!(result.busy.len(), 3);

    // First block clipped to window start
    assert_eq!(result.busy[0].start, window_start);
    assert_eq!(
        result.busy[0].end,
        Utc.with_ymd_and_hms(2026, 3, 16, 9, 30, 0).unwrap()
    );

    // Last block clipped to window end
    assert_eq!(
        result.busy[2].start,
        Utc.with_ymd_and_hms(2026, 3, 16, 16, 30, 0).unwrap()
    );
    assert_eq!(result.busy[2].end, window_end);
}

// ── Test 9: All-day event across multiple streams ───────────────────────────

#[test]
fn all_day_event_across_streams() {
    // Stream A has an all-day event (08:00-17:00)
    let stream_a = stream("work", vec![
        event("2026-03-16T08:00:00Z", "2026-03-16T17:00:00Z"),
    ]);
    // Stream B has a partial event
    let stream_b = stream("personal", vec![
        event("2026-03-16T12:00:00Z", "2026-03-16T13:00:00Z"),
    ]);

    let window_start = Utc.with_ymd_and_hms(2026, 3, 16, 8, 0, 0).unwrap();
    let window_end = Utc.with_ymd_and_hms(2026, 3, 16, 17, 0, 0).unwrap();

    let result = merge_availability(
        &[stream_a, stream_b],
        window_start,
        window_end,
        PrivacyLevel::Full,
    );

    // Should be entirely busy — one block spanning the whole window
    assert_eq!(result.busy.len(), 1);
    assert_eq!(result.busy[0].start, window_start);
    assert_eq!(result.busy[0].end, window_end);
    assert_eq!(result.busy[0].source_count, 2); // Both streams contributed

    // No free slots
    assert_eq!(result.free.len(), 0);
}

// ── Test 10: Window metadata preserved ──────────────────────────────────────

#[test]
fn window_metadata_preserved() {
    let window_start = Utc.with_ymd_and_hms(2026, 3, 16, 8, 0, 0).unwrap();
    let window_end = Utc.with_ymd_and_hms(2026, 3, 16, 17, 0, 0).unwrap();

    let result = merge_availability(&[], window_start, window_end, PrivacyLevel::Opaque);

    assert_eq!(result.window_start, window_start);
    assert_eq!(result.window_end, window_end);
    assert_eq!(result.privacy, PrivacyLevel::Opaque);
}

// ── Test 11: Multiple events in single stream merge correctly ───────────────

#[test]
fn multiple_events_in_single_stream_merge() {
    let stream_a = stream("work", vec![
        event("2026-03-16T09:00:00Z", "2026-03-16T10:00:00Z"),
        event("2026-03-16T09:30:00Z", "2026-03-16T10:30:00Z"), // Overlaps with previous
        event("2026-03-16T14:00:00Z", "2026-03-16T15:00:00Z"),
    ]);

    let window_start = Utc.with_ymd_and_hms(2026, 3, 16, 8, 0, 0).unwrap();
    let window_end = Utc.with_ymd_and_hms(2026, 3, 16, 17, 0, 0).unwrap();

    let result = merge_availability(&[stream_a], window_start, window_end, PrivacyLevel::Full);

    // First two events merge into 09:00-10:30, third is separate
    assert_eq!(result.busy.len(), 2);
    assert_eq!(
        result.busy[0].end,
        Utc.with_ymd_and_hms(2026, 3, 16, 10, 30, 0).unwrap()
    );
}

// ── Test 12: find_first_free_across with no qualifying slot ─────────────────

#[test]
fn find_first_free_across_no_qualifying_slot() {
    // Fill the entire window with events
    let stream_a = stream("work", vec![
        event("2026-03-16T08:00:00Z", "2026-03-16T17:00:00Z"),
    ]);

    let window_start = Utc.with_ymd_and_hms(2026, 3, 16, 8, 0, 0).unwrap();
    let window_end = Utc.with_ymd_and_hms(2026, 3, 16, 17, 0, 0).unwrap();

    let slot = find_first_free_across(&[stream_a], window_start, window_end, 30);
    assert!(slot.is_none());
}

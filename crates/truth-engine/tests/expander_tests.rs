//! Tests for RRULE expansion — TDD RED phase.
//!
//! All tests should compile but fail with `todo!()` panics until implementation.

use chrono::{TimeZone, Timelike, Utc};
use truth_engine::expand_rrule;

// ---------------------------------------------------------------------------
// CTO's exact example: 3rd Tuesday of each month, America/Los_Angeles
// ---------------------------------------------------------------------------

#[test]
fn third_tuesday_monthly_first_three_instances() {
    // FREQ=MONTHLY;BYDAY=TU;BYSETPOS=3 → 3rd Tuesday of each month
    // Starting 2026-02-17 at 14:00 PST (UTC-8)
    let result = expand_rrule(
        "FREQ=MONTHLY;BYDAY=TU;BYSETPOS=3",
        "2026-02-17T14:00:00",
        60,
        "America/Los_Angeles",
        None,
        Some(3),
    )
    .expect("should expand successfully");

    assert_eq!(result.len(), 3, "should produce exactly 3 instances");

    // Feb 2026: 3rd Tuesday is Feb 17
    // 14:00 PST = 22:00 UTC (UTC-8)
    assert_eq!(
        result[0].start,
        Utc.with_ymd_and_hms(2026, 2, 17, 22, 0, 0).unwrap()
    );
    assert_eq!(
        result[0].end,
        Utc.with_ymd_and_hms(2026, 2, 17, 23, 0, 0).unwrap()
    );

    // Mar 2026: 3rd Tuesday is Mar 17
    // 14:00 PDT = 21:00 UTC (UTC-7) — DST started Mar 8
    assert_eq!(
        result[1].start,
        Utc.with_ymd_and_hms(2026, 3, 17, 21, 0, 0).unwrap()
    );
    assert_eq!(
        result[1].end,
        Utc.with_ymd_and_hms(2026, 3, 17, 22, 0, 0).unwrap()
    );

    // Apr 2026: 3rd Tuesday is Apr 21
    // 14:00 PDT = 21:00 UTC (UTC-7)
    assert_eq!(
        result[2].start,
        Utc.with_ymd_and_hms(2026, 4, 21, 21, 0, 0).unwrap()
    );
    assert_eq!(
        result[2].end,
        Utc.with_ymd_and_hms(2026, 4, 21, 22, 0, 0).unwrap()
    );
}

#[test]
fn dst_transition_shifts_utc_offset() {
    // Feb is PST (UTC-8), Mar is PDT (UTC-7) after spring forward on Mar 8
    // Local time stays 14:00, but UTC representation changes
    let result = expand_rrule(
        "FREQ=MONTHLY;BYDAY=TU;BYSETPOS=3",
        "2026-02-17T14:00:00",
        60,
        "America/Los_Angeles",
        None,
        Some(2),
    )
    .expect("should expand successfully");

    assert_eq!(result.len(), 2);

    // Feb: 14:00 PST → UTC offset is -8 → 22:00 UTC
    let feb_utc_hour = result[0].start.hour();
    assert_eq!(feb_utc_hour, 22, "Feb should be 22:00 UTC (PST, UTC-8)");

    // Mar: 14:00 PDT → UTC offset is -7 → 21:00 UTC
    let mar_utc_hour = result[1].start.hour();
    assert_eq!(mar_utc_hour, 21, "Mar should be 21:00 UTC (PDT, UTC-7)");
}

// ---------------------------------------------------------------------------
// Basic RRULE tests
// ---------------------------------------------------------------------------

#[test]
fn daily_count_five() {
    let result = expand_rrule(
        "FREQ=DAILY;COUNT=5",
        "2026-03-01T09:00:00",
        30,
        "UTC",
        None,
        None, // COUNT is in the RRULE itself
    )
    .expect("should expand successfully");

    assert_eq!(
        result.len(),
        5,
        "FREQ=DAILY;COUNT=5 should produce 5 instances"
    );

    // Check consecutive days
    for (i, instance) in result.iter().enumerate().take(5) {
        let day = 1 + i as u32;
        let expected_start = Utc.with_ymd_and_hms(2026, 3, day, 9, 0, 0).unwrap();
        assert_eq!(instance.start, expected_start, "day {} mismatch", i);
        // 30-minute duration
        let expected_end = Utc.with_ymd_and_hms(2026, 3, day, 9, 30, 0).unwrap();
        assert_eq!(instance.end, expected_end, "day {} end mismatch", i);
    }
}

#[test]
fn weekly_mon_wed_fri_count_six() {
    let result = expand_rrule(
        "FREQ=WEEKLY;BYDAY=MO,WE,FR;COUNT=6",
        "2026-03-02T10:00:00", // Monday
        45,
        "UTC",
        None,
        None,
    )
    .expect("should expand successfully");

    assert_eq!(result.len(), 6, "should produce 6 instances");

    // Week 1: Mon Mar 2, Wed Mar 4, Fri Mar 6
    assert_eq!(
        result[0].start,
        Utc.with_ymd_and_hms(2026, 3, 2, 10, 0, 0).unwrap()
    );
    assert_eq!(
        result[1].start,
        Utc.with_ymd_and_hms(2026, 3, 4, 10, 0, 0).unwrap()
    );
    assert_eq!(
        result[2].start,
        Utc.with_ymd_and_hms(2026, 3, 6, 10, 0, 0).unwrap()
    );

    // Week 2: Mon Mar 9, Wed Mar 11, Fri Mar 13
    assert_eq!(
        result[3].start,
        Utc.with_ymd_and_hms(2026, 3, 9, 10, 0, 0).unwrap()
    );
    assert_eq!(
        result[4].start,
        Utc.with_ymd_and_hms(2026, 3, 11, 10, 0, 0).unwrap()
    );
    assert_eq!(
        result[5].start,
        Utc.with_ymd_and_hms(2026, 3, 13, 10, 0, 0).unwrap()
    );
}

#[test]
fn biweekly_tue_thu() {
    // Every other week on Tuesday and Thursday
    let result = expand_rrule(
        "FREQ=WEEKLY;INTERVAL=2;BYDAY=TU,TH",
        "2026-03-03T11:00:00", // Tuesday
        60,
        "UTC",
        None,
        Some(4),
    )
    .expect("should expand successfully");

    assert_eq!(result.len(), 4, "should produce 4 instances");

    // Week 1: Tue Mar 3, Thu Mar 5
    assert_eq!(
        result[0].start,
        Utc.with_ymd_and_hms(2026, 3, 3, 11, 0, 0).unwrap()
    );
    assert_eq!(
        result[1].start,
        Utc.with_ymd_and_hms(2026, 3, 5, 11, 0, 0).unwrap()
    );

    // Skip week 2, Week 3: Tue Mar 17, Thu Mar 19
    assert_eq!(
        result[2].start,
        Utc.with_ymd_and_hms(2026, 3, 17, 11, 0, 0).unwrap()
    );
    assert_eq!(
        result[3].start,
        Utc.with_ymd_and_hms(2026, 3, 19, 11, 0, 0).unwrap()
    );
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

#[test]
fn empty_rrule_returns_error() {
    let result = expand_rrule("", "2026-03-01T09:00:00", 30, "UTC", None, None);
    assert!(result.is_err(), "empty RRULE should return an error");
}

#[test]
fn invalid_timezone_returns_error() {
    let result = expand_rrule(
        "FREQ=DAILY;COUNT=1",
        "2026-03-01T09:00:00",
        30,
        "Mars/Olympus_Mons",
        None,
        None,
    );
    assert!(result.is_err(), "invalid timezone should return an error");
}

#[test]
fn count_zero_returns_empty() {
    let result = expand_rrule(
        "FREQ=DAILY",
        "2026-03-01T09:00:00",
        30,
        "UTC",
        None,
        Some(0),
    )
    .expect("COUNT=0 should succeed with empty result");

    assert!(result.is_empty(), "COUNT=0 should produce no instances");
}

#[test]
fn single_instance_count_one() {
    let result = expand_rrule(
        "FREQ=DAILY",
        "2026-03-01T09:00:00",
        60,
        "UTC",
        None,
        Some(1),
    )
    .expect("COUNT=1 should succeed");

    assert_eq!(result.len(), 1, "COUNT=1 should produce exactly 1 instance");
    assert_eq!(
        result[0].start,
        Utc.with_ymd_and_hms(2026, 3, 1, 9, 0, 0).unwrap()
    );
    assert_eq!(
        result[0].end,
        Utc.with_ymd_and_hms(2026, 3, 1, 10, 0, 0).unwrap()
    );
}

// ---------------------------------------------------------------------------
// Until boundary
// ---------------------------------------------------------------------------

#[test]
fn until_boundary_limits_expansion() {
    // Daily from Mar 1, but only until Mar 4 → should get Mar 1, 2, 3, 4
    let result = expand_rrule(
        "FREQ=DAILY",
        "2026-03-01T09:00:00",
        30,
        "UTC",
        Some("2026-03-04T23:59:59"),
        None,
    )
    .expect("should expand with until boundary");

    assert_eq!(result.len(), 4, "should produce 4 instances (Mar 1-4)");
    assert_eq!(
        result[3].start,
        Utc.with_ymd_and_hms(2026, 3, 4, 9, 0, 0).unwrap()
    );
}

// ---------------------------------------------------------------------------
// Duration correctness
// ---------------------------------------------------------------------------

#[test]
fn duration_applied_correctly() {
    let result = expand_rrule(
        "FREQ=DAILY;COUNT=1",
        "2026-03-01T09:00:00",
        90, // 1.5 hours
        "UTC",
        None,
        None,
    )
    .expect("should expand");

    assert_eq!(result.len(), 1);
    assert_eq!(
        result[0].start,
        Utc.with_ymd_and_hms(2026, 3, 1, 9, 0, 0).unwrap()
    );
    assert_eq!(
        result[0].end,
        Utc.with_ymd_and_hms(2026, 3, 1, 10, 30, 0).unwrap()
    );
}

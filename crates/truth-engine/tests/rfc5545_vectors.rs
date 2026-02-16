//! RFC 5545 compliance test vectors — derived from Section 3.8.5 examples.
//!
//! These tests supplement the 11 expander tests with additional RFC-derived
//! recurrence rule patterns: bi-weekly multi-day, yearly, leap year, EXDATE,
//! COUNT, INTERVAL, BYSETPOS, and multi-rule intersections.

use chrono::Datelike;
use truth_engine::{expand_rrule, ExpandedEvent};

// ---------------------------------------------------------------------------
// Helper: extract (year, month, day) tuples from expanded events
// ---------------------------------------------------------------------------

fn dates(events: &[ExpandedEvent]) -> Vec<(i32, u32, u32)> {
    events
        .iter()
        .map(|e| (e.start.year(), e.start.month(), e.start.day()))
        .collect()
}

// ===========================================================================
// 1. Every other week on Tuesday and Thursday (INTERVAL=2, multi-BYDAY)
// ===========================================================================

#[test]
fn biweekly_tue_thu_alternating_weeks() {
    // Start on Tue 2026-01-06, expand 8 instances.
    // Week of Jan 5:  Tue Jan 6, Thu Jan 8
    // Skip week of Jan 12
    // Week of Jan 19: Tue Jan 20, Thu Jan 22
    // Skip week of Jan 26
    // Week of Feb 2:  Tue Feb 3, Thu Feb 5
    // Skip week of Feb 9
    // Week of Feb 16: Tue Feb 17, Thu Feb 19
    let result = expand_rrule(
        "FREQ=WEEKLY;INTERVAL=2;BYDAY=TU,TH",
        "2026-01-06T10:00:00",
        60,
        "UTC",
        None,
        Some(8),
    )
    .expect("should expand biweekly TU,TH");

    assert_eq!(result.len(), 8, "should produce 8 instances");

    let d = dates(&result);
    // Week 1 (Jan 5)
    assert_eq!(d[0], (2026, 1, 6), "1st: Tue Jan 6");
    assert_eq!(d[1], (2026, 1, 8), "2nd: Thu Jan 8");
    // Skip Jan 12 week
    // Week 3 (Jan 19)
    assert_eq!(d[2], (2026, 1, 20), "3rd: Tue Jan 20");
    assert_eq!(d[3], (2026, 1, 22), "4th: Thu Jan 22");
    // Skip Jan 26 week
    // Week 5 (Feb 2)
    assert_eq!(d[4], (2026, 2, 3), "5th: Tue Feb 3");
    assert_eq!(d[5], (2026, 2, 5), "6th: Thu Feb 5");
    // Skip Feb 9 week
    // Week 7 (Feb 16)
    assert_eq!(d[6], (2026, 2, 17), "7th: Tue Feb 17");
    assert_eq!(d[7], (2026, 2, 19), "8th: Thu Feb 19");
}

// ===========================================================================
// 2. Yearly by month and day — June 15 every year
// ===========================================================================

#[test]
fn yearly_june_15() {
    let result = expand_rrule(
        "FREQ=YEARLY;BYMONTH=6;BYMONTHDAY=15",
        "2026-06-15T12:00:00",
        60,
        "UTC",
        None,
        Some(4),
    )
    .expect("should expand yearly June 15");

    assert_eq!(result.len(), 4);

    let d = dates(&result);
    assert_eq!(d[0], (2026, 6, 15));
    assert_eq!(d[1], (2027, 6, 15));
    assert_eq!(d[2], (2028, 6, 15));
    assert_eq!(d[3], (2029, 6, 15));
}

// ===========================================================================
// 3. Leap year Feb 29 handling — skips non-leap years
// ===========================================================================

#[test]
fn leap_year_feb_29() {
    // FREQ=YEARLY;BYMONTH=2;BYMONTHDAY=29
    // Start on 2024-02-29 (leap year). Next leap year is 2028.
    // 2025, 2026, 2027 are NOT leap years → no Feb 29.
    // 2028, 2032 are leap years.
    let result = expand_rrule(
        "FREQ=YEARLY;BYMONTH=2;BYMONTHDAY=29",
        "2024-02-29T08:00:00",
        60,
        "UTC",
        None,
        Some(3),
    )
    .expect("should expand yearly Feb 29");

    assert_eq!(result.len(), 3, "should produce 3 leap-year instances");

    let d = dates(&result);
    assert_eq!(d[0], (2024, 2, 29), "first: 2024 leap year");
    assert_eq!(
        d[1],
        (2028, 2, 29),
        "second: 2028 leap year (skipped 2025-2027)"
    );
    assert_eq!(
        d[2],
        (2032, 2, 29),
        "third: 2032 leap year (skipped 2029-2031)"
    );
}

// ===========================================================================
// 4. EXDATE exclusions — weekly Tuesday with 3 excluded dates
// ===========================================================================

#[test]
fn exdate_excludes_specific_dates() {
    // Weekly on Tuesday starting 2026-03-03, expand over 6 weeks.
    // Exclude Mar 10, Mar 17, Mar 31.
    // Expected: Mar 3, Mar 24, Apr 7 (the 3 non-excluded Tuesdays in range)
    //
    // Uses expand_rrule_with_exdates (new API with EXDATE support).
    let result = truth_engine::expander::expand_rrule_with_exdates(
        "FREQ=WEEKLY;BYDAY=TU",
        "2026-03-03T10:00:00",
        60,
        "UTC",
        Some("2026-04-07T23:59:59"),
        None,
        &[
            "2026-03-10T10:00:00",
            "2026-03-17T10:00:00",
            "2026-03-31T10:00:00",
        ],
    )
    .expect("should expand with exdates");

    let d = dates(&result);
    assert_eq!(d.len(), 3, "6 Tuesdays minus 3 excluded = 3 remaining");
    assert_eq!(d[0], (2026, 3, 3), "Tue Mar 3 — kept");
    assert_eq!(d[1], (2026, 3, 24), "Tue Mar 24 — kept");
    assert_eq!(d[2], (2026, 4, 7), "Tue Apr 7 — kept");

    // Explicitly assert excluded dates are absent.
    let excluded = vec![(2026, 3, 10), (2026, 3, 17), (2026, 3, 31)];
    for ex in &excluded {
        assert!(
            !d.contains(ex),
            "date {:?} should be excluded by EXDATE",
            ex
        );
    }
}

// ===========================================================================
// 5. COUNT limit — FREQ=DAILY;COUNT=5 produces exactly 5 instances
// ===========================================================================

#[test]
fn rfc5545_count_limit_daily_five() {
    // COUNT=5 inside the RRULE string itself (no external count override).
    let result = expand_rrule(
        "FREQ=DAILY;COUNT=5",
        "2026-06-01T08:00:00",
        45,
        "America/New_York",
        None,
        None,
    )
    .expect("should expand daily count 5");

    assert_eq!(result.len(), 5, "COUNT=5 must produce exactly 5 instances");

    // Jun 1 2026 is EDT (UTC-4), so 08:00 EDT = 12:00 UTC.
    let d = dates(&result);
    for i in 0..5u32 {
        assert_eq!(d[i as usize], (2026, 6, 1 + i), "day {} mismatch", i);
    }
    // Double check: day 0 → Jun 1, day 4 → Jun 5
    assert_eq!(d[0], (2026, 6, 1));
    assert_eq!(d[4], (2026, 6, 5));
}

// ===========================================================================
// 6. INTERVAL — FREQ=MONTHLY;INTERVAL=3 (quarterly)
// ===========================================================================

#[test]
fn monthly_interval_three_quarterly() {
    // Starting Mar 15 2026, every 3 months → Mar, Jun, Sep, Dec, Mar...
    let result = expand_rrule(
        "FREQ=MONTHLY;INTERVAL=3",
        "2026-03-15T09:00:00",
        60,
        "UTC",
        None,
        Some(5),
    )
    .expect("should expand quarterly");

    assert_eq!(result.len(), 5);

    let d = dates(&result);
    assert_eq!(d[0], (2026, 3, 15), "Mar 2026");
    assert_eq!(d[1], (2026, 6, 15), "Jun 2026");
    assert_eq!(d[2], (2026, 9, 15), "Sep 2026");
    assert_eq!(d[3], (2026, 12, 15), "Dec 2026");
    assert_eq!(d[4], (2027, 3, 15), "Mar 2027");
}

// ===========================================================================
// 7. BYSETPOS + BYDAY — last weekday of each month
// ===========================================================================

#[test]
fn last_weekday_of_month_bysetpos_neg1() {
    // BYDAY=MO,TU,WE,TH,FR;BYSETPOS=-1 → last weekday of each month.
    // Starting Jan 2026, expand 6 months.
    //
    // Expected last weekdays:
    //   Jan 2026: Fri Jan 30
    //   Feb 2026: Fri Feb 27
    //   Mar 2026: Tue Mar 31
    //   Apr 2026: Thu Apr 30
    //   May 2026: Fri May 29
    //   Jun 2026: Tue Jun 30
    let result = expand_rrule(
        "FREQ=MONTHLY;BYDAY=MO,TU,WE,TH,FR;BYSETPOS=-1",
        "2026-01-30T17:00:00",
        60,
        "UTC",
        None,
        Some(6),
    )
    .expect("should expand last weekday of month");

    assert_eq!(result.len(), 6);

    let d = dates(&result);
    assert_eq!(d[0], (2026, 1, 30), "Last weekday of Jan 2026 = Fri 30");
    assert_eq!(d[1], (2026, 2, 27), "Last weekday of Feb 2026 = Fri 27");
    assert_eq!(d[2], (2026, 3, 31), "Last weekday of Mar 2026 = Tue 31");
    assert_eq!(d[3], (2026, 4, 30), "Last weekday of Apr 2026 = Thu 30");
    assert_eq!(d[4], (2026, 5, 29), "Last weekday of May 2026 = Fri 29");
    assert_eq!(d[5], (2026, 6, 30), "Last weekday of Jun 2026 = Tue 30");
}

// ===========================================================================
// 8. Multi-rule intersection: BYMONTH + BYDAY + BYSETPOS
//    Second Tuesday of January and June only
// ===========================================================================

#[test]
fn second_tuesday_of_jan_and_jun() {
    // FREQ=MONTHLY with BYMONTH=1,6 restricts which months fire.
    // BYDAY=TU;BYSETPOS=2 selects the 2nd Tuesday within each month.
    //
    // (Using FREQ=YEARLY with BYSETPOS would pick from the combined yearly set,
    // not per-month. FREQ=MONTHLY is the correct way per RFC 5545.)
    //
    // Jan 2026: Tuesdays are 6, 13, 20, 27 → 2nd Tue = Jan 13
    // Jun 2026: Tuesdays are 2, 9, 16, 23, 30 → 2nd Tue = Jun 9
    // Jan 2027: Tuesdays are 5, 12, 19, 26 → 2nd Tue = Jan 12
    // Jun 2027: Tuesdays are 1, 8, 15, 22, 29 → 2nd Tue = Jun 8
    let result = expand_rrule(
        "FREQ=MONTHLY;BYMONTH=1,6;BYDAY=TU;BYSETPOS=2",
        "2026-01-13T14:00:00",
        60,
        "UTC",
        None,
        Some(4),
    )
    .expect("should expand 2nd Tuesday of Jan & Jun");

    assert_eq!(result.len(), 4);

    let d = dates(&result);
    assert_eq!(d[0], (2026, 1, 13), "2nd Tue of Jan 2026");
    assert_eq!(d[1], (2026, 6, 9), "2nd Tue of Jun 2026");
    assert_eq!(d[2], (2027, 1, 12), "2nd Tue of Jan 2027");
    assert_eq!(d[3], (2027, 6, 8), "2nd Tue of Jun 2027");
}

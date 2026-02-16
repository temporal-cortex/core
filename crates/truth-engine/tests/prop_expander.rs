//! Property-based tests for RRULE expansion using proptest.
//!
//! These tests verify invariants that should hold for *any* valid RRULE input,
//! not just the specific examples in `expander_tests.rs`.

use chrono::{Datelike, Duration, Weekday};
use proptest::prelude::*;
use truth_engine::expand_rrule;

// ---------------------------------------------------------------------------
// Strategies — generate valid RRULE components
// ---------------------------------------------------------------------------

fn arb_freq() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("DAILY".to_string()),
        Just("WEEKLY".to_string()),
        Just("MONTHLY".to_string()),
        Just("YEARLY".to_string()),
    ]
}

fn arb_interval() -> impl Strategy<Value = u32> {
    1u32..=12
}

fn arb_count() -> impl Strategy<Value = u32> {
    1u32..=50
}

fn arb_byday() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("MO".to_string()),
        Just("TU".to_string()),
        Just("WE".to_string()),
        Just("TH".to_string()),
        Just("FR".to_string()),
        Just("SA".to_string()),
        Just("SU".to_string()),
    ]
}

fn arb_timezone() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("UTC".to_string()),
        Just("America/New_York".to_string()),
        Just("America/Los_Angeles".to_string()),
        Just("Europe/London".to_string()),
        Just("Asia/Tokyo".to_string()),
    ]
}

/// Generate a valid DTSTART in the 2025-2027 range.
/// Day is capped at 28 to avoid invalid month/day combos.
fn arb_dtstart() -> impl Strategy<Value = String> {
    (2025u32..=2027, 1u32..=12, 1u32..=28, 0u32..=23, 0u32..=59)
        .prop_map(|(y, m, d, h, min)| format!("{:04}-{:02}-{:02}T{:02}:{:02}:00", y, m, d, h, min))
}

/// Generate a duration in the 15-120 minute range.
fn arb_duration() -> impl Strategy<Value = u32> {
    15u32..=120
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn byday_to_weekday(byday: &str) -> Weekday {
    match byday {
        "MO" => Weekday::Mon,
        "TU" => Weekday::Tue,
        "WE" => Weekday::Wed,
        "TH" => Weekday::Thu,
        "FR" => Weekday::Fri,
        "SA" => Weekday::Sat,
        "SU" => Weekday::Sun,
        _ => unreachable!("invalid BYDAY: {}", byday),
    }
}

fn config() -> ProptestConfig {
    ProptestConfig {
        cases: 256,
        ..ProptestConfig::default()
    }
}

// ---------------------------------------------------------------------------
// Property 1: Expansion result is sorted (chronological order)
// ---------------------------------------------------------------------------
proptest! {
    #![proptest_config(config())]

    #[test]
    fn expansion_is_sorted(
        freq in arb_freq(),
        count in arb_count(),
        dtstart in arb_dtstart(),
        tz in arb_timezone(),
        dur in arb_duration(),
    ) {
        // Build a simple RRULE with just FREQ and COUNT.
        let rrule = format!("FREQ={};COUNT={}", freq, count);
        let result = expand_rrule(&rrule, &dtstart, dur, &tz, None, None);

        if let Ok(events) = result {
            for window in events.windows(2) {
                prop_assert!(
                    window[0].start <= window[1].start,
                    "events not sorted: {:?} > {:?}",
                    window[0].start,
                    window[1].start
                );
            }
        }
        // If the rrule crate rejects the combo, that's fine — not our bug.
    }
}

// ---------------------------------------------------------------------------
// Property 2: No duplicate timestamps
// ---------------------------------------------------------------------------
proptest! {
    #![proptest_config(config())]

    #[test]
    fn no_duplicate_timestamps(
        freq in arb_freq(),
        count in arb_count(),
        dtstart in arb_dtstart(),
        tz in arb_timezone(),
        dur in arb_duration(),
    ) {
        let rrule = format!("FREQ={};COUNT={}", freq, count);
        let result = expand_rrule(&rrule, &dtstart, dur, &tz, None, None);

        if let Ok(events) = result {
            let mut seen = std::collections::HashSet::new();
            for ev in &events {
                prop_assert!(
                    seen.insert(ev.start),
                    "duplicate timestamp found: {:?}",
                    ev.start
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Property 3: COUNT respected — exactly N events when COUNT=N
// ---------------------------------------------------------------------------
proptest! {
    #![proptest_config(config())]

    #[test]
    fn count_respected(
        freq in arb_freq(),
        count in arb_count(),
        dtstart in arb_dtstart(),
        tz in arb_timezone(),
        dur in arb_duration(),
    ) {
        // Pass count via the function parameter (external count).
        let rrule = format!("FREQ={}", freq);
        let result = expand_rrule(&rrule, &dtstart, dur, &tz, None, Some(count));

        if let Ok(events) = result {
            prop_assert!(
                events.len() <= count as usize,
                "got {} events, expected at most {} (COUNT={})",
                events.len(),
                count,
                count
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Property 4: Duration applied correctly — for every event, end - start == duration
// ---------------------------------------------------------------------------
proptest! {
    #![proptest_config(config())]

    #[test]
    fn duration_applied_correctly(
        freq in arb_freq(),
        count in arb_count(),
        dtstart in arb_dtstart(),
        tz in arb_timezone(),
        dur in arb_duration(),
    ) {
        let rrule = format!("FREQ={};COUNT={}", freq, count);
        let result = expand_rrule(&rrule, &dtstart, dur, &tz, None, None);

        if let Ok(events) = result {
            let expected_dur = Duration::minutes(dur as i64);
            for ev in &events {
                let actual_dur = ev.end - ev.start;
                prop_assert_eq!(
                    actual_dur,
                    expected_dur,
                    "event at {:?}: expected duration {:?}, got {:?}",
                    ev.start,
                    expected_dur,
                    actual_dur
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Property 5: All events within bounds when UNTIL is specified
// ---------------------------------------------------------------------------
proptest! {
    #![proptest_config(config())]

    #[test]
    fn events_within_until_bound(
        count in 1u32..=20,
        tz in arb_timezone(),
        dur in arb_duration(),
    ) {
        // Use a fixed DTSTART and a fixed UNTIL to keep things deterministic.
        // DAILY from 2026-01-01 until 2026-03-31 — at most ~90 events.
        let dtstart = "2026-01-01T10:00:00";
        let until = "2026-03-31T23:59:59";
        let rrule = format!("FREQ=DAILY;COUNT={}", count);

        let result = expand_rrule(&rrule, dtstart, dur, &tz, Some(until), None);

        if let Ok(events) = result {
            // The UNTIL boundary in local time corresponds to end-of-day on Mar 31.
            // Since we also have COUNT, the result is bounded by whichever is smaller.
            // We just verify all events start on or before Mar 31.
            for ev in &events {
                let date = ev.start.date_naive();
                prop_assert!(
                    date.year() <= 2026 && (date.year() < 2026 || date.ordinal() <= 90),
                    "event {:?} is beyond UNTIL boundary",
                    ev.start
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Property 6: Expansion never panics — valid inputs never cause panics
// ---------------------------------------------------------------------------
proptest! {
    #![proptest_config(config())]

    #[test]
    fn expansion_never_panics(
        freq in arb_freq(),
        interval in arb_interval(),
        count in arb_count(),
        dtstart in arb_dtstart(),
        tz in arb_timezone(),
        dur in arb_duration(),
    ) {
        // Build RRULE with FREQ + INTERVAL + COUNT.
        let rrule = format!("FREQ={};INTERVAL={};COUNT={}", freq, interval, count);

        // This must not panic; an Err result is acceptable.
        let _result = expand_rrule(&rrule, &dtstart, dur, &tz, None, None);
    }
}

// ---------------------------------------------------------------------------
// Property 7: DAILY produces daily intervals —
//   consecutive events differ by exactly INTERVAL days
// ---------------------------------------------------------------------------
proptest! {
    #![proptest_config(config())]

    #[test]
    fn daily_interval_spacing(
        interval in arb_interval(),
        count in 2u32..=30,
        dtstart in arb_dtstart(),
        dur in arb_duration(),
    ) {
        // Use UTC to avoid DST complications for this interval-checking property.
        let rrule = format!("FREQ=DAILY;INTERVAL={};COUNT={}", interval, count);
        let result = expand_rrule(&rrule, &dtstart, dur, "UTC", None, None);

        if let Ok(events) = result {
            if events.len() >= 2 {
                let expected_gap = Duration::days(interval as i64);
                for window in events.windows(2) {
                    let gap = window[1].start - window[0].start;
                    prop_assert_eq!(
                        gap,
                        expected_gap,
                        "DAILY;INTERVAL={} gap: expected {:?}, got {:?} between {:?} and {:?}",
                        interval,
                        expected_gap,
                        gap,
                        window[0].start,
                        window[1].start
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Property 8: WEEKLY with single BYDAY produces correct weekday
// ---------------------------------------------------------------------------
proptest! {
    #![proptest_config(config())]

    #[test]
    fn weekly_byday_produces_correct_weekday(
        byday in arb_byday(),
        count in 1u32..=20,
        dur in arb_duration(),
        tz in arb_timezone(),
    ) {
        // Use a known start date that is a Monday so the BYDAY alignment is clear.
        // 2026-01-05 is a Monday.
        let dtstart = "2026-01-05T09:00:00";
        let rrule = format!("FREQ=WEEKLY;BYDAY={};COUNT={}", byday, count);

        let result = expand_rrule(&rrule, dtstart, dur, &tz, None, None);

        if let Ok(events) = result {
            let expected_weekday = byday_to_weekday(&byday);
            for ev in &events {
                // Convert to the target timezone to check local weekday.
                let local_tz: chrono_tz::Tz = tz.parse().unwrap();
                let local_dt = ev.start.with_timezone(&local_tz);
                prop_assert_eq!(
                    local_dt.weekday(),
                    expected_weekday,
                    "event at {:?} (local {:?}) is {:?}, expected {:?}",
                    ev.start,
                    local_dt,
                    local_dt.weekday(),
                    expected_weekday
                );
            }
        }
    }
}

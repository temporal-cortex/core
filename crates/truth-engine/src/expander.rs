//! RRULE expansion -- converts recurrence rule strings into concrete datetime instances.
//!
//! Wraps the `rrule` crate (v0.13) and `chrono-tz` to provide deterministic expansion
//! of RFC 5545 recurrence rules with correct DST handling.

use crate::error::{Result, TruthError};
use chrono::{DateTime, Duration, Utc};
use rrule::RRuleSet;

/// A single expanded event instance with start and end times.
#[derive(Debug, Clone, PartialEq)]
pub struct ExpandedEvent {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

/// Expand an RRULE string into concrete datetime instances.
///
/// # Arguments
/// - `rrule` -- RFC 5545 RRULE string (e.g., "FREQ=WEEKLY;BYDAY=TU,TH")
/// - `dtstart` -- Local datetime string (e.g., "2026-02-17T14:00:00")
/// - `duration_minutes` -- Duration of each instance in minutes
/// - `timezone` -- IANA timezone (e.g., "America/Los_Angeles")
/// - `until` -- Optional end boundary for expansion (local datetime string)
/// - `count` -- Optional maximum number of instances (overrides COUNT in rrule)
///
/// # Errors
/// Returns `TruthError::InvalidRule` if the RRULE string is empty or unparseable.
/// Returns `TruthError::InvalidTimezone` if the timezone is not a valid IANA identifier.
pub fn expand_rrule(
    rrule: &str,
    dtstart: &str,
    duration_minutes: u32,
    timezone: &str,
    until: Option<&str>,
    count: Option<u32>,
) -> Result<Vec<ExpandedEvent>> {
    expand_rrule_with_exdates(
        rrule,
        dtstart,
        duration_minutes,
        timezone,
        until,
        count,
        &[],
    )
}

/// Expand an RRULE string into concrete datetime instances, with EXDATE exclusions.
///
/// Identical to [`expand_rrule`] but accepts a list of exception dates that will be
/// excluded from the recurrence set (RFC 5545 Section 3.8.5.1).
///
/// # Arguments
/// - `rrule` -- RFC 5545 RRULE string (e.g., "FREQ=WEEKLY;BYDAY=TU,TH")
/// - `dtstart` -- Local datetime string (e.g., "2026-02-17T14:00:00")
/// - `duration_minutes` -- Duration of each instance in minutes
/// - `timezone` -- IANA timezone (e.g., "America/Los_Angeles")
/// - `until` -- Optional end boundary for expansion (local datetime string)
/// - `count` -- Optional maximum number of instances (overrides COUNT in rrule)
/// - `exdates` -- Slice of local datetime strings to exclude (same format as `dtstart`)
///
/// # Errors
/// Returns `TruthError::InvalidRule` if the RRULE string is empty or unparseable.
/// Returns `TruthError::InvalidTimezone` if the timezone is not a valid IANA identifier.
pub fn expand_rrule_with_exdates(
    rrule: &str,
    dtstart: &str,
    duration_minutes: u32,
    timezone: &str,
    until: Option<&str>,
    count: Option<u32>,
    exdates: &[&str],
) -> Result<Vec<ExpandedEvent>> {
    // Validate inputs.
    if rrule.is_empty() {
        return Err(TruthError::InvalidRule("empty RRULE string".to_string()));
    }

    // Short-circuit: caller explicitly wants zero instances.
    if count == Some(0) {
        return Ok(Vec::new());
    }

    // Validate timezone by parsing it as a chrono-tz Tz.
    let _tz: chrono_tz::Tz = timezone
        .parse()
        .map_err(|_| TruthError::InvalidTimezone(timezone.to_string()))?;

    // Convert the dtstart from "2026-02-17T14:00:00" to iCalendar format "20260217T140000".
    let dtstart_ical = dtstart.replace(['-', ':'], "");

    // Build the RRULE text block. We may need to inject COUNT or UNTIL.
    let mut rrule_str = rrule.to_string();

    // If the caller provides an external `count`, inject it into the RRULE
    // (unless the RRULE already has a COUNT).
    if let Some(c) = count {
        if !rrule_str.to_uppercase().contains("COUNT=") {
            rrule_str = format!("{};COUNT={}", rrule_str, c);
        }
    }

    // If the caller provides an `until`, inject it into the RRULE.
    // The rrule crate requires UNTIL and DTSTART to share the same timezone.
    // For UTC, UNTIL must end with "Z"; for other timezones, use bare local time.
    if let Some(until_str) = until {
        if !rrule_str.to_uppercase().contains("UNTIL=") {
            let mut until_ical = until_str.replace(['-', ':'], "");
            if timezone == "UTC" {
                until_ical.push('Z');
            }
            rrule_str = format!("{};UNTIL={}", rrule_str, until_ical);
        }
    }

    // Build the full iCalendar RRULE text with DTSTART and optional EXDATE lines.
    let mut rrule_text = format!(
        "DTSTART;TZID={}:{}\nRRULE:{}",
        timezone, dtstart_ical, rrule_str
    );

    // Append EXDATE lines if any exclusion dates were provided.
    if !exdates.is_empty() {
        let exdate_icals: Vec<String> = exdates.iter().map(|d| d.replace(['-', ':'], "")).collect();
        rrule_text.push_str(&format!(
            "\nEXDATE;TZID={}:{}",
            timezone,
            exdate_icals.join(",")
        ));
    }

    // Parse and expand.
    let rrule_set: RRuleSet = rrule_text
        .parse()
        .map_err(|e| TruthError::InvalidRule(format!("{}", e)))?;

    // Determine the max count for expansion to prevent unbounded expansion.
    // When we have exdates, we need a higher limit because the rrule crate's
    // `.all(limit)` counts BEFORE exdate filtering, so we may need more raw
    // instances to get `count` results after exclusion. Add exdate count as buffer.
    let exdate_buffer = exdates.len() as u16;
    let max_count: u16 = count
        .map(|c| (c as u16).saturating_add(exdate_buffer))
        .unwrap_or(500);

    let instances = rrule_set.all(max_count);
    let duration = Duration::minutes(duration_minutes as i64);

    let mut events: Vec<ExpandedEvent> = instances
        .dates
        .into_iter()
        .map(|dt| {
            let start_utc: DateTime<Utc> = dt.with_timezone(&Utc);
            ExpandedEvent {
                start: start_utc,
                end: start_utc + duration,
            }
        })
        .collect();

    // If the caller specified an external count limit, truncate to that many results.
    // (EXDATE filtering by the rrule crate may have already reduced the count, but
    // the `.all()` limit is a pre-filter cap, not a post-filter cap.)
    if let Some(c) = count {
        events.truncate(c as usize);
    }

    Ok(events)
}

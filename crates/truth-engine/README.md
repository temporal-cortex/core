# truth-engine

Deterministic RRULE expansion with DST handling for AI calendar agents.

LLMs struggle with recurrence rule math — they hallucinate dates, mishandle DST transitions, and can't reliably compute "3rd Tuesday of each month." The Truth Engine replaces inference with deterministic computation using the `rrule` crate and `chrono-tz`.

## Usage

```rust
use truth_engine::{expand_rrule, find_conflicts, find_free_slots, ExpandedEvent};

// Expand a recurrence rule into concrete instances
let events = expand_rrule(
    "FREQ=MONTHLY;BYDAY=TU;BYSETPOS=3",       // 3rd Tuesday of each month
    "2026-02-17T14:00:00",                      // start date (local time)
    60,                                          // 60-minute duration
    "America/Los_Angeles",                       // IANA timezone
    Some("2026-06-30T23:59:59"),                 // expand until
    None,                                        // no count limit
).unwrap();

// Each event has start/end as DateTime<Utc>
for event in &events {
    println!("{} → {}", event.start, event.end);
}

// Detect overlapping events between two schedules
let conflicts = find_conflicts(&schedule_a, &schedule_b);
for c in &conflicts {
    println!("Overlap: {} minutes", c.overlap_minutes);
}

// Find free slots in a time window
let free = find_free_slots(
    &busy_events,
    window_start,  // DateTime<Utc>
    window_end,    // DateTime<Utc>
);
```

## Features

### RRULE Expansion

- Full RFC 5545 recurrence rule support via the `rrule` crate v0.13
- `FREQ`: DAILY, WEEKLY, MONTHLY, YEARLY
- `BYDAY`, `BYMONTH`, `BYMONTHDAY`, `BYSETPOS`, `INTERVAL`, `COUNT`, `UNTIL`
- EXDATE exclusions via `expand_rrule_with_exdates()`
- DST-aware: events at 14:00 Pacific stay at 14:00 Pacific across DST transitions (UTC offset shifts automatically)
- Leap year handling: `BYMONTHDAY=29` in February correctly skips non-leap years

### Conflict Detection

- Pairwise overlap detection between two event lists
- Overlap defined as `a.start < b.end && b.start < a.end`
- Adjacent events (end == start) are NOT conflicts
- Returns overlap duration in minutes

### Free/Busy Computation

- Merges overlapping busy periods
- Computes free gaps within a time window
- `find_first_free_slot()` for minimum-duration search

## API

### `expand_rrule(rrule, dtstart, duration_minutes, timezone, until, count)`

Expands an RRULE string into concrete `ExpandedEvent` instances.

### `expand_rrule_with_exdates(rrule, dtstart, duration_minutes, timezone, until, count, exdates)`

Same as above but excludes specific dates (RFC 5545 EXDATE).

### `find_conflicts(events_a, events_b) -> Vec<Conflict>`

Finds all pairwise overlaps between two event lists.

### `find_free_slots(events, window_start, window_end) -> Vec<FreeSlot>`

Computes free time slots within a window, merging overlapping busy periods.

### `find_first_free_slot(events, window_start, window_end, min_duration_minutes) -> Option<FreeSlot>`

Finds the earliest free slot of at least the given duration.

## Architecture

```
expander.rs  ← RRULE string → Vec<ExpandedEvent> (wraps rrule + chrono-tz)
conflict.rs  ← Two event lists → Vec<Conflict> (pairwise overlap detection)
freebusy.rs  ← Events + window → Vec<FreeSlot> (gap computation)
dst.rs       ← DstPolicy enum (Skip, ShiftForward, WallClock)
error.rs     ← TruthError enum (InvalidRule, InvalidTimezone, Expansion)
```

## Testing

33 tests across four suites:

- **11 expander tests** — CTO's monthly 3rd Tuesday example, DST transitions, daily/weekly/biweekly, COUNT, UNTIL, duration
- **7 conflict tests** — overlapping, non-overlapping, adjacent, contained, multiple, empty
- **7 free/busy tests** — single event, merged overlapping, empty, min-duration, fully booked
- **8 RFC 5545 compliance vectors** — biweekly multi-day, yearly, leap year Feb 29, EXDATE, COUNT, INTERVAL, BYSETPOS last weekday, multi-rule intersection

```bash
cargo test -p truth-engine
```

## License

MIT OR Apache-2.0

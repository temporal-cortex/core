//! # truth-engine
//!
//! Deterministic calendar computation for AI agents.
//!
//! The Truth Engine provides mathematically correct recurrence rule expansion,
//! conflict detection, free/busy computation, and multi-calendar availability
//! merging that LLMs cannot reliably perform via inference.
//!
//! ## Modules
//!
//! - [`expander`] — RRULE string → list of concrete datetime instances
//! - [`dst`] — DST transition policies (skip, shift, etc.)
//! - [`conflict`] — Detect overlapping events in expanded schedules
//! - [`freebusy`] — Compute free time slots from event lists
//! - [`availability`] — Merge N event streams into unified busy/free with privacy control
//! - [`error`] — Error types

pub mod availability;
pub mod conflict;
pub mod dst;
pub mod error;
pub mod expander;
pub mod freebusy;

pub use availability::{
    merge_availability, find_first_free_across, BusyBlock, EventStream, PrivacyLevel,
    UnifiedAvailability,
};
pub use conflict::find_conflicts;
pub use error::TruthError;
pub use expander::{expand_rrule, expand_rrule_with_exdates, ExpandedEvent};
pub use freebusy::{find_free_slots, FreeSlot};

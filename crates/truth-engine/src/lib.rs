//! # truth-engine
//!
//! Deterministic RRULE expansion with DST handling for AI calendar agents.
//!
//! The Truth Engine provides mathematically correct recurrence rule expansion
//! that LLMs cannot reliably perform via inference. It wraps the `rrule` crate
//! and adds DST-aware timezone handling via `chrono-tz`.
//!
//! ## Modules
//!
//! - [`expander`] — RRULE string → list of concrete datetime instances
//! - [`dst`] — DST transition policies (skip, shift, etc.)
//! - [`conflict`] — Detect overlapping events in expanded schedules
//! - [`freebusy`] — Compute free time slots from event lists
//! - [`error`] — Error types

pub mod conflict;
pub mod dst;
pub mod error;
pub mod expander;
pub mod freebusy;

pub use conflict::find_conflicts;
pub use error::TruthError;
pub use expander::{expand_rrule, expand_rrule_with_exdates, ExpandedEvent};
pub use freebusy::find_free_slots;

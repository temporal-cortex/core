//! DST transition policies for recurring events.

/// Policy for handling events that fall during DST transitions.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum DstPolicy {
    /// Skip instances that fall in the DST gap (e.g., 2:30 AM during spring forward)
    Skip,
    /// Shift to the next valid time after the gap
    ShiftForward,
    /// Use wall clock time (maintain local time, adjust UTC offset)
    #[default]
    WallClock,
}

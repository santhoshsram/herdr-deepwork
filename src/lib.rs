//! A focus timer for the herdr tab bar.

pub mod state;

/// The line `status` prints when no timer runs.
pub const IDLE_LINE: &str = "🍅";

/// The line for the tab bar.
///
/// A timer file that is missing, unreadable or off the contract shape counts
/// as no timer, so this never fails.
#[must_use]
pub fn status_line() -> String {
    let _timer = state::load_current();
    IDLE_LINE.to_string()
}

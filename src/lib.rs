//! A focus timer for the herdr tab bar.

pub mod format;
pub mod state;
pub mod timeline;

pub use format::IDLE_LINE;

/// The line for the tab bar.
///
/// A timer file that is missing, unreadable or off the contract shape counts
/// as no timer, so this never fails.
#[must_use]
pub fn status_line() -> String {
    let timer = state::load_current();
    format::line(timeline::view(&timer, state::Timestamp::now()))
}

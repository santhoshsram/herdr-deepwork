//! The tab bar text and the sidebar token for each state.

use crate::timeline::View;

/// The tab bar line when no timer shows, the head of every other line, and
/// the sidebar token while a focus counts down.
pub const TOMATO: &str = "🍅";
/// The sidebar token while a focus is held.
pub const PAUSED: &str = "⏸";

/// The sidebar `focus` token for `view`; `None` when the token is cleared.
#[must_use]
pub fn focus_token(view: View) -> Option<&'static str> {
    match view {
        View::Focus { .. } => Some(TOMATO),
        View::Paused { .. } => Some(PAUSED),
        _ => None,
    }
}

/// A duration as the tab bar shows it: minutes rounded up, `25m` or `1h 05m`.
#[must_use]
pub fn duration(secs: u64) -> String {
    let mins = secs.div_ceil(60);
    if mins < 60 {
        format!("{mins}m")
    } else {
        format!("{}h {:02}m", mins / 60, mins % 60)
    }
}

/// `text` without control characters, so it cannot break a line of output.
#[must_use]
pub fn printable(text: &str) -> String {
    text.chars().filter(|c| !c.is_control()).collect()
}

/// The tab bar line for `view`.
#[must_use]
pub fn line(view: View) -> String {
    match view {
        View::Idle => TOMATO.to_string(),
        View::Focus { left_secs } => format!("{TOMATO} {}", duration(left_secs)),
        View::Paused { left_secs } => format!("{TOMATO} Paused {}", duration(left_secs)),
        View::FocusDoneBreak { break_mins } => format!(
            "{TOMATO} Focus done · break {}",
            duration(u64::from(break_mins) * 60)
        ),
        View::FocusDone => format!("{TOMATO} Focus done"),
        View::Break { left_secs } => format!("{TOMATO} Break {}", duration(left_secs)),
        View::BreakOver => format!("{TOMATO} Break over"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_round_up() {
        assert_eq!(duration(1), "1m");
        assert_eq!(duration(60), "1m");
        assert_eq!(duration(61), "2m");
        assert_eq!(duration(18 * 60 + 30), "19m");
        assert_eq!(duration(59 * 60 + 1), "1h 00m");
        assert_eq!(duration(65 * 60), "1h 05m");
        assert_eq!(duration(180 * 60), "3h 00m");
    }

    #[test]
    fn control_characters_are_dropped() {
        assert_eq!(printable("wo\u{1b}[31m\trk\u{7}\n"), "wo[31mrk");
        assert_eq!(printable("work ✓"), "work ✓");
    }

    #[test]
    fn lines_match_the_contract() {
        assert_eq!(line(View::Idle), "🍅");
        assert_eq!(line(View::Focus { left_secs: 1110 }), "🍅 19m");
        assert_eq!(line(View::Paused { left_secs: 720 }), "🍅 Paused 12m");
        assert_eq!(
            line(View::FocusDoneBreak { break_mins: 5 }),
            "🍅 Focus done · break 5m"
        );
        assert_eq!(line(View::FocusDone), "🍅 Focus done");
        assert_eq!(line(View::Break { left_secs: 210 }), "🍅 Break 4m");
        assert_eq!(line(View::BreakOver), "🍅 Break over");
    }
}

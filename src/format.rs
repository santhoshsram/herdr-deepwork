//! The tab bar text for each state.

use crate::timeline::View;

/// The line for the tab bar when no timer shows.
pub const IDLE_LINE: &str = "🍅";

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

/// The tab bar line for `view`.
#[must_use]
pub fn line(view: View) -> String {
    match view {
        View::Idle => IDLE_LINE.to_string(),
        View::Focus { left_secs } => format!("{IDLE_LINE} {}", duration(left_secs)),
        View::Paused { left_secs } => format!("{IDLE_LINE} Paused {}", duration(left_secs)),
        View::FocusDoneBreak { break_mins } => format!(
            "{IDLE_LINE} Focus done · break {}",
            duration(u64::from(break_mins) * 60)
        ),
        View::FocusDone => format!("{IDLE_LINE} Focus done"),
        View::Break { left_secs } => format!("{IDLE_LINE} Break {}", duration(left_secs)),
        View::BreakOver => format!("{IDLE_LINE} Break over"),
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

//! The tab bar line, with the away hint, and the sidebar tomato kept in step.

use std::time::Duration;

use crate::format;
use crate::herdr::{Call, Herdr, is_pane_reply};
use crate::state::{self, Timer, Timestamp};
use crate::timeline::{self, View};

/// How long herdr gets to answer, so the whole run stays within 200 ms.
const HERDR_BUDGET: Duration = Duration::from_millis(150);

const TOMATO: &str = "🍅";
const PAUSED: &str = "⏸";

/// The line for the tab bar.
///
/// A timer file that is missing, unreadable or off the contract shape counts
/// as no timer, and a herdr that is missing or silent costs only the hint and
/// the sidebar tomato, so this never fails.
#[must_use]
pub fn status_line() -> String {
    let herdr = Herdr::from_env(HERDR_BUDGET);
    let timer = state::load_current();
    let view = timeline::view(&timer, Timestamp::now());
    let active = std::env::var("HERDR_ACTIVE_PANE_ID").ok();
    line_for(&herdr, &timer, view, active.as_deref())
}

fn line_for(herdr: &Herdr, timer: &Timer, view: View, active: Option<&str>) -> String {
    let home = timer.home_pane.as_deref().filter(|p| !p.is_empty());
    let token = match view {
        View::Focus { .. } => Some(TOMATO),
        View::Paused { .. } => Some(PAUSED),
        _ => None,
    };
    let tomato_call = home.and_then(|pane| match token {
        Some(value) => herdr.set_focus_token(pane, value),
        None => herdr.clear_focus_token(pane),
    });

    let line = format::line(view);
    let Some((pane, name)) = away_target(timer, view, active) else {
        finish_quietly(tomato_call);
        return line;
    };
    // Both calls are already running side by side under one deadline.
    let exists = herdr
        .get_pane(pane)
        .and_then(Call::finish)
        .is_some_and(|reply| is_pane_reply(&reply));
    finish_quietly(tomato_call);
    if exists {
        format!("{line} ← {name}")
    } else {
        line
    }
}

/// The home pane and its printable name when a focus counts down and the
/// active pane is another one.
fn away_target<'a>(
    timer: &'a Timer,
    view: View,
    active: Option<&str>,
) -> Option<(&'a str, String)> {
    let View::Focus { .. } = view else {
        return None;
    };
    let pane = timer.home_pane.as_deref().filter(|p| !p.is_empty())?;
    let active = active.filter(|a| !a.is_empty())?;
    if active == pane {
        return None;
    }
    let name = printable(timer.home_name.as_deref()?);
    (!name.is_empty()).then_some((pane, name))
}

/// Waits for a call whose answer does not matter, so its effect lands before
/// the process exits.
fn finish_quietly(call: Option<Call>) {
    if let Some(call) = call {
        let _ = call.finish();
    }
}

/// `text` without control characters, so it cannot break the one-line output.
fn printable(text: &str) -> String {
    text.chars().filter(|c| !c.is_control()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn focus_timer(home: &str, name: &str) -> Timer {
        Timer {
            home_pane: Some(home.to_string()),
            home_name: Some(name.to_string()),
            ..Timer::default()
        }
    }

    #[test]
    fn control_characters_are_dropped() {
        assert_eq!(printable("wo\u{1b}[31m\trk\u{7}\n"), "wo[31mrk");
        assert_eq!(printable("work ✓"), "work ✓");
    }

    #[test]
    fn away_only_while_a_focus_counts_down() {
        let timer = focus_timer("p1", "work");
        let focus = View::Focus { left_secs: 60 };
        assert_eq!(
            away_target(&timer, focus, Some("p2")),
            Some(("p1", "work".to_string()))
        );
        assert_eq!(away_target(&timer, focus, Some("p1")), None);
        assert_eq!(away_target(&timer, focus, None), None);
        assert_eq!(away_target(&timer, focus, Some("")), None);
        assert_eq!(
            away_target(&timer, View::Paused { left_secs: 60 }, Some("p2")),
            None
        );
        assert_eq!(away_target(&timer, View::Idle, Some("p2")), None);
    }
}

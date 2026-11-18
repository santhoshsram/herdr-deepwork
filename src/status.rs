//! The tab bar line, with the away hint, and the sidebar tomato kept in step.

use std::time::Duration;

use crate::format;
use crate::herdr::{Herdr, finish_quietly};
use crate::state::{self, Timer, Timestamp};
use crate::timeline::{self, View};

/// How long herdr gets to answer, so the whole run stays within 200 ms.
const HERDR_BUDGET: Duration = Duration::from_millis(150);

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
    // Outside a focus or pause the last token expires by itself, so no call.
    let tomato_call = home
        .zip(format::focus_token(view))
        .and_then(|(pane, token)| herdr.report_focus_token(pane, Some(token)));

    let line = format::line(view);
    // The hint shows only while a focus counts down.
    let away = match view {
        View::Focus { .. } => timer.away_target(active),
        _ => None,
    };
    // The token call is already running, so the lookup runs beside it under
    // the same deadline.
    let line = match away {
        Some((pane, name)) if herdr.has_pane(pane) => format!("{line} ← {name}"),
        _ => line,
    };
    finish_quietly(tomato_call);
    line
}

//! The control popup: pause, resume, stop, go back home or end a break.

use crate::format;
use crate::herdr::is_pane_reply;
use crate::keys::Key;
use crate::panel::{Context, Outcome};
use crate::state::{self, Mode, Timer, Timestamp};
use crate::timeline::{self, View};

const NOT_ANSWERING: &str = "herdr is not answering";
const NOT_SAVED: &str = "Could not save";

const TOMATO: &str = "🍅";
const PAUSED: &str = "⏸";

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RED: &str = "\x1b[31m";
const REVERSE: &str = "\x1b[7m";

/// What the timer is doing, as far as the buttons care.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Focus,
    Paused,
    Break,
}

/// The phase of a running timer; `None` when nothing runs.
fn phase(view: View) -> Option<Phase> {
    match view {
        View::Focus { .. } => Some(Phase::Focus),
        View::Paused { .. } => Some(Phase::Paused),
        // The break has begun even while the tab bar still reports the focus
        // as done.
        View::FocusDoneBreak { .. } | View::Break { .. } => Some(Phase::Break),
        View::Idle | View::FocusDone | View::BreakOver => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Pause,
    Resume,
    Stop,
    Back,
    EndBreak,
}

/// The control popup's state: the timer it shows and what it drew for it.
#[derive(Debug)]
pub struct ControlPopup {
    timer: Timer,
    phase: Phase,
    /// The pane to go back to, when the focus is away from it.
    away: Option<Away>,
    error: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Away {
    pane: String,
    name: String,
}

impl ControlPopup {
    /// Opens the popup for `timer`; `None` when no timer runs.
    ///
    /// Away means a focus counts down, the user is in another pane and herdr
    /// still knows the home pane; a home herdr cannot find or does not answer
    /// about is treated as the current pane.
    #[must_use]
    pub fn open(ctx: &Context, timer: &Timer) -> Option<Self> {
        let view = timeline::view(timer, Timestamp::now());
        let phase = phase(view)?;
        let away = if phase == Phase::Focus {
            away_target(ctx, timer)
        } else {
            None
        };
        Some(Self {
            timer: timer.clone(),
            phase,
            away,
            error: None,
        })
    }

    /// The screen, one string per line.
    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        let view = timeline::view(&self.timer, Timestamp::now());
        let button = |primary: bool, text: &str| {
            if primary {
                format!("{REVERSE} {text} {RESET}")
            } else {
                format!(" {text} ")
            }
        };
        let (buttons, keys): (Vec<String>, Vec<(&str, &str)>) = match (self.phase, &self.away) {
            (Phase::Focus, Some(away)) => (
                vec![
                    button(true, &format!("Back to {} ⏎", away.name)),
                    button(false, "Pause p"),
                    button(false, "Stop s"),
                ],
                vec![
                    ("⏎", "back"),
                    ("p", "pause"),
                    ("s", "stop"),
                    ("esc", "close"),
                ],
            ),
            (Phase::Focus, None) => (
                vec![button(true, "Pause ⏎"), button(false, "Stop s")],
                vec![("⏎", "pause"), ("s", "stop"), ("esc", "close")],
            ),
            (Phase::Paused, _) => (
                vec![button(true, "Resume ⏎"), button(false, "Stop s")],
                vec![("⏎", "resume"), ("s", "stop"), ("esc", "close")],
            ),
            (Phase::Break, _) => (
                vec![button(true, "End break ⏎")],
                vec![("⏎", "end break"), ("esc", "close")],
            ),
        };
        let what = match (self.phase, self.timer.home_name.as_deref()) {
            (Phase::Break, _) | (_, None) => String::from("check your agents"),
            (_, Some(name)) => format!("on {BOLD}{}{RESET}", printable(name)),
        };
        let hints: Vec<String> = keys
            .iter()
            .map(|(key, what)| format!("{BOLD}{key}{RESET} {what}"))
            .collect();
        vec![
            format!("{DIM}focus{RESET}"),
            String::new(),
            format!("{BOLD}{}{RESET}", format::line(view)),
            what,
            String::new(),
            buttons.join("  "),
            // Always one line, so the keys below do not jump when it appears.
            format!("{RED}{}{RESET}", self.error.unwrap_or_default()),
            hints.join("  "),
        ]
    }

    /// Acts on one key.
    pub fn press(&mut self, key: Key, ctx: &Context) -> Outcome {
        self.error = None;
        if key == Key::Esc {
            return Outcome::Close;
        }
        let Some(action) = self.action(key) else {
            return Outcome::Stay;
        };
        // Reread: the timer may have changed since the popup was drawn, and a
        // key must never act on a screen the user is no longer looking at.
        let saved = ctx.load();
        let now = Timestamp::now();
        let view = timeline::view(&saved, now);
        if !self.shows(&saved, view) {
            return match ControlPopup::open(ctx, &saved) {
                Some(fresh) => {
                    *self = fresh;
                    Outcome::Stay
                }
                None => Outcome::Idle,
            };
        }
        self.act(action, ctx, &saved, view, now)
    }

    /// What `key` asks for on this screen.
    fn action(&self, key: Key) -> Option<Action> {
        match (self.phase, key) {
            (Phase::Focus, Key::Enter) if self.away.is_some() => Some(Action::Back),
            (Phase::Focus, Key::Enter | Key::Char('p')) => Some(Action::Pause),
            (Phase::Focus | Phase::Paused, Key::Char('s')) => Some(Action::Stop),
            (Phase::Paused, Key::Enter) => Some(Action::Resume),
            (Phase::Break, Key::Enter) => Some(Action::EndBreak),
            _ => None,
        }
    }

    /// Whether `saved` is still the timer this screen was drawn for.
    fn shows(&self, saved: &Timer, view: View) -> bool {
        phase(view) == Some(self.phase)
            && saved.home_pane == self.timer.home_pane
            && saved.ends_at == self.timer.ends_at
            && saved.left_secs == self.timer.left_secs
    }

    fn act(
        &mut self,
        action: Action,
        ctx: &Context,
        saved: &Timer,
        view: View,
        now: Timestamp,
    ) -> Outcome {
        let home = saved.home_pane.as_deref();
        let next = match (action, view) {
            (Action::Back, _) => return self.back(ctx),
            (Action::Pause, View::Focus { left_secs }) => Timer {
                mode: Mode::Paused,
                ends_at: None,
                left_secs: Some(left_secs),
                ..saved.clone()
            },
            (Action::Resume, View::Paused { left_secs }) => Timer {
                mode: Mode::Focus,
                ends_at: Some(Timestamp(now.0 + left_secs)),
                left_secs: None,
                ..saved.clone()
            },
            (Action::Stop | Action::EndBreak, _) => ended(saved),
            // The screen was checked against the timer, so no other pair.
            _ => return Outcome::Stay,
        };
        let Some(path) = ctx.timer_path.as_deref() else {
            self.error = Some(NOT_SAVED);
            return Outcome::Stay;
        };
        if state::save(path, &next).is_err() {
            self.error = Some(NOT_SAVED);
            return Outcome::Stay;
        }
        // The tab bar sets the token again every second, so a silent herdr
        // only delays it.
        if let Some(pane) = home {
            let call = match action {
                Action::Pause => ctx.herdr.set_focus_token(pane, PAUSED),
                Action::Resume => ctx.herdr.set_focus_token(pane, TOMATO),
                Action::Stop => ctx.herdr.clear_focus_token(pane),
                Action::Back | Action::EndBreak => None,
            };
            if let Some(call) = call {
                let _ = call.finish();
            }
        }
        Outcome::Close
    }

    /// Switches to the home pane; the timer is untouched.
    fn back(&mut self, ctx: &Context) -> Outcome {
        let answered = self
            .away
            .as_ref()
            .and_then(|away| ctx.herdr.spawn(&["agent", "focus", &away.pane]))
            .and_then(|call| call.finish())
            .is_some();
        if answered {
            Outcome::Close
        } else {
            self.error = Some(NOT_ANSWERING);
            Outcome::Stay
        }
    }
}

/// `timer` with nothing running; the remembered choices and the last home
/// stay, so status knows where to clear the token.
fn ended(timer: &Timer) -> Timer {
    Timer {
        mode: Mode::Idle,
        ends_at: None,
        left_secs: None,
        message: None,
        message_ends_at: None,
        ..timer.clone()
    }
}

fn away_target(ctx: &Context, timer: &Timer) -> Option<Away> {
    let pane = timer.home_pane.as_deref().filter(|p| !p.is_empty())?;
    let here = ctx.pane.as_deref()?;
    if here == pane {
        return None;
    }
    let name = printable(timer.home_name.as_deref()?);
    if name.is_empty() {
        return None;
    }
    let exists = ctx
        .herdr
        .get_pane(pane)
        .and_then(|call| call.finish())
        .is_some_and(|reply| is_pane_reply(&reply));
    exists.then(|| Away {
        pane: pane.to_owned(),
        name,
    })
}

/// `text` without control characters, so it cannot break the screen.
fn printable(text: &str) -> String {
    text.chars().filter(|c| !c.is_control()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::panel::is_running;

    #[test]
    fn running_states_have_a_phase() {
        assert_eq!(phase(View::Focus { left_secs: 5 }), Some(Phase::Focus));
        assert_eq!(phase(View::Paused { left_secs: 5 }), Some(Phase::Paused));
        assert_eq!(phase(View::Break { left_secs: 5 }), Some(Phase::Break));
        assert_eq!(
            phase(View::FocusDoneBreak { break_mins: 5 }),
            Some(Phase::Break)
        );
        assert_eq!(phase(View::FocusDone), None);
        assert_eq!(phase(View::BreakOver), None);
        assert_eq!(phase(View::Idle), None);
    }

    #[test]
    fn a_phase_exists_exactly_when_the_timer_runs() {
        for view in [
            View::Idle,
            View::Focus { left_secs: 1 },
            View::Paused { left_secs: 1 },
            View::FocusDoneBreak { break_mins: 2 },
            View::FocusDone,
            View::Break { left_secs: 1 },
            View::BreakOver,
        ] {
            assert_eq!(phase(view).is_some(), is_running(view), "{view:?}");
        }
    }

    #[test]
    fn ending_keeps_the_remembered_choices() {
        let timer = Timer {
            mode: Mode::Focus,
            home_pane: Some("p1".into()),
            home_name: Some("work".into()),
            ends_at: Some(Timestamp(10)),
            last_focus_mins: 50,
            last_break_mins: 10,
            last_custom_mins: Some(40),
            ..Timer::default()
        };
        let idle = ended(&timer);
        assert_eq!(idle.mode, Mode::Idle);
        assert_eq!(idle.home_pane.as_deref(), Some("p1"));
        assert_eq!(idle.ends_at, None);
        assert_eq!(idle.last_focus_mins, 50);
        assert_eq!(idle.last_break_mins, 10);
        assert_eq!(idle.last_custom_mins, Some(40));
    }
}

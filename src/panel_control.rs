//! The control popup: pause, resume, stop, go back home or end a break.

use crate::format;
use crate::herdr::{Call, finish_quietly};
use crate::keys::Key;
use crate::panel::{self, BOLD, Context, DIM, NOT_ANSWERING, Outcome, RED, RESET, REVERSE, hint};
use crate::state::{Mode, Timer, Timestamp};
use crate::timeline::{self, Phase, View, phase};

/// Row of the buttons on screen, counted from 1.
const BUTTON_ROW: u16 = 6;
/// One column, so a 24-character home name still fits the popup.
const BUTTON_GAP: &str = " ";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Pause,
    Resume,
    Stop,
    Back,
    EndBreak,
}

/// One button of a phase, which is also its key hint.
struct Button {
    action: Action,
    /// The key as drawn on the button and in the hint.
    key: &'static str,
    /// The keys that press it, the drawn one among them.
    keys: &'static [Key],
    text: String,
    hint: &'static str,
}

impl Button {
    fn new(
        action: Action,
        (key, keys): (&'static str, &'static [Key]),
        text: impl Into<String>,
        hint: &'static str,
    ) -> Self {
        Self {
            action,
            key,
            keys,
            text: text.into(),
            hint,
        }
    }

    fn label(&self) -> String {
        format!("{} {}", self.text, self.key)
    }
}

const ENTER: (&str, &[Key]) = ("⏎", &[Key::Enter]);
const P: (&str, &[Key]) = ("p", &[Key::Char('p')]);
const S: (&str, &[Key]) = ("s", &[Key::Char('s')]);
/// At home Enter pauses, and p still does, as it does when away.
const ENTER_OR_P: (&str, &[Key]) = ("⏎", &[Key::Enter, Key::Char('p')]);

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

/// The control popup while herdr looks up the home pane.
#[derive(Debug)]
pub struct Opening {
    popup: ControlPopup,
    lookup: Option<(Away, Call)>,
}

impl Opening {
    /// The popup, away only once herdr answers that the home pane exists.
    #[must_use]
    pub fn finish(mut self) -> ControlPopup {
        self.popup.away = self
            .lookup
            .and_then(|(away, call)| call.found_pane().then_some(away));
        self.popup
    }
}

impl ControlPopup {
    /// Starts opening the popup for `timer`; `None` when no timer runs.
    /// [`Opening::finish`] waits for herdr.
    ///
    /// Away means a focus counts down, the user is in another pane and herdr
    /// still knows the home pane; a home herdr cannot find or does not answer
    /// about is treated as the current pane.
    #[must_use]
    pub fn begin(ctx: &Context, timer: &Timer) -> Option<Opening> {
        let view = timeline::view(timer, Timestamp::now());
        let phase = phase(view)?;
        let lookup = match phase {
            Phase::Focus => timer.away_target(ctx.pane.as_deref()),
            Phase::Paused | Phase::Break => None,
        }
        .and_then(|(pane, name)| {
            let call = ctx.herdr.get_pane(pane)?;
            let pane = pane.to_owned();
            Some((Away { pane, name }, call))
        });
        let popup = Self {
            timer: timer.clone(),
            phase,
            away: None,
            error: None,
        };
        Some(Opening { popup, lookup })
    }

    /// The screen, one string per line.
    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        let view = timeline::view(&self.timer, Timestamp::now());
        let buttons = self.buttons();
        let drawn: Vec<String> = buttons
            .iter()
            .enumerate()
            .map(|(i, button)| {
                let label = button.label();
                if i == 0 {
                    format!("{REVERSE} {label} {RESET}")
                } else {
                    format!(" {label} ")
                }
            })
            .collect();
        let what = match (self.phase, self.timer.home_name.as_deref()) {
            (Phase::Break, _) | (_, None) => String::from("check your agents"),
            (_, Some(name)) => format!("on {BOLD}{}{RESET}", format::printable(name)),
        };
        let hints: Vec<String> = buttons
            .iter()
            .map(|button| (button.key, button.hint))
            .chain([("esc", "close")])
            .map(|(key, what)| hint(key, what))
            .collect();
        vec![
            format!("{DIM}focus{RESET}"),
            String::new(),
            format!("{BOLD}{}{RESET}", format::line(view)),
            what,
            String::new(),
            drawn.join(BUTTON_GAP),
            // Always one line, so the keys below do not jump when it appears.
            format!("{RED}{}{RESET}", self.error.unwrap_or_default()),
            hints.join("  "),
        ]
    }

    /// The buttons of this screen, the first being the primary one.
    fn buttons(&self) -> Vec<Button> {
        let stop = || Button::new(Action::Stop, S, "Stop", "stop");
        match (self.phase, &self.away) {
            (Phase::Focus, Some(away)) => vec![
                Button::new(
                    Action::Back,
                    ENTER,
                    format!("Back to {}", away.name),
                    "back",
                ),
                Button::new(Action::Pause, P, "Pause", "pause"),
                stop(),
            ],
            (Phase::Focus, None) => vec![
                Button::new(Action::Pause, ENTER_OR_P, "Pause", "pause"),
                stop(),
            ],
            (Phase::Paused, _) => vec![
                Button::new(Action::Resume, ENTER, "Resume", "resume"),
                stop(),
            ],
            (Phase::Break, _) => vec![Button::new(
                Action::EndBreak,
                ENTER,
                "End break",
                "end break",
            )],
        }
    }

    /// The action of the button under a click.
    fn button_at(&self, col: u16, row: u16) -> Option<Action> {
        if row != BUTTON_ROW {
            return None;
        }
        let buttons = self.buttons();
        let labels: Vec<String> = buttons.iter().map(Button::label).collect();
        let index = panel::hit(&labels, 1, BUTTON_GAP.len(), col)?;
        Some(buttons[index].action)
    }

    /// Acts on one key; a click acts as the button it hits.
    pub fn press(&mut self, key: Key, ctx: &Context) -> Outcome {
        let action = match key {
            Key::Click { col, row } => match self.button_at(col, row) {
                None => return Outcome::Stay,
                hit => hit,
            },
            key => self
                .buttons()
                .into_iter()
                .find(|button| button.keys.contains(&key))
                .map(|button| button.action),
        };
        self.error = None;
        if key == Key::Esc {
            return Outcome::Close;
        }
        // Reread: the timer may have changed since the popup was drawn, and a
        // key must never act on a screen the user is no longer looking at.
        // Any key redraws for the new timer, even one with no button.
        let saved = ctx.load();
        let now = Timestamp::now();
        let view = timeline::view(&saved, now);
        if !self.shows(&saved, view) {
            return Outcome::Reopen(saved);
        }
        let Some(action) = action else {
            return Outcome::Stay;
        };
        self.act(action, ctx, &saved, view, now)
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
        // Beside the new timer, the view it leads to, which picks the sidebar
        // token. A break shows no token, so ending one leaves the sidebar be.
        let (next, leads_to) = match (action, view) {
            (Action::Back, _) => return self.back(ctx),
            (Action::Pause, View::Focus { left_secs }) => (
                Timer {
                    mode: Mode::Paused,
                    ends_at: None,
                    left_secs: Some(left_secs),
                    ..saved.clone()
                },
                Some(View::Paused { left_secs }),
            ),
            (Action::Resume, View::Paused { left_secs }) => (
                Timer {
                    mode: Mode::Focus,
                    ends_at: Some(Timestamp(now.0 + left_secs)),
                    left_secs: None,
                    ..saved.clone()
                },
                Some(View::Focus { left_secs }),
            ),
            (Action::Stop, _) => (ended(saved), Some(View::Idle)),
            (Action::EndBreak, _) => (ended(saved), None),
            // The screen was checked against the timer, so no other pair.
            _ => return Outcome::Stay,
        };
        if let Err(message) = ctx.save(&next) {
            self.error = Some(message);
            return Outcome::Stay;
        }
        // The tab bar sets the token again every second, so a silent herdr
        // only delays it.
        if let (Some(pane), Some(view)) = (saved.home_pane.as_deref(), leads_to) {
            finish_quietly(
                ctx.herdr
                    .report_focus_token(pane, format::focus_token(view)),
            );
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
/// stay.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buttons_are_found_where_they_are_drawn() {
        let timer = Timer {
            mode: Mode::Paused,
            left_secs: Some(60),
            ..Timer::default()
        };
        let popup = ControlPopup {
            timer,
            phase: Phase::Paused,
            away: None,
            error: None,
        };
        // ` Resume ⏎ ` is 10 columns, a gap of 1, then ` Stop s `.
        assert_eq!(popup.button_at(1, BUTTON_ROW), Some(Action::Resume));
        assert_eq!(popup.button_at(10, BUTTON_ROW), Some(Action::Resume));
        assert_eq!(popup.button_at(11, BUTTON_ROW), None);
        assert_eq!(popup.button_at(12, BUTTON_ROW), Some(Action::Stop));
        assert_eq!(popup.button_at(19, BUTTON_ROW), Some(Action::Stop));
        assert_eq!(popup.button_at(20, BUTTON_ROW), None);
        assert_eq!(popup.button_at(2, BUTTON_ROW + 1), None);
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

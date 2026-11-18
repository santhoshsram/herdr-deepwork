//! The start popup: pick a focus and a break length, then start.

use crate::herdr::Herdr;
use crate::keys::Key;
use crate::panel::{Context, Outcome, is_running};
use crate::state::{self, Mode, Timer, Timestamp};
use crate::timeline;

const PRESETS: [u16; 3] = [15, 25, 50];
const BREAKS: [u8; 4] = [0, 2, 5, 10];
/// Index of the custom chip on the Focus row, after the presets.
const CUSTOM: usize = PRESETS.len();
/// Index of the 5 minute break, the first-run choice.
const DEFAULT_BREAK: usize = 2;
const MAX_NAME: usize = 24;
const MAX_DIGITS: usize = 3;
const FOCUS_RANGE: std::ops::RangeInclusive<u16> = 1..=180;

const RANGE_ERROR: &str = "1 to 180";
const NOT_ANSWERING: &str = "herdr is not answering";
const NOT_SAVED: &str = "Could not save";

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RED: &str = "\x1b[31m";
const REVERSE: &str = "\x1b[7m";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Row {
    Focus,
    Break,
}

/// The start popup's state.
#[derive(Debug)]
pub struct StartPopup {
    name: String,
    row: Row,
    focus_sel: usize,
    break_sel: usize,
    /// Digits of the custom length.
    custom: String,
    /// Whether the user typed into `custom` since reaching it; the first
    /// digit replaces what was shown.
    typing: bool,
    error: Option<&'static str>,
}

/// Opens the popup for the pane in `ctx`, preselecting the choices in `timer`.
///
/// A pane herdr cannot describe is named `home`.
#[must_use]
pub fn open(ctx: &Context, timer: &Timer) -> StartPopup {
    let name = ctx
        .pane
        .as_deref()
        .and_then(|pane| ctx.herdr.get_pane(pane)?.finish())
        .map_or_else(|| String::from("home"), |reply| home_name(&reply));
    StartPopup::new(name, timer)
}

/// The name for a pane from the reply to `pane get`: its terminal title,
/// else its agent kind, else `home`; cut to 23 characters and `…` past 24.
#[must_use]
pub fn home_name(reply: &str) -> String {
    let pane = serde_json::from_str::<serde_json::Value>(reply)
        .ok()
        .and_then(|v| v.pointer("/result/pane").cloned());
    let text = |key: &str| {
        let raw = pane.as_ref()?.get(key)?.as_str()?;
        let clean: String = raw.chars().filter(|c| !c.is_control()).collect();
        let clean = clean.trim();
        (!clean.is_empty()).then(|| clean.to_owned())
    };
    let name = text("terminal_title_stripped")
        .or_else(|| text("agent"))
        .unwrap_or_else(|| String::from("home"));
    if name.chars().count() > MAX_NAME {
        let head: String = name.chars().take(MAX_NAME - 1).collect();
        format!("{head}…")
    } else {
        name
    }
}

impl StartPopup {
    fn new(name: String, timer: &Timer) -> Self {
        let preset = PRESETS.iter().position(|&m| m == timer.last_focus_mins);
        // A last focus off the presets was a custom one, so it shows there.
        let custom = match preset {
            Some(_) => timer.last_custom_mins,
            None => Some(timer.last_focus_mins),
        };
        Self {
            name,
            row: Row::Focus,
            focus_sel: preset.unwrap_or(CUSTOM),
            break_sel: BREAKS
                .iter()
                .position(|&m| m == timer.last_break_mins)
                .unwrap_or(DEFAULT_BREAK),
            custom: custom.map(|m| m.to_string()).unwrap_or_default(),
            typing: false,
            error: None,
        }
    }

    /// The focus length chosen, when it is valid.
    fn minutes(&self) -> Option<u16> {
        let minutes = match PRESETS.get(self.focus_sel) {
            Some(&preset) => preset,
            None => self.custom.parse().ok()?,
        };
        FOCUS_RANGE.contains(&minutes).then_some(minutes)
    }

    /// The screen, one string per line.
    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        let mut focus: Vec<String> = PRESETS
            .iter()
            .enumerate()
            .map(|(i, m)| self.chip(Row::Focus, i, &m.to_string()))
            .collect();
        focus.push(self.chip(Row::Focus, CUSTOM, &self.custom_label()));
        let breaks: Vec<String> = BREAKS
            .iter()
            .enumerate()
            .map(|(i, &m)| {
                let label = if m == 0 {
                    String::from("off")
                } else {
                    m.to_string()
                };
                self.chip(Row::Break, i, &label)
            })
            .collect();
        let hint = |key: &str, what: &str| format!("{BOLD}{key}{RESET} {what}");
        vec![
            format!("{DIM}focus{RESET}"),
            String::new(),
            format!("Focus on {BOLD}{}{RESET}", self.name),
            String::new(),
            format!("{DIM}Focus{RESET}  {} {DIM}min{RESET}", focus.join(" ")),
            // Always one line, so the Break row does not jump when it appears.
            format!("       {RED}{}{RESET}", self.error.unwrap_or_default()),
            format!("{DIM}Break{RESET}  {} {DIM}min{RESET}", breaks.join(" ")),
            String::new(),
            [
                hint("⏎", "start"),
                hint("← →", "change"),
                hint("↑ ↓", "focus / break"),
                hint("type a number", "custom minutes"),
                hint("esc", "close"),
            ]
            .join("  "),
        ]
    }

    /// The custom chip: the number once selected, else the word `custom`.
    fn custom_label(&self) -> String {
        if self.custom.is_empty() || self.focus_sel != CUSTOM {
            String::from("custom")
        } else if self.row == Row::Focus {
            format!("{}▌", self.custom)
        } else {
            self.custom.clone()
        }
    }

    /// The selected chip of the active row is reversed, the selected chip of
    /// the other row is bold, so both choices stay visible.
    fn chip(&self, row: Row, index: usize, label: &str) -> String {
        let selected = match row {
            Row::Focus => self.focus_sel == index,
            Row::Break => self.break_sel == index,
        };
        match (selected, self.row == row) {
            (true, true) => format!("{REVERSE} {label} {RESET}"),
            (true, false) => format!("{BOLD} {label} {RESET}"),
            (false, _) => format!(" {label} "),
        }
    }

    /// Acts on one key.
    pub fn press(&mut self, key: Key, ctx: &Context) -> Outcome {
        self.error = None;
        match key {
            Key::Esc => return Outcome::Close,
            Key::Enter => return self.start(ctx),
            Key::Left => self.step(false),
            Key::Right => self.step(true),
            Key::Up | Key::Down => {
                self.row = match self.row {
                    Row::Focus => Row::Break,
                    Row::Break => Row::Focus,
                };
            }
            Key::Digit(d) => self.digit(d),
            Key::Backspace if self.focus_sel == CUSTOM => {
                self.custom.pop();
                self.typing = true;
            }
            Key::Backspace | Key::Char(_) | Key::Other => {}
        }
        Outcome::Stay
    }

    fn step(&mut self, forward: bool) {
        self.typing = false;
        let (sel, last) = match self.row {
            Row::Focus => (&mut self.focus_sel, CUSTOM),
            Row::Break => (&mut self.break_sel, BREAKS.len() - 1),
        };
        *sel = if forward {
            (*sel + 1).min(last)
        } else {
            sel.saturating_sub(1)
        };
    }

    fn digit(&mut self, digit: u8) {
        self.row = Row::Focus;
        self.focus_sel = CUSTOM;
        if !self.typing {
            self.custom.clear();
            self.typing = true;
        }
        // A leading zero would only use up one of the three digits.
        if self.custom.len() < MAX_DIGITS && !(self.custom.is_empty() && digit == 0) {
            self.custom.push(char::from(b'0' + digit));
        }
    }

    /// Starts the focus, unless the choice is invalid or a timer began since
    /// the popup opened.
    fn start(&mut self, ctx: &Context) -> Outcome {
        let Some(minutes) = self.minutes() else {
            self.error = Some(RANGE_ERROR);
            return Outcome::Stay;
        };
        // Reread: another popup or a timer may have started since this opened.
        let saved = ctx.load();
        let now = Timestamp::now();
        if is_running(timeline::view(&saved, now)) {
            return Outcome::Running;
        }
        let Some(pane) = ctx.pane.as_deref() else {
            self.error = Some(NOT_ANSWERING);
            return Outcome::Stay;
        };
        let Some(path) = ctx.timer_path.as_deref() else {
            self.error = Some(NOT_SAVED);
            return Outcome::Stay;
        };
        let started = self.started(&saved, pane, minutes, now);
        if state::save(path, &started).is_err() {
            self.error = Some(NOT_SAVED);
            return Outcome::Stay;
        }
        // The tab bar sets the token again every second, so a silent herdr
        // only delays it.
        set_tomato(&ctx.herdr, pane);
        Outcome::Close
    }

    fn started(&self, saved: &Timer, pane: &str, minutes: u16, now: Timestamp) -> Timer {
        let break_mins = BREAKS[self.break_sel];
        // A custom number equal to a preset is remembered as the preset.
        let last_custom_mins = if PRESETS.contains(&minutes) {
            saved.last_custom_mins
        } else {
            Some(minutes)
        };
        Timer {
            mode: Mode::Focus,
            home_pane: Some(pane.to_owned()),
            home_name: Some(self.name.clone()),
            ends_at: Some(Timestamp(now.0 + u64::from(minutes) * 60)),
            left_secs: None,
            break_mins,
            last_focus_mins: minutes,
            last_break_mins: break_mins,
            last_custom_mins,
            message: None,
            message_ends_at: None,
        }
    }
}

fn set_tomato(herdr: &Herdr, pane: &str) {
    if let Some(call) = herdr.set_focus_token(pane, "🍅") {
        let _ = call.finish();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pane(title: &str, agent: Option<&str>) -> String {
        let mut pane = serde_json::json!({"terminal_title_stripped": title});
        if let Some(agent) = agent {
            pane["agent"] = serde_json::json!(agent);
        }
        serde_json::json!({"result": {"pane": pane}}).to_string()
    }

    #[test]
    fn name_falls_back_from_title_to_agent_to_home() {
        assert_eq!(home_name(&pane("build", Some("claude"))), "build");
        assert_eq!(home_name(&pane("", Some("claude"))), "claude");
        assert_eq!(home_name(&pane("", None)), "home");
        assert_eq!(home_name("not json"), "home");
    }

    #[test]
    fn long_names_are_cut_to_23_and_an_ellipsis() {
        let name = home_name(&pane("abcdefghijklmnopqrstuvwxyz", None));
        assert_eq!(name, "abcdefghijklmnopqrstuvw…");
        let fits = "abcdefghijklmnopqrstuvwx";
        assert_eq!(home_name(&pane(fits, None)), fits);
    }

    #[test]
    fn the_first_digit_replaces_the_number_shown() {
        let timer = Timer {
            last_custom_mins: Some(40),
            ..Timer::default()
        };
        let mut popup = StartPopup::new("home".into(), &timer);
        popup.step(true);
        popup.step(true);
        assert_eq!(popup.custom, "40");
        popup.digit(2);
        popup.digit(5);
        assert_eq!(popup.custom, "25");
        assert_eq!(popup.minutes(), Some(25));
        popup.digit(0);
        popup.digit(0);
        assert_eq!(popup.custom, "250");
        assert_eq!(popup.minutes(), None);
    }

    #[test]
    fn an_unusual_last_focus_opens_on_custom() {
        let timer = Timer {
            last_focus_mins: 40,
            last_custom_mins: Some(40),
            ..Timer::default()
        };
        let popup = StartPopup::new("home".into(), &timer);
        assert_eq!(popup.focus_sel, CUSTOM);
        assert_eq!(popup.minutes(), Some(40));
    }
}

//! The start popup: pick a focus and a break length, then start.

use std::sync::LazyLock;

use crate::format;
use crate::herdr::{Call, finish_quietly};
use crate::keys::Key;
use crate::panel::{
    self, BOLD, Chip, Context, DIM, MARGIN, NOT_ANSWERING, Outcome, RED, RESET, is_running,
};
use crate::state::{BREAK_CHOICES, FOCUS_RANGE, Mode, Timer, Timestamp};
use crate::timeline;

const PRESETS: [u16; 3] = [15, 25, 50];
/// Index of the custom chip on the Focus row, after the presets.
const CUSTOM: usize = PRESETS.len();
/// The name of a pane herdr cannot describe.
const UNNAMED: &str = "home";
const MAX_NAME: usize = 24;
const MAX_DIGITS: usize = 3;
/// Rows of the two option rows on screen, each 3 lines of boxes, counted
/// from 1.
const FOCUS_ROWS: std::ops::RangeInclusive<u16> = 4..=6;
const BREAK_ROWS: std::ops::RangeInclusive<u16> = 8..=10;
/// Columns taken by `Focus` and `Break` and the space after them.
const LABEL_WIDTH: usize = 8;
/// Column of the first chip, counted from 1.
const FIRST_CHIP_COL: usize = MARGIN.len() + LABEL_WIDTH + 1;
/// The custom chip keeps this width, so the row does not shift as digits
/// are typed.
const CUSTOM_WIDTH: usize = 6;

static RANGE_ERROR: LazyLock<String> =
    LazyLock::new(|| format!("{} to {}", FOCUS_RANGE.start(), FOCUS_RANGE.end()));

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

/// The start popup while herdr looks up the pane it opens for.
#[derive(Debug)]
pub struct Opening {
    popup: StartPopup,
    lookup: Option<Call>,
}

/// Starts opening the popup for the pane in `ctx`, preselecting the choices
/// in `timer`; [`Opening::finish`] waits for herdr.
#[must_use]
pub fn begin(ctx: &Context, timer: &Timer) -> Opening {
    Opening {
        popup: StartPopup::new(String::from(UNNAMED), timer),
        lookup: ctx
            .pane
            .as_deref()
            .and_then(|pane| ctx.herdr.get_pane(pane)),
    }
}

impl Opening {
    /// The popup, named from herdr's answer; a pane herdr cannot describe is
    /// named `home`.
    #[must_use]
    pub fn finish(mut self) -> StartPopup {
        if let Some(reply) = self.lookup.and_then(Call::finish) {
            self.popup.name = home_name(&reply);
        }
        self.popup
    }
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
        let clean = format::printable(raw);
        let clean = clean.trim();
        (!clean.is_empty()).then(|| clean.to_owned())
    };
    let name = text("terminal_title_stripped")
        .or_else(|| text("agent"))
        .unwrap_or_else(|| String::from(UNNAMED));
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
        let break_index = |mins: u8| BREAK_CHOICES.iter().position(|&m| m == mins);
        let break_sel = break_index(timer.last_break_mins)
            .or_else(|| break_index(Timer::default().last_break_mins))
            .expect("the first-run break is one of the choices");
        Self {
            name,
            row: Row::Focus,
            focus_sel: preset.unwrap_or(CUSTOM),
            break_sel,
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
        let row = |which: Row, labels: Vec<String>, name: &str| -> Vec<String> {
            let styled: Vec<(String, Chip)> = labels
                .into_iter()
                .enumerate()
                .map(|(i, label)| {
                    let style = self.chip(which, i);
                    (label, style)
                })
                .collect();
            let [top, middle, bottom] = panel::chips(&styled);
            let pad = " ".repeat(LABEL_WIDTH);
            vec![
                format!("{MARGIN}{pad}{top}"),
                format!("{MARGIN}{DIM}{name:<LABEL_WIDTH$}{RESET}{middle}  {DIM}min{RESET}"),
                format!("{MARGIN}{pad}{bottom}"),
            ]
        };
        let mut lines = vec![
            String::new(),
            format!("{MARGIN}Focus on {BOLD}{}{RESET}", self.name),
            String::new(),
        ];
        lines.extend(row(Row::Focus, self.focus_labels(), "Focus"));
        // Always one line, so the Break row does not jump when it appears.
        lines.push(format!(
            "{MARGIN}{}{RED}{}{RESET}",
            " ".repeat(LABEL_WIDTH),
            self.error.unwrap_or_default()
        ));
        lines.extend(row(Row::Break, break_labels(), "Break"));
        lines.extend([
            panel::rule(),
            panel::hints(&[("⏎", "start"), ("← →", "change"), ("↑ ↓", "focus / break")]),
            panel::hints(&[("type a number", "custom minutes"), ("esc", "close")]),
        ]);
        lines
    }

    fn focus_labels(&self) -> Vec<String> {
        let mut labels: Vec<String> = PRESETS.iter().map(u16::to_string).collect();
        labels.push(format!("{:^CUSTOM_WIDTH$}", self.custom_label()));
        labels
    }

    /// The chip under a click: its row and its index in the row.
    fn chip_at(&self, col: u16, row: u16) -> Option<(Row, usize)> {
        let (which, labels) = if FOCUS_ROWS.contains(&row) {
            (Row::Focus, self.focus_labels())
        } else if BREAK_ROWS.contains(&row) {
            (Row::Break, break_labels())
        } else {
            return None;
        };
        let index = panel::hit(&labels, FIRST_CHIP_COL, col)?;
        Some((which, index))
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

    /// The chosen chip of the active row is filled, the chosen chip of the
    /// other row outlined, so both choices stay visible.
    fn chip(&self, row: Row, index: usize) -> Chip {
        let selected = match row {
            Row::Focus => self.focus_sel == index,
            Row::Break => self.break_sel == index,
        };
        match (selected, self.row == row) {
            (true, true) => Chip::Selected,
            (true, false) => Chip::Picked,
            (false, _) => Chip::Plain,
        }
    }

    /// Acts on one key.
    pub fn press(&mut self, key: Key, ctx: &Context) -> Outcome {
        if let Key::Click { col, row } = key {
            return self.click(col, row, ctx);
        }
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
            Key::Backspace | Key::Char(_) | Key::Click { .. } | Key::Other => {}
        }
        Outcome::Stay
    }

    /// A break chip selects; a focus chip starts, except the custom chip,
    /// which first selects and starts only once chosen with a number typed.
    fn click(&mut self, col: u16, row: u16, ctx: &Context) -> Outcome {
        let Some((which, index)) = self.chip_at(col, row) else {
            return Outcome::Stay;
        };
        self.error = None;
        self.typing = false;
        let chosen = self.row == Row::Focus && self.focus_sel == CUSTOM && !self.custom.is_empty();
        self.row = which;
        match which {
            Row::Break => {
                self.break_sel = index;
                Outcome::Stay
            }
            Row::Focus => {
                self.focus_sel = index;
                if index == CUSTOM && !chosen {
                    Outcome::Stay
                } else {
                    self.start(ctx)
                }
            }
        }
    }

    fn step(&mut self, forward: bool) {
        self.typing = false;
        let (sel, last) = match self.row {
            Row::Focus => (&mut self.focus_sel, CUSTOM),
            Row::Break => (&mut self.break_sel, BREAK_CHOICES.len() - 1),
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
            self.error = Some(RANGE_ERROR.as_str());
            return Outcome::Stay;
        };
        // Reread: another popup or a timer may have started since this opened.
        let saved = ctx.load();
        let now = Timestamp::now();
        if is_running(timeline::view(&saved, now)) {
            return Outcome::Reopen(saved);
        }
        let Some(pane) = ctx.pane.as_deref() else {
            self.error = Some(NOT_ANSWERING);
            return Outcome::Stay;
        };
        let started = self.started(&saved, pane, minutes, now);
        if let Err(message) = ctx.save(&started) {
            self.error = Some(message);
            return Outcome::Stay;
        }
        // The tab bar sets the token again every second, so a silent herdr
        // only delays it.
        finish_quietly(ctx.herdr.report_focus_token(pane, Some(format::TOMATO)));
        Outcome::Close
    }

    fn started(&self, saved: &Timer, pane: &str, minutes: u16, now: Timestamp) -> Timer {
        let break_mins = BREAK_CHOICES[self.break_sel];
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

fn break_labels() -> Vec<String> {
    BREAK_CHOICES
        .iter()
        .map(|&m| {
            if m == 0 {
                String::from("off")
            } else {
                m.to_string()
            }
        })
        .collect()
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
    fn chips_are_found_where_they_are_drawn() {
        let popup = StartPopup::new("home".into(), &Timer::default());
        let plain = |line: &str| {
            let mut out = String::new();
            let mut chars = line.chars();
            while let Some(c) = chars.next() {
                if c == '\x1b' {
                    for e in chars.by_ref() {
                        if e == 'm' {
                            break;
                        }
                    }
                } else {
                    out.push(c);
                }
            }
            out
        };
        let lines = popup.lines();
        let focus_row = *FOCUS_ROWS.start() + 1;
        let break_row = *BREAK_ROWS.start() + 1;
        let focus = plain(&lines[usize::from(focus_row) - 1]);
        let breaks = plain(&lines[usize::from(break_row) - 1]);
        let at = |line: &str, needle: &str| {
            let start = line.find(needle).expect("drawn");
            u16::try_from(line[..start].chars().count() + 1).expect("fits")
        };
        assert_eq!(
            popup.chip_at(at(&focus, "50"), focus_row),
            Some((Row::Focus, 2))
        );
        assert_eq!(
            popup.chip_at(at(&focus, "custom"), focus_row),
            Some((Row::Focus, CUSTOM))
        );
        // The box edges count as the chip too.
        assert_eq!(
            popup.chip_at(at(&focus, "custom"), *FOCUS_ROWS.start()),
            Some((Row::Focus, CUSTOM))
        );
        assert_eq!(
            popup.chip_at(at(&breaks, "10"), *BREAK_ROWS.end()),
            Some((Row::Break, 3))
        );
        assert_eq!(
            popup.chip_at(at(&breaks, "off") - 2, break_row),
            Some((Row::Break, 0))
        );
        // The column between two chips is neither.
        assert_eq!(popup.chip_at(at(&breaks, "2") - 3, break_row), None);
        assert_eq!(
            popup.chip_at(at(&breaks, "10"), *FOCUS_ROWS.end() + 1),
            None
        );
        assert_eq!(popup.chip_at(1, break_row), None);
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

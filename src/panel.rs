//! `herdr-focus panel`: the popup, start or control, drawn on the terminal.

use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use std::process::{Command, ExitCode, Stdio};
use std::time::Duration;

use crate::herdr::Herdr;
use crate::keys::KeyReader;
use crate::panel_control::{self, ControlPopup};
use crate::panel_start::{self, StartPopup};
use crate::state::{self, Timer};
use crate::timeline::{self, View};

/// How long herdr gets to answer a call the popup waits on.
const HERDR_BUDGET: Duration = Duration::from_secs(1);

const CLEAR: &str = "\x1b[H\x1b[2J";
const HIDE_CURSOR: &str = "\x1b[?25l";
const SHOW_CURSOR: &str = "\x1b[?25h";
const MOUSE_ON: &str = "\x1b[?1000h\x1b[?1006h";
const MOUSE_OFF: &str = "\x1b[?1006l\x1b[?1000l";

/// Ends any style below.
pub const RESET: &str = "\x1b[0m";
/// Bold text.
pub const BOLD: &str = "\x1b[1m";
/// Faint text.
pub const DIM: &str = "\x1b[2m";
/// Red text.
pub const RED: &str = "\x1b[31m";
/// Dark text on the accent blue.
pub const ON_BLUE: &str = "\x1b[30;44m";
/// The terminal's grey, as a muted fill.
pub const ON_GREY: &str = "\x1b[100m";

/// Shown when herdr cannot be reached or does not answer in time.
pub const NOT_ANSWERING: &str = "herdr is not answering";
/// Shown when the timer file cannot be written.
pub const NOT_SAVED: &str = "Could not save";

/// A key hint: the key, then what it does, fainter.
#[must_use]
pub fn hint(key: &str, what: &str) -> String {
    format!("{key} {DIM}{what}{RESET}")
}

/// What a popup needs from outside: herdr, the timer file, the pane the user
/// was in.
#[derive(Debug)]
pub struct Context {
    /// The `herdr` command.
    pub herdr: Herdr,
    /// The timer file; `None` when no state folder is known.
    pub timer_path: Option<PathBuf>,
    /// The pane focused before the popup opened.
    pub pane: Option<String>,
}

impl Context {
    fn from_env() -> Self {
        Self {
            herdr: Herdr::from_env(HERDR_BUDGET),
            timer_path: state::current_path(),
            pane: current_pane(),
        }
    }

    /// The saved timer, or the first-run timer when it is missing or invalid.
    #[must_use]
    pub fn load(&self) -> Timer {
        state::load_or_default(self.timer_path.as_deref())
    }

    /// Writes `timer` to the timer file.
    ///
    /// # Errors
    /// Returns [`NOT_SAVED`], the message to show, when no state folder is
    /// known or the file cannot be written.
    pub fn save(&self, timer: &Timer) -> Result<(), &'static str> {
        let path = self.timer_path.as_deref().ok_or(NOT_SAVED)?;
        state::save(path, timer).map_err(|_| NOT_SAVED)
    }
}

/// `focused_pane_id` from `$HERDR_PLUGIN_CONTEXT_JSON`, else
/// `$HERDR_ACTIVE_PANE_ID`.
fn current_pane() -> Option<String> {
    let from_context = std::env::var("HERDR_PLUGIN_CONTEXT_JSON")
        .ok()
        .and_then(|json| serde_json::from_str::<serde_json::Value>(&json).ok())
        .and_then(|v| v.get("focused_pane_id")?.as_str().map(str::to_owned))
        .filter(|pane| !pane.is_empty());
    from_context.or_else(|| {
        std::env::var("HERDR_ACTIVE_PANE_ID")
            .ok()
            .filter(|pane| !pane.is_empty())
    })
}

/// Whether a timer is running or held, so no second one may start.
///
/// A message that only reports an ended timer does not count: the start popup
/// opens over it.
#[must_use]
pub fn is_running(view: View) -> bool {
    timeline::phase(view).is_some()
}

/// Columns of space left of every popup line.
pub const MARGIN: &str = "  ";
/// Columns of the divider above the key hints.
const RULE_WIDTH: usize = 44;
/// Columns of one option in a row of options.
pub const OPTION_WIDTH: usize = 8;
/// Columns between two buttons.
const BUTTON_GAP: &str = "  ";

/// A row of options drawn as one bar of equal-width cells, like a segmented
/// control: the chosen cell filled with the accent, the rest with a muted
/// fill.
#[must_use]
pub fn options<S: AsRef<str>>(labels: &[S], chosen: usize) -> String {
    labels
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let fill = if i == chosen { ON_BLUE } else { ON_GREY };
            format!("{fill}{:^OPTION_WIDTH$}{RESET}", label.as_ref())
        })
        .collect()
}

/// The index of the option under column `col`, in a row drawn by
/// [`options`] from column `first_col`; columns count from 1.
#[must_use]
pub fn option_at(count: usize, first_col: usize, col: u16) -> Option<usize> {
    let offset = usize::from(col).checked_sub(first_col)?;
    let index = offset / OPTION_WIDTH;
    (index < count).then_some(index)
}

/// Buttons on one line, each its label with 2 columns of fill on either
/// side; the first filled with the accent when `primary`, the rest muted.
#[must_use]
pub fn buttons<S: AsRef<str>>(labels: &[S], primary: bool) -> String {
    let drawn: Vec<String> = labels
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let fill = if primary && i == 0 { ON_BLUE } else { ON_GREY };
            format!("{fill}  {}  {RESET}", label.as_ref())
        })
        .collect();
    drawn.join(BUTTON_GAP)
}

/// The index of the button under column `col`, in a line drawn by
/// [`buttons`] from column `first_col`; columns count from 1.
#[must_use]
pub fn button_at<S: AsRef<str>>(labels: &[S], first_col: usize, col: u16) -> Option<usize> {
    let col = usize::from(col);
    let mut first = first_col;
    for (index, label) in labels.iter().enumerate() {
        let last = first + label.as_ref().chars().count() + 3;
        if (first..=last).contains(&col) {
            return Some(index);
        }
        first = last + 1 + BUTTON_GAP.len();
    }
    None
}

/// The divider above the key hints.
#[must_use]
pub fn rule() -> String {
    format!("{MARGIN}{DIM}{}{RESET}", "─".repeat(RULE_WIDTH))
}

/// Key hints on one line.
#[must_use]
pub fn hints(pairs: &[(&str, &str)]) -> String {
    let hints: Vec<String> = pairs.iter().map(|(key, what)| hint(key, what)).collect();
    format!("{MARGIN}{}", hints.join("  "))
}

/// The terminal in the mode the popup needs, put back on drop.
struct Terminal {
    saved: Option<String>,
}

impl Terminal {
    /// Turns off line editing and echo through `stty`, only when stdin is a
    /// terminal.
    fn enter() -> Self {
        let saved = std::io::stdin().is_terminal().then(stty_save).flatten();
        if saved.is_some() {
            stty(&["-icanon", "-echo"]);
        }
        draw(&format!("{HIDE_CURSOR}{MOUSE_ON}"));
        Self { saved }
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        draw(&format!("{MOUSE_OFF}{SHOW_CURSOR}"));
        if let Some(saved) = &self.saved {
            stty(&[saved]);
        }
    }
}

fn stty_save() -> Option<String> {
    let out = Command::new("stty")
        .arg("-g")
        .stdin(Stdio::inherit())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    let saved = String::from_utf8(out.stdout).ok()?.trim().to_owned();
    (out.status.success() && !saved.is_empty()).then_some(saved)
}

fn stty(args: &[&str]) {
    let _ = Command::new("stty")
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

/// Writes to the terminal; a closed terminal is the popup's own end, which
/// the stdin EOF reports.
fn draw(text: &str) {
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(text.as_bytes());
    let _ = out.flush();
}

fn draw_frame(lines: &[String]) {
    draw(&format!("{CLEAR}{}", lines.join("\r\n")));
}

enum Screen {
    Start(StartPopup),
    Control(ControlPopup),
}

/// What a screen does with a key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Keep the popup open.
    Stay,
    /// Close the popup.
    Close,
    /// The saved timer is not the one on screen, so the popup that fits it
    /// takes over.
    Reopen(Timer),
}

/// A popup waiting for herdr to answer about a pane.
enum Opening {
    Start(panel_start::Opening),
    Control(panel_control::Opening),
}

impl Opening {
    /// Starts opening the popup that fits the timer: control while it runs,
    /// else start.
    fn begin(ctx: &Context, timer: &Timer) -> Self {
        match ControlPopup::begin(ctx, timer) {
            Some(opening) => Self::Control(opening),
            None => Self::Start(panel_start::begin(ctx, timer)),
        }
    }

    fn finish(self) -> Screen {
        match self {
            Self::Start(opening) => Screen::Start(opening.finish()),
            Self::Control(opening) => Screen::Control(opening.finish()),
        }
    }
}

/// The popup that fits the timer, once herdr has answered.
fn screen_for(ctx: &Context, timer: &Timer) -> Screen {
    Opening::begin(ctx, timer).finish()
}

/// Runs the popup until a key closes it or stdin reaches EOF.
#[must_use]
pub fn run() -> ExitCode {
    let ctx = Context::from_env();
    // herdr answers while stty sets up the terminal.
    let opening = Opening::begin(&ctx, &ctx.load());
    let _terminal = Terminal::enter();
    let mut screen = opening.finish();
    let mut keys = KeyReader::from_stdin();
    let mut shown = Vec::new();
    loop {
        let frame = match &screen {
            Screen::Start(popup) => popup.lines(),
            Screen::Control(popup) => popup.lines(),
        };
        // Clearing to draw the same frame again would only flicker.
        if frame != shown {
            draw_frame(&frame);
            shown = frame;
        }
        let Some(key) = keys.next_key() else { break };
        let outcome = match &mut screen {
            Screen::Start(popup) => popup.press(key, &ctx),
            Screen::Control(popup) => popup.press(key, &ctx),
        };
        match outcome {
            Outcome::Stay => {}
            Outcome::Close => break,
            Outcome::Reopen(timer) => screen = screen_for(&ctx, &timer),
        }
    }
    ExitCode::SUCCESS
}

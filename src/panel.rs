//! `herdr-focus panel`: the popup, start or control, drawn on the terminal.

use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use std::process::{Command, ExitCode, Stdio};
use std::time::Duration;

use crate::herdr::Herdr;
use crate::keys::KeyReader;
use crate::panel_control::ControlPopup;
use crate::panel_start::{self, StartPopup};
use crate::state::{self, Timer};
use crate::timeline::View;

/// How long herdr gets to answer a call the popup waits on.
const HERDR_BUDGET: Duration = Duration::from_secs(1);

const CLEAR: &str = "\x1b[H\x1b[2J";
const HIDE_CURSOR: &str = "\x1b[?25l";
const SHOW_CURSOR: &str = "\x1b[?25h";

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
            timer_path: state::state_dir()
                .map(|dir| state::timer_path(&dir, &state::session_name())),
            pane: current_pane(),
        }
    }

    /// The saved timer, or the first-run timer when it is missing or invalid.
    #[must_use]
    pub fn load(&self) -> Timer {
        self.timer_path
            .as_deref()
            .and_then(|path| state::load(path).ok())
            .unwrap_or_default()
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
    matches!(
        view,
        View::Focus { .. } | View::Paused { .. } | View::FocusDoneBreak { .. } | View::Break { .. }
    )
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
        draw(HIDE_CURSOR);
        Self { saved }
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        draw(SHOW_CURSOR);
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Keep the popup open.
    Stay,
    /// Close the popup.
    Close,
    /// A timer runs, so the control popup takes over.
    Running,
    /// No timer runs, so the start popup takes over.
    Idle,
}

/// The popup that fits the timer: control while it runs, else start.
fn screen_for(ctx: &Context, timer: &Timer) -> Screen {
    match ControlPopup::open(ctx, timer) {
        Some(popup) => Screen::Control(popup),
        None => Screen::Start(panel_start::open(ctx, timer)),
    }
}

/// Runs the popup until a key closes it or stdin reaches EOF.
#[must_use]
pub fn run() -> ExitCode {
    let ctx = Context::from_env();
    let mut screen = screen_for(&ctx, &ctx.load());
    let _terminal = Terminal::enter();
    let mut keys = KeyReader::from_stdin();
    loop {
        draw_frame(&match &screen {
            Screen::Start(popup) => popup.lines(),
            Screen::Control(popup) => popup.lines(),
        });
        let Some(key) = keys.next_key() else { break };
        let outcome = match &mut screen {
            Screen::Start(popup) => popup.press(key, &ctx),
            Screen::Control(popup) => popup.press(key, &ctx),
        };
        match outcome {
            Outcome::Stay => {}
            Outcome::Close => break,
            Outcome::Running | Outcome::Idle => screen = screen_for(&ctx, &ctx.load()),
        }
    }
    ExitCode::SUCCESS
}

//! `herdr-focus panel` takes SGR mouse clicks on its buttons and options as it takes keys.

use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

const HOME: &str = "w0:p1";
const NAME: &str = "notes";

/// A panel that has not exited by then is hung.
const PANEL_TIMEOUT: Duration = Duration::from_secs(5);

const ESC: &[u8] = b"\x1b";

/// A temp state folder holding the timer file, the fake herdr and its log.
struct StateDir {
    path: PathBuf,
}

impl StateDir {
    fn new(tag: &str) -> Self {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!(
            "herdr-focus-panel-mouse-{}-{}-{}",
            std::process::id(),
            n,
            tag
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp state dir");
        let state = Self { path };
        state.write_herdr();
        state
    }

    fn timer_path(&self) -> PathBuf {
        self.path
            .join("sessions")
            .join("default")
            .join("timer.json")
    }

    fn write_timer(&self, contents: &str) {
        let file = self.timer_path();
        fs::create_dir_all(file.parent().expect("timer file has a parent"))
            .expect("create session dir");
        fs::write(&file, contents).expect("write timer file");
    }

    fn read_timer_text(&self) -> Option<String> {
        fs::read_to_string(self.timer_path()).ok()
    }

    fn read_timer(&self) -> Value {
        let text = self.read_timer_text().expect("panel wrote timer.json");
        serde_json::from_str(&text).expect("timer.json is JSON")
    }

    fn herdr_path(&self) -> PathBuf {
        self.path.join("fake-herdr")
    }

    fn log_path(&self) -> PathBuf {
        self.path.join("herdr.log")
    }

    fn stdout_path(&self) -> PathBuf {
        self.path.join("panel.stdout")
    }

    /// Writes the fake herdr: a POSIX sh script that appends its argv as one
    /// line to the log and prints canned JSON.
    fn write_herdr(&self) {
        let script = FAKE_HERDR.replace("@LOG@", &self.log_path().to_string_lossy());
        let path = self.herdr_path();
        fs::write(&path, script).expect("write fake herdr");
        let mut perms = fs::metadata(&path).expect("stat fake herdr").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).expect("chmod fake herdr");

        // The first exec of a fresh script is slow on macOS; warm it, then
        // drop the warm-up call from the log.
        Command::new(&path)
            .args(["pane", "list"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("warm up fake herdr");
        fs::write(self.log_path(), "").expect("truncate herdr log");
    }

    /// The argv lines the fake herdr has logged so far.
    fn herdr_calls(&self) -> Vec<String> {
        fs::read_to_string(self.log_path())
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    /// Everything the panel has written to stdout so far, escapes included.
    fn raw_output(&self) -> String {
        let raw = fs::read(self.stdout_path()).unwrap_or_default();
        String::from_utf8_lossy(&raw).into_owned()
    }

    /// The popup as the terminal shows it after everything written so far.
    fn screen(&self) -> Screen {
        Screen::draw(&self.raw_output())
    }
}

impl Drop for StateDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

const FAKE_HERDR: &str = r#"#!/bin/sh
echo "$*" >> '@LOG@'
case "$1 $2" in
  "pane get")
    printf '{"id":"cli:pane:get","result":{"pane":{"agent":"claude","pane_id":"%s","terminal_title_stripped":"herdr-focus","workspace_id":"w0","tab_id":"w0:t1"},"type":"pane_info"}}\n' "$3"
    ;;
  "pane list")
    printf '{"id":"cli:pane:list","result":{"panes":[],"type":"pane_list"}}\n'
    ;;
  "pane report-metadata")
    printf '{"id":"cli:pane:report-metadata","result":{"type":"ok"}}\n'
    ;;
  "agent focus")
    printf '{"id":"cli:agent:focus","result":{"type":"ok"}}\n'
    ;;
  *)
    printf '{}\n'
    ;;
esac
"#;

/// A grid of the characters a terminal would show, rows and columns 1-based.
///
/// Understands what a popup may use to place text: carriage return, line
/// feed, cursor position `ESC [ row ; col H` and clearing the screen. Every
/// other escape sequence is skipped. Every character is one column wide.
struct Screen {
    rows: Vec<Vec<char>>,
}

impl Screen {
    fn draw(raw: &str) -> Self {
        let mut rows: Vec<Vec<char>> = Vec::new();
        let (mut row, mut col) = (0usize, 0usize);
        let put = |rows: &mut Vec<Vec<char>>, row: usize, col: usize, c: char| {
            if rows.len() <= row {
                rows.resize(row + 1, Vec::new());
            }
            if rows[row].len() <= col {
                rows[row].resize(col + 1, ' ');
            }
            rows[row][col] = c;
        };
        let mut chars = raw.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '\x1b' => match chars.next() {
                    Some('[') => {
                        let mut params = String::new();
                        let mut last = None;
                        for next in chars.by_ref() {
                            if ('\u{40}'..='\u{7e}').contains(&next) {
                                last = Some(next);
                                break;
                            }
                            params.push(next);
                        }
                        match last {
                            Some('H' | 'f') if !params.starts_with('?') => {
                                let mut parts = params.split(';');
                                let number = |part: Option<&str>| {
                                    part.and_then(|p| p.parse::<usize>().ok())
                                        .unwrap_or(1)
                                        .max(1)
                                };
                                row = number(parts.next()) - 1;
                                col = number(parts.next()) - 1;
                            }
                            Some('J') if params == "2" => rows.clear(),
                            _ => {}
                        }
                    }
                    Some(']') => {
                        while let Some(next) = chars.next() {
                            if next == '\x07' {
                                break;
                            }
                            if next == '\x1b' {
                                chars.next();
                                break;
                            }
                        }
                    }
                    _ => {}
                },
                '\r' => col = 0,
                '\n' => row += 1,
                c if c.is_control() => {}
                c => {
                    put(&mut rows, row, col, c);
                    col += 1;
                }
            }
        }
        Self { rows }
    }

    fn text(&self, row: usize) -> String {
        self.rows
            .get(row)
            .map(|cells| cells.iter().collect())
            .unwrap_or_default()
    }

    /// The 1-based `(col, row)` of the middle of the first `needle` found in
    /// a row accepted by `row_ok`.
    fn find(&self, row_ok: impl Fn(&str) -> bool, needle: &str) -> Option<(usize, usize)> {
        (0..self.rows.len()).find_map(|row| {
            let text = self.text(row);
            if !row_ok(&text) {
                return None;
            }
            let start = text.find(needle)?;
            let col = text[..start].chars().count();
            Some((col + needle.chars().count() / 2 + 1, row + 1))
        })
    }

    /// The middle line of the start popup's Focus options: `Focus   │ 15 │ … min`.
    fn focus_option(&self, label: &str) -> Option<(usize, usize)> {
        self.find(
            |row| row.trim_start().starts_with("Focus") && row.trim_end().ends_with("min"),
            label,
        )
    }

    /// The middle line of the start popup's Break options: `Break   │ off │ … min`.
    fn break_option(&self, label: &str) -> Option<(usize, usize)> {
        self.find(|row| row.trim_start().starts_with("Break"), label)
    }

    /// The first place `text` is drawn.
    fn anywhere(&self, text: &str) -> Option<(usize, usize)> {
        self.find(|_| true, text)
    }
}

/// A running `herdr-focus panel` with piped stdin.
struct Panel {
    child: Child,
    stdin: Option<ChildStdin>,
}

/// What a finished panel left behind.
struct Run {
    status: ExitStatus,
    /// stdout as written, escapes included.
    raw: String,
}

impl Panel {
    /// Starts the panel in the pane `HOME`.
    fn start(state: &StateDir) -> Self {
        let stdout = fs::File::create(state.stdout_path()).expect("create stdout file");
        let mut child = Command::new(env!("CARGO_BIN_EXE_herdr-focus"))
            .arg("panel")
            .env("HERDR_PLUGIN_STATE_DIR", &state.path)
            .env("HERDR_BIN_PATH", state.herdr_path())
            .env(
                "HERDR_PLUGIN_CONTEXT_JSON",
                format!(r#"{{"focused_pane_id":"{HOME}"}}"#),
            )
            .env_remove("HERDR_SESSION")
            .env_remove("HERDR_ACTIVE_PANE_ID")
            .stdin(Stdio::piped())
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::null())
            .spawn()
            .expect("run herdr-focus panel");
        let stdin = child.stdin.take();
        Self { child, stdin }
    }

    /// Sends bytes. A panel that already exited is not an error.
    fn send(&mut self, bytes: &[u8]) {
        if let Some(stdin) = self.stdin.as_mut() {
            let _ = stdin.write_all(bytes);
            let _ = stdin.flush();
        }
    }

    /// A left click at `(col, row)`: press, then release.
    fn click(&mut self, (col, row): (usize, usize)) {
        self.send(format!("\x1b[<0;{col};{row}M\x1b[<0;{col};{row}m").as_bytes());
    }

    /// Closes stdin and waits for the exit; a hung panel fails the test.
    fn finish(mut self, state: &StateDir) -> Run {
        drop(self.stdin.take());
        let started = Instant::now();
        let status = loop {
            if let Some(status) = self.child.try_wait().expect("poll panel") {
                break status;
            }
            if started.elapsed() > PANEL_TIMEOUT {
                let _ = self.child.kill();
                let _ = self.child.wait();
                panic!("panel did not exit within {PANEL_TIMEOUT:?}");
            }
            thread::sleep(Duration::from_millis(10));
        };
        Run {
            status,
            raw: state.raw_output(),
        }
    }
}

/// Waits until the popup shows `place`, then returns where; fails on timeout.
fn wait_for_place(
    state: &StateDir,
    what: &str,
    place: impl Fn(&Screen) -> Option<(usize, usize)>,
) -> (usize, usize) {
    let started = Instant::now();
    while started.elapsed() < PANEL_TIMEOUT {
        if let Some(found) = place(&state.screen()) {
            return found;
        }
        thread::sleep(Duration::from_millis(10));
    }
    panic!("the popup never showed {what}: {:?}", state.raw_output());
}

/// Formats Unix seconds as `YYYY-MM-DDTHH:MM:SSZ`.
fn rfc3339(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// Parses `YYYY-MM-DDTHH:MM:SSZ` into Unix seconds.
fn parse_rfc3339(text: &str) -> u64 {
    assert_eq!(text.len(), 20, "not YYYY-MM-DDTHH:MM:SSZ: {text}");
    assert!(text.ends_with('Z'), "not UTC: {text}");
    let num = |from: usize, to: usize| -> i64 {
        text[from..to]
            .parse()
            .unwrap_or_else(|_| panic!("not a number in {text}"))
    };
    let (year, month, day) = (num(0, 4), num(5, 7), num(8, 10));
    let (hour, minute, second) = (num(11, 13), num(14, 16), num(17, 19));
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    u64::try_from(days * 86_400 + hour * 3600 + minute * 60 + second).expect("after the epoch")
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_secs()
}

/// A focus on `HOME` as `NAME`, ending in 10 min.
fn focus_json() -> String {
    json!({
        "mode": "focus",
        "home_pane": HOME,
        "home_name": NAME,
        "ends_at": rfc3339(now_secs() + 600),
        "left_secs": null,
        "break_mins": 5,
        "last_focus_mins": 25,
        "last_break_mins": 5,
        "last_custom_mins": null,
        "message": null,
        "message_ends_at": null,
    })
    .to_string()
}

fn assert_exit_zero(run: &Run) {
    assert_eq!(run.status.code(), Some(0), "output: {:?}", run.raw);
}

/// The calls that act on the timer or the sidebar or the focused pane.
fn acting_calls(state: &StateDir) -> Vec<String> {
    state
        .herdr_calls()
        .into_iter()
        .filter(|call| call.starts_with("pane report-metadata") || call.starts_with("agent focus"))
        .collect()
}

#[test]
fn mouse_reporting_on_and_off() {
    let on = "\x1b[?1006h";
    let off = "\x1b[?1006l";
    for (tag, timer) in [("start", None), ("control", Some(focus_json()))] {
        let state = StateDir::new(&format!("reporting-{tag}"));
        if let Some(timer) = timer {
            state.write_timer(&timer);
        }
        let mut panel = Panel::start(&state);
        panel.send(ESC);
        let run = panel.finish(&state);

        assert_exit_zero(&run);
        let first_on = run.raw.find(on);
        let last_off = run.raw.rfind(off);
        assert!(first_on.is_some(), "{tag}: never turned on: {:?}", run.raw);
        assert!(last_off.is_some(), "{tag}: never turned off: {:?}", run.raw);
        assert!(
            first_on < last_off,
            "{tag}: reporting was turned off before it was turned on: {:?}",
            run.raw
        );
        assert!(
            run.raw.rfind(on) < last_off,
            "{tag}: reporting was left on at exit: {:?}",
            run.raw
        );
    }
}

#[test]
fn mouse_selects_and_starts() {
    // Clicking Break 10 only selects it: nothing starts.
    let state = StateDir::new("select");
    let mut panel = Panel::start(&state);
    let ten = wait_for_place(&state, "Break 10", |screen| screen.break_option("10"));
    panel.click(ten);
    let run = panel.finish(&state);
    assert_exit_zero(&run);
    assert_eq!(
        state.read_timer_text(),
        None,
        "selecting a break wrote timer.json"
    );
    assert!(
        acting_calls(&state).is_empty(),
        "selecting a break acted on herdr: {:?}",
        state.herdr_calls()
    );

    // Break 10, then Focus 50: a 50 min focus with a 10 min break.
    let state = StateDir::new("start");
    let mut panel = Panel::start(&state);
    let ten = wait_for_place(&state, "Break 10", |screen| screen.break_option("10"));
    let fifty = wait_for_place(&state, "Focus 50", |screen| screen.focus_option("50"));
    let before = now_secs();
    panel.click(ten);
    panel.click(fifty);
    let run = panel.finish(&state);
    let after = now_secs();

    assert_exit_zero(&run);
    let timer = state.read_timer();
    assert_eq!(timer["mode"], "focus", "{timer}");
    assert_eq!(timer["home_pane"], HOME, "{timer}");
    assert_eq!(timer["home_name"], "herdr-focus", "{timer}");
    assert_eq!(timer["left_secs"], Value::Null, "{timer}");
    assert_eq!(timer["break_mins"], 10, "{timer}");
    assert_eq!(timer["last_focus_mins"], 50, "{timer}");
    assert_eq!(timer["last_break_mins"], 10, "{timer}");
    let ends = parse_rfc3339(timer["ends_at"].as_str().expect("ends_at is a string"));
    // Written times are whole seconds, so allow one second either way.
    assert!(
        (before + 3000 - 1..=after + 3000 + 1).contains(&ends),
        "ends_at {} is not 50 min after the start: {timer}",
        timer["ends_at"]
    );
    let calls = state.herdr_calls();
    assert!(
        calls.iter().any(|call| {
            call.starts_with(&format!("pane report-metadata {HOME} "))
                && call.contains("--token focus=🍅")
        }),
        "no focus=🍅 call on {HOME}: {calls:?}"
    );
}

#[test]
fn mouse_stop() {
    let state = StateDir::new("stop");
    state.write_timer(&focus_json());
    let mut panel = Panel::start(&state);
    let stop = wait_for_place(&state, "Stop", |screen| screen.anywhere("Stop"));
    panel.click(stop);
    let run = panel.finish(&state);

    assert_exit_zero(&run);
    let timer = state.read_timer();
    assert_eq!(timer["mode"], "idle", "{timer}");
    assert_eq!(timer["message"], Value::Null, "{timer}");
    let calls = state.herdr_calls();
    assert!(
        calls.iter().any(|call| {
            call.starts_with(&format!("pane report-metadata {HOME} "))
                && call.contains("--source herdr-focus")
                && call.contains("--clear-token focus")
        }),
        "no clear-token call on {HOME}: {calls:?}"
    );
}

#[test]
fn mouse_click_elsewhere() {
    // Start popup: far outside, and on the name, which is text, not an option.
    let state = StateDir::new("elsewhere-start");
    let mut panel = Panel::start(&state);
    let name = wait_for_place(&state, "the home name", |screen| {
        screen.find(
            |row| row.trim_start().starts_with("Focus on"),
            "herdr-focus",
        )
    });
    panel.click((120, 40));
    panel.click(name);
    let run = panel.finish(&state);
    assert_exit_zero(&run);
    assert_eq!(
        state.read_timer_text(),
        None,
        "a click elsewhere wrote timer.json"
    );
    assert!(
        acting_calls(&state).is_empty(),
        "a click elsewhere acted on herdr: {:?}",
        state.herdr_calls()
    );

    // Control popup: the same, and the state stays as it was.
    let state = StateDir::new("elsewhere-control");
    let fixture = focus_json();
    state.write_timer(&fixture);
    let mut panel = Panel::start(&state);
    let name = wait_for_place(&state, "the home name", |screen| {
        screen.find(|row| row.contains("on notes"), NAME)
    });
    panel.click((120, 40));
    panel.click(name);
    let run = panel.finish(&state);
    assert_exit_zero(&run);
    assert_eq!(
        state.read_timer(),
        serde_json::from_str::<Value>(&fixture).expect("fixture is JSON"),
        "a click elsewhere changed the state"
    );
    assert!(
        acting_calls(&state).is_empty(),
        "a click elsewhere acted on herdr: {:?}",
        state.herdr_calls()
    );
}

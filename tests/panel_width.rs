//! No line either popup draws is wider than the popup's inside.

use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

const HOME: &str = "w0:p1";
const OTHER: &str = "w0:p2";

/// The longest home name the spec allows.
const LONG_NAME: &str = "abcdefghijklmnopqrstuvwx";

/// The most columns a popup line may take: the manifest's width of 50, less
/// herdr's border on each side and a 2-column right margin.
const MAX_COLUMNS: usize = 46;

/// A panel that has not exited by then is hung.
const PANEL_TIMEOUT: Duration = Duration::from_secs(5);

const ESC: &[u8] = b"\x1b";

/// A temp state folder holding the timer file and the fake herdr.
struct StateDir {
    path: PathBuf,
}

impl StateDir {
    fn new(tag: &str) -> Self {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!(
            "herdr-deepwork-panel-width-{}-{}-{}",
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

    fn herdr_path(&self) -> PathBuf {
        self.path.join("fake-herdr")
    }

    fn stdout_path(&self) -> PathBuf {
        self.path.join("panel.stdout")
    }

    /// Writes the fake herdr: a POSIX sh script that prints canned JSON, with
    /// a pane whose title is `LONG_NAME`.
    fn write_herdr(&self) {
        let path = self.herdr_path();
        fs::write(&path, FAKE_HERDR.replace("@NAME@", LONG_NAME)).expect("write fake herdr");
        let mut perms = fs::metadata(&path).expect("stat fake herdr").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).expect("chmod fake herdr");

        // The first exec of a fresh script is slow on macOS; warm it.
        // On Linux, exec fails with "Text file busy" while another test
        // thread's fork still holds the write handle; it clears in moments.
        let mut tries = 0;
        loop {
            match Command::new(&path)
                .args(["pane", "list"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
            {
                Ok(_) => break,
                Err(e) if e.kind() == std::io::ErrorKind::ExecutableFileBusy && tries < 100 => {
                    tries += 1;
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(e) => panic!("warm up fake herdr: {e}"),
            }
        }
    }
}

impl Drop for StateDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

const FAKE_HERDR: &str = r#"#!/bin/sh
case "$1 $2" in
  "pane get")
    printf '{"id":"cli:pane:get","result":{"pane":{"agent":"claude","pane_id":"%s","terminal_title_stripped":"@NAME@","workspace_id":"w0","tab_id":"w0:t1"},"type":"pane_info"}}\n' "$3"
    ;;
  "pane list")
    printf '{"id":"cli:pane:list","result":{"panes":[],"type":"pane_list"}}\n'
    ;;
  "pane report-metadata")
    printf '{"id":"cli:pane:report-metadata","result":{"type":"ok"}}\n'
    ;;
  *)
    printf '{}\n'
    ;;
esac
"#;

/// A running `herdr-deepwork panel` with piped stdin.
struct Panel {
    child: Child,
    stdin: Option<ChildStdin>,
}

impl Panel {
    /// Starts the panel in the pane `focused`.
    fn start(state: &StateDir, focused: &str) -> Self {
        let stdout = fs::File::create(state.stdout_path()).expect("create stdout file");
        let mut child = Command::new(env!("CARGO_BIN_EXE_herdr-deepwork"))
            .arg("panel")
            .env("HERDR_PLUGIN_STATE_DIR", &state.path)
            .env("HERDR_BIN_PATH", state.herdr_path())
            .env(
                "HERDR_PLUGIN_CONTEXT_JSON",
                format!(r#"{{"focused_pane_id":"{focused}"}}"#),
            )
            .env_remove("HERDR_SESSION")
            .env_remove("HERDR_ACTIVE_PANE_ID")
            .stdin(Stdio::piped())
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::null())
            .spawn()
            .expect("run herdr-deepwork panel");
        let stdin = child.stdin.take();
        Self { child, stdin }
    }

    /// Sends key bytes. A panel that already exited is not an error.
    fn send(&mut self, bytes: &[u8]) {
        if let Some(stdin) = self.stdin.as_mut() {
            let _ = stdin.write_all(bytes);
            let _ = stdin.flush();
        }
    }

    /// Closes stdin and waits for the exit; a hung panel fails the test.
    /// Returns what the panel drew, with ANSI escape sequences removed.
    fn finish(mut self, state: &StateDir) -> String {
        drop(self.stdin.take());
        let started = Instant::now();
        loop {
            if self.child.try_wait().expect("poll panel").is_some() {
                break;
            }
            if started.elapsed() > PANEL_TIMEOUT {
                let _ = self.child.kill();
                let _ = self.child.wait();
                panic!("panel did not exit within {PANEL_TIMEOUT:?}");
            }
            thread::sleep(Duration::from_millis(10));
        }
        let raw = fs::read(state.stdout_path()).expect("read panel stdout");
        strip_ansi(&String::from_utf8_lossy(&raw))
    }
}

/// Runs the panel in `focused`, closes it with Esc, and returns what it drew.
fn draw_popup(state: &StateDir, focused: &str) -> String {
    let mut panel = Panel::start(state, focused);
    panel.send(ESC);
    panel.finish(state)
}

/// Removes CSI and OSC sequences and two-byte escapes. A CSI that moves the
/// cursor home or clears the screen leaves a newline, so one frame's last line
/// never runs into the next frame's first.
fn strip_ansi(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\x1b' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('[') => {
                for next in chars.by_ref() {
                    if ('\u{40}'..='\u{7e}').contains(&next) {
                        if next == 'H' || next == 'J' {
                            out.push('\n');
                        }
                        break;
                    }
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
        }
    }
    out
}

/// Asserts the panel drew something and no drawn line is over `MAX_COLUMNS`
/// chars.
fn assert_fits(tag: &str, screen: &str) {
    assert!(
        screen.chars().any(|c| !c.is_whitespace()),
        "{tag}: the panel drew nothing"
    );
    for line in screen.replace("\r\n", "\n").split('\n') {
        let columns = line.chars().count();
        assert!(
            columns <= MAX_COLUMNS,
            "{tag}: a line is {columns} columns, over {MAX_COLUMNS}: {line:?}"
        );
    }
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

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_secs()
}

/// A timer file value in the contract shape, with a `LONG_NAME` home on `HOME`.
fn timer_value(mode: &str) -> Value {
    json!({
        "mode": mode,
        "home_pane": HOME,
        "home_name": LONG_NAME,
        "ends_at": null,
        "left_secs": null,
        "break_mins": 5,
        "last_focus_mins": 25,
        "last_break_mins": 5,
        "last_custom_mins": null,
        "message": null,
        "message_ends_at": null,
    })
}

/// A focus running, ending in 10 min.
fn focus_json() -> String {
    let mut timer = timer_value("focus");
    timer["ends_at"] = json!(rfc3339(now_secs() + 600));
    timer.to_string()
}

/// A focus held at 12 min.
fn paused_json() -> String {
    let mut timer = timer_value("paused");
    timer["left_secs"] = json!(720);
    timer.to_string()
}

/// A break ending in 3 min.
fn break_json() -> String {
    let mut timer = timer_value("break");
    timer["ends_at"] = json!(rfc3339(now_secs() + 180));
    timer.to_string()
}

#[test]
fn popup_lines_fit_start() {
    let state = StateDir::new("start");
    let screen = draw_popup(&state, HOME);
    assert_fits("start", &screen);
}

#[test]
fn popup_lines_fit_control() {
    let cases: [(&str, String, &str); 4] = [
        ("focus-home", focus_json(), HOME),
        ("focus-away", focus_json(), OTHER),
        ("paused", paused_json(), HOME),
        ("break", break_json(), HOME),
    ];
    for (tag, fixture, focused) in cases {
        let state = StateDir::new(tag);
        state.write_timer(&fixture);
        let screen = draw_popup(&state, focused);
        assert_fits(tag, &screen);
    }
}

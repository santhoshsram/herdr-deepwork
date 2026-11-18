//! `herdr-focus panel` with no timer running shows the start popup and starts a focus.

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
const OTHER: &str = "w0:p2";

/// A panel that has not exited by then is hung.
const PANEL_TIMEOUT: Duration = Duration::from_secs(5);

const RIGHT: &[u8] = b"\x1b[C";
const DOWN: &[u8] = b"\x1b[B";
const ENTER: &[u8] = b"\r";
const ESC: &[u8] = b"\x1b";

/// How the fake herdr reports the pane's agent.
#[derive(Clone, Copy)]
enum Agent {
    /// `"agent":"<kind>"`.
    Kind(&'static str),
    /// `"agent":null`.
    Null,
    /// No `agent` key at all.
    Absent,
}

/// A temp state folder holding the timer file, the fake herdr and its log.
struct StateDir {
    path: PathBuf,
}

impl StateDir {
    fn new(tag: &str) -> Self {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!(
            "herdr-focus-panel-start-{}-{}-{}",
            std::process::id(),
            n,
            tag
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp state dir");
        let state = Self { path };
        state.write_herdr();
        state.set_pane("herdr-focus", Agent::Kind("claude"));
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

    /// Replaces the timer file the way a status run would: write a sibling,
    /// then rename it over the old file.
    fn replace_timer(&self, contents: &str) {
        let file = self.timer_path();
        let temp = file.with_file_name("timer.json.tmp");
        fs::write(&temp, contents).expect("write temp timer file");
        fs::rename(&temp, &file).expect("rename timer file");
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

    fn pane_path(&self) -> PathBuf {
        self.path.join("pane-get.json")
    }

    fn stdout_path(&self) -> PathBuf {
        self.path.join("panel.stdout")
    }

    /// Sets what `pane get` answers.
    fn set_pane(&self, title: &str, agent: Agent) {
        let mut pane = json!({
            "pane_id": HOME,
            "terminal_title_stripped": title,
            "workspace_id": "w0",
            "tab_id": "w0:t1",
        });
        match agent {
            Agent::Kind(kind) => pane["agent"] = json!(kind),
            Agent::Null => pane["agent"] = Value::Null,
            Agent::Absent => {}
        }
        let answer = json!({
            "id": "cli:pane:get",
            "result": {"pane": pane, "type": "pane_info"},
        });
        fs::write(self.pane_path(), format!("{answer}\n")).expect("write pane answer");
    }

    /// Writes the fake herdr: a POSIX sh script that appends its argv as one
    /// line to the log and prints canned JSON.
    fn write_herdr(&self) {
        let script = FAKE_HERDR
            .replace("@LOG@", &self.log_path().to_string_lossy())
            .replace("@PANE@", &self.pane_path().to_string_lossy());
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
    cat '@PANE@'
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

/// A running `herdr-focus panel` with piped stdin.
struct Panel {
    child: Child,
    stdin: Option<ChildStdin>,
}

/// What a finished panel left behind.
struct Run {
    status: ExitStatus,
    /// stdout with ANSI escape sequences removed.
    screen: String,
}

impl Panel {
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

    /// Sends key bytes. A panel that already exited is not an error.
    fn send(&mut self, bytes: &[u8]) {
        if let Some(stdin) = self.stdin.as_mut() {
            let _ = stdin.write_all(bytes);
            let _ = stdin.flush();
        }
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
        let raw = fs::read(state.stdout_path()).expect("read panel stdout");
        Run {
            status,
            screen: strip_ansi(&String::from_utf8_lossy(&raw)),
        }
    }
}

/// Runs the panel, sends every key chunk in order, then closes stdin.
fn run_panel(state: &StateDir, keys: &[&[u8]]) -> Run {
    let mut panel = Panel::start(state);
    for chunk in keys {
        panel.send(chunk);
    }
    panel.finish(state)
}

/// Removes CSI and OSC sequences and two-byte escapes.
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

/// A timer file value in the contract shape.
fn timer_value(mode: &str, last_focus: u64, last_break: u64, last_custom: Value) -> Value {
    json!({
        "mode": mode,
        "home_pane": null,
        "home_name": null,
        "ends_at": null,
        "left_secs": null,
        "break_mins": last_break,
        "last_focus_mins": last_focus,
        "last_break_mins": last_break,
        "last_custom_mins": last_custom,
        "message": null,
        "message_ends_at": null,
    })
}

/// An idle timer file that remembers the given choices.
fn idle_json(last_focus: u64, last_break: u64, last_custom: Value) -> String {
    timer_value("idle", last_focus, last_break, last_custom).to_string()
}

/// A focus running on `home`, ending in 10 min.
fn focus_json(home: &str, name: &str) -> String {
    let mut timer = timer_value("focus", 25, 5, Value::Null);
    timer["home_pane"] = json!(home);
    timer["home_name"] = json!(name);
    timer["ends_at"] = json!(rfc3339(now_secs() + 600));
    timer.to_string()
}

/// A focus on `home`, held at 12 min.
fn paused_json(home: &str, name: &str) -> String {
    let mut timer = timer_value("paused", 25, 5, Value::Null);
    timer["home_pane"] = json!(home);
    timer["home_name"] = json!(name);
    timer["left_secs"] = json!(720);
    timer.to_string()
}

/// A break ending in 3 min.
fn break_json() -> String {
    let mut timer = timer_value("break", 25, 5, Value::Null);
    timer["ends_at"] = json!(rfc3339(now_secs() + 180));
    timer.to_string()
}

fn assert_exit_zero(run: &Run) {
    assert_eq!(run.status.code(), Some(0), "screen: {}", run.screen);
}

/// Asserts `timer` is a focus just started on `HOME` as `name`, with the
/// given lengths, ending `focus_mins` after a moment in `[before, after]`.
fn assert_started(
    timer: &Value,
    name: &str,
    focus_mins: u64,
    break_mins: u64,
    (before, after): (u64, u64),
) {
    assert_eq!(timer["mode"], "focus", "{timer}");
    assert_eq!(timer["home_pane"], HOME, "{timer}");
    assert_eq!(timer["home_name"], name, "{timer}");
    assert_eq!(timer["left_secs"], Value::Null, "{timer}");
    assert_eq!(timer["break_mins"], break_mins, "{timer}");
    assert_eq!(timer["last_focus_mins"], focus_mins, "{timer}");
    assert_eq!(timer["last_break_mins"], break_mins, "{timer}");
    assert_eq!(timer["message"], Value::Null, "{timer}");
    assert_eq!(timer["message_ends_at"], Value::Null, "{timer}");
    let ends = parse_rfc3339(timer["ends_at"].as_str().expect("ends_at is a string"));
    // Written times are whole seconds, so allow one second either way.
    let low = before + focus_mins * 60 - 1;
    let high = after + focus_mins * 60 + 1;
    assert!(
        (low..=high).contains(&ends),
        "ends_at {} is not {focus_mins} min after the start: {timer}",
        timer["ends_at"]
    );
}

/// Whether `call` sets the `focus` token on `HOME` to `value`.
fn is_set_call(call: &str, value: &str) -> bool {
    call.starts_with(&format!("pane report-metadata {HOME} "))
        && call.contains("--source herdr-focus")
        && call.contains(&format!("--token focus={value}"))
}

fn report_calls(state: &StateDir) -> Vec<String> {
    state
        .herdr_calls()
        .into_iter()
        .filter(|call| call.starts_with("pane report-metadata"))
        .collect()
}

#[test]
fn panel_first_run_defaults() {
    let state = StateDir::new("first-run");
    let before = now_secs();
    let run = run_panel(&state, &[ENTER]);
    let after = now_secs();

    assert_exit_zero(&run);
    assert!(
        run.screen.contains("Focus on herdr-focus"),
        "screen: {}",
        run.screen
    );
    assert_started(&state.read_timer(), "herdr-focus", 25, 5, (before, after));
    assert_eq!(state.read_timer()["last_custom_mins"], Value::Null);
    assert!(
        state
            .herdr_calls()
            .iter()
            .any(|call| call == &format!("pane get {HOME}")),
        "no pane get for the focused pane: {:?}",
        state.herdr_calls()
    );
}

#[test]
fn panel_remembers_choices() {
    let state = StateDir::new("remembers");
    // Focus 25 -> 50 is one step right; Break 5 -> 10 is down, then one right.
    let before = now_secs();
    let run = run_panel(&state, &[RIGHT, DOWN, RIGHT, ENTER]);
    let after = now_secs();
    assert_exit_zero(&run);
    assert_started(&state.read_timer(), "herdr-focus", 50, 10, (before, after));

    // Back to idle, as after the focus ran out, keeping the remembered choices.
    let mut idle = state.read_timer();
    idle["mode"] = json!("idle");
    idle["ends_at"] = Value::Null;
    state.replace_timer(&idle.to_string());

    let before = now_secs();
    let run = run_panel(&state, &[ENTER]);
    let after = now_secs();
    assert_exit_zero(&run);
    assert_started(&state.read_timer(), "herdr-focus", 50, 10, (before, after));
}

#[test]
fn panel_custom_length() {
    let state = StateDir::new("custom");
    let before = now_secs();
    let run = run_panel(&state, &[b"40", ENTER]);
    let after = now_secs();

    assert_exit_zero(&run);
    let timer = state.read_timer();
    assert_started(&timer, "herdr-focus", 40, 5, (before, after));
    assert_eq!(timer["last_custom_mins"], 40, "{timer}");
}

#[test]
fn panel_rejects_bad_custom() {
    let cases: [(&str, Vec<&[u8]>); 3] = [
        ("zero", vec![b"0", ENTER]),
        ("181", vec![b"181", ENTER]),
        // Right twice reaches custom, with no number typed.
        ("empty", vec![RIGHT, RIGHT, ENTER]),
    ];
    for (tag, keys) in cases {
        let state = StateDir::new(&format!("bad-{tag}"));
        let run = run_panel(&state, &keys);
        assert!(
            run.screen.contains("1 to 180"),
            "{tag}: no `1 to 180` on screen: {}",
            run.screen
        );
        assert_eq!(
            state.read_timer_text(),
            None,
            "{tag}: the panel wrote timer.json"
        );
        assert!(
            report_calls(&state).is_empty(),
            "{tag}: the panel set the sidebar token: {:?}",
            state.herdr_calls()
        );
    }
}

#[test]
fn panel_custom_equal_preset() {
    for (tag, last_custom) in [("seen", json!(40)), ("never", Value::Null)] {
        let state = StateDir::new(&format!("equal-{tag}"));
        state.write_timer(&idle_json(50, 10, last_custom.clone()));
        let before = now_secs();
        let run = run_panel(&state, &[b"25", ENTER]);
        let after = now_secs();

        assert_exit_zero(&run);
        let timer = state.read_timer();
        assert_started(&timer, "herdr-focus", 25, 10, (before, after));
        assert_eq!(
            timer["last_custom_mins"], last_custom,
            "{tag}: last_custom_mins changed: {timer}"
        );
    }
}

#[test]
fn panel_escape_starts_nothing() {
    let cases: [(&str, Vec<&[u8]>); 2] = [("plain", vec![ESC]), ("after-typing", vec![b"40", ESC])];
    for (tag, keys) in cases {
        let state = StateDir::new(&format!("esc-{tag}"));
        let run = run_panel(&state, &keys);

        assert_exit_zero(&run);
        assert_eq!(
            state.read_timer_text(),
            None,
            "{tag}: the panel wrote timer.json"
        );
        assert!(
            report_calls(&state).is_empty(),
            "{tag}: the panel set the sidebar token: {:?}",
            state.herdr_calls()
        );
    }
}

#[test]
fn panel_home_name_rules() {
    let thirty = "abcdefghijklmnopqrstuvwxyz0123";
    let cut = "abcdefghijklmnopqrstuvw…";
    let twenty_four = "abcdefghijklmnopqrstuvwx";
    let cases: [(&str, &str, Agent, &str); 6] = [
        ("title", "my build", Agent::Kind("claude"), "my build"),
        ("agent", "", Agent::Kind("claude"), "claude"),
        ("no-agent-absent", "", Agent::Absent, "home"),
        ("no-agent-null", "", Agent::Null, "home"),
        ("long", thirty, Agent::Kind("claude"), cut),
        ("max", twenty_four, Agent::Kind("claude"), twenty_four),
    ];
    for (tag, title, agent, name) in cases {
        let state = StateDir::new(&format!("name-{tag}"));
        state.set_pane(title, agent);
        let run = run_panel(&state, &[ENTER]);

        assert_exit_zero(&run);
        assert!(
            run.screen.contains(&format!("Focus on {name}")),
            "{tag}: no `Focus on {name}` on screen: {}",
            run.screen
        );
        let timer = state.read_timer();
        assert_eq!(timer["home_name"], name, "{tag}: {timer}");
    }
}

#[test]
fn panel_start_sets_tomato() {
    let state = StateDir::new("tomato");
    let run = run_panel(&state, &[ENTER]);

    assert_exit_zero(&run);
    let calls = state.herdr_calls();
    assert!(
        calls.iter().any(|call| is_set_call(call, "🍅")),
        "no focus=🍅 call on {HOME}: {calls:?}"
    );
    assert!(
        calls.iter().all(|call| !call.contains("--clear-token")),
        "the token was cleared: {calls:?}"
    );
}

#[test]
fn panel_never_starts_second_timer() {
    // A timer already runs when the panel starts. The running focus is on
    // another pane, so the popup for it leaves its state alone on Enter.
    let state = StateDir::new("second-running");
    let running = focus_json(OTHER, "other");
    state.write_timer(&running);
    let run = run_panel(&state, &[ENTER]);
    assert!(!run.screen.is_empty(), "the panel drew nothing");
    assert_eq!(
        state.read_timer_text().as_deref(),
        Some(running.as_str()),
        "the running timer changed"
    );
    assert!(
        report_calls(&state).is_empty(),
        "the sidebar token was set: {:?}",
        state.herdr_calls()
    );

    // Paused and break popups act on Enter, but none may move home.
    for (tag, timer) in [
        ("paused", paused_json(OTHER, "other")),
        ("break", break_json()),
    ] {
        let state = StateDir::new(&format!("second-{tag}"));
        state.write_timer(&timer);
        let run = run_panel(&state, &[ENTER]);
        assert!(!run.screen.is_empty(), "{tag}: the panel drew nothing");
        let after = state.read_timer();
        let before: Value = serde_json::from_str(&timer).expect("fixture is JSON");
        assert_eq!(
            after["home_pane"], before["home_pane"],
            "{tag}: home pane changed: {after}"
        );
        assert_eq!(
            after["home_name"], before["home_name"],
            "{tag}: home name changed: {after}"
        );
        assert!(
            state
                .herdr_calls()
                .iter()
                .all(|call| !call.contains(&format!("report-metadata {HOME}"))),
            "{tag}: the sidebar token moved to {HOME}: {:?}",
            state.herdr_calls()
        );
    }

    // A timer starts after the popup opened, before Enter.
    let state = StateDir::new("second-race");
    state.write_timer(&idle_json(25, 5, Value::Null));
    let mut panel = Panel::start(&state);
    thread::sleep(Duration::from_millis(400));
    let running = focus_json(OTHER, "other");
    state.replace_timer(&running);
    panel.send(ENTER);
    let run = panel.finish(&state);
    assert!(
        run.screen.contains("Focus on herdr-focus"),
        "the popup opened before the timer started: {}",
        run.screen
    );
    assert_eq!(
        state.read_timer_text().as_deref(),
        Some(running.as_str()),
        "the timer that started after the popup opened changed"
    );
    assert!(
        report_calls(&state).is_empty(),
        "the sidebar token was set: {:?}",
        state.herdr_calls()
    );
}

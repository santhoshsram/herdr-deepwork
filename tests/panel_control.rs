//! `herdr-deepwork panel` with a timer running shows the control popup and acts on its keys.

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
const NAME: &str = "notes";

/// A panel that has not exited by then is hung.
const PANEL_TIMEOUT: Duration = Duration::from_secs(5);

const ENTER: &[u8] = b"\r";

/// A temp state folder holding the timer file, the fake herdr and its log.
struct StateDir {
    path: PathBuf,
}

impl StateDir {
    fn new(tag: &str) -> Self {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!(
            "herdr-deepwork-panel-control-{}-{}-{}",
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

    fn session_dir(&self) -> PathBuf {
        self.path.join("sessions").join("default")
    }

    fn timer_path(&self) -> PathBuf {
        self.session_dir().join("timer.json")
    }

    fn write_timer(&self, contents: &str) {
        fs::create_dir_all(self.session_dir()).expect("create session dir");
        fs::write(self.timer_path(), contents).expect("write timer file");
    }

    /// Replaces the timer file the way a status run would: write a sibling,
    /// then rename it over the old file.
    fn replace_timer(&self, contents: &str) {
        let file = self.timer_path();
        let temp = file.with_file_name("timer.json.tmp");
        fs::write(&temp, contents).expect("write temp timer file");
        fs::rename(&temp, &file).expect("rename timer file");
    }

    fn read_timer(&self) -> Value {
        let text = fs::read_to_string(self.timer_path()).expect("timer.json exists");
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

    /// Makes `pane get <pane>` fail with `pane_not_found`.
    fn close_pane(&self, pane: &str) {
        fs::write(self.path.join(format!("missing-{pane}")), "").expect("mark pane closed");
    }

    /// Makes `agent focus` hang for 3 s.
    fn hang_agent_focus(&self) {
        fs::write(self.path.join("hang-agent-focus"), "").expect("mark agent focus slow");
    }

    /// Directories the lock covers, deepest first.
    fn locked_dirs(&self) -> [PathBuf; 3] {
        [
            self.session_dir(),
            self.path.join("sessions"),
            self.path.clone(),
        ]
    }

    /// Makes the state folder and the folders under it read-only.
    fn lock(&self) {
        for dir in self.locked_dirs() {
            fs::set_permissions(&dir, fs::Permissions::from_mode(0o555))
                .expect("make state folder read-only");
        }
    }

    /// Writes the fake herdr: a POSIX sh script that appends its argv as one
    /// line to the log and prints canned JSON.
    fn write_herdr(&self) {
        let script = FAKE_HERDR
            .replace("@LOG@", &self.log_path().to_string_lossy())
            .replace("@DIR@", &self.path.to_string_lossy());
        let path = self.herdr_path();
        fs::write(&path, script).expect("write fake herdr");
        let mut perms = fs::metadata(&path).expect("stat fake herdr").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).expect("chmod fake herdr");

        // The first exec of a fresh script is slow on macOS; warm it, then
        // drop the warm-up call from the log.
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

    /// The panel's stdout so far, with ANSI escape sequences removed.
    fn screen(&self) -> String {
        let raw = fs::read(self.stdout_path()).unwrap_or_default();
        strip_ansi(&String::from_utf8_lossy(&raw))
    }
}

impl Drop for StateDir {
    fn drop(&mut self) {
        for dir in self.locked_dirs() {
            let _ = fs::set_permissions(&dir, fs::Permissions::from_mode(0o755));
        }
        let _ = fs::remove_dir_all(&self.path);
    }
}

const FAKE_HERDR: &str = r#"#!/bin/sh
echo "$*" >> '@LOG@'
case "$1 $2" in
  "pane get")
    if [ -e "@DIR@/missing-$3" ]; then
      printf '{"error":{"code":"pane_not_found","message":"pane %s not found"},"id":"cli:pane:get"}\n' "$3"
      exit 1
    fi
    printf '{"id":"cli:pane:get","result":{"pane":{"agent":"claude","pane_id":"%s","terminal_title_stripped":"herdr-deepwork","workspace_id":"w0","tab_id":"w0:t1"},"type":"pane_info"}}\n' "$3"
    ;;
  "pane list")
    printf '{"id":"cli:pane:list","result":{"panes":[],"type":"pane_list"}}\n'
    ;;
  "pane report-metadata")
    printf '{"id":"cli:pane:report-metadata","result":{"type":"ok"}}\n'
    ;;
  "agent focus")
    if [ -e "@DIR@/hang-agent-focus" ]; then
      exec sleep 3 >/dev/null 2>&1
    fi
    printf '{"id":"cli:agent:focus","result":{"type":"ok"}}\n'
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

/// What a finished panel left behind.
struct Run {
    status: ExitStatus,
    /// stdout with ANSI escape sequences removed.
    screen: String,
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

    fn is_running(&mut self) -> bool {
        self.child.try_wait().expect("poll panel").is_none()
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
            screen: state.screen(),
        }
    }
}

/// Waits until the panel's screen satisfies `ready`; false on timeout.
fn wait_for_screen(state: &StateDir, ready: impl Fn(&str) -> bool) -> bool {
    let started = Instant::now();
    while started.elapsed() < PANEL_TIMEOUT {
        if ready(&state.screen()) {
            return true;
        }
        thread::sleep(Duration::from_millis(10));
    }
    false
}

/// Runs the panel in `focused`, sends every key chunk in order, then closes stdin.
fn run_panel(state: &StateDir, focused: &str, keys: &[&[u8]]) -> Run {
    let mut panel = Panel::start(state, focused);
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
fn timer_value(mode: &str) -> Value {
    json!({
        "mode": mode,
        "home_pane": null,
        "home_name": null,
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

/// A focus on `HOME` as `NAME`, ending at the whole second `ends`.
fn focus_ending(ends: u64) -> Value {
    let mut timer = timer_value("focus");
    timer["home_pane"] = json!(HOME);
    timer["home_name"] = json!(NAME);
    timer["ends_at"] = json!(rfc3339(ends));
    timer
}

/// A focus on `HOME` as `NAME`, ending in 10 min.
fn focus_json() -> String {
    focus_ending(now_secs() + 600).to_string()
}

/// A focus on `HOME` as `NAME`, held at 12 min.
fn paused_json() -> String {
    let mut timer = timer_value("paused");
    timer["home_pane"] = json!(HOME);
    timer["home_name"] = json!(NAME);
    timer["left_secs"] = json!(720);
    timer.to_string()
}

/// A break ending in 3 min, after a focus on `HOME`.
fn break_json() -> String {
    let mut timer = timer_value("break");
    timer["home_pane"] = json!(HOME);
    timer["home_name"] = json!(NAME);
    timer["ends_at"] = json!(rfc3339(now_secs() + 180));
    timer.to_string()
}

fn assert_exit_zero(run: &Run) {
    assert_eq!(run.status.code(), Some(0), "screen: {}", run.screen);
}

fn assert_shows(screen: &str, texts: &[&str]) {
    for text in texts {
        assert!(screen.contains(text), "no `{text}` on screen: {screen}");
    }
}

/// Whether `call` sets the `deepwork` token on `HOME` to `value`.
fn is_set_call(call: &str, value: &str) -> bool {
    call.starts_with(&format!("pane report-metadata {HOME} "))
        && call.contains("--source herdr-deepwork")
        && call.contains(&format!("--token deepwork={value}"))
}

/// Whether `call` clears the `deepwork` token on `HOME`.
fn is_clear_call(call: &str) -> bool {
    call.starts_with(&format!("pane report-metadata {HOME} "))
        && call.contains("--source herdr-deepwork")
        && call.contains("--clear-token deepwork")
}

fn is_agent_focus_home(call: &str) -> bool {
    call == format!("agent focus {HOME}")
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
fn control_pause_at_home() {
    let state = StateDir::new("pause-home");
    let fixture = focus_json();
    let ends = parse_rfc3339(
        serde_json::from_str::<Value>(&fixture).expect("fixture is JSON")["ends_at"]
            .as_str()
            .expect("fixture has ends_at"),
    );
    state.write_timer(&fixture);
    let before = now_secs();
    let run = run_panel(&state, HOME, &[ENTER]);
    let after = now_secs();

    assert_exit_zero(&run);
    assert_shows(&run.screen, &["Pause ⏎", "Stop s"]);
    let timer = state.read_timer();
    assert_eq!(timer["mode"], "paused", "{timer}");
    assert_eq!(timer["home_pane"], HOME, "{timer}");
    assert_eq!(timer["home_name"], NAME, "{timer}");
    let left = timer["left_secs"].as_u64().expect("left_secs is a number");
    // Whole seconds, so allow one second either way.
    assert!(
        (ends.saturating_sub(after + 1)..=ends - before + 1).contains(&left),
        "left_secs {left} is not the time left until {ends}: {timer}"
    );
    let calls = state.herdr_calls();
    assert!(
        calls.iter().any(|call| is_set_call(call, "⏸")),
        "no deepwork=⏸ call on {HOME}: {calls:?}"
    );
}

#[test]
fn control_back_to_home() {
    let state = StateDir::new("back");
    let fixture = focus_json();
    state.write_timer(&fixture);
    let run = run_panel(&state, OTHER, &[ENTER]);

    assert_exit_zero(&run);
    assert_shows(
        &run.screen,
        &[&format!("Back to {NAME} ⏎"), "Pause p", "Stop s"],
    );
    assert_eq!(
        state.read_timer(),
        serde_json::from_str::<Value>(&fixture).expect("fixture is JSON"),
        "the state changed"
    );
    let calls = state.herdr_calls();
    assert!(
        calls.iter().any(|call| is_agent_focus_home(call)),
        "no `agent focus {HOME}` call: {calls:?}"
    );
    assert!(
        calls
            .iter()
            .all(|call| !call.starts_with("pane report-metadata")),
        "the sidebar token changed: {calls:?}"
    );
}

#[test]
fn control_pause_when_away() {
    let state = StateDir::new("pause-away");
    state.write_timer(&focus_json());
    let run = run_panel(&state, OTHER, &[b"p"]);

    assert_exit_zero(&run);
    let timer = state.read_timer();
    assert_eq!(timer["mode"], "paused", "{timer}");
    assert!(timer["left_secs"].is_u64(), "{timer}");
    let calls = state.herdr_calls();
    assert!(
        calls.iter().any(|call| is_set_call(call, "⏸")),
        "no deepwork=⏸ call on {HOME}: {calls:?}"
    );
    assert!(
        calls.iter().all(|call| !call.starts_with("agent focus")),
        "the panel switched panes: {calls:?}"
    );
}

#[test]
fn control_resume() {
    let state = StateDir::new("resume");
    state.write_timer(&paused_json());
    let before = now_secs();
    let run = run_panel(&state, HOME, &[ENTER]);
    let after = now_secs();

    assert_exit_zero(&run);
    assert_shows(&run.screen, &["Resume ⏎", "Stop s"]);
    let timer = state.read_timer();
    assert_eq!(timer["mode"], "focus", "{timer}");
    assert_eq!(timer["home_pane"], HOME, "{timer}");
    assert_eq!(timer["home_name"], NAME, "{timer}");
    let ends = parse_rfc3339(timer["ends_at"].as_str().expect("ends_at is a string"));
    // Written times are whole seconds, so allow one second either way.
    assert!(
        (before + 720 - 1..=after + 720 + 1).contains(&ends),
        "ends_at {} is not 720 s after the resume: {timer}",
        timer["ends_at"]
    );
    let calls = state.herdr_calls();
    assert!(
        calls.iter().any(|call| is_set_call(call, "🍅")),
        "no deepwork=🍅 call on {HOME}: {calls:?}"
    );
}

#[test]
fn control_stop() {
    let cases: [(&str, String, &str); 3] = [
        ("focus-home", focus_json(), HOME),
        ("focus-away", focus_json(), OTHER),
        ("paused", paused_json(), HOME),
    ];
    for (tag, fixture, focused) in cases {
        let state = StateDir::new(&format!("stop-{tag}"));
        state.write_timer(&fixture);
        let run = run_panel(&state, focused, &[b"s"]);

        assert_exit_zero(&run);
        assert_shows(&run.screen, &["Stop s"]);
        let timer = state.read_timer();
        assert_eq!(timer["mode"], "idle", "{tag}: {timer}");
        assert_eq!(timer["message"], Value::Null, "{tag}: {timer}");
        let calls = state.herdr_calls();
        assert!(
            calls.iter().any(|call| is_clear_call(call)),
            "{tag}: no clear-token call on {HOME}: {calls:?}"
        );
    }
}

#[test]
fn control_end_break() {
    let state = StateDir::new("end-break");
    state.write_timer(&break_json());
    let run = run_panel(&state, HOME, &[ENTER]);

    assert_exit_zero(&run);
    assert_shows(&run.screen, &["End break ⏎"]);
    let timer = state.read_timer();
    assert_eq!(timer["mode"], "idle", "{timer}");
    assert_eq!(timer["message"], Value::Null, "{timer}");
    assert_eq!(timer["message_ends_at"], Value::Null, "{timer}");
}

#[test]
fn control_stale_popup_redraws() {
    for (tag, key) in [("enter", ENTER), ("stop", b"s".as_slice())] {
        let state = StateDir::new(&format!("stale-{tag}"));
        state.write_timer(&focus_json());
        let mut panel = Panel::start(&state, HOME);
        assert!(
            wait_for_screen(&state, |screen| !screen.is_empty()),
            "{tag}: the panel drew nothing"
        );

        // The focus is paused from elsewhere after the popup opened.
        let changed = paused_json();
        state.replace_timer(&changed);
        panel.send(key);
        let _run = panel.finish(&state);

        assert_eq!(
            state.read_timer(),
            serde_json::from_str::<Value>(&changed).expect("fixture is JSON"),
            "{tag}: the key changed the new state"
        );
        let calls = acting_calls(&state);
        assert!(calls.is_empty(), "{tag}: the key acted on herdr: {calls:?}");
    }
}

#[test]
fn control_save_failure() {
    let state = StateDir::new("save-failure");
    let fixture = focus_json();
    state.write_timer(&fixture);
    let mut panel = Panel::start(&state, HOME);
    assert!(
        wait_for_screen(&state, |screen| !screen.is_empty()),
        "the panel drew nothing"
    );

    state.lock();
    panel.send(ENTER);
    assert!(
        wait_for_screen(&state, |screen| screen.contains("Could not save")),
        "no `Could not save` on screen: {}",
        state.screen()
    );
    assert!(panel.is_running(), "the popup closed");
    let _run = panel.finish(&state);

    assert_eq!(
        state.read_timer(),
        serde_json::from_str::<Value>(&fixture).expect("fixture is JSON"),
        "the state file changed"
    );
}

#[test]
fn control_herdr_silent() {
    let state = StateDir::new("silent");
    let fixture = focus_json();
    state.write_timer(&fixture);
    state.hang_agent_focus();
    let mut panel = Panel::start(&state, OTHER);
    assert!(
        wait_for_screen(&state, |screen| screen.contains("Back to")),
        "no Back button on screen: {}",
        state.screen()
    );

    panel.send(ENTER);
    assert!(
        wait_for_screen(&state, |screen| screen.contains("herdr is not answering")),
        "no `herdr is not answering` on screen: {}",
        state.screen()
    );
    assert!(panel.is_running(), "the popup closed");
    let _run = panel.finish(&state);

    assert_eq!(
        state.read_timer(),
        serde_json::from_str::<Value>(&fixture).expect("fixture is JSON"),
        "the state file changed"
    );
}

#[test]
fn control_home_closed() {
    let state = StateDir::new("home-closed");
    let fixture = focus_json();
    state.write_timer(&fixture);
    state.close_pane(HOME);
    let run = run_panel(&state, OTHER, &[]);

    assert_shows(&run.screen, &["Pause ⏎", "Stop s"]);
    assert!(
        !run.screen.contains("Back to"),
        "the popup offers Back to a closed pane: {}",
        run.screen
    );
    assert_eq!(
        state.read_timer(),
        serde_json::from_str::<Value>(&fixture).expect("fixture is JSON"),
        "the state changed"
    );
    assert!(
        state
            .herdr_calls()
            .iter()
            .all(|call| !call.starts_with("agent focus")),
        "the panel switched panes: {:?}",
        state.herdr_calls()
    );
}

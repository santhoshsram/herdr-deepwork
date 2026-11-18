//! `herdr-deepwork status` prints the idle tomato whatever the timer file holds.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const IDLE_LINE: &str = "🍅\n";

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// A temp state folder that removes itself on drop.
struct StateDir {
    path: PathBuf,
}

impl StateDir {
    fn new(tag: &str) -> Self {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!(
            "herdr-deepwork-status-idle-{}-{}-{}",
            std::process::id(),
            n,
            tag
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp state dir");
        Self { path }
    }

    fn timer_path(&self, session: &str) -> PathBuf {
        self.path.join("sessions").join(session).join("timer.json")
    }

    fn write_timer(&self, session: &str, contents: &str) {
        let file = self.timer_path(session);
        fs::create_dir_all(file.parent().expect("timer file has a parent"))
            .expect("create session dir");
        fs::write(&file, contents).expect("write timer file");
    }
}

impl Drop for StateDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Runs `herdr-deepwork status` with a clean herdr environment.
/// `session` of `None` leaves `HERDR_SESSION` unset.
fn run_status(state: &Path, session: Option<&str>) -> (Output, Duration) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_herdr-deepwork"));
    cmd.arg("status")
        .env("HERDR_PLUGIN_STATE_DIR", state)
        .env("HERDR_BIN_PATH", state.join("no-such-herdr"))
        .env_remove("HERDR_SESSION")
        .env_remove("HERDR_ACTIVE_PANE_ID")
        .env_remove("HERDR_PLUGIN_CONTEXT_JSON");
    if let Some(name) = session {
        cmd.env("HERDR_SESSION", name);
    }
    let started = Instant::now();
    let output = cmd.output().expect("run herdr-deepwork status");
    (output, started.elapsed())
}

fn assert_idle_line(output: &Output) {
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), IDLE_LINE);
    assert!(
        output.stderr.is_empty(),
        "stderr not empty: {}",
        String::from_utf8_lossy(&output.stderr)
    );
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

fn now_plus_secs(secs: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_secs();
    rfc3339(now + secs)
}

/// A full-shape timer file. `ends_at` is a raw JSON value.
fn timer_json(mode: &str, ends_at: &str, last_focus_mins: u32) -> String {
    format!(
        r#"{{"mode":"{mode}","home_pane":"p1","home_name":"work","ends_at":{ends_at},"left_secs":null,"break_mins":5,"last_focus_mins":{last_focus_mins},"last_break_mins":5,"last_custom_mins":null,"message":null,"message_ends_at":null}}"#
    )
}

fn idle_json() -> String {
    timer_json("idle", "null", 25)
}

/// A focus that is still running 30 minutes from now.
fn live_focus_json() -> String {
    timer_json("focus", &format!("\"{}\"", now_plus_secs(30 * 60)), 25)
}

#[test]
fn status_idle_without_file() {
    let state = StateDir::new("nofile");
    let (output, _) = run_status(&state.path, None);
    assert_idle_line(&output);

    let (output, _) = run_status(&state.path, Some("work"));
    assert_idle_line(&output);
}

#[test]
fn status_idle_on_broken_file() {
    for (tag, contents) in [
        ("empty", ""),
        ("text", "this is not json"),
        ("truncated", r#"{"mode":"focus","home_pane":"#),
    ] {
        let state = StateDir::new(tag);
        state.write_timer("default", contents);
        let (output, _) = run_status(&state.path, None);
        assert_idle_line(&output);
    }
}

#[test]
fn status_idle_on_unknown_shape() {
    let missing_key = idle_json().replace(r#","message_ends_at":null"#, "");
    assert!(!missing_key.contains("message_ends_at"));
    let cases = [
        ("missing_key", missing_key),
        ("focus_mins_zero", timer_json("idle", "null", 0)),
        ("focus_without_end", timer_json("focus", "null", 25)),
        ("other_shape", r#"{"hello":"world"}"#.to_string()),
        ("array", "[1,2,3]".to_string()),
    ];
    for (tag, contents) in cases {
        let state = StateDir::new(tag);
        state.write_timer("default", &contents);
        let (output, _) = run_status(&state.path, None);
        assert_idle_line(&output);
    }
}

#[test]
fn status_reads_session_file() {
    // The file the command should read holds an idle timer; the other
    // session's file holds a running focus. Reading the wrong one would
    // not print the idle tomato.
    let state = StateDir::new("named");
    state.write_timer("work", &idle_json());
    state.write_timer("default", &live_focus_json());
    let (output, _) = run_status(&state.path, Some("work"));
    assert_idle_line(&output);

    let state = StateDir::new("unset");
    state.write_timer("default", &idle_json());
    state.write_timer("work", &live_focus_json());
    let (output, _) = run_status(&state.path, None);
    assert_idle_line(&output);

    let state = StateDir::new("empty-session");
    state.write_timer("default", &idle_json());
    state.write_timer("work", &live_focus_json());
    let (output, _) = run_status(&state.path, Some(""));
    assert_idle_line(&output);

    // A broken file in the session being read falls back to idle even
    // when another session holds a running focus.
    let state = StateDir::new("named-broken");
    state.write_timer("work", "not json");
    state.write_timer("default", &live_focus_json());
    let (output, _) = run_status(&state.path, Some("work"));
    assert_idle_line(&output);
}

#[test]
fn status_is_fast() {
    let limit = Duration::from_millis(200);

    let missing = StateDir::new("fast-missing");
    let empty = StateDir::new("fast-empty");
    empty.write_timer("default", "");
    let broken = StateDir::new("fast-broken");
    broken.write_timer("default", "not json");
    let unknown = StateDir::new("fast-unknown");
    unknown.write_timer("default", &timer_json("focus", "null", 0));

    for state in [&missing, &empty, &broken, &unknown] {
        // The fastest of 3 runs, so parallel test load cannot fail the limit.
        let mut fastest = Duration::MAX;
        for _ in 0..3 {
            let (output, elapsed) = run_status(&state.path, None);
            assert_idle_line(&output);
            fastest = fastest.min(elapsed);
        }
        assert!(
            fastest < limit,
            "fastest of 3 runs took {fastest:?}, limit {limit:?}, state {}",
            state.path.display()
        );
    }
}

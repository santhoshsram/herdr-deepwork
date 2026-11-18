//! `herdr-focus status` follows the timer timeline from the saved times and the clock.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// A temp state folder that removes itself on drop.
struct StateDir {
    path: PathBuf,
}

impl StateDir {
    fn new(tag: &str) -> Self {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!(
            "herdr-focus-status-timeline-{}-{}-{}",
            std::process::id(),
            n,
            tag
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp state dir");
        Self { path }
    }

    fn write_timer(&self, contents: &str) {
        let file = self
            .path
            .join("sessions")
            .join("default")
            .join("timer.json");
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

/// Runs `herdr-focus status` with a clean herdr environment.
fn run_status(state: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_herdr-focus"))
        .arg("status")
        .env("HERDR_PLUGIN_STATE_DIR", state)
        .env("HERDR_BIN_PATH", state.join("no-such-herdr"))
        .env_remove("HERDR_SESSION")
        .env_remove("HERDR_ACTIVE_PANE_ID")
        .env_remove("HERDR_PLUGIN_CONTEXT_JSON")
        .output()
        .expect("run herdr-focus status")
}

/// Asserts a clean exit and that stdout is exactly `line` plus a newline.
fn assert_line(output: &Output, line: &str) {
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), format!("{line}\n"));
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

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_secs()
}

fn in_secs(secs: u64) -> String {
    rfc3339(now_secs() + secs)
}

fn ago_secs(secs: u64) -> String {
    rfc3339(now_secs() - secs)
}

/// A focus timer file: `ends_at` is a time, `break_mins` the break after it.
fn focus_json(ends_at: &str, break_mins: u8) -> String {
    format!(
        r#"{{"mode":"focus","home_pane":"p1","home_name":"work","ends_at":"{ends_at}","left_secs":null,"break_mins":{break_mins},"last_focus_mins":25,"last_break_mins":5,"last_custom_mins":null,"message":null,"message_ends_at":null}}"#
    )
}

fn paused_json(left_secs: u64) -> String {
    format!(
        r#"{{"mode":"paused","home_pane":"p1","home_name":"work","ends_at":null,"left_secs":{left_secs},"break_mins":5,"last_focus_mins":25,"last_break_mins":5,"last_custom_mins":null,"message":null,"message_ends_at":null}}"#
    )
}

fn break_json(ends_at: &str) -> String {
    format!(
        r#"{{"mode":"break","home_pane":null,"home_name":null,"ends_at":"{ends_at}","left_secs":null,"break_mins":5,"last_focus_mins":25,"last_break_mins":5,"last_custom_mins":null,"message":null,"message_ends_at":null}}"#
    )
}

/// Writes `contents` as the timer file of a fresh state folder and runs status.
fn status_for(tag: &str, contents: &str) -> Output {
    let state = StateDir::new(tag);
    state.write_timer(contents);
    run_status(&state.path)
}

#[test]
fn status_focus_rounds_up() {
    let output = status_for("rounds-up", &focus_json(&in_secs(18 * 60 + 30), 5));
    assert_line(&output, "🍅 19m");
}

#[test]
fn status_focus_hours() {
    let output = status_for("hours", &focus_json(&in_secs(65 * 60), 5));
    assert_line(&output, "🍅 1h 05m");
}

#[test]
fn status_paused() {
    let output = status_for("paused", &paused_json(720));
    assert_line(&output, "🍅 Paused 12m");
}

#[test]
fn status_focus_done_with_break() {
    let output = status_for("done-break", &focus_json(&ago_secs(10), 5));
    assert_line(&output, "🍅 Focus done · break 5m");
}

#[test]
fn status_break_counts_down() {
    // The focus ended 90 s ago and the break runs 5 min, so 3.5 min are left.
    let output = status_for("break-down", &focus_json(&ago_secs(90), 5));
    assert_line(&output, "🍅 Break 4m");
}

#[test]
fn status_break_over() {
    let output = status_for("break-over", &break_json(&ago_secs(10)));
    assert_line(&output, "🍅 Break over");
}

#[test]
fn status_idle_after_break_over() {
    let output = status_for("after-over", &break_json(&ago_secs(120)));
    assert_line(&output, "🍅");
}

#[test]
fn status_focus_done_no_break() {
    let output = status_for("done-no-break", &focus_json(&ago_secs(10), 0));
    assert_line(&output, "🍅 Focus done");
}

#[test]
fn status_skips_passed_states() {
    let output = status_for("skips", &focus_json(&ago_secs(40 * 60), 5));
    assert_line(&output, "🍅");
}

#[test]
fn status_paused_does_not_move() {
    let state = StateDir::new("paused-still");
    state.write_timer(&paused_json(720));
    let first = run_status(&state.path);
    sleep(Duration::from_secs(1));
    let second = run_status(&state.path);
    assert_line(&first, "🍅 Paused 12m");
    assert_line(&second, "🍅 Paused 12m");
}

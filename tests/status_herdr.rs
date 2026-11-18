//! `herdr-deepwork status` adds the away hint and keeps the sidebar tomato, talking to a fake herdr.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

const HOME: &str = "w0:p1";
const OTHER: &str = "w0:p2";

/// How the fake herdr answers.
#[derive(Clone, Copy)]
enum Herdr {
    /// Every pane exists.
    Normal,
    /// No pane exists: `pane get` fails with `pane_not_found`.
    MissingPane,
    /// Logs the call, then stays silent for 2 s.
    Silent,
}

/// A temp state folder holding the timer file, the fake herdr and its log.
struct StateDir {
    path: PathBuf,
}

impl StateDir {
    fn new(tag: &str) -> Self {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!(
            "herdr-deepwork-status-herdr-{}-{}-{}",
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

    fn herdr_path(&self) -> PathBuf {
        self.path.join("fake-herdr")
    }

    fn log_path(&self) -> PathBuf {
        self.path.join("herdr.log")
    }

    /// Writes the fake herdr: a POSIX sh script that appends its argv as one
    /// line to the log and prints canned JSON.
    fn write_herdr(&self, kind: Herdr) {
        let missing = matches!(kind, Herdr::MissingPane);
        let silent = matches!(kind, Herdr::Silent);
        let script = FAKE_HERDR
            .replace("@LOG@", &self.log_path().to_string_lossy())
            .replace("@MISSING@", if missing { "1" } else { "0" })
            .replace("@SILENT@", if silent { "1" } else { "0" });
        let path = self.herdr_path();
        fs::write(&path, script).expect("write fake herdr");
        let mut perms = fs::metadata(&path).expect("stat fake herdr").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).expect("chmod fake herdr");

        // The first exec of a fresh script is slow on macOS; warm it so status
        // meets its herdr deadline, then drop the warm-up call from the log.
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
}

impl Drop for StateDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

const FAKE_HERDR: &str = r#"#!/bin/sh
echo "$*" >> '@LOG@'
if [ '@SILENT@' = 1 ]; then
  exec sleep 2 >/dev/null 2>&1
fi
case "$1 $2" in
  "pane get")
    if [ '@MISSING@' = 1 ]; then
      printf '{"error":{"code":"pane_not_found","message":"pane %s not found"},"id":"cli:pane:get"}\n' "$3"
      exit 1
    fi
    printf '{"id":"cli:pane:get","result":{"pane":{"agent":"claude","pane_id":"%s","terminal_title_stripped":"herdr-deepwork","workspace_id":"w0","tab_id":"w0:t1"},"type":"pane_info"}}\n' "$3"
    ;;
  "pane list")
    if [ '@MISSING@' = 1 ]; then
      printf '{"id":"cli:pane:list","result":{"panes":[],"type":"pane_list"}}\n'
    else
      printf '{"id":"cli:pane:list","result":{"panes":[{"agent":"claude","pane_id":"w0:p1","terminal_title_stripped":"herdr-deepwork","workspace_id":"w0","tab_id":"w0:t1"},{"agent":"claude","pane_id":"w0:p2","terminal_title_stripped":"other","workspace_id":"w0","tab_id":"w0:t1"}],"type":"pane_list"}}\n'
    fi
    ;;
  "agent get")
    if [ '@MISSING@' = 1 ]; then
      printf '{"error":{"code":"agent_not_found","message":"agent %s not found"},"id":"cli:agent:get"}\n' "$3"
      exit 1
    fi
    printf '{"id":"cli:agent:get","result":{"agent":{"agent":"claude","pane_id":"%s","workspace_id":"w0","tab_id":"w0:t1"},"type":"agent_info"}}\n' "$3"
    ;;
  "agent list")
    if [ '@MISSING@' = 1 ]; then
      printf '{"id":"cli:agent:list","result":{"agents":[],"type":"agent_list"}}\n'
    else
      printf '{"id":"cli:agent:list","result":{"agents":[{"agent":"claude","pane_id":"w0:p1","workspace_id":"w0","tab_id":"w0:t1"},{"agent":"claude","pane_id":"w0:p2","workspace_id":"w0","tab_id":"w0:t1"}],"type":"agent_list"}}\n'
    fi
    ;;
  "pane report-metadata")
    printf '{"id":"cli:pane:report-metadata","result":{"type":"ok"}}\n'
    ;;
  *)
    printf '{}\n'
    ;;
esac
"#;

/// Runs `herdr-deepwork status` against the fake herdr. `active` is the value of
/// `HERDR_ACTIVE_PANE_ID`, or unset when `None`.
fn run_status(state: &StateDir, active: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_herdr-deepwork"));
    command
        .arg("status")
        .env("HERDR_PLUGIN_STATE_DIR", &state.path)
        .env("HERDR_BIN_PATH", state.herdr_path())
        .env_remove("HERDR_SESSION")
        .env_remove("HERDR_PLUGIN_CONTEXT_JSON");
    match active {
        Some(pane) => command.env("HERDR_ACTIVE_PANE_ID", pane),
        None => command.env_remove("HERDR_ACTIVE_PANE_ID"),
    };
    command.output().expect("run herdr-deepwork status")
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

/// A focus timer file with home pane `w0:p1`, named `home_name`.
fn focus_named_json(home_name: &str, ends_at: &str, break_mins: u8) -> String {
    format!(
        r#"{{"mode":"focus","home_pane":"{HOME}","home_name":"{home_name}","ends_at":"{ends_at}","left_secs":null,"break_mins":{break_mins},"last_focus_mins":25,"last_break_mins":5,"last_custom_mins":null,"message":null,"message_ends_at":null}}"#
    )
}

fn focus_json(ends_at: &str, break_mins: u8) -> String {
    focus_named_json("work", ends_at, break_mins)
}

fn paused_json(left_secs: u64) -> String {
    format!(
        r#"{{"mode":"paused","home_pane":"{HOME}","home_name":"work","ends_at":null,"left_secs":{left_secs},"break_mins":5,"last_focus_mins":25,"last_break_mins":5,"last_custom_mins":null,"message":null,"message_ends_at":null}}"#
    )
}

/// An idle timer file that still remembers its last home pane.
fn idle_json() -> String {
    format!(
        r#"{{"mode":"idle","home_pane":"{HOME}","home_name":"work","ends_at":null,"left_secs":null,"break_mins":5,"last_focus_mins":25,"last_break_mins":5,"last_custom_mins":null,"message":null,"message_ends_at":null}}"#
    )
}

/// Writes the timer file and the fake herdr into a fresh state folder.
fn setup(tag: &str, kind: Herdr, timer: &str) -> StateDir {
    let state = StateDir::new(tag);
    state.write_timer(timer);
    state.write_herdr(kind);
    state
}

/// Whether `call` sets the `deepwork` token on the home pane to `value`, expiring in 3 s.
fn is_set_call(call: &str, value: &str) -> bool {
    call.starts_with(&format!("pane report-metadata {HOME} "))
        && call.contains("--source herdr-deepwork")
        && call.contains(&format!("--token deepwork={value}"))
        && call.contains("--ttl-ms 3000")
}

#[test]
fn status_away_hint() {
    let state = setup(
        "away",
        Herdr::Normal,
        &focus_json(&in_secs(18 * 60 + 30), 5),
    );
    let output = run_status(&state, Some(OTHER));
    assert_line(&output, "🍅 19m ← work");
}

#[test]
fn status_home_no_hint() {
    let state = setup(
        "home",
        Herdr::Normal,
        &focus_json(&in_secs(18 * 60 + 30), 5),
    );
    let at_home = run_status(&state, Some(HOME));
    assert_line(&at_home, "🍅 19m");

    let unset = run_status(&state, None);
    assert_line(&unset, "🍅 19m");
}

#[test]
fn status_paused_no_hint() {
    let state = setup("paused-away", Herdr::Normal, &paused_json(720));
    let output = run_status(&state, Some(OTHER));
    assert_line(&output, "🍅 Paused 12m");
}

#[test]
fn status_sets_sidebar_tomato() {
    let focus = setup("set-focus", Herdr::Normal, &focus_json(&in_secs(600), 5));
    let output = run_status(&focus, Some(HOME));
    assert_line(&output, "🍅 10m");
    let calls = focus.herdr_calls();
    assert!(
        calls.iter().any(|call| is_set_call(call, "🍅")),
        "no deepwork=🍅 call with --ttl-ms 3000 on {HOME}: {calls:?}"
    );
    assert!(
        calls.iter().all(|call| !call.contains("deepwork=⏸")),
        "unexpected deepwork=⏸ call during a focus: {calls:?}"
    );

    let paused = setup("set-paused", Herdr::Normal, &paused_json(720));
    let output = run_status(&paused, Some(HOME));
    assert_line(&output, "🍅 Paused 12m");
    let calls = paused.herdr_calls();
    assert!(
        calls.iter().any(|call| is_set_call(call, "⏸")),
        "no deepwork=⏸ call with --ttl-ms 3000 on {HOME}: {calls:?}"
    );
    assert!(
        calls.iter().all(|call| !call.contains("deepwork=🍅")),
        "unexpected deepwork=🍅 call while paused: {calls:?}"
    );
}

#[test]
fn status_idle_runs_no_herdr() {
    let cases = [
        ("quiet-idle", idle_json(), "🍅"),
        ("quiet-break", focus_json(&ago_secs(90), 5), "🍅 Break 4m"),
        ("quiet-passed", focus_json(&ago_secs(40 * 60), 5), "🍅"),
        ("quiet-done", focus_json(&ago_secs(10), 0), "🍅 Focus done"),
    ];
    for (tag, timer, line) in cases {
        let state = setup(tag, Herdr::Normal, &timer);
        let output = run_status(&state, Some(HOME));
        assert_line(&output, line);
        let calls = state.herdr_calls();
        assert!(calls.is_empty(), "{tag}: herdr was called: {calls:?}");
    }
}

#[test]
fn status_survives_silent_herdr() {
    let state = setup(
        "silent",
        Herdr::Silent,
        &focus_json(&in_secs(18 * 60 + 30), 5),
    );
    // The fastest of 3 runs, so parallel test load cannot fail the limit.
    let mut fastest = Duration::MAX;
    for _ in 0..3 {
        let started = Instant::now();
        let output = run_status(&state, Some(OTHER));
        fastest = fastest.min(started.elapsed());
        assert_line(&output, "🍅 19m");
    }
    assert!(
        fastest < Duration::from_millis(200),
        "fastest of 3 runs took {fastest:?} with a silent herdr"
    );
}

#[test]
fn status_calls_only_allowed_herdr() {
    const ALLOWED: [&str; 5] = [
        "pane get",
        "pane list",
        "agent list",
        "agent get",
        "pane report-metadata",
    ];
    // A home name with control bytes must not reach the output.
    let dirty = focus_named_json(r"wo\u001b[31m\trk\u0007", &in_secs(18 * 60 + 30), 5);
    let timers = [
        ("allowed-away", dirty),
        ("allowed-focus", focus_json(&in_secs(600), 5)),
        ("allowed-paused", paused_json(720)),
        ("allowed-idle", idle_json()),
        ("allowed-break", focus_json(&ago_secs(90), 5)),
    ];
    for (tag, timer) in timers {
        let state = setup(tag, Herdr::Normal, &timer);
        let output = run_status(&state, Some(OTHER));
        assert_eq!(output.status.code(), Some(0), "{tag}: exit code");

        let (last, body) = output
            .stdout
            .split_last()
            .expect("status printed something");
        assert_eq!(*last, b'\n', "{tag}: stdout ends with a newline");
        assert!(
            body.iter().all(|byte| *byte >= 0x20),
            "{tag}: control byte in stdout: {:?}",
            output.stdout
        );

        for call in state.herdr_calls() {
            let mut words = call.split_whitespace();
            let subcommand = format!(
                "{} {}",
                words.next().unwrap_or_default(),
                words.next().unwrap_or_default()
            );
            assert!(
                ALLOWED.contains(&subcommand.as_str()),
                "{tag}: herdr subcommand outside the allowed list: {call}"
            );
        }
    }
}

#[test]
fn status_home_closed_no_hint() {
    let state = setup(
        "closed",
        Herdr::MissingPane,
        &focus_json(&in_secs(18 * 60 + 30), 5),
    );
    let output = run_status(&state, Some(OTHER));
    assert_line(&output, "🍅 19m");
}

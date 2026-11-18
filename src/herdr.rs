//! Calls to the `herdr` command, each abandoned if it stays silent too long.

use std::ffi::OsString;
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

const POLL: Duration = Duration::from_millis(1);

/// How long a sidebar token lives, in ms, unless it is set again.
const TOKEN_TTL_MS: &str = "3000";

/// The `herdr` program and how long each call may take to answer.
#[derive(Debug, Clone)]
pub struct Herdr {
    program: OsString,
    budget: Duration,
}

/// A running `herdr` call.
#[derive(Debug)]
pub struct Call {
    child: Child,
    deadline: Instant,
}

impl Herdr {
    /// `$HERDR_BIN_PATH` when set and non-empty, else `herdr` on `PATH`,
    /// with each call cut off `budget` after it starts.
    #[must_use]
    pub fn from_env(budget: Duration) -> Self {
        let program = std::env::var_os("HERDR_BIN_PATH")
            .filter(|p| !p.is_empty())
            .unwrap_or_else(|| OsString::from("herdr"));
        Self { program, budget }
    }

    /// Starts `herdr <args>`; `None` when it cannot be started.
    ///
    /// The clock starts once the program is running: the OS can hold the
    /// first launch of a new executable for a few hundred ms, which says
    /// nothing about how fast herdr answers.
    #[must_use]
    pub fn spawn(&self, args: &[&str]) -> Option<Call> {
        let child = Command::new(&self.program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        Some(Call {
            child,
            deadline: Instant::now() + self.budget,
        })
    }

    /// Starts a call that sets the sidebar token `focus` on `pane` to
    /// `token`, or clears it when `token` is `None`.
    ///
    /// A set expires on its own after [`TOKEN_TTL_MS`], so the status command
    /// only refreshes it while a timer runs and never has to clear it.
    #[must_use]
    pub fn report_focus_token(&self, pane: &str, token: Option<&str>) -> Option<Call> {
        let mut args = vec!["pane", "report-metadata", pane, "--source", "herdr-focus"];
        let set = token.map(|value| format!("focus={value}"));
        match &set {
            Some(set) => args.extend(["--token", set.as_str(), "--ttl-ms", TOKEN_TTL_MS]),
            None => args.extend(["--clear-token", "focus"]),
        }
        self.spawn(&args)
    }

    /// Starts a call that looks `pane` up; [`Call::found_pane`] reads the
    /// answer.
    #[must_use]
    pub fn get_pane(&self, pane: &str) -> Option<Call> {
        self.spawn(&["pane", "get", pane])
    }

    /// Whether herdr answers in time that `pane` exists.
    #[must_use]
    pub fn has_pane(&self, pane: &str) -> bool {
        self.get_pane(pane).is_some_and(Call::found_pane)
    }
}

impl Call {
    /// Waits for the call and returns its stdout when it exits successfully
    /// before the deadline; a call still running then is killed.
    #[must_use]
    pub fn finish(mut self) -> Option<String> {
        loop {
            match self.child.try_wait() {
                Ok(Some(status)) => {
                    if !status.success() {
                        return None;
                    }
                    let mut out = String::new();
                    self.child.stdout.take()?.read_to_string(&mut out).ok()?;
                    return Some(out);
                }
                Ok(None) if Instant::now() < self.deadline => sleep(POLL),
                _ => {
                    let _ = self.child.kill();
                    let _ = self.child.wait();
                    return None;
                }
            }
        }
    }

    /// Waits for a `pane get` call; whether it answered with a pane.
    #[must_use]
    pub fn found_pane(self) -> bool {
        self.finish().is_some_and(|reply| is_pane_reply(&reply))
    }
}

/// Waits for a call whose answer does not matter, so its effect lands before
/// the process exits.
pub fn finish_quietly(call: Option<Call>) {
    if let Some(call) = call {
        let _ = call.finish();
    }
}

/// Whether the reply to `pane get` describes a pane.
fn is_pane_reply(reply: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(reply)
        .is_ok_and(|v| v.pointer("/result/pane").is_some_and(|p| p.is_object()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pane_reply_needs_a_pane() {
        assert!(is_pane_reply(
            r#"{"result":{"pane":{"pane_id":"w0:p1"},"type":"pane_info"}}"#
        ));
        assert!(!is_pane_reply(r#"{"error":{"code":"pane_not_found"}}"#));
        assert!(!is_pane_reply(""));
        assert!(!is_pane_reply("{}"));
    }

    #[test]
    fn a_missing_program_is_no_call() {
        let herdr = Herdr {
            program: OsString::from("/no/such/herdr"),
            budget: Duration::from_millis(50),
        };
        assert!(herdr.get_pane("p").is_none());
    }
}

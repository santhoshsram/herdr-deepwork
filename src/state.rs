//! The saved timer: where it lives, its shape, and loading it.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize, Serializer};

const BREAK_CHOICES: [u8; 4] = [0, 2, 5, 10];
const FOCUS_RANGE: std::ops::RangeInclusive<u16> = 1..=180;

/// Why a timer file is not a valid timer.
#[derive(Debug)]
pub enum StateError {
    /// The file could not be read or written.
    Io(std::io::Error),
    /// The text is not JSON of the contract shape.
    Json(serde_json::Error),
    /// A value is out of range or a null sits where the mode needs a value.
    Invalid(&'static str),
}

impl fmt::Display for StateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "cannot read or write timer file: {e}"),
            Self::Json(e) => write!(f, "timer file is not valid: {e}"),
            Self::Invalid(why) => write!(f, "timer file is not valid: {why}"),
        }
    }
}

impl std::error::Error for StateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Json(e) => Some(e),
            Self::Invalid(_) => None,
        }
    }
}

/// What the timer is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// No timer runs.
    #[default]
    Idle,
    /// A focus counts down.
    Focus,
    /// A focus is held at `left_secs`.
    Paused,
    /// A break counts down.
    Break,
}

/// The one-minute message the tab bar shows after a timer ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Message {
    /// A focus just ended.
    FocusDone,
    /// A break just ended.
    BreakOver,
}

/// A UTC instant in whole seconds since the Unix epoch.
///
/// Written and read as `YYYY-MM-DDTHH:MM:SSZ`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(pub u64);

impl Timestamp {
    /// The current time; a clock set before 1970 reads as the epoch.
    #[must_use]
    pub fn now() -> Self {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        Self(secs)
    }

    /// Parses `YYYY-MM-DDTHH:MM:SSZ`.
    ///
    /// # Errors
    /// Returns [`StateError::Invalid`] for any other text or an impossible date.
    pub fn parse(text: &str) -> Result<Self, StateError> {
        const BAD: StateError = StateError::Invalid("bad RFC 3339 time");
        let b = text.as_bytes();
        let separators = [
            (4, b'-'),
            (7, b'-'),
            (10, b'T'),
            (13, b':'),
            (16, b':'),
            (19, b'Z'),
        ];
        if b.len() != 20 || separators.iter().any(|&(i, c)| b[i] != c) {
            return Err(BAD);
        }
        let field = |from: usize, to: usize| -> Result<u64, StateError> {
            let digits = &b[from..to];
            if digits.iter().all(u8::is_ascii_digit) {
                Ok(digits.iter().fold(0, |n, d| n * 10 + u64::from(d - b'0')))
            } else {
                Err(BAD)
            }
        };
        let year = field(0, 4)?;
        let month = field(5, 7)?;
        let day = field(8, 10)?;
        let (hour, minute, second) = (field(11, 13)?, field(14, 16)?, field(17, 19)?);
        if !(1..=12).contains(&month)
            || day < 1
            || day > days_in_month(year, month)
            || hour > 23
            || minute > 59
            || second > 59
        {
            return Err(BAD);
        }
        let days = days_from_civil(year, month, day);
        Ok(Self(days * 86_400 + hour * 3600 + minute * 60 + second))
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (year, month, day) = civil_from_days(self.0 / 86_400);
        let rem = self.0 % 86_400;
        write!(
            f,
            "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
            rem / 3600,
            (rem % 3600) / 60,
            rem % 60
        )
    }
}

impl Serialize for Timestamp {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Timestamp {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).map_err(de::Error::custom)
    }
}

fn is_leap(year: u64) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

fn days_in_month(year: u64, month: u64) -> u64 {
    match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Days since 1970-01-01 for a valid civil date (Hinnant's algorithm).
fn days_from_civil(year: u64, month: u64, day: u64) -> u64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y / 400;
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The civil date for days since 1970-01-01 (Hinnant's algorithm).
fn civil_from_days(days: u64) -> (u64, u64, u64) {
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + u64::from(month <= 2);
    (year, month, day)
}

/// A key that must be present, though its value may be null.
fn required<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(deserializer)
}

/// The contents of `timer.json`, in the exact contract shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Timer {
    /// What the timer is doing.
    pub mode: Mode,
    /// Pane that started the focus.
    #[serde(deserialize_with = "required")]
    pub home_pane: Option<String>,
    /// Display name of the home pane.
    #[serde(deserialize_with = "required")]
    pub home_name: Option<String>,
    /// When the running focus or break ends.
    #[serde(deserialize_with = "required")]
    pub ends_at: Option<Timestamp>,
    /// Whole seconds left while paused.
    #[serde(deserialize_with = "required")]
    pub left_secs: Option<u64>,
    /// Length of the break after the current focus, in minutes.
    pub break_mins: u8,
    /// Focus length last chosen, in minutes.
    pub last_focus_mins: u16,
    /// Break length last chosen, in minutes.
    pub last_break_mins: u8,
    /// Custom focus length last typed, in minutes.
    #[serde(deserialize_with = "required")]
    pub last_custom_mins: Option<u16>,
    /// The one-minute message to show.
    #[serde(deserialize_with = "required")]
    pub message: Option<Message>,
    /// When the message stops showing.
    #[serde(deserialize_with = "required")]
    pub message_ends_at: Option<Timestamp>,
}

impl Default for Timer {
    /// No timer, with the first-run choices of 25 minutes and a 5 minute break.
    fn default() -> Self {
        Self {
            mode: Mode::Idle,
            home_pane: None,
            home_name: None,
            ends_at: None,
            left_secs: None,
            break_mins: 5,
            last_focus_mins: 25,
            last_break_mins: 5,
            last_custom_mins: None,
            message: None,
            message_ends_at: None,
        }
    }
}

impl Timer {
    /// Parses and checks the text of a `timer.json`.
    ///
    /// # Errors
    /// Returns [`StateError::Json`] when the text is not the contract shape
    /// and [`StateError::Invalid`] when a value is out of range or a null sits
    /// where the mode needs a value.
    pub fn parse(text: &str) -> Result<Self, StateError> {
        let timer: Self = serde_json::from_str(text).map_err(StateError::Json)?;
        timer.validate()?;
        Ok(timer)
    }

    fn validate(&self) -> Result<(), StateError> {
        let invalid = StateError::Invalid;
        if !BREAK_CHOICES.contains(&self.break_mins) {
            return Err(invalid("break_mins out of range"));
        }
        if !BREAK_CHOICES.contains(&self.last_break_mins) {
            return Err(invalid("last_break_mins out of range"));
        }
        if !FOCUS_RANGE.contains(&self.last_focus_mins) {
            return Err(invalid("last_focus_mins out of range"));
        }
        if self
            .last_custom_mins
            .is_some_and(|m| !FOCUS_RANGE.contains(&m))
        {
            return Err(invalid("last_custom_mins out of range"));
        }
        if self.message.is_some() != self.message_ends_at.is_some() {
            return Err(invalid("message and message_ends_at disagree"));
        }
        let home = self.home_pane.is_some() && self.home_name.is_some();
        let needs = match self.mode {
            Mode::Idle => true,
            Mode::Focus => home && self.ends_at.is_some(),
            Mode::Paused => home && self.left_secs.is_some(),
            Mode::Break => self.ends_at.is_some(),
        };
        if needs {
            Ok(())
        } else {
            Err(invalid("a value the mode needs is null"))
        }
    }
}

/// The state folder: `$HERDR_PLUGIN_STATE_DIR` when set and non-empty, else
/// the folder holding the running executable.
///
/// Returns `None` when neither is known.
#[must_use]
pub fn state_dir() -> Option<PathBuf> {
    non_empty_env("HERDR_PLUGIN_STATE_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::current_exe()
                .ok()?
                .parent()
                .map(Path::to_path_buf)
        })
}

/// The session name: `$HERDR_SESSION`, or `default` when unset or empty.
#[must_use]
pub fn session_name() -> String {
    non_empty_env("HERDR_SESSION").unwrap_or_else(|| "default".to_string())
}

/// The timer file for `session` inside `state_dir`.
#[must_use]
pub fn timer_path(state_dir: &Path, session: &str) -> PathBuf {
    state_dir.join("sessions").join(session).join("timer.json")
}

/// Reads and checks the timer file at `path`.
///
/// # Errors
/// Returns a [`StateError`] when the file cannot be read or is not a valid timer.
pub fn load(path: &Path) -> Result<Timer, StateError> {
    let text = fs::read_to_string(path).map_err(StateError::Io)?;
    Timer::parse(&text)
}

/// The timer for the current session, or the first-run timer when the file
/// is missing or not valid.
#[must_use]
pub fn load_current() -> Timer {
    state_dir()
        .and_then(|dir| load(&timer_path(&dir, &session_name())).ok())
        .unwrap_or_default()
}

/// Writes `timer` to `path`, creating the folders, by writing a sibling file
/// and renaming it over `path` so a reader never sees half a file.
///
/// # Errors
/// Returns a [`StateError`] when the timer cannot be serialized or the file
/// cannot be written; `path` is then unchanged.
pub fn save(path: &Path, timer: &Timer) -> Result<(), StateError> {
    let mut text = serde_json::to_string(timer).map_err(StateError::Json)?;
    text.push('\n');
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(dir).map_err(StateError::Io)?;
    let temp = dir.join(format!(".timer.json.{}.tmp", std::process::id()));
    let written = fs::write(&temp, text).and_then(|()| fs::rename(&temp, path));
    if let Err(e) = written {
        let _ = fs::remove_file(&temp);
        return Err(StateError::Io(e));
    }
    Ok(())
}

fn non_empty_env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDLE: &str = r#"{"mode":"idle","home_pane":null,"home_name":null,"ends_at":null,"left_secs":null,"break_mins":5,"last_focus_mins":25,"last_break_mins":5,"last_custom_mins":null,"message":null,"message_ends_at":null}"#;

    #[test]
    fn timestamp_round_trips() {
        let t = Timestamp::parse("2026-11-18T09:05:07Z").expect("valid time");
        assert_eq!(t.to_string(), "2026-11-18T09:05:07Z");
        assert_eq!(
            Timestamp::parse("1970-01-01T00:00:00Z").expect("epoch").0,
            0
        );
        assert_eq!(
            Timestamp::parse("2024-02-29T23:59:59Z")
                .expect("leap day")
                .to_string(),
            "2024-02-29T23:59:59Z"
        );
    }

    #[test]
    fn timestamp_rejects_bad_text() {
        for text in [
            "",
            "2026-13-01T00:00:00Z",
            "2026-02-30T00:00:00Z",
            "2026-11-18 09:05:07Z",
            "2026-11-18T09:05:07+00:00",
        ] {
            assert!(Timestamp::parse(text).is_err(), "{text}");
        }
    }

    #[test]
    fn idle_file_parses() {
        assert_eq!(Timer::parse(IDLE).expect("valid"), Timer::default());
    }

    #[test]
    fn unknown_shapes_are_errors() {
        let focus_without_end = IDLE.replace("idle", "focus");
        let missing = IDLE.replace(r#","message_ends_at":null"#, "");
        let zero = IDLE.replace(r#""last_focus_mins":25"#, r#""last_focus_mins":0"#);
        for text in ["", "[]", "{}", &focus_without_end, &missing, &zero] {
            assert!(Timer::parse(text).is_err(), "{text}");
        }
    }
}

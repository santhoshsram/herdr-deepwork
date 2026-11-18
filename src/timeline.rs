//! What the timer shows at a given moment, derived from the saved times.

use crate::state::{Message, Mode, Timer, Timestamp};

const MINUTE: u64 = 60;

/// One state of the tab bar line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum View {
    /// No timer and no message.
    Idle,
    /// A focus counts down.
    Focus {
        /// Whole seconds until the focus ends.
        left_secs: u64,
    },
    /// A focus is held.
    Paused {
        /// Whole seconds the focus had left when paused.
        left_secs: u64,
    },
    /// A focus just ended and a break has begun.
    FocusDoneBreak {
        /// Length of the break in minutes.
        break_mins: u8,
    },
    /// A focus just ended and no break follows.
    FocusDone,
    /// A break counts down.
    Break {
        /// Whole seconds until the break ends.
        left_secs: u64,
    },
    /// A break just ended.
    BreakOver,
}

/// What a running timer is doing, as far as the control popup cares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    /// A focus counts down.
    Focus,
    /// A focus is held.
    Paused,
    /// A break counts down.
    Break,
}

/// The phase of a running timer; `None` when nothing runs.
#[must_use]
pub fn phase(view: View) -> Option<Phase> {
    match view {
        View::Focus { .. } => Some(Phase::Focus),
        View::Paused { .. } => Some(Phase::Paused),
        // The break has begun even while the tab bar still reports the focus
        // as done.
        View::FocusDoneBreak { .. } | View::Break { .. } => Some(Phase::Break),
        View::Idle | View::FocusDone | View::BreakOver => None,
    }
}

/// What the timer shows at `now`.
///
/// A state the clock already passed is skipped, never shown late.
#[must_use]
pub fn view(timer: &Timer, now: Timestamp) -> View {
    match timer.mode {
        Mode::Idle => message_view(timer, now),
        Mode::Paused => timer
            .left_secs
            .map_or(View::Idle, |left_secs| View::Paused { left_secs }),
        Mode::Focus => timer
            .ends_at
            .map_or(View::Idle, |end| focus_view(end, timer.break_mins, now)),
        Mode::Break => timer.ends_at.map_or(View::Idle, |end| break_view(end, now)),
    }
}

fn focus_view(end: Timestamp, break_mins: u8, now: Timestamp) -> View {
    if now < end {
        return View::Focus {
            left_secs: end.0 - now.0,
        };
    }
    let since_end = now.0 - end.0;
    if since_end < MINUTE {
        return if break_mins == 0 {
            View::FocusDone
        } else {
            View::FocusDoneBreak { break_mins }
        };
    }
    if break_mins == 0 {
        return View::Idle;
    }
    break_view(Timestamp(end.0 + u64::from(break_mins) * MINUTE), now)
}

fn break_view(end: Timestamp, now: Timestamp) -> View {
    if now < end {
        View::Break {
            left_secs: end.0 - now.0,
        }
    } else if now.0 - end.0 < MINUTE {
        View::BreakOver
    } else {
        View::Idle
    }
}

fn message_view(timer: &Timer, now: Timestamp) -> View {
    match (timer.message, timer.message_ends_at) {
        (Some(Message::FocusDone), Some(until)) if now < until => View::FocusDone,
        (Some(Message::BreakOver), Some(until)) if now < until => View::BreakOver,
        _ => View::Idle,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: u64 = 1_000_000;

    fn timer(mode: Mode, ends_at: u64, break_mins: u8) -> Timer {
        Timer {
            mode,
            home_pane: Some("p1".to_string()),
            home_name: Some("work".to_string()),
            ends_at: Some(Timestamp(ends_at)),
            break_mins,
            ..Timer::default()
        }
    }

    #[test]
    fn focus_counts_down_then_flows_into_the_break() {
        let t = timer(Mode::Focus, T, 5);
        assert_eq!(view(&t, Timestamp(T - 90)), View::Focus { left_secs: 90 });
        assert_eq!(
            view(&t, Timestamp(T + 59)),
            View::FocusDoneBreak { break_mins: 5 }
        );
        assert_eq!(view(&t, Timestamp(T + 90)), View::Break { left_secs: 210 });
        assert_eq!(view(&t, Timestamp(T + 300)), View::BreakOver);
        assert_eq!(view(&t, Timestamp(T + 359)), View::BreakOver);
        assert_eq!(view(&t, Timestamp(T + 360)), View::Idle);
    }

    #[test]
    fn focus_without_break_shows_done_for_a_minute() {
        let t = timer(Mode::Focus, T, 0);
        assert_eq!(view(&t, Timestamp(T + 59)), View::FocusDone);
        assert_eq!(view(&t, Timestamp(T + 60)), View::Idle);
    }

    #[test]
    fn break_mode_ends_in_break_over_then_idle() {
        let t = timer(Mode::Break, T, 5);
        assert_eq!(view(&t, Timestamp(T - 1)), View::Break { left_secs: 1 });
        assert_eq!(view(&t, Timestamp(T)), View::BreakOver);
        assert_eq!(view(&t, Timestamp(T + 60)), View::Idle);
    }

    #[test]
    fn idle_shows_an_unexpired_message_only() {
        let t = Timer {
            message: Some(Message::FocusDone),
            message_ends_at: Some(Timestamp(T)),
            ..Timer::default()
        };
        assert_eq!(view(&t, Timestamp(T - 1)), View::FocusDone);
        assert_eq!(view(&t, Timestamp(T)), View::Idle);
    }

    #[test]
    fn running_states_have_a_phase() {
        assert_eq!(phase(View::Focus { left_secs: 5 }), Some(Phase::Focus));
        assert_eq!(phase(View::Paused { left_secs: 5 }), Some(Phase::Paused));
        assert_eq!(phase(View::Break { left_secs: 5 }), Some(Phase::Break));
        assert_eq!(
            phase(View::FocusDoneBreak { break_mins: 5 }),
            Some(Phase::Break)
        );
        assert_eq!(phase(View::FocusDone), None);
        assert_eq!(phase(View::BreakOver), None);
        assert_eq!(phase(View::Idle), None);
    }

    #[test]
    fn paused_ignores_the_clock() {
        let t = Timer {
            mode: Mode::Paused,
            left_secs: Some(720),
            ..timer(Mode::Paused, T, 5)
        };
        assert_eq!(view(&t, Timestamp(T)), View::Paused { left_secs: 720 });
        assert_eq!(
            view(&t, Timestamp(T + 999)),
            View::Paused { left_secs: 720 }
        );
    }
}

# herdr-focus v1

## Goal

Ship the whole plugin as designed: one key starts a focus timer on the agent
the user is in, the tab bar shows the time left, and a break follows. It
passes the one test because it adds one key, no setting and no lock.

## Users and journeys

The user is the developer in `product.md#who-uses-it`.

- **J1 Start.** Given no timer runs and the user is in agent A, when they
  press `prefix+f` and Enter, then a focus of the last chosen length (25 min
  on first run) starts on A, and the status shows `🍅 25:00` counting down.
- **J2 Custom length.** Given the start popup is open, when the user types
  `40` and Enter, then a 40 min focus starts, and the next start popup
  offers 40.
- **J3 Away.** Given a focus runs on A, when the user switches to agent B,
  then the status adds `← <A>`. When they press `prefix+f` and Enter, herdr
  focuses A and the suffix goes.
- **J4 Pause.** Given a focus runs, when the user presses `prefix+f` and
  Enter (at home) or `p` (away), then the countdown stops and shows
  `🍅 Paused <mm:ss>`. Resume continues from the same time.
- **J5 Stop.** Given a focus or pause, when the user presses `prefix+f` and
  `s`, then the timer ends with no break and the status goes idle.
- **J6 Break.** Given a focus with a 5 min break reaches 0:00, then the
  break starts on its own, the status shows `🍅 Focus done · break 4:59`
  for 1 min, then `🍅 Break <mm:ss>`. At 0:00 it shows `🍅 Break over` for
  1 min, then idle. No focus starts on its own.
- **J7 Restart.** Given a timer runs, when the user detaches, or the herdr
  server restarts, and comes back, then the status shows the same time left
  as if nothing happened.

## Experience

`foreman/ux.md` holds the places, navigation, popups and status strings, and
`mock.html` at the repo root shows them. This section adds what `ux.md`
leaves open.

- **Time format.** `m:ss` below 10 min, `mm:ss` up to 59:59, `h:mm:ss`
  from 1 hour (custom lengths go to 180 min).
- **Agent name.** `<agent>` is the home pane's terminal title as herdr strips
  it (`terminal_title_stripped`), read when the focus starts.
- **Break off.** When a focus with break `off` ends, the status shows
  `🍅 Focus done` for 1 min, then idle.
- **Paused break.** The break has no pause; its control popup offers only
  End break, as `ux.md#control-popup` shows.
- **Start popup keys.** `←→` move within the row. Typing digits selects
  custom on the Focus row and replaces the typed number; Backspace deletes a
  digit. Enter with an empty or out-of-range custom number keeps the popup
  open with `1 to 180` shown under the row. Enter otherwise starts the timer
  and closes the popup. Esc closes it with nothing started.
- **Control popup keys.** The ⏎ button is the first one in each state row
  of `ux.md#control-popup`. Paused: ⏎ resume, `s` stop. Break: ⏎ end
  break. Esc closes with no change.
- **Mouse.** Each button and option in the popup is clickable, the same as
  its key.
- **Marker.** 🍅 after home's sidebar row during focus, ⏸ while paused, none
  during break or idle.
- **Home closed.** If the home pane closes during focus, the timer keeps
  running, the status shows no `←` suffix, and the control popup shows the
  Home buttons.
- **Popup during a flash.** `prefix+f` during `🍅 Focus done` (break off) or
  `🍅 Break over` opens the start popup.

## Acceptance criteria

- AC1: When no timer runs, then `herdr-focus status` prints `🍅`. Proof: test
- AC2: When the user starts a focus from the start popup, then the status
  prints `🍅 <time left>` and the home row shows 🍅. Proof: screenshot
- AC3: When the start popup opens, then it names the current agent and
  preselects the last focus and break choices; on first run 25 and 5.
  Proof: test
- AC4: When the user types a custom number, then 1 to 180 starts that
  length and any other value keeps the popup open with `1 to 180`. Proof:
  test
- AC5: When a focus runs and the active pane is not home, then the status
  ends with `← <agent>`; at home it does not. Proof: test
- AC6: When the user picks Back to `<agent>`, then herdr focuses the home
  agent. Proof: screenshot
- AC7: When the user pauses, then the status shows `🍅 Paused <time>` and
  the time does not change until resume; the marker shows ⏸. Proof: test
- AC8: When the user stops, then the status prints `🍅` and the marker
  goes. Proof: test
- AC9: When a focus ends with a break set, then the status follows J6
  exactly, and no focus starts. Proof: test
- AC10: When a focus ends with break off, then the status shows
  `🍅 Focus done` for 1 min, then `🍅`. Proof: test
- AC11: When the state file holds a running timer and the process starts
  fresh at a later time, then the status shows the correct time left.
  Proof: test
- AC12: When the state file is missing, empty or not valid JSON, then the
  status prints `🍅` and exits 0. Proof: test
- AC13: When the status command runs, then it prints one line and exits 0
  within 200 ms, in every state. Proof: command output
- AC14: When a timer runs and the user starts another, then it cannot: the
  key opens the control popup, never a second start. Proof: test
- AC15: When the plugin runs, then nothing opens unless the user presses
  `prefix+f`, and no sound or system notification plays. Proof: test
- AC16: When a user installs with `herdr plugin install
  santhoshsram/herdr-focus` on a Mac with no Rust toolchain, then the
  README paste is the only other step and the status shows. Proof:
  command output
- AC17: When the user clicks a popup button or option, then it acts like
  its key. Proof: video

## Contracts

- **Status output.** One line on stdout, exit 0, in every state, exactly
  the strings in `ux.md#states` with the time format above. Idle prints
  `🍅`; dimness is herdr's choice, not the plugin's.
- **State file.** `state.json` in `HERDR_PLUGIN_STATE_DIR`. It holds: the
  mode (idle, focus, paused, break), the home pane id and agent name, the
  end time as a UTC instant (focus and break), the time left (paused), the
  break length for this focus, the last focus and break choices, and when
  the current flash ends. Writes are atomic (write then rename), so a read
  never sees half a file. An unknown or broken file reads as idle with
  first-run choices.
- **Plugin manifest.** `herdr-plugin.toml` with id `focus`, the action
  `focus.open` that opens the popup pane, and the install step.
- **Sidebar marker.** Pane metadata token `focus` on the home pane, value
  `🍅` or `⏸`, cleared when the timer stops, ends or leaves home.
- **README paste.** The `[[keys.command]]` binding for `prefix+f`, the
  `tab_bar_right` command entry, and `$focus` in the agent rows, as in
  `mock.html`. It works pasted as is, with no path for the user to edit.
- **CLI.** `herdr-focus status` and `herdr-focus panel`. Nothing else is
  public.

## Alternatives considered

- Sidebar strip for the time: rejected, herdr 0.9.5 has no plugin sidebar
  strip (`tech-stack.md#contested-choices`).
- A daemon that ticks the timer: rejected, the end time in the state file
  is enough and survives restarts.
- An end-of-focus popup: rejected, it swallows keystrokes meant for an
  agent (`product-guidelines.md#what-must-never-happen`).
- A config file for lengths: rejected, the popup remembers choices.
- A mouse start from the tab bar or a menu: not possible in herdr 0.9.5;
  issue #2.

## Non-functional

- The status command runs every second, so it starts and exits in under
  200 ms and makes no network call.
- Prebuilt binary for macOS on Apple silicon. Other platforms: none in v1.
- The timer drifts by less than 1 s per focus, because it reads the end
  time, not a tick count.

## Reuse and untouched

- Reused: herdr's popup pane, tab bar command entry, pane metadata tokens,
  plugin state dir, and `herdr agent` for the focused pane and for focusing
  home.
- Untouched: the user's herdr `config.toml` (the user pastes once) and
  herdr itself.

## Out of scope

- Muting other agents' toasts (#1).
- Starting from the mouse outside the popup (#2).
- Marketplace publishing (#3).
- Linux and Intel Mac binaries.
- Counts, history, sounds, notifications, multiple timers, a config file.

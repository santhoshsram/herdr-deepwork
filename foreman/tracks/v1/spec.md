# herdr-focus v1

## Goal

Ship the whole plugin as designed: one key starts a focus timer on the agent
the user is in, the tab bar shows the time left, and a break follows. It
passes the one test because it adds one key, no setting and no lock.

## Users and journeys

The user is the developer in `product.md#who-uses-it`.

- **J1 Start.** Given no timer runs and the user is in agent A, when they
  press `prefix+f` and Enter, then a focus of the last chosen length (25 min
  on first run) starts with A as home, and the status shows `🍅 25:00`
  counting down.
- **J2 Custom length.** Given the start popup is open, when the user types
  `40` and Enter, then a 40 min focus starts, and the next start popup
  preselects custom showing 40.
- **J3 Away.** Given a focus runs on home A, when the user switches to agent
  B, then the status adds the nudge `← <home>`. When they press `prefix+f`
  and Enter, herdr focuses A and the nudge goes.
- **J4 Pause.** Given a focus runs, when the user presses `prefix+f` and `p`
  (or Enter at home), then the countdown stops and shows
  `🍅 Paused <time>`. Resume continues from the same time.
- **J5 Stop.** Given a focus or pause, when the user presses `prefix+f` and
  `s`, then the timer ends with no break and the status goes idle.
- **J6 Break.** Given a focus with a 5 min break reaches 0:00, then the
  break starts on its own, the status shows `🍅 Focus done · break 4:59`
  for 1 min, then `🍅 Break <time>`. At 0:00 it shows `🍅 Break over` for
  1 min, then idle. No focus starts on its own.
- **J7 Restart.** Given a timer runs, when the user detaches, or the herdr
  server restarts, and comes back, then the status shows the same time left
  and the marker is back on home.
- **J8 End break.** Given a break runs, when the user presses `prefix+f` and
  Enter, then the break ends and the status goes idle at once, with no
  `🍅 Break over`.
- **J9 Install.** Given herdr 0.9.5 on an Apple silicon Mac with no Rust
  toolchain, when the user runs `herdr plugin install
  santhoshsram/herdr-focus` and pastes the README block into `config.toml`,
  then the tab bar shows `🍅`.

## Experience

`foreman/ux.md` holds the places, navigation, popups and status strings, and
`mock.html` at the repo root shows them. This section adds what `ux.md`
leaves open.

- **Time format.** `m:ss` below 10 min, `mm:ss` up to 59:59, `h:mm:ss`
  from 1 hour (custom lengths go to 180 min). Time left is whole seconds,
  rounded up, so a 25 min focus shows `25:00` at start and `0:01` in its
  last second.
- **Popup and focus.** The current agent, home at start, and Home vs Away in
  the control popup are all judged from the pane focused just before
  `prefix+f`. The popup pane never counts as away.
- **Home name.** `<home>` is the home pane's terminal title as herdr strips
  it (`terminal_title_stripped`), read when the focus starts. If that is
  empty, it is the agent kind (`claude`); if the pane is not an agent, the
  word `home`. A focus can start on any pane.
- **Break off.** When a focus with break `off` ends, the status shows
  `🍅 Focus done` for 1 min, then idle.
- **No pause in a break.** The break control popup offers only End break,
  as `ux.md#control-popup` shows.
- **Start popup keys.** `←→` move within the row, `↑↓` switch rows. Typing
  a digit on either row selects custom on the Focus row. The first digit
  after custom is selected replaces the shown number, later digits append,
  and at most 3 digits are kept. Backspace deletes the last digit. Enter
  with an empty or out-of-range custom number keeps the popup open with
  `1 to 180` shown under the row. Enter otherwise starts the timer and
  closes the popup. Esc closes it with nothing started.
- **Remembered choices.** Choices are remembered only when Enter starts a
  timer. A custom number equal to a preset is remembered as that preset.
- **Control popup keys.** Home: ⏎ or `p` pause, `s` stop. Away: ⏎ back,
  `p` pause, `s` stop. Paused: ⏎ resume, `s` stop. Break: ⏎ end break. Esc
  closes with no change; any other key does nothing.
- **Mouse.** Each button and option in the popup is clickable and acts like
  its key.
- **Home closed.** If the home pane closes during focus, the timer keeps
  running, the status shows no nudge, and the control popup shows the Home
  buttons.
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
- AC5: When a focus runs and the focused pane is not home, then the status
  ends with `← <home>`; at home it does not. Proof: test
- AC6: When the user picks Back to `<home>`, then herdr focuses the home
  pane. Proof: screenshot
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
- AC12: When the state file is missing, empty, not valid JSON, or of an
  unknown shape, then the status prints `🍅` and exits 0, and the start
  popup preselects 25 and 5. Proof: test
- AC13: When the status command runs, then it prints one line and exits 0
  within 200 ms, in every state, including while herdr cannot answer.
  Proof: command output
- AC14: When a timer runs, then no second one can start: the key opens the
  control popup, and Enter in a start popup opened earlier starts nothing
  and shows the control popup. Proof: test
- AC15: When `herdr-focus status` runs in every state, then it invokes no
  herdr command other than reading panes and setting the marker, and its
  output holds no BEL or other control character. Proof: test
- AC16: When a user installs on a Mac with no `cargo` on PATH, then J9
  holds. Proof: command output (install) and screenshot (status in the tab
  bar)
- AC17: When the user clicks a popup button or option, then it acts like
  its key. Proof: video
- AC18: When the user ends a break, then the status prints `🍅` at once.
  Proof: test
- AC19: When home's terminal title is empty, then `<home>` is the agent
  kind, or `home` for a pane that is not an agent. Proof: test
- AC20: When the herdr server restarts during a focus, then the marker is
  back on home within 2 s of the server answering. Proof: screenshot

## Contracts

- **Status output.** One line on stdout, exit 0, in every state, exactly
  the strings in `ux.md#states` with the time format above. Idle prints
  `🍅`; dimness is herdr's choice, not the plugin's. If herdr cannot report
  the focused pane within the time limit, the status prints without the
  nudge.
- **State file.** `state.json` in the plugin state dir, this exact shape:

  ```json
  {
    "mode": "idle | focus | paused | break",
    "home_pane": "string or null",
    "home_name": "string or null",
    "ends_at": "RFC 3339 UTC or null",
    "left_secs": "int or null",
    "break_mins": "0 | 2 | 5 | 10",
    "last_focus_mins": "1..180",
    "last_break_mins": "0 | 2 | 5 | 10",
    "flash": "focus_done | break_over | null",
    "flash_ends_at": "RFC 3339 UTC or null"
  }
  ```

  `ends_at` is set in focus and break; `left_secs` in paused. Any other
  shape, missing key or out-of-range value is unknown, and unknown reads as
  idle with first-run choices. Writes are atomic (write then rename), so a
  read never sees half a file. A popup action is never undone by a status
  run at the same moment. A popup acts on the state at the moment of its
  key. One state per herdr server; two herdr sessions never share a timer.
- **Plugin manifest.** `herdr-plugin.toml` with id `focus`, the action
  `focus.open` that opens the popup pane, and a `[[build]]` step that
  installs this version's prebuilt macOS arm64 binary from the GitHub
  release, with no Rust toolchain.
- **Sidebar marker.** Pane metadata token `focus` on home: `🍅` in focus,
  `⏸` in paused, while home exists, including after a server restart.
  Absent in every other state.
- **README paste.** The `[[keys.command]]` binding for `prefix+f`, the
  `tab_bar_right` command entry, and `$focus` in the agent rows, as in
  `mock.html`. It works pasted as is, with no path for the user to edit.
  If the tab bar command does not get the binary on PATH or the plugin
  state dir, the paste calls the binary at herdr's fixed plugin install
  path, written with `~`, and the binary finds the state dir itself.
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

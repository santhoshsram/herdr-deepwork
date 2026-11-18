# herdr-focus v1

## Goal

Build the whole plugin as designed. One key starts a focus timer on the
agent the user is in. The tab bar shows the minutes left, and a break
follows. It adds one key and nothing else: no settings and no locks.

## Users and journeys

The user is the developer in `product.md#who-uses-it`. "Home" is the pane the
user was in when they started the focus.

- **J1 Start.** Given no timer runs and the user is in agent A, when they
  press `prefix+f` and Enter, then a focus starts with A as home. It uses the
  last chosen length, or 25 min on first run. The tab bar shows `🍅 25m`
  and counts down.
- **J2 Custom length.** Given the start popup is open, when the user types
  `40` and Enter, then a 40 min focus starts. The next time the start popup
  opens, it has custom selected and shows 40.
- **J3 Leave home.** Given a focus runs on home A, when the user switches to
  agent B, then the tab bar adds `← <A's name>` after the time. When they
  press `prefix+f` and Enter, herdr switches back to A and the `←` hint goes
  away.
- **J4 Pause.** Given a focus runs, when the user presses `prefix+f` and `p`
  (or Enter while in home), then the countdown stops and the tab bar shows
  `🍅 Paused <time>`. When they resume, it counts down from the same time.
- **J5 Stop.** Given a focus runs or is paused, when the user presses
  `prefix+f` and `s`, then the timer ends with no break, and the tab bar
  shows a plain `🍅`.
- **J6 Break.** Given a focus with a 5 min break runs out, then the break
  starts on its own. For the break's first minute the tab bar shows
  `🍅 Focus done · break 5m`. After that it shows `🍅 Break 4m` and counts
  down. When the break runs out, the tab bar shows `🍅 Break over` for 1
  min, then a plain `🍅`. The next focus never starts on its own.
- **J7 Restart.** Given a timer runs, when the user detaches, or the herdr
  server restarts, and then comes back, then the tab bar shows what the
  timer would show had nothing happened. If a focus still runs, the 🍅 after
  home's sidebar row is back too.
- **J8 End a break early.** Given a break runs, when the user presses
  `prefix+f` and Enter (the popup's only button is "End break"), then the
  break ends and the tab bar shows a plain `🍅` at once. It skips
  `🍅 Break over`, because the user ended the break themselves.
- **J9 Install.** Given herdr 0.9.5 on an Apple silicon Mac or on Linux
  (x86_64 or arm64), with no Rust toolchain: the user runs
  `herdr plugin install santhoshsram/herdr-focus`. Then they copy the config
  from the README and paste it into their herdr `config.toml`. The tab bar
  then shows `🍅`.

## Experience

`foreman/ux.md` lists the places, keys, popups and tab bar text. `mock.html`
at the repo root shows them. This section covers what `ux.md` leaves open.

- **Time format.** Minutes only, never seconds: `25m`, `4m`. From 1 hour up,
  `1h 05m` (a custom focus can be up to 180 min). Minutes round up, so a 25
  min focus shows `25m` in its first minute and `1m` in its last.
- **Which pane counts.** Pressing `prefix+f` opens a popup pane, and herdr
  moves focus to it. So "the pane the user is in" always means the pane
  focused just before `prefix+f`. That pane decides the name in the start
  popup, which pane becomes home, and whether the user is in home or away
  from it. The popup itself never counts as away from home.
- **Agent name.** The name shown in `Focus on <name>`, `← <name>` and
  `Back to <name>` is the pane's terminal title, as herdr reports it in
  `terminal_title_stripped`. It is read once, when the focus starts. If the
  title is empty, the name is the agent's kind, such as `claude`. If the
  pane runs no agent, such as a plain shell, the name is the word `home`.
  A name longer than 24 characters is cut to 23 and ends with `…`. A focus
  can start in any pane.
- **The `←` hint.** It shows only while a focus counts down and the user is
  away from home. It never shows while paused.
- **Break off.** Break `off` means no break. When such a focus runs out, the
  tab bar shows `🍅 Focus done` for 1 min, then a plain `🍅`.
- **No pause during a break.** During a break the popup offers only "End
  break", as `ux.md#control-popup` shows.
- **Start popup keys.** The popup opens with the Focus row selected. `←→`
  move along a row, and `↑↓` switch between the Focus and Break rows.
  Reaching custom with `←→` shows the last custom number the user started,
  or the word `custom` if they never started one. Typing a digit on either
  row selects custom on the Focus row. The first digit typed replaces the
  number shown; each later digit adds to the end, up to 3 digits. Backspace
  deletes the last digit. If the custom number is empty or outside 1 to 180,
  Enter keeps the popup open and shows `1 to 180` under the row. Otherwise
  Enter starts the timer and closes the popup. Esc closes the popup and
  starts nothing.
- **Remembered choices.** The popup remembers the focus and break lengths
  only when Enter starts a timer. If the user types a custom number equal to
  a preset, such as 25, the popup remembers it as that preset.
- **Popup keys while a timer runs.** In home: Enter or `p` pauses, `s`
  stops. Away from home: Enter switches back to home, `p` pauses, `s` stops.
  Paused: Enter resumes, `s` stops. Break: Enter ends the break. Esc closes
  the popup and changes nothing. Any other key does nothing. Every action
  closes the popup.
- **Mouse.** Clicking a button or option in the popup does the same as its
  key.
- **One popup at a time.** Pressing `prefix+f` while the popup is open does
  not open a second one.
- **When something fails.** If herdr does not answer, or the plugin cannot
  save its state, the popup stays open and shows one line under its
  buttons: `herdr is not answering` or `Could not save`. The timer does not
  change.
- **If the home pane is closed.** If the user closes the home pane during a
  focus, or herdr gives it a new id after a restart, the timer keeps
  running. The `←` hint stops showing, and the popup shows the buttons for
  being in home.
- **During a 1-minute message.** Pressing `prefix+f` while the tab bar shows
  `🍅 Focus done` (break off) or `🍅 Break over` opens the start popup.

## Acceptance criteria

- AC1: When no timer runs and no 1-minute message shows, then
  `herdr-focus status` prints `🍅`. Proof: test
- AC2: When the user starts a focus from the start popup, then the tab bar
  shows `🍅 <minutes left>`, and home's sidebar row shows 🍅. Proof:
  screenshot
- AC3: When the start popup opens, then it names the current agent and has
  the last focus and break lengths selected; on first run 25 and 5. Proof:
  test
- AC4: When the user types a custom number, then 1 to 180 starts a focus of
  that length, and any other value keeps the popup open with `1 to 180`.
  Proof: test
- AC5: When a focus counts down and the user is away from home, then the
  tab bar ends with `← <home's name>`. In home, or after the home pane is
  closed, it does not. Proof: test
- AC6: When the user picks `Back to <name>`, then herdr switches to the home
  pane. Proof: screenshot
- AC7: When the user pauses, then the tab bar shows `🍅 Paused <time>`, the
  time stays the same until they resume, and resume counts down from it.
  Home's sidebar row shows ⏸. Proof: test
- AC8: When the user stops, then the tab bar shows `🍅` and the 🍅 after
  home's sidebar row goes away. Proof: test
- AC9: When a focus with a break runs out, then the tab bar follows J6
  exactly, the 🍅 after home's sidebar row goes away, and no focus starts.
  Proof: test
- AC10: When a focus with break off runs out, then the tab bar shows
  `🍅 Focus done` for 1 min, then `🍅`. Proof: test
- AC11: When the plugin starts fresh after any amount of time with a timer
  saved, then the tab bar shows what it would show had it run the whole
  time. This includes times past the focus end, the break end, or the end
  of a 1-minute message. Proof: test
- AC12: When the saved state is missing, empty, not valid JSON, or not in
  the shape under Contracts, then the tab bar shows `🍅`, the command exits
  0, and the start popup has 25 and 5 selected. Proof: test
- AC13: When the status command runs, in any state and even while herdr
  does not answer, then it prints one line and exits 0 within 200 ms.
  Proof: command output
- AC14: When a timer runs, then no second timer can start. `prefix+f` opens
  the popup for the running timer. Enter in a start popup that was opened
  before the timer started does nothing but switch the popup to the running
  timer's buttons. Proof: test
- AC15: When the status command runs, in any state, then the only herdr
  commands it runs read pane information or set and clear the 🍅 or ⏸ after
  home's row. Its output holds no bell or other control character. Proof:
  test
- AC16: When a user installs on an Apple silicon Mac or on Linux with no
  `cargo` installed, then J9 holds. Proof: command output (install) and
  screenshot (tab bar)
- AC17: When the user clicks a popup button or option, then it does the same
  as its key. Proof: video
- AC18: When the user ends a break early, then the tab bar shows `🍅` at
  once. Proof: test
- AC19: When home's terminal title is empty, then its name is the agent's
  kind, or `home` for a pane that runs no agent. A title longer than 24
  characters ends with `…`. Proof: test
- AC20: When the herdr server restarts during a focus, then the first status
  run after herdr answers puts the 🍅 back after home's sidebar row. Proof:
  test
- AC21: When the timer changes while a popup is open (for example the focus
  runs out), then a key in that popup does nothing but redraw it for the
  new state. Proof: test
- AC22: When herdr does not answer or saving fails during a popup action,
  then the popup shows its one-line message and the timer does not change.
  Proof: test

## Contracts

- **What the status command prints.** One line, exit code 0, in every state.
  The text is exactly as in `ux.md#states`, with the time format above. With
  no timer it prints `🍅`. herdr may show it dim; the plugin adds no styling.
  If herdr cannot say which pane is focused within the 200 ms limit, the
  line leaves out the `←` hint.
- **Saved state.** One file, `state.json`, in the folder herdr gives the
  plugin for state. Its exact shape:

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
    "last_custom_mins": "1..180 or null",
    "message": "focus_done | break_over | null",
    "message_ends_at": "RFC 3339 UTC or null"
  }
  ```

  `ends_at` is set during a focus or a break; `left_secs` while paused.
  `message` is the 1-minute message the tab bar shows. If the file has any
  other shape, a missing key, a value out of range, or a null where the mode
  needs a value, the plugin treats it as no timer with first-run choices.
- **Time comes from the saved end times.** Nothing ticks in the background.
  Every state follows from the saved times and the clock. A focus that ends
  at time T starts its break at T, and `Focus done` shows from T to T+1
  min. A break that ends at T shows `Break over` until T+1 min. If the
  clock is already past a state, that state is skipped, never shown late.
- **Saving.** The plugin writes a new file and then renames it over the old
  one, so nothing ever reads half a file. A status run never undoes a popup
  action made at the same moment. A popup key acts on the state as it is
  when the key is pressed.
- **One timer per herdr server.** Two herdr servers (`herdr --session`)
  never share a timer. The status command and the popup of one server always
  use the same state file.
- **Plugin manifest.** `herdr-plugin.toml` has id `focus`, the action
  `focus.open` that opens the popup, and a `[[build]]` step. That step
  downloads this version's prebuilt binary for the user's machine from the
  GitHub release: macOS arm64, Linux x86_64 or Linux arm64. It needs no Rust
  toolchain. On any other machine, or if the download fails, it prints a
  one-line reason, exits non-zero and installs nothing.
- **The 🍅 after home's sidebar row.** The plugin sets it as pane metadata
  token `focus` on the home pane: `🍅` during a focus, `⏸` while paused. It
  stays as long as the home pane exists, including after a server restart.
  In every other state the plugin clears it.
- **Config the user copies from the README.** A plugin cannot add a key
  binding, a tab bar entry or a sidebar token to herdr by itself. So the
  README holds a block of config for the user to copy into their herdr
  `config.toml`, as `mock.html` shows. It sets three things: `prefix+f` to
  open the popup, the tab bar entry that runs `herdr-focus status`, and
  `$focus` in the sidebar's agent rows. The block works pasted unchanged
  into a `config.toml` that does not already set those keys. The README
  names the keys that may already exist (`tab_bar_right`, the agent rows)
  and says to merge the new lines into them. The user never types a path.
  herdr runs the tab bar entry as a plain shell command, outside the plugin,
  so it may not find the `herdr-focus` program or the plugin's state folder.
  If so, the block calls the program by its full install path, written with
  `~`, and the program finds the state folder itself.
- **Commands.** `herdr-focus status` and `herdr-focus panel`. Nothing else is
  public.

## Alternatives considered

- A strip in the sidebar for the time: rejected, herdr 0.9.5 lets plugins
  add no sidebar strip (`tech-stack.md#contested-choices`).
- A background process that ticks the timer: rejected. The saved end time
  is enough and survives restarts.
- A popup when a focus ends: rejected. It would catch keys the user meant
  for an agent (`product-guidelines.md#what-must-never-happen`).
- A config file for lengths: rejected. The popup remembers the last choice.
- Starting from the mouse in the tab bar or a menu: herdr 0.9.5 cannot do
  it; issue #2.

## Non-functional

- herdr runs the status command every second, so it must finish in under
  200 ms and make no network call.
- Prebuilt binaries for macOS on Apple silicon, and Linux on x86_64 and
  arm64.
- The timer is off by less than 1 s per focus, because it reads the saved
  end time instead of counting ticks.

## Reuse and untouched

- Reused from herdr: the popup pane, the tab bar command entry, pane
  metadata tokens, the plugin state folder, and `herdr agent` to find the
  focused pane and to switch back to home.
- Untouched: herdr itself, and the user's `config.toml` (the user pastes the
  README config once; the plugin never edits it).

## Out of scope

- Muting other agents' notifications during a focus (#1).
- Starting from the mouse outside the popup (#2).
- Publishing to the herdr plugin marketplace (#3).
- Intel Mac binaries.
- Counts, history, sounds, notifications, more than one timer, a config
  file.

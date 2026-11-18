# Plan: herdr-focus v1

Spec: `spec.md`. Tasks run in order; a task runs once its deps are done.

## Tasks

- [x] 01 feat(status): status prints the idle tomato | AC: AC1, AC12, AC13 | deps: none [75b6fd8]
- [x] 02 feat(status): status follows the timer timeline | AC: AC7, AC9, AC10, AC11 | deps: 01 [8554240]
- [x] 03 feat(status): status adds the away hint and keeps the sidebar tomato | AC: AC5, AC8, AC13, AC15, AC20 | deps: 02 [af3689a]
- [ ] 04 feat(panel): start popup starts a focus | AC: AC2, AC3, AC4, AC14, AC19 | deps: 02
- [ ] 05 feat(panel): control popup pauses, resumes, stops, goes back, ends a break | AC: AC6, AC7, AC8, AC18, AC21, AC22 | deps: 04
- [ ] 06 feat(panel): mouse clicks act like keys | AC: AC17 | deps: 05
- [ ] 07 feat(plugin): manifest, install step and README config | AC: AC16 | deps: 03, 05
- [ ] 08 ci(release): build and publish the three binaries | AC: AC16 | deps: 07

## Shared contract (all tasks)

Program: one Rust crate, binary `herdr-focus`, dependencies std, `serde`,
`serde_json` only (`tech-stack.md#never-added-without-a-decision-line`).
Raw terminal mode, if needed, goes through `stty`; RFC 3339 is formatted
and parsed by hand.

State folder: `$HERDR_PLUGIN_STATE_DIR` when set and non-empty, else the
folder holding the running executable. Timer file:
`<state folder>/sessions/<session>/timer.json`, where `<session>` is
`$HERDR_SESSION`, or `default` when unset or empty. Shape, verbatim from
spec §Contracts:

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

RFC 3339 values are written as `YYYY-MM-DDTHH:MM:SSZ`.

herdr is called as `$HERDR_BIN_PATH` when set, else `herdr` on PATH. Tests
point `HERDR_BIN_PATH` at a fake script that logs each argv line to a file
and prints canned JSON.

Time format (spec §Experience): minutes only, rounded up. Below 60 min
`<m>m` (`25m`, `4m`, `1m`); from 60 min `<h>h <mm>m` (`1h 05m`).

Status lines (`ux.md#states`, spec §Experience), byte for byte:

```
🍅
🍅 18m
🍅 18m ← <home name>
🍅 Paused 12m
🍅 Focus done · break 5m
🍅 Break 4m
🍅 Break over
🍅 Focus done
```

## 01 feat(status): status prints the idle tomato

### Goal
A Cargo project exists, and `herdr-focus status` prints `🍅` when no timer
runs, whatever the timer file holds or lacks.
Spec: §Contracts "What the status command prints", "Saved state", AC1, AC12, AC13

### Context
- Read first: spec §Contracts, `foreman/code_styleguides/rust.md`
- Mirror: none, first task

### Contract
Command: `herdr-focus status`. Exit 0. stdout exactly `🍅\n`. stderr empty.
Holds when the timer file is missing, empty, not JSON, of another shape, has
a missing key, an out-of-range value, a null the mode needs, or mode
`idle` with `message` null.
Free choice: module layout, lib/bin split, error types.

### Behaviour
- B1 With no timer file, `status` shall print `🍅` and exit 0 -> status_idle_without_file
- B2 When the file is empty or not JSON, `status` shall print `🍅` and exit 0 -> status_idle_on_broken_file
- B3 When the file has an unknown shape (missing key, `last_focus_mins` 0, `mode` `focus` with `ends_at` null), `status` shall print `🍅` -> status_idle_on_unknown_shape
- B4 `status` shall read `$HERDR_PLUGIN_STATE_DIR/sessions/$HERDR_SESSION/timer.json`, and `sessions/default/` when `HERDR_SESSION` is unset -> status_reads_session_file
- B5 `status` shall finish within 200 ms in every case above -> status_is_fast

### Tests
- File: tests/status_idle.rs (test-writer owns; implementer never edits)
- Invariants: each test uses its own temp state folder; stdout is one line

### Files
- Cargo.toml, .gitignore (target/), src/main.rs, src/lib.rs, src/state.rs

### Out of scope
- Any non-idle output (task 02); herdr calls (task 03)

## 02 feat(status): status follows the timer timeline

### Goal
`status` prints the right line for every mode and time, derived from the
saved times and the clock, skipping states the clock already passed.
Spec: §Users and journeys J4, J6, J7, §Experience "Time format", "Break off", §Contracts "Time comes from the saved end times", AC7, AC9, AC10, AC11

### Context
- Read first: spec §Contracts, §Experience; this plan's Shared contract
- Mirror: src/state.rs (task 01)

### Contract
Status lines from the Shared contract. Timeline: a focus ending at T with
`break_mins` b > 0 is a break ending at T + b min; `🍅 Focus done · break
<time>` shows from T to T+1 min, then `🍅 Break <time>`. A break ending at
T shows `🍅 Break over` until T+1 min, then `🍅`. A focus ending at T with
`break_mins` 0 shows `🍅 Focus done` until T+1 min, then `🍅`. Paused shows
`🍅 Paused <left_secs as time>`. The away hint is task 03: here the home
pane always counts as focused.
`status` may rewrite `timer.json` to record a transition, atomically
(write a temp file in the same folder, then rename).
Free choice: whether status persists transitions or recomputes each run.

### Behaviour
- B1 With a focus ending in 18 min 30 s, `status` shall print `🍅 19m` -> status_focus_rounds_up
- B2 With a focus ending in 65 min, `status` shall print `🍅 1h 05m` -> status_focus_hours
- B3 With mode `paused` and `left_secs` 720, `status` shall print `🍅 Paused 12m` -> status_paused
- B4 With a focus that ended 10 s ago and break 5, `status` shall print `🍅 Focus done · break 5m` -> status_focus_done_with_break
- B5 With a focus that ended 90 s ago and break 5, `status` shall print `🍅 Break 4m` -> status_break_counts_down
- B6 With a break that ended 10 s ago, `status` shall print `🍅 Break over` -> status_break_over
- B7 With a break that ended 2 min ago, `status` shall print `🍅` -> status_idle_after_break_over
- B8 With a focus that ended 10 s ago and break 0, `status` shall print `🍅 Focus done` -> status_focus_done_no_break
- B9 With a focus that ended 40 min ago and break 5, `status` shall print `🍅` -> status_skips_passed_states
- B10 Two runs a second apart while paused shall print the same line -> status_paused_does_not_move

### Tests
- File: tests/status_timeline.rs (test-writer owns; implementer never edits)
- Invariants: fixtures set times relative to the real clock; no clock override in production code

### Files
- src/state.rs, src/timeline.rs, src/format.rs, src/main.rs

### Out of scope
- The away hint and sidebar tomato (task 03); popups (tasks 04-06)

## 03 feat(status): status adds the away hint and keeps the sidebar tomato

### Goal
While a focus counts down and the user is in another pane, the tab bar adds
`← <home name>`. The 🍅 or ⏸ after home's sidebar row is set while a focus
runs or is paused, comes back after a herdr restart, and goes otherwise.
Spec: §Experience "The ← hint", "If the home pane is closed", §Contracts "The 🍅 after home's sidebar row", AC5, AC8, AC13, AC15, AC20

### Context
- Read first: spec §Contracts, `tech-stack.md#what-it-is-made-of` (herdr facts)
- Mirror: src/timeline.rs (task 02)

### Contract
Away: `$HERDR_ACTIVE_PANE_ID` is set and differs from `home_pane`. Then the
focus line is `🍅 <time> ← <home_name>`. Paused and other modes never add it.
If herdr reports the home pane does not exist, the line has no hint.
Sidebar tomato, set: `<herdr> pane report-metadata <home_pane> --source
herdr-focus --token focus=🍅` (`focus=⏸` while paused). Cleared:
`<herdr> pane report-metadata <home_pane> --source herdr-focus --clear-token
focus`. `--ttl-ms` may be added.
An idle `timer.json` keeps the last `home_pane`, so `status` knows where to
clear the token. Timing checks take the fastest of 3 runs, because the test
suite runs files in parallel and a single run can be slowed by load.
The only herdr subcommands `status` may run: `pane get`, `pane list`,
`agent list`, `agent get`, `pane report-metadata`. Any herdr call that does
not answer within 150 ms is abandoned; the line prints without the hint.
Output never holds a byte below 0x20 other than the final newline.
Free choice: set every run or only on change; how the home pane's existence
is checked; `--ttl-ms` value.

### Behaviour
- B1 When a focus counts down and `HERDR_ACTIVE_PANE_ID` differs from home, `status` shall print `🍅 <time> ← <home_name>` -> status_away_hint
- B2 When `HERDR_ACTIVE_PANE_ID` equals home or is unset, `status` shall print no hint -> status_home_no_hint
- B3 While paused and away, `status` shall print no hint -> status_paused_no_hint
- B4 During a focus, `status` shall have called `pane report-metadata <home> ... --token focus=🍅`; while paused, `focus=⏸` -> status_sets_sidebar_tomato
- B5 When idle or in a break after a focus, `status` shall clear the `focus` token on the last home -> status_clears_sidebar_tomato
- B6 When the fake herdr sleeps 2 s, `status` shall still print one line and exit 0 within 200 ms -> status_survives_silent_herdr
- B7 `status` shall call no herdr subcommand outside the allowed list, and print no control byte -> status_calls_only_allowed_herdr
- B8 When herdr says the home pane does not exist, `status` shall print no hint -> status_home_closed_no_hint

### Tests
- File: tests/status_herdr.rs (test-writer owns; implementer never edits)
- Invariants: fake herdr via `HERDR_BIN_PATH` logs argv; no real herdr is touched

### Files
- src/herdr.rs, src/status.rs, src/main.rs

### Out of scope
- Popups; the install step

## 04 feat(panel): start popup starts a focus

### Goal
`herdr-focus panel` with no timer running shows the start popup: it names
the current pane, preselects the last choices, takes a typed custom length,
and Enter starts a focus with that pane as home.
Spec: §Users and journeys J1, J2, §Experience "Which pane counts", "Agent name", "Start popup keys", "Remembered choices", AC2, AC3, AC4, AC14, AC19

### Context
- Read first: `ux.md#start-popup`, spec §Experience, `mock.html` popup
- Mirror: src/status.rs (task 03)

### Contract
Current pane: `focused_pane_id` from `$HERDR_PLUGIN_CONTEXT_JSON`, else
`$HERDR_ACTIVE_PANE_ID`. Name: `<herdr> pane get <id>` →
`.result.pane.terminal_title_stripped`; if empty, `.result.pane.agent`; if
no agent, `home`; over 24 characters, the first 23 plus `…`.
Screen text (tests strip ANSI escapes): `Focus on <name>`, a Focus row
`15 25 50 custom min`, a Break row `off 2 5 10 min`, key hints. On bad
custom input: `1 to 180`.
Keys (bytes on stdin): `\x1b[D`/`\x1b[C` left/right, `\x1b[A`/`\x1b[B`
switch rows, digits, `\x7f` Backspace, `\r` Enter, `\x1b` alone Esc.
Enter on a valid choice writes `timer.json`: mode `focus`, `home_pane`,
`home_name`, `ends_at` now + length, `break_mins`, the `last_*` fields;
then sets the sidebar tomato as in task 03; then exits 0. Esc exits 0 and
writes nothing.
If a timer runs when `panel` starts, or when Enter is pressed, it shows the
control popup (task 05) instead and starts nothing.
Free choice: drawing, layout, colours; stdin not a TTY skips raw mode.

### Behaviour
- B1 On first run, the popup shall show `Focus on <name>` with 25 and 5 selected; Enter shall start a 25 min focus with break 5 -> panel_first_run_defaults
- B2 After a run that started 50 and 10, the next popup shall preselect 50 and 10 -> panel_remembers_choices
- B3 Typing `40` then Enter shall start 40 min and set `last_custom_mins` 40 -> panel_custom_length
- B4 Typing `0`, `181` or nothing on custom then Enter shall show `1 to 180` and write nothing -> panel_rejects_bad_custom
- B5 Typing `25` on custom shall be remembered as the preset 25 (`last_custom_mins` unchanged) -> panel_custom_equal_preset
- B6 Esc shall exit 0 and write nothing -> panel_escape_starts_nothing
- B7 The name shall follow the rule: title, then agent kind, then `home`, cut to 23 + `…` -> panel_home_name_rules
- B8 Enter shall set the sidebar tomato on the new home -> panel_start_sets_tomato
- B9 When a timer is already running, Enter shall start nothing -> panel_never_starts_second_timer

### Tests
- File: tests/panel_start.rs (test-writer owns; implementer never edits)
- Invariants: `panel` runs with piped stdin and a fake herdr; temp state folder

### Files
- src/panel.rs, src/panel_start.rs, src/keys.rs, src/main.rs

### Out of scope
- Control popup actions (task 05); mouse (task 06)

## 05 feat(panel): control popup pauses, resumes, stops, goes back, ends a break

### Goal
With a timer running, `herdr-focus panel` shows the control popup for the
current state, and each key does what `ux.md#control-popup` says.
Spec: §Users and journeys J3, J4, J5, J8, §Experience "Popup keys while a timer runs", "When something fails", "If the home pane is closed", AC6, AC7, AC8, AC18, AC21, AC22

### Context
- Read first: `ux.md#control-popup`, spec §Experience
- Mirror: src/panel_start.rs (task 04)

### Contract
Screen text: in home `Pause ⏎` and `Stop s`; away `Back to <home_name> ⏎`,
`Pause p`, `Stop s`; paused `Resume ⏎`, `Stop s`; break `End break ⏎`.
Keys: home Enter or `p` pause, `s` stop; away Enter back, `p` pause, `s`
stop; paused Enter resume, `s` stop; break Enter end break; Esc closes; any
other key does nothing. Every action exits 0 after writing.
Pause: mode `paused`, `left_secs` = whole seconds left, sidebar `focus=⏸`.
Resume: mode `focus`, `ends_at` = now + `left_secs`, sidebar `focus=🍅`.
Stop: mode `idle`, `message` null, sidebar token cleared. End break: mode
`idle`, `message` null. Back: `<herdr> agent focus <home_pane>`; state
unchanged.
Before acting, the popup rereads the state; if it no longer matches what it
shows, the key only redraws it. If herdr does not answer within 1 s, or the
write fails, the popup shows `herdr is not answering` or `Could not save`,
stays open, and the state file is unchanged.
Free choice: drawing, layout.

### Behaviour
- B1 In home, Enter shall pause: `paused`, `left_secs` set, `focus=⏸` reported -> control_pause_at_home
- B2 Away, Enter shall call `agent focus <home>` and change no state -> control_back_to_home
- B3 Away, `p` shall pause -> control_pause_when_away
- B4 Paused, Enter shall resume with `ends_at` = now + `left_secs` -> control_resume
- B5 `s` shall stop: `idle`, token cleared -> control_stop
- B6 In a break, Enter shall end it: `idle`, no `message` -> control_end_break
- B7 When the state changes after the popup opened, a key shall only redraw -> control_stale_popup_redraws
- B8 When the state folder is not writable, a key shall show `Could not save` and keep the popup open -> control_save_failure
- B9 When the fake herdr hangs on `agent focus`, Back shall show `herdr is not answering` -> control_herdr_silent
- B10 When home is closed, the popup shall show the in-home buttons -> control_home_closed

### Tests
- File: tests/panel_control.rs (test-writer owns; implementer never edits)
- Invariants: piped stdin, fake herdr, temp state folder

### Files
- src/panel_control.rs, src/panel.rs

### Out of scope
- Mouse (task 06)

## 06 feat(panel): mouse clicks act like keys

### Goal
Clicking a button or option in either popup does the same as its key.
Spec: §Experience "Mouse", AC17

### Context
- Read first: `tech-stack.md#what-it-is-made-of` (clicks arrive as SGR mouse reports)
- Mirror: src/keys.rs (task 04)

### Contract
The popup turns on SGR mouse reporting (`\x1b[?1000h\x1b[?1006h`) and turns
it off on exit. A left-button release `\x1b[<0;<col>;<row>m` on a button or
option acts as that button's or option's key. Columns and rows are 1-based
within the popup.
Free choice: how hit areas are stored.

### Behaviour
- B1 The popup shall write `\x1b[?1006h` on start and `\x1b[?1006l` on exit -> mouse_reporting_on_and_off
- B2 A click on Break `10` shall select it, and a click on Focus `50` shall start a 50 min focus with break 10, as `mock.html` does -> mouse_selects_and_starts
- B3 A click on `Stop` in the control popup shall stop the timer -> mouse_stop
- B4 A click outside any button shall do nothing -> mouse_click_elsewhere

### Tests
- File: tests/panel_mouse.rs (test-writer owns; implementer never edits)
- Invariants: tests find a button's position from the popup's own output

### Files
- src/keys.rs, src/panel.rs

### Out of scope
- Any new button or layout change beyond what clicks need

## 07 feat(plugin): manifest, install step and README config

### Goal
`herdr plugin install santhoshsram/herdr-focus` installs the right prebuilt
binary into the state folder, and the README holds the config to paste.
Spec: §Users and journeys J9, §Contracts "Plugin manifest", "Config the user copies from the README", AC16

### Context
- Read first: spec §Contracts, the herdr plugin docs facts in `tech-stack.md`
- Mirror: none

### Contract
`herdr-plugin.toml`: `id = "focus"`, `platforms = ["macos", "linux"]`,
action `open` running `$HERDR_BIN_PATH plugin pane open --plugin focus
--entrypoint panel`, pane `panel` with `placement = "popup"` running the
binary's `panel`, one `[[build]]` running `scripts/install.sh`.
`scripts/install.sh` (POSIX sh): picks the asset by `uname -s`/`uname -m`
(`Darwin arm64`, `Linux x86_64`, `Linux aarch64`), downloads it from the
GitHub release matching the manifest `version`, places it in the plugin
folder and copies it to `~/.local/state/herdr/plugins/focus/herdr-focus`.
On any other machine or a failed download: one line on stderr, exit
non-zero, nothing copied. `HERDR_FOCUS_RELEASE_URL` overrides the download
base, for tests.
README: install command, the config block verbatim from `mock.html`'s setup
section with the tab bar command
`~/.local/state/herdr/plugins/focus/herdr-focus status`, and which existing
keys to merge into.
Free choice: README wording beyond those parts.

### Behaviour
- B1 On an unsupported `uname`, install.sh shall exit non-zero with one stderr line and copy nothing -> install_refuses_unknown_machine
- B2 With a local release URL serving a fake binary, install.sh shall place it at `<HOME>/.local/state/herdr/plugins/focus/herdr-focus`, executable -> install_copies_binary
- B3 When the download fails, install.sh shall exit non-zero and copy nothing -> install_failed_download
- B4 The manifest shall parse and name the action, pane and build step above -> manifest_shape
- B5 The README shall hold the tab bar command line verbatim -> readme_has_config_block

### Tests
- File: tests/install.rs (test-writer owns; implementer never edits)
- Invariants: HOME is a temp dir; `uname` is faked through PATH; no network

### Files
- herdr-plugin.toml, scripts/install.sh, README.md

### Out of scope
- Building or publishing binaries (task 08)

## 08 ci(release): build and publish the three binaries

### Goal
Pushing a `v*` tag builds `herdr-focus` for macOS arm64, Linux x86_64 and
Linux arm64 and attaches them to the GitHub release, named as install.sh
expects.
Spec: §Non-functional, §Contracts "Plugin manifest", AC16

### Context
- Read first: scripts/install.sh (task 07)
- Mirror: none

### Contract
`.github/workflows/release.yml`; asset names exactly as install.sh reads
them. A CI workflow runs the "Everything CI runs" command on pull requests.
Free choice: runners, cross-compilation method.

### Behaviour
- none: checked by a real release in prove

### Tests
- File: none
- Invariants: workflows hold no secret value beyond `GITHUB_TOKEN`

### Files
- .github/workflows/release.yml, .github/workflows/ci.yml

### Out of scope
- Publishing to the marketplace (#3)

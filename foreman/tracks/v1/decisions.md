# Decisions: herdr-focus v1

One line per question, tagged user, ux or eng. Status is open, settled or
parked. A settled line has its decision and its evidence: a file section,
`research/<slug>.md`, `spec §<section>`, or the user's words. A decision
starting `review:` is listed under Review notes in the PR. A `|` inside a
cell is written `\|`.

| tag | question | status | decision | evidence |
|---|---|---|---|---|
| user | Who uses this, and what are they trying to do | settled | One developer running many agents in a maximized herdr, staying with one agent until the break | `product.md#who-uses-it` |
| user | What do they do today instead | settled | A browser pomodoro they cannot see in a maximized terminal, then thrashing between agents | `product.md` statement |
| user | The smallest version that passes the one test | settled | The whole designed plugin; it is already one key, one status, one popup | user: "Most of the decisions are already taken"; `spec §Goal` |
| user | What is out | settled | Toast muting (#1), mouse start (#2), marketplace (#3); trackers, sounds, notifications, config file, multiple timers | `product.md#what-it-refuses-to-be`, roadmap issues |
| ux | First thing seen, done, seen after | settled | Dim `🍅` in the tab bar; `prefix+f` opens the start popup; Enter starts; status counts down | `ux.md#navigation`, `ux.md#states` |
| ux | Nearest existing screen or command | settled | Tab bar datetime entry (replaced), herdr popup pane, sidebar agent row | `ux.md#fixed-places` |
| ux | States: empty, loading, error, done | settled | Idle, focus, away, paused, focus done, break, break over; status never blank | `ux.md#states`, `product-guidelines.md#what-must-never-happen` |
| ux | Mockup or exact output | settled | Exact status strings and popups | `ux.md`, `mock.html` |
| eng | Options, the winner, and what flips it | settled | One Rust binary, `status` and `panel` subcommands, `state.json`, no daemon; a failed spike flips its part | `tech-stack.md#contested-choices` |
| eng | Contracts that change: CLI output, API, schema, file format | settled | Status line, `state.json`, manifest, marker token, README paste, two subcommands | `spec §Contracts` |
| eng | What is reused | settled | herdr's popup pane, tab bar command entry, pane metadata tokens, plugin state dir | `tech-stack.md#what-it-is-made-of` |
| eng | What stays untouched | settled | The user's herdr `config.toml`; herdr itself | `product-guidelines.md#what-must-never-happen` |
| ux | Time format for long focuses | settled | Minutes only, `25m`, `1h 05m`; seconds change too often and distract | user: "I basically dont want to show seconds as it changes too often and distracts"; `spec §Experience` |
| ux | What `<agent>` reads as | settled | review: home pane's `terminal_title_stripped`, read at start | `spec §Experience`, `herdr agent list` output |
| ux | Status when a focus with break off ends | settled | review: `🍅 Focus done` for 1 min, then idle; the mock showed `Break over`, a mock bug | `spec §Experience` |
| ux | Invalid custom minutes | settled | review: popup stays open with `1 to 180` under the row | `spec §Experience` |
| ux | Home pane closes during focus | settled | review: timer runs on, no `←` suffix, Home buttons | `spec §Experience` |
| eng | How away is detected and how Back focuses home | settled | review: `herdr agent list` gives the focused pane; `herdr agent focus` focuses home; `HERDR_ACTIVE_PANE_ID` if the spike shows it works | `herdr agent --help` |
| eng | Platforms in v1 | settled | macOS Apple silicon, Linux x86_64 and arm64; no Intel Mac | user: "Add linux builds"; `spec §Non-functional` |
| ux | challenger r1 FIX §Experience: the popup pane reads as the focused pane | settled | Judge from the pane focused before `prefix+f` | `spec §Experience` |
| ux | challenger r1 FIX §Experience: digit typing builds two ways | settled | First digit replaces, later append, 3 max, Backspace deletes | `spec §Experience` |
| ux | challenger r1 FIX §Users and journeys: J2 display and when choices save | settled | Custom shows 40; saved only on Enter; preset-equal saves as preset | `spec §Users and journeys`, `spec §Experience` |
| ux | challenger r1 CONTEST §Experience: End break outcome | settled | review: straight to idle, no `Break over`; the mock does this (`act("endbreak")` clears the flash) | `mock.html`, `spec §Users and journeys` J8 |
| ux | challenger r1 FIX §Experience: control popup keys incomplete | settled | Full key map per state; other keys do nothing | `spec §Experience` |
| ux | challenger r1 FIX §Experience: rounding of time left | settled | Round up whole seconds | `spec §Experience` |
| user | challenger r1 CONTEST §Experience: empty title, and a non-agent pane | settled | review: name falls back to agent kind, then `home`; any pane can be home | `spec §Experience` |
| eng | challenger r1 FIX §Contracts: state file shape not exact | settled | Exact JSON shape | `spec §Contracts` |
| eng | challenger r1 FIX §Contracts: marker after restart | settled | Marker present in focus and paused while home exists, including after restart | `spec §Contracts`, AC20 |
| eng | challenger r1 FIX §Contracts: concurrent writes and stale popups | settled | Popup action never undone; acts on state at keypress | `spec §Contracts`, AC14 |
| eng | challenger r1 FIX §Contracts: herdr unreachable | settled | Status prints without the nudge | `spec §Contracts`, AC13 |
| eng | challenger r1 CONTEST §Contracts: tab bar command may lack PATH and state dir | settled | review: fall back to herdr's fixed plugin install path with `~`; binary finds the state dir itself; spike in implement | `spec §Contracts`, `herdr plugin install --help` |
| eng | challenger r1 FIX §Contracts: one timer per server | settled | One state per herdr server | `spec §Contracts` |
| eng | challenger r1 FIX §Acceptance criteria: AC12 coverage | settled | Unknown shape and first-run choices added | `spec §Acceptance criteria` |
| eng | challenger r1 FIX §Acceptance criteria: AC15 untestable, BEL | settled | Reworded to a testable claim | `spec §Acceptance criteria` |
| eng | challenger r1 FIX §Acceptance criteria: AC16 proof | settled | Command output plus screenshot | `spec §Acceptance criteria` |
| user | challenger r1 FIX §Users and journeys: no Install or End break journey | settled | J8 End break, J9 Install | `spec §Users and journeys` |
| ux | challenger r1 FIX §Users and journeys: vocabulary drift | settled | "nudge" and `<home>` throughout | `spec §Users and journeys`, `spec §Experience` |
| eng | challenger r1 FIX §Contracts: manifest install step | settled | `[[build]]` installs the prebuilt release binary | `spec §Contracts` |
| eng | challenger r2 FIX §Contracts: transitions the clock already passed | settled | Timeline derived from stored times; passed states skipped | `spec §Contracts` Timeline, AC11 |
| ux | challenger r2 FIX §Users and journeys: J6 flash time vs round-up | settled | Superseded by minutes only: `Focus done · break 5m`, then `Break 4m` | `spec §Users and journeys` J6 |
| eng | challenger r2 FIX §Acceptance criteria: AC20 proof | settled | Proof: test | `spec §Acceptance criteria` |
| eng | challenger r2 FIX §Acceptance criteria: status must clear the marker | settled | AC15 allows clearing; AC9 checks it | `spec §Acceptance criteria` |
| eng | challenger r2 FIX §Acceptance criteria: AC1 vs flash states | settled | "idle with no flash showing" | `spec §Acceptance criteria` |
| ux | challenger r2 FIX §Experience: nudge while paused | settled | Paused never shows the nudge | `spec §Experience` |
| ux | challenger r2 FIX §Experience: control popup closes after actions | settled | Every action closes it | `spec §Experience` |
| ux | challenger r2 FIX §Contracts: stale control popup | settled | Key does nothing but redraw | `spec §Contracts`, AC21 |
| user | challenger r2 FIX §Experience: home vs vocabulary | settled | product.md: home is the pane a focus started on; popup `<agent>` uses the same rule | `product.md#vocabulary`, `spec §Experience` |
| eng | challenger r2 FIX §Experience: pane id gone after restart | settled | Home closed applies; spike added | `spec §Experience`, `tech-stack.md#open` |
| eng | challenger r2 FIX §Contracts: unnamed time limit | settled | 200 ms budget | `spec §Contracts` |
| ux | challenger r2 FIX §Contracts: paste collides with existing keys | settled | README names each colliding key and says to replace it | `spec §Contracts` |
| eng | challenger r2 FIX §Contracts: build step off-platform or failed download | settled | Exits non-zero with a reason, installs nothing | `spec §Contracts` |
| eng | challenger r2 FIX §Contracts: null where mode needs a value | settled | Unknown | `spec §Contracts` |
| eng | challenger r2 FIX §Acceptance criteria: resume and home closed untested | settled | AC7 and AC5 extended | `spec §Acceptance criteria` |
| ux | challenger r2 FIX §Experience: popup start row and replace mode | settled | Opens on Focus row; any digit or Backspace ends replace mode | `spec §Experience` |
| ux | challenger r2 FIX ux.md#states: idle dimness | settled | "herdr may dim it" | `ux.md#states` |
| ux | challenger r2 CONTEST §Experience: custom reached by arrows with no prior custom | settled | review: last custom if any, else `custom` with no number | `spec §Experience`, `mock.html` |
| ux | challenger r2 CONTEST §Experience: long home names | settled | review: cut at 24 characters with `…` | `spec §Experience`, AC19 |
| ux | challenger r2 CONTEST §Experience: `prefix+f` with the popup open | settled | review: no second popup | `spec §Experience` |
| eng | challenger r2 CONTEST §Contracts: herdr silent or write fails in the popup | settled | review: one-line message, state unchanged | `spec §Experience`, AC22 |
| eng | challenger r2 CONTEST §Contracts: telling servers apart without the state dir | settled | review: state keyed by herdr server; how the tab bar command learns it is a spike | `spec §Contracts`, `tech-stack.md#open` |
| eng | Where the program and timer files live | settled | All in `~/.local/state/herdr/plugins/focus/`: the program copied at install and update, and `sessions/<name>/timer.json` per herdr session | user: "go ahead"; herdr checks in `tech-stack.md#what-it-is-made-of` |
| eng | Copy the program or link it | settled | Copy: the tab bar then depends on nothing in herdr's install folders; a link would also work | user: "go ahead" |
| eng | How status sets the sidebar tomato | settled | Set or cleared on every run, no `--ttl-ms`; `report-metadata` and `pane get` run side by side, each abandoned after 150 ms | task 03 hand-back |
| eng | When the away hint shows | settled | Only when `pane get <home>` exits 0 with `result.pane`; control characters stripped from the name; empty `HERDR_ACTIVE_PANE_ID` counts as unset | task 03 hand-back |
| eng | Test stand-in for herdr runs slow the first time | settled | Tests run the stand-in once before timing, because macOS checks a new file for about 370 ms on its first run | task 03 dispute |
| eng | Start popup when herdr is slow or the pane is unknown | settled | `pane get` gets 1 s; on failure the name is `home` and the popup still opens; Enter with no current pane shows `herdr is not answering` | task 04 hand-back |
| eng | Keyboard mode in the popup | settled | `stty -icanon -echo` when stdin is a terminal, restored on exit; a lone Esc counts after 50 ms; leading `0` dropped, as in `mock.html` | task 04 hand-back |
| eng | Control popup details | settled | Away needs a known current pane, a non-empty home name and `pane get <home>` within 1 s; stale check compares phase, home, `ends_at`, `left_secs`; a stale popup whose timer ended switches to the start popup; Stop and End break keep the last home | task 05 hand-back |
| eng | Mouse handling | settled | Only a left-button release counts; click areas come from the same lists that draw the rows; a Focus length click starts, a Break click selects, the custom field needs a second click to start, as in `mock.html` | task 06 hand-back |
| eng | Popup size | settled | review: `width = 60`, `height = 20`, picked by the implementer; check against the real popup in prove | task 07 hand-back |
| eng | How the action opens the popup | settled | `sh -c` wrapper so `$HERDR_BIN_PATH` expands; pane runs the state-folder copy through `sh -c`, so the program has one copy | user: "yes make it and test it" |
| eng | Linux binary compatibility | settled | review: built on ubuntu-24.04 runners, so they need glibc 2.39 or newer (no Ubuntu 22.04, Debian 12); a static musl build would widen this | task 08 hand-back |
| eng | Release tag and version | settled | review: the tag must be `v` plus `herdr-plugin.toml` `version`; the workflow does not check it | task 08 hand-back |
| eng | How the sidebar 🍅 goes away | settled | Set with a 3-second expiry every status run during focus or pause; the status command never removes it, so no herdr call runs when no timer runs; the popup's Stop still removes it at once | user: "yes go ahead"; simplify efficiency review |
| eng | Code review findings left open | settled | review: (1) on Linux with `XDG_STATE_HOME` set, herdr may use a different state folder than `~/.local/state`, unverified; (2) a status run can re-set the 🍅 just after Stop removes it, gone within 3 s; (3) the start popup's hint line is about 79 columns in a 60-column popup, checked in prove | code review a7a898f |

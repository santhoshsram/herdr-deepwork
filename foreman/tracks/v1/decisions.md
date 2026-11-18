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
| ux | Time format for long focuses | settled | review: `m:ss`, `mm:ss`, `h:mm:ss` from 1 hour | `spec §Experience` |
| ux | What `<agent>` reads as | settled | review: home pane's `terminal_title_stripped`, read at start | `spec §Experience`, `herdr agent list` output |
| ux | Status when a focus with break off ends | settled | review: `🍅 Focus done` for 1 min, then idle; the mock showed `Break over`, a mock bug | `spec §Experience` |
| ux | Invalid custom minutes | settled | review: popup stays open with `1 to 180` under the row | `spec §Experience` |
| ux | Home pane closes during focus | settled | review: timer runs on, no `←` suffix, Home buttons | `spec §Experience` |
| eng | How away is detected and how Back focuses home | settled | review: `herdr agent list` gives the focused pane; `herdr agent focus` focuses home; `HERDR_ACTIVE_PANE_ID` if the spike shows it works | `herdr agent --help` |
| eng | Platforms in v1 | settled | review: macOS Apple silicon only | `tech-stack.md`, `spec §Non-functional` |

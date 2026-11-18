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
| user | The smallest version that passes the one test | open | | |
| user | What is out | settled | Toast muting (#1), mouse start (#2), marketplace (#3); trackers, sounds, notifications, config file, multiple timers | `product.md#what-it-refuses-to-be`, roadmap issues |
| ux | First thing seen, done, seen after | settled | Dim `🍅` in the tab bar; `prefix+f` opens the start popup; Enter starts; status counts down | `ux.md#navigation`, `ux.md#states` |
| ux | Nearest existing screen or command | settled | Tab bar datetime entry (replaced), herdr popup pane, sidebar agent row | `ux.md#fixed-places` |
| ux | States: empty, loading, error, done | settled | Idle, focus, away, paused, focus done, break, break over; status never blank | `ux.md#states`, `product-guidelines.md#what-must-never-happen` |
| ux | Mockup or exact output | settled | Exact status strings and popups | `ux.md`, `mock.html` |
| eng | Options, the winner, and what flips it | settled | One Rust binary, `status` and `panel` subcommands, `state.json`, no daemon; a failed spike flips its part | `tech-stack.md#contested-choices` |
| eng | Contracts that change: CLI output, API, schema, file format | open | | |
| eng | What is reused | settled | herdr's popup pane, tab bar command entry, pane metadata tokens, plugin state dir | `tech-stack.md#what-it-is-made-of` |
| eng | What stays untouched | settled | The user's herdr `config.toml`; herdr itself | `product-guidelines.md#what-must-never-happen` |

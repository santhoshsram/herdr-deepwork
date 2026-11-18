# herdr-focus: ux

The visual reference is `mock.html` at the repo root (untracked, the user's).
Root `DESIGN.md` does not exist: open.

## Fixed places

- **Status:** the tab bar's right side, in place of the datetime entry.
- **Popup:** a herdr popup pane, only when the user presses `prefix+f`.
- **Marker:** 🍅 (⏸ when paused) after the home agent's sidebar row.

## Navigation

One key, `prefix+f` (`ctrl+space f`). It opens the start popup when idle and
the control popup while a timer runs. In the popup: `←→` change, `↑↓` switch
rows, type a number for custom minutes (1 to 180), Enter starts, esc closes.

## Start popup

"Focus on <agent>". Focus row: 15 · 25 · 50 · custom. Break row: off · 2 · 5
· 10. First run is 25 min focus, 5 min break; later runs repeat the last
choice.

## Control popup

| State | Buttons |
|---|---|
| Home | Pause ⏎ · Stop s |
| Away | Back to <home> ⏎ · Pause p · Stop s |
| Paused | Resume · Stop |
| Break | End break |

## States

| State | Status |
|---|---|
| Idle | `🍅` (dim) |
| Focus | `🍅 18:42` |
| Focus, away | `🍅 18:42 ← <home>` |
| Paused | `🍅 Paused 12:05` |
| Focus ends (1 min) | `🍅 Focus done · break 4:59` |
| Break | `🍅 Break 4:12` |
| Break ends (1 min, then idle) | `🍅 Break over` |

A break starts on its own when focus ends. The next focus never does.

## Install

One paste into herdr's `config.toml`: the `prefix+f` binding, the
`tab_bar_right` command entry, and `$focus` in the agent rows. The README
holds the paste, as `mock.html` shows it.

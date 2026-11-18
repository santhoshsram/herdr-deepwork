# herdr-focus: ux

The visual reference is `mock.html` at the repo root (untracked, the user's).
Root `DESIGN.md` is not needed: herdr renders the status as plain text,
and the popup uses the terminal theme.

## Fixed places

- **Status:** the last entry on the tab bar's right side; the user's other
  entries stay.
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

## Popup look

A 50 x 13 popup with a 2-column margin. Options are one bar of equal cells,
like a segmented control: the choice filled with the accent, the others
grey, the same in both rows; the active row's label is bright. Buttons are
filled pills, the main one accent. A line sits above the key hints.

## Control popup

| State | Buttons |
|---|---|
| Home | Pause ⏎ · Stop s |
| Away | Back to <home> ⏎, then Pause p · Stop s on the next row |
| Paused | Resume · Stop |
| Break | End break |

## States

| State | Status |
|---|---|
| Idle | `🍅` (herdr may dim it) |
| Focus | `🍅 18m` |
| Focus, away | `🍅 18m ← <home>` |
| Paused | `🍅 Paused 12m` |
| Focus ends (1 min) | `🍅 Focus done · break 5m` |
| Break | `🍅 Break 4m` |
| Break ends (1 min, then idle) | `🍅 Break over` |

A break starts on its own when focus ends. The next focus never does.

## Install

A plugin cannot add a key binding, a tab bar entry or a sidebar token to
herdr by itself. So the user pastes one line from the README into their
coding agent, which follows `SETUP.md`: it installs the plugin if missing,
backs up `config.toml`, and merges in `prefix+f`, the tab bar entry (last)
and `$focus` on the sidebar rows, keeping what the user has. The README also
has the block for doing it by hand.

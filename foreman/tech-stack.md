# herdr-focus: tech stack

## What it is made of

- A herdr 0.9.5 TUI plugin: `herdr-plugin.toml` declares the `focus.open`
  action, the popup pane (placement popup) and a `[[build]]` step.
- One Rust binary, `herdr-focus`, with two subcommands:
  - `herdr-focus status`: herdr runs it every second as a `[ui] tab_bar_right`
    command entry, through `/bin/sh -lc`, and shows its last output line.
  - `herdr-focus panel`: the popup, opened by `prefix+f` through a
    `[[keys.command]]` `plugin_action` binding.
- State: one `state.json` in `HERDR_PLUGIN_STATE_DIR`. It holds the end time,
  not a countdown, so the timer survives detach and server restart.
- Sidebar marker: `herdr pane report-metadata <pane> --source focus --token
  focus=🍅`, rendered as `$focus` in the user's
  `[ui.sidebar.agents.rows_by_agent]` rows.
- Release: a prebuilt macOS binary, so install needs no Rust toolchain.

## Contested choices

- **Rust, not shell or Node.** The status command runs every second; a
  single native binary starts fast and has no runtime to install.
- **Tab bar, not a sidebar strip.** herdr 0.9.5 lets a plugin add a tab bar
  entry but not a sidebar strip, and the tab bar shows in a maximized window.
- **Choices in the popup, not a config file.** The popup remembers the last
  focus and break, so there is nothing to configure.
- **TUI plugin, not a herdr desktop app mod.** The user runs the TUI.
- **No daemon.** The status command derives everything from `state.json`
  and the clock, so no process has to stay alive.
- **No mouse entry point.** herdr 0.9.5 has hardcoded right-click menus, an
  unused plugin `contexts` field and a tab bar that takes no clicks. The
  mouse works only inside the popup.

## Never added without a decision line

- A dependency (crate) beyond the standard library and serde/serde_json.
- A background process or daemon.
- A second state file.
- Any write to the user's herdr `config.toml`.

## Open

- Spike: the mouse works inside the popup pane.
- Spike: `HERDR_ACTIVE_PANE_ID` reaches a tab bar command, for the away nudge.
- Spike: the cost of a 1 s `/bin/sh -lc` login shell.
- Spike: `$focus` renders in the sidebar row next to `state_icon`.
- Spike: pane ids survive a herdr server restart.
- Spike: a tab bar command gets the binary on PATH, the plugin state dir,
  and which herdr server it belongs to.
- Build commands in `workflow.md` are `none yet` until `Cargo.toml` exists.

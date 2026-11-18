# herdr-focus: tech stack

## What it is made of

- A herdr 0.9.5 TUI plugin: `herdr-plugin.toml` declares the `focus.open`
  action, the popup pane (placement popup) and a `[[build]]` step.
- One Rust binary, `herdr-focus`, with two subcommands:
  - `herdr-focus status`: herdr runs it every second as a `[ui] tab_bar_right`
    command entry, through `/bin/sh -lc`, and shows its last output line.
  - `herdr-focus panel`: the popup, opened by `prefix+f` through a
    `[[keys.command]]` `plugin_action` binding.
- Files: everything lives in the plugin's state folder,
  `~/.local/state/herdr/plugins/focus/`. The `[[build]]` step copies the
  program there, because herdr installs each version into a new folder and
  the tab bar line needs a path that never changes. Each herdr session has
  its own `sessions/<name>/timer.json`, because the state folder is shared
  by all sessions. The file holds the end time, not a countdown, so the
  timer survives detach and server restart.
- herdr facts the design rests on, checked on herdr 0.9.5 on 2026-11-18:
  - The tab bar command gets `HERDR_ACTIVE_PANE_ID`, `HERDR_SESSION`,
    `HERDR_SOCKET_PATH` and `HERDR_BIN_PATH`, but no plugin state folder.
  - The popup gets the pane focused before it in `HERDR_PLUGIN_CONTEXT_JSON`
    (`focused_pane_id`), and mouse clicks.
  - `/bin/sh -lc` costs about 6 ms and `herdr agent list` about 5 ms.
  - `$focus` shows in the sidebar row: `● v1 · 🍅`.
  - A pane keeps its id across a server restart; its metadata tokens do not.
- Sidebar marker: `herdr pane report-metadata <pane> --source focus --token
  focus=🍅`, rendered as `$focus` in the user's
  `[ui.sidebar.agents.rows_by_agent]` rows.
- Release: prebuilt binaries for macOS (Apple silicon) and Linux (x86_64,
  arm64), so install needs no Rust toolchain.

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
- A file outside the plugin's state folder.
- Any write to the user's herdr `config.toml`.

## Open

none

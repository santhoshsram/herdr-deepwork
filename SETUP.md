# Set up herdr-focus

You are a coding agent setting up herdr-focus for the user. herdr-focus is a
focus timer plugin for herdr. Setup is two steps: install the plugin, then
add its config to herdr's `config.toml`. Run each command yourself and read
its output. Ask the user only where this file says to.

## 1. Install the plugin

1. Run `herdr --version`. herdr must be 0.9.5 or newer. If it is older, tell
   the user to update herdr and stop.
2. Run `herdr plugin list`. If it lists `focus`, the plugin is installed:
   skip to "2. Add the config".
3. Ask the user to confirm the install, because herdr runs the plugin's
   install step on their machine. On yes, run
   `herdr plugin install --yes santhoshsram/herdr-focus`. It downloads a
   ready-made program for macOS on Apple silicon or Linux on x86_64 or arm64.
   On any other machine it fails: tell the user and stop.
4. Check that `~/.local/state/herdr/plugins/focus/herdr-focus` exists.

## 2. Add the config

The config file is `$HERDR_CONFIG_PATH` if that is set, else
`~/.config/herdr/config.toml`. If it is a symlink, edit the file it points
to. If it does not exist, create it.

Copy it to `config.toml.bak-<YYYYmmdd-HHMMSS>` next to it before changing
anything. Change only what is listed here. Keep everything else, including
comments, exactly as it is. A run on an already set-up config changes
nothing.

### The key

Add this table, unless a `[[keys.command]]` with `command = "focus.open"`
exists:

```toml
[[keys.command]]
key = "prefix+f"
type = "plugin_action"
command = "focus.open"
description = "focus timer"
```

If `prefix+f` is already bound to something else, ask the user which key to
use instead.

### The tab bar

The timer is this entry in `tab_bar_right`, under `[ui]`:

```toml
{ type = "command", command = "~/.local/state/herdr/plugins/focus/herdr-focus status", interval_seconds = 1, timeout_seconds = 1 }
```

- Skip this if `tab_bar_right` already has an entry whose command runs
  `herdr-focus status`.
- If `tab_bar_right` exists, add the entry as its last item and keep the
  other items.
- If it does not exist, add `tab_bar_right = [ <the entry> ]` to the `[ui]`
  table, creating `[ui]` if needed.
- Each inline table must stay on one line: TOML does not allow line breaks
  inside `{ }`.
- If the list now has other items and `tab_bar_right_separator` is not set,
  add `tab_bar_right_separator = " | "` to `[ui]`.

### The sidebar

`$focus` shows a 🍅 on the sidebar row of the agent the user focuses on. Add
`"$focus"` as the last item of the first row of:

- `rows` under `[ui.sidebar.agents]`
- every agent's rows under `[ui.sidebar.agents.rows_by_agent]`

If `rows` is not set, copy herdr's default from `herdr --default-config`
first, then add `"$focus"`. Skip any row that already has it.

### Check and apply

1. Run `herdr config check`. If it does not print `config: ok`, restore the
   backup, show the user the error and stop.
2. Run `herdr server reload-config`.

## Done

Tell the user, in a few lines:

- what you installed and what you changed in the config, and where the
  backup is;
- to press their herdr prefix, then `f` (for example `ctrl+space`, then
  `f`), to start a focus.

## Undo

Restore the backup, then run `herdr plugin uninstall focus`.

# herdr-focus

A focus timer for the herdr tab bar. Start a focus from a popup and the tab
bar shows the time left, with a 🍅 next to the agent you were working with;
when the focus ends, a break starts on its own.

## Install

```
herdr plugin install santhoshsram/herdr-focus
```

This downloads a ready-made program for your machine (macOS on Apple silicon,
or Linux on x86_64 or arm64). You do not need Rust.

## Set up herdr

Paste this into `~/.config/herdr/config.toml`:

```toml
# key
[[keys.command]]
key = "prefix+f"
type = "plugin_action"
command = "focus.open"
description = "focus timer"

# tab bar: replaces the datetime entry
[ui]
tab_bar_right = [
  { type = "zoom" },
  { type = "command", command = "~/.local/state/herdr/plugins/focus/herdr-focus status",
    interval_seconds = 1, timeout_seconds = 1 },
]

# sidebar: "$focus" shows the 🍅 after the agent you focus on.
# Add the same rows for other agents you use, such as codex or pi.
[ui.sidebar.agents.rows_by_agent]
claude = [
  ["state_icon", "workspace", "tab", "$focus"],
  ["terminal_title_stripped"],
]
```

If your `config.toml` already has `tab_bar_right` or the agent rows, do not
paste a second copy. Add the new lines to the ones you have: the `command`
entry goes into your `tab_bar_right` list, and `"$focus"` goes into the row
you want the 🍅 on.

## Use it

Press your herdr prefix, then `f` (for example `ctrl+space`, then `f`). A popup opens where you pick how long to focus and how long to
break. It remembers your last choice, so Enter repeats it. You can also
click the buttons and options with the mouse.

While you focus, the tab bar shows the time left. Press your prefix, then
`f` again to pause, resume or stop.

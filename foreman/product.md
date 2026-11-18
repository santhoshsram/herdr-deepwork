# herdr-focus: product

herdr-focus is for a herdr user running several coding agents at once who
keeps switching sessions while an agent works, and loses context. They start
a pomodoro on one agent session and see the time left in herdr's tab bar,
even in a maximized window. It nudges them to stay in that session until the
break, and never locks them out or hides anything.

## Who uses it

One developer who lives in tmux-style terminals and runs herdr maximized on
macOS, with many coding agents across several workspaces and herdr
notifications turned off. They reach for herdr-focus at three moments:
starting a stretch of work on one agent, feeling the pull to check another
agent while this one works, and the break, when checking the others is the
point.

Built for that developer and shared as a herdr plugin as is.

## The one test

Does this help the user stay with one agent longer, without adding a step, a
setting or a lock?

## What it refuses to be

- A blocker. It never locks, hides the sidebar, blocks switching, or opens
  anything the user did not ask for. A popup that appears on its own can
  swallow keystrokes meant for an agent.
- A settings app. No config file; the popup holds the only choices and
  remembers them.
- A tracker. No counts, history, streaks or stats.
- Several timers. One timer per herdr server; two would split focus.
- A notifier. The tab bar is the only signal.
- A herdr desktop app mod. The TUI only.

## Vocabulary

One word per concept. No synonyms.

- **agent**: one herdr agent pane, as the agents sidebar lists it.
- **home**: the pane a focus started on, usually an agent.
- **away**: the focused pane is not home.
- **focus**: one timed stretch of work on home.
- **break**: the timed stretch after a focus, for checking other agents.
- **timer**: the one running focus or break. At most one exists.
- **idle**, **paused**: the states with no timer counting down.
- **popup**: what `ctrl+space f` opens. The only place to start, pause or
  stop, and the only mouse surface.
- **status**: the text the plugin puts at the right of herdr's tab bar.
  Always starts with 🍅.
- **marker**: the 🍅 or ⏸ after home's row in the agents sidebar.
- **nudge**: the `← <home>` the status adds when away.
- **session**: a herdr session (`herdr --session`), never an agent.

## Open

- Muting other agents' herdr toasts during a focus. Deferred past v1; the
  user keeps herdr notifications off.
- Starting from the mouse. herdr 0.9.5 has no clickable tab bar entry or
  plugin menu item; needs an upstream feature request.
- Publishing to the herdr plugin marketplace, which needs a public repo.

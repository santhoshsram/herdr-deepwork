# herdr-deepwork: guidelines

## Voice

- Status and popup text is plain words a glance can read: `🍅 Paused 12m`,
  `🍅 Break over`. The countdown is the only number.
- No symbol the user has to decode. A symbol earns its place only when it
  reads without a legend: 🍅, ⏸, `←`. Rejected in the mock: ☕ (renders as
  an unreadable glyph), `⌃␣` (shorthand for a key), `0-9` (reads as one
  digit).
- Every status starts with 🍅, so the tab bar entry always reads as this
  plugin.
- Calm. No exclamation marks, praise or motivation.
- Banned in UI text: "session" for an agent; "pomodoro" (the word is
  "focus"); "productivity", "streak", "goal".

## What the user never has to do

- Learn a second key. `ctrl+space f` does the obvious thing in every state.
- Open or edit a plugin config file. Choices live in the popup, which
  remembers the last ones, so Enter repeats them.
- Say which agent is home. It is the one they are in when they start, and
  the popup names it.
- Do time math. The status shows the minutes left. No seconds: a number
  that changes every second pulls the eye.
- Edit herdr's `config.toml` more than once. At install they copy one block
  of config from the README into it.

## What must never happen

- A popup, pane or window opens without the user pressing the key. It
  would swallow keystrokes meant for an agent.
- The status goes blank while a timer runs. herdr clears a tab bar command
  entry on empty output, failure or timeout, so the status command always
  prints a line within its timeout.
- The timer loses time or state on detach, client restart or herdr server
  restart.
- Two timers exist at once.
- The plugin blocks, delays or undoes a switch between agents, or hides any
  herdr UI.
- The plugin edits the user's herdr `config.toml`.
- A sound or a system notification.

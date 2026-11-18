# Proof: herdr-focus v1

Built at `b96c6fd`; the live checks first ran at `4eef0ba`, and the lines P1
and P2 touched (AC3, AC16, AC17) were re-run at `b96c6fd`. `cargo test` re-ran
at `b96c6fd`: 81 passed, 0 failed (`proof/cargo-test.txt`). Commands that touch `$HOME` ran with HOME set to a
fresh temp dir (`/tmp/hfp`, short because herdr's socket path must fit in
104 bytes). Real-herdr checks ran in a separate herdr server under that
home, nested in a tab of the user's herdr, with the README config block
pasted in and the prefix set to `ctrl+b`. "Screen" evidence is herdr's own
text capture of that server (`herdr pane read`).
Media live under `proof/`, which git ignores: each line says what its file
shows, so the line stands without it.

| AC | Proof | Evidence | Result |
|---|---|---|---|
| AC1 | test | `cargo test`: status_idle 5 passed; real herdr tab bar showed `🍅` with no timer | pass |
| AC2 | screenshot | `proof/ac2-focus-started.txt`: tab bar `🍅 25m`, sidebar `● v1 · 🍅` on the home row | pass |
| AC3 | test | `cargo test` panel_start 9 passed, panel_width 2 passed; `proof/ac3-start-popup.txt` shows `Focus on alpha-agent`; `proof/ac3-start-popup-fixed.txt` shows the hints on two whole lines and the last choice (custom 1, break 2) preselected | pass |
| AC4 | test | `cargo test`: panel_rejects_bad_custom, panel_custom_length passed; live `1` then Enter started a 1 min focus | pass |
| AC5 | test | `cargo test` status_herdr 8 passed; `proof/ac5-away-hint.txt`: tab bar `🍅 25m ← alpha-agent` while on beta | pass |
| AC6 | screenshot | `proof/ac6-control-away.txt`: `Back to alpha-agent ⏎`; after Enter herdr reported pane w1:p1 focused and the hint left (`proof/ac6-back-home.txt`) | pass |
| AC7 | test | `cargo test` control_pause_at_home, control_resume passed; `proof/ac7-paused.txt`: `🍅 Paused 25m`, sidebar `⏸` | pass |
| AC8 | test | `cargo test` control_stop passed; `proof/ac8-stopped.txt`: tab bar `🍅`, sidebar mark gone at once | pass |
| AC9 | test | `proof/ac9-timeline.txt`, live 1 min focus with 2 min break: `Focus done · break 2m` at 07:21:21, sidebar 🍅 gone at 07:21:23, `Break 1m` at 07:22:21, `Break over` at 07:23:22, `🍅` at 07:24:20, no focus started | pass |
| AC10 | test | `cargo test` status_focus_done_no_break passed | pass |
| AC11 | test | `cargo test` status_skips_passed_states, status_break_over passed | pass |
| AC12 | test | `cargo test` status_idle_on_broken_file, status_idle_on_unknown_shape passed | pass |
| AC13 | command output | `proof/ac13-timing.txt`: 30 runs via `/bin/sh -lc` against live herdr, median 12.7 ms, max 14.3 ms, exit 0, one line | pass |
| AC14 | test | `cargo test` panel_never_starts_second_timer passed | pass |
| AC15 | test | `cargo test` status_calls_only_allowed_herdr, status_idle_runs_no_herdr passed; `proof/ac13-timing.txt` shows 0 control bytes | pass |
| AC16 | command output | `proof/ac16-install.txt`: install step with no cargo on PATH exit 0, binary in `bin/` and the state folder; the README block, pasted unchanged, passes `herdr config check` (`config: ok`) after P1; tab bar showed `🍅` | pass |
| AC17 | video | clicks sent through herdr's mouse input: `Resume` resumed the timer (`proof/ac17-after-click-resume.json`), `Stop` at the new button spacing stopped it (`proof/ac17-after-click-stop.json`); a hand click is in Manual | manual |
| AC18 | test | `cargo test` control_end_break passed | pass |
| AC19 | test | `cargo test` panel_home_name_rules passed; `proof/ac19-name-fallback.txt`: empty title after restart showed `Focus on claude` | pass |
| AC20 | test | `proof/ac20-after-restart.txt`: after `herdr server stop` and restart, tab bar `🍅 Paused 25m`, sidebar `⏸` back | pass |
| AC21 | test | `cargo test` control_stale_popup_redraws passed | pass |
| AC22 | test | `cargo test` control_save_failure, control_herdr_silent passed | pass |

## Manual

For the user to run; prove could not.

- AC17: open the popup with `ctrl+space f` and click a focus length, then
  open it again and click Stop. Holds when each click does what its key
  does. Not run here because this session cannot move a real mouse; the
  click was sent as herdr mouse input instead.
- AC16, from GitHub (after the first release): `herdr plugin install santhoshsram/herdr-focus` on
  a Mac with no cargo. Not run here because no GitHub release exists yet.

## Adversarial

- README config block pasted into a fresh `config.toml` (AC16) -> broke:
  herdr rejects the split inline table; filed as task P1, fixed in
  `75a5b9b`, re-run: held
- Start popup at its manifest width of 60 (AC3) -> broke: the hint line
  wraps mid-word; filed as task P2, fixed in `6363011`, re-run: held
- Away control row with a 24-character name (AC6, AC17) -> broke with P2's
  test (57 columns); fixed in `6363011`, Stop click at the new spacing: held
- Herdr server restart while paused (AC20, J7) -> held
- Home title empty after restart (AC19) -> held: falls back to `claude`
- Status run with live herdr 30 times through a login shell (AC13) -> held

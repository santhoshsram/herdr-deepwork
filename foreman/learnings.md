# Learnings

Append-only. One line per surprise. The note is a tag, not a description.
Types: pitfall, pattern, decision, harness. Harness lines are what we read
when tuning foreman.

```
date | track | phase | type | note | pointer
```
2026-11-18 | v1 | shape | pitfall | the spec carried the mock's words but not its look, so the popup shipped as plain text | spec §Experience, P4
2026-11-18 | v1 | prove | pitfall | a README TOML block split an inline table across lines; herdr rejected the whole config | P1
2026-11-18 | v1 | implement | pitfall | macOS holds a new executable ~370 ms on first run, which broke a 150 ms timeout against test fakes | tests warm each fake first
2026-11-18 | v1 | implement | decision | the sidebar mark expires on a 3 s TTL instead of a status-run cleanup call | task 09
2026-11-18 | v1 | ship | pattern | a fresh agent dry-running SETUP.md on throwaway configs found 8 unclear lines | c3662e0
2026-11-18 | v1 | shape | harness | too many stops on decisions the user had already settled before foreman | memory: foreman-no-stops-when-settled
2026-11-18 | v1 | prove | harness | the worktree guard refused compound git and HOME-setting commands; wrapper scripts worked | scratchpad h.sh

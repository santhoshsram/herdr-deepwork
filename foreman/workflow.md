# Workflow

## Commands
| Do | Run |
|---|---|
| Build | `cargo build` |
| Test | `cargo test` |
| Test files | `cargo test` |
| Lint | `cargo fmt --check && cargo clippy --all-targets -- -D warnings` |
| Run | `cargo run -- status` |
| Everything CI runs | `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` |

Read by `gate.sh`: a Run cell is exactly one code span, or the row is
skipped. A `|` inside a command is written `\|`.

## Test paths
`tests/*`

cargo test runs every test: Rust cannot run one integration test file by
path, so red and green run the whole suite.

Globs on repo-relative paths; a * also matches /. The test-writer may write only
these, the implementer never.

## Principles
- The user's experience decides ties. The one test is in product.md.
- The tech stack is deliberate. A new dependency gets a decision line before code.
- Non-interactive, CI-aware commands. `CI=true` for watch-mode tools.

## Quality gates (per task)
- Tests red before the implementer, green after. Failure cases covered.
- Lint clean. Style guides in code_styleguides/ followed.
- User-visible change has its docs and spec change in the same task.
- No secret value in output, logs, files or tests.

## Branches, commits, PRs
- Branch per track, `<type>/<id>`. Never commit to main.
- Conventional Commits. Scopes: status, panel, state, plugin, foreman.
- Stage explicit paths.
- PR title is the squash subject. Body: summary, why, changes, review notes,
  tested, not in this PR, breaking change, Closes #N.

## Done
Merged to main and working in the user's own herdr. Released: a GitHub
release with the prebuilt macOS binary. Track archived.

## Roadmap

GitHub issues labelled `roadmap`, one open issue per future track.

| Do | Run |
|---|---|
| List | `gh issue list --label roadmap --state open --limit 1000 --search "sort:created-asc"` |
| Add | `gh issue create --label roadmap --title <title> --body <body>` |

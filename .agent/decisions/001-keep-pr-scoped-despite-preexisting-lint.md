# Decision 001: keep the PR scoped; pre-existing clippy/rustfmt failures stay out

- run: 20261007-020332-deterministic-cli-output-sort-account-ro
- date: 2026-10-07
- status: decided autonomously

## Context

The acceptance contract asked for `cargo clippy -D warnings` and `cargo fmt --check` to pass.
Both fail on the base branch already: 5 clippy findings in `src/engine.rs` and `src/money.rs`,
rustfmt diffs in `src/engine.rs`, `src/money.rs`, `src/bin/generate_load.rs`. None of those
files are in the task's scope, and the contract also required the engine/types/money files
untouched. The two criteria contradict on this base.

## Options

1. **Scope the lint criterion to the change** — prove the branch adds no clippy findings
   (count on base vs branch) and that the changed files are rustfmt-clean. Keep the PR small.
2. Fix the 5 clippy sites and format the 3 files in this branch — mechanical, but widens a
   "deterministic output" PR into a lint sweep of the ledger core.
3. Separate chore branch/PR for the lint sweep.

## Choice

Option 1 now; option 3 is the right follow-up and is noted in the PR. Inferred from the task
statement (scope is output ordering + test + README) and repo evidence (failures predate the
branch). Evidence: lint-delta #14 (base=branch finding count), fmt-changed #15.

## Consequences

Repo-wide `clippy -D warnings` stays red until the chore PR lands; CI does not exist in this
repo yet, so nothing is blocked. Reviewer should treat `lint`/`fmt` failures #4–#7 as
pre-existing, not as findings against this change.

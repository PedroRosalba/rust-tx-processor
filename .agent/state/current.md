---
run_id: 20261007-020332-deterministic-cli-output-sort-account-ro
task: "Deterministic CLI output: sort account rows by client id; integration test running the binary on transactions.csv vs accounts.csv; README row-order note"
repo: rust-tx-processor
branch: feature/deterministic-cli-output-sort-account-ro
base_branch: master
risk: MEDIUM
path_floor: TRIVIAL
human_required: false
human_level: visibility
status: done
iteration: 2
started_at: 2026-10-07T02:03:32Z
updated_at: 2026-10-07T02:09:52Z
driver: ralph-loop
acceptance: .agent/acceptance/20261007-020332-deterministic-cli-output-sort-account-ro.yaml
evidence: .agent/evidence/20261007-020332-deterministic-cli-output-sort-account-ro.jsonl
pr: https://github.com/PedroRosalba/rust-tx-processor/pull/1
next_action: none — run done
---

# Rosalbito run 20261007-020332-deterministic-cli-output-sort-account-ro

> Resume protocol: a fresh agent reads this file first, runs `caps-check.sh`, and continues
> from `next_action`. Keep every section truthful. Never promote a guess to a fact.

## Task

Deterministic CLI output: sort account rows by client id; integration test running the binary on transactions.csv vs accounts.csv; README row-order note

## Classification

- risk: MEDIUM (floor TRIVIAL) — why: changes the observable output contract of a financial ledger CLI; 3 files incl. a new integration test; reversible; no DB/concurrency
- human: level=visibility required=false — reason: task fully specified; expected output fixture already sorted by client id

## Plan

1. `src/main.rs` — in `write_accounts`, collect `ledger.values()` into a `Vec<&Account>`, sort by `client`, then serialize. Proves: `cargo test --test cli`, determinism loop (acceptance #2).
2. `tests/cli.rs` — new integration test: locate the binary via `env!("CARGO_BIN_EXE_rust-tx-processor")`, run it on `transactions.csv` with the manifest dir as cwd, compare stdout bytes to `accounts.csv` (normalize trailing newline only if needed; prefer exact). Proves: `cargo test --test cli`.
3. `README.md` — replace the "order of rows is undefined — hash map iteration" note with "rows are sorted by client id". Proves: acceptance #6 grep.
4. Verify all: build, test, clippy, fmt via verify.sh. Then fresh-context correctness review.

## Verified

- build — evidence #1 (exit 0)
- test — evidence #10 (exit 0)
- cli — evidence #11 (exit 0)
- determinism — evidence #12 (exit 0)
- readme — evidence #13 (exit 0)
- lint-delta (no new clippy findings vs master) — evidence #14 (exit 0)
- fmt-changed (rustfmt --check on changed files) — evidence #15 (exit 0)

## Failed

- lint (repo-wide clippy -D warnings) — evidence #4, #5 — pre-existing in engine.rs/money.rs, see decisions/001
- fmt (repo-wide cargo fmt --check) — evidence #6, #7 — pre-existing in engine.rs/money.rs/generate_load.rs, see decisions/001

## Reviews

- round 1: correctness (fresh context) — ACCEPT — no findings; edge cases #22, determinism via cmp #23, engine untouched #21, clippy by file #24 (0 in changed files). Notes: header-only input emits zero bytes (pre-existing); CSV fixture needs LF → .gitattributes added.

## Open decisions

<!-- only genuine decisions for Pedro; otherwise write a decision record -->

## Log

- 2026-10-07T02:03:32Z run created
- 2026-10-07T02:03:33Z classified MEDIUM/visibility; acceptance contract and plan written; loop armed
- 2026-10-07T02:06:37Z gate re-derived: test #10, cli #11, determinism #12, readme #13, lint-delta #14, fmt-changed #15 all pass; decision 001 written (pre-existing lint out of scope)
- 2026-10-07T02:09:17Z review round 1 ACCEPT; .gitattributes added for LF fixtures (#25)
- 2026-10-07T02:09:52Z run done — PR https://github.com/PedroRosalba/rust-tx-processor/pull/1

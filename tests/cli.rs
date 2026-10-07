//! Integration tests for the CLI binary: output must be deterministic and
//! match the committed `accounts.csv` fixture exactly.

use std::path::Path;
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_rust-tx-processor");
const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

fn run_cli() -> Output {
    Command::new(BIN)
        .arg("transactions.csv")
        .current_dir(MANIFEST_DIR)
        .output()
        .expect("failed to spawn rust-tx-processor binary")
}

#[test]
fn output_matches_accounts_csv_exactly() {
    let output = run_cli();
    assert!(
        output.status.success(),
        "binary exited with {:?}; stderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let expected = std::fs::read(Path::new(MANIFEST_DIR).join("accounts.csv"))
        .expect("failed to read accounts.csv");

    assert_eq!(
        output.stdout,
        expected,
        "stdout differs from accounts.csv\n--- stdout ---\n{}\n--- accounts.csv ---\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&expected)
    );

    // The fixture rows must be in ascending client-id order, so matching it
    // proves the CLI sorts its output.
    let stdout = String::from_utf8(output.stdout).expect("stdout is not UTF-8");
    let ids: Vec<u16> = stdout
        .lines()
        .skip(1) // header
        .filter(|line| !line.is_empty())
        .map(|line| {
            line.split(',')
                .next()
                .and_then(|id| id.parse().ok())
                .unwrap_or_else(|| panic!("bad client id in row: {line}"))
        })
        .collect();
    assert!(!ids.is_empty(), "no account rows in output");
    assert!(
        ids.windows(2).all(|pair| pair[0] < pair[1]),
        "client ids are not strictly ascending: {ids:?}"
    );
}

#[test]
fn repeated_runs_produce_identical_bytes() {
    let first = run_cli();
    let second = run_cli();
    assert!(first.status.success() && second.status.success());
    assert_eq!(
        first.stdout, second.stdout,
        "two consecutive runs produced different output"
    );
}

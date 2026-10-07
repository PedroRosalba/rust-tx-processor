use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io;
use std::process;

use csv::ReaderBuilder;
use rust_tx_processor::{Account, PaymentEngine, Transaction};

fn main() {
    if let Err(err) = run() {
        eprintln!("fatal error: {err}");
        process::exit(1);
    }
}

#[derive(Debug)]
pub enum CliError {
    MissingArgument,
    Io(std::io::Error),
    Csv(csv::Error),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingArgument => {
                write!(f, "missing argument: expected path to transactions.csv")
            }
            Self::Io(err) => write!(f, "io error: {err}"),
            Self::Csv(err) => write!(f, "csv error: {err}"),
        }
    }
}

impl Error for CliError {}

impl From<std::io::Error> for CliError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<csv::Error> for CliError {
    fn from(err: csv::Error) -> Self {
        Self::Csv(err)
    }
}

fn run() -> Result<(), CliError> {
    let input_path = env::args().nth(1).ok_or(CliError::MissingArgument)?;

    let mut engine = PaymentEngine::new();

    let input = File::open(&input_path)?;
    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(input);

    for result in reader.deserialize::<Transaction>() {
        match result {
            Ok(tx) => {
                if let Err(err) = engine.compute_tx(tx) {
                    eprintln!("transaction rejected: {err}");
                }
            }
            Err(err) => {
                eprintln!("invalid row: {err}");
            }
        }
    }

    write_accounts(&engine.client_ledger)?;
    Ok(())
}

fn write_accounts(ledger: &HashMap<u16, Account>) -> Result<(), CliError> {
    let mut writer = csv::WriterBuilder::new()
        .has_headers(true)
        .from_writer(io::stdout());

    // HashMap iteration order is unspecified; sort by client id so the
    // output is deterministic across runs.
    let mut accounts: Vec<&Account> = ledger.values().collect();
    accounts.sort_by_key(|account| account.client);

    for account in accounts {
        writer.serialize(account)?;
    }

    writer.flush()?;
    Ok(())
}

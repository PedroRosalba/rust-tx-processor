use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::process;
use std::time::Instant;

use rand::Rng;

const MIN_ROWS: u64 = 10;
const MAX_ROWS: u64 = 50_000_000;
const DEFAULT_ROWS: u64 = 10_000_000;
const DEFAULT_OUTPUT: &str = "massive_load.csv";
const MAX_CLIENT_ID: u16 = 1_000;

#[derive(Clone, Copy)]
struct DepositRef {
    client: u16,
    tx_id: u32,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("generate_load error: {err}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let rows = parse_rows()?;
    let output_path = env::args()
        .nth(2)
        .unwrap_or_else(|| DEFAULT_OUTPUT.to_string());

    eprintln!(
        "generating {rows} rows into {output_path} (clients 1..={MAX_CLIENT_ID})"
    );

    let started = Instant::now();
    let file = File::create(&output_path)?;
    let mut writer = BufWriter::with_capacity(8 * 1024 * 1024, file);
    let mut rng = rand::thread_rng();

    writeln!(writer, "type,client,tx,amount")?;

    let mut open_deposits: Vec<DepositRef> = Vec::with_capacity((rows / 4) as usize);
    let mut disputed_deposits: Vec<DepositRef> = Vec::with_capacity((rows / 10) as usize);

    for tx_id in 1..=rows {
        let client = rng.gen_range(1..=MAX_CLIENT_ID);
        let roll: u8 = rng.gen_range(0..100);

        let kind = if roll < 40 {
            "deposit"
        } else if roll < 70 {
            "withdrawal"
        } else if roll < 80 {
            "dispute"
        } else if roll < 90 {
            "resolve"
        } else {
            "chargeback"
        };

        match kind {
            "deposit" => {
                let amount = random_amount(&mut rng);
                writeln!(writer, "deposit,{client},{tx_id},{amount}")?;
                open_deposits.push(DepositRef {
                    client,
                    tx_id: tx_id as u32,
                });
            }
            "withdrawal" => {
                let amount = random_amount(&mut rng);
                writeln!(writer, "withdrawal,{client},{tx_id},{amount}")?;
            }
            "dispute" => {
                if let Some(deposit) = pick_open_deposit(&mut rng, &mut open_deposits) {
                    writeln!(
                        writer,
                        "dispute,{},{},",
                        deposit.client, deposit.tx_id
                    )?;
                    disputed_deposits.push(deposit);
                } else {
                    let amount = random_amount(&mut rng);
                    writeln!(writer, "deposit,{client},{tx_id},{amount}")?;
                    open_deposits.push(DepositRef {
                        client,
                        tx_id: tx_id as u32,
                    });
                }
            }
            "resolve" => {
                if let Some(deposit) = pick_disputed_deposit(&mut rng, &mut disputed_deposits) {
                    writeln!(
                        writer,
                        "resolve,{},{},",
                        deposit.client, deposit.tx_id
                    )?;
                    open_deposits.push(deposit);
                } else if let Some(deposit) = pick_open_deposit(&mut rng, &mut open_deposits) {
                    writeln!(
                        writer,
                        "dispute,{},{},",
                        deposit.client, deposit.tx_id
                    )?;
                    disputed_deposits.push(deposit);
                } else {
                    let amount = random_amount(&mut rng);
                    writeln!(writer, "deposit,{client},{tx_id},{amount}")?;
                    open_deposits.push(DepositRef {
                        client,
                        tx_id: tx_id as u32,
                    });
                }
            }
            "chargeback" => {
                if let Some(deposit) = pick_disputed_deposit(&mut rng, &mut disputed_deposits) {
                    writeln!(
                        writer,
                        "chargeback,{},{},",
                        deposit.client, deposit.tx_id
                    )?;
                } else if let Some(deposit) = pick_open_deposit(&mut rng, &mut open_deposits) {
                    writeln!(
                        writer,
                        "dispute,{},{},",
                        deposit.client, deposit.tx_id
                    )?;
                    disputed_deposits.push(deposit);
                } else {
                    let amount = random_amount(&mut rng);
                    writeln!(writer, "deposit,{client},{tx_id},{amount}")?;
                    open_deposits.push(DepositRef {
                        client,
                        tx_id: tx_id as u32,
                    });
                }
            }
            _ => unreachable!(),
        }

        if tx_id % 1_000_000 == 0 {
            eprintln!("  wrote {tx_id} rows...");
        }
    }

    writer.flush()?;
    eprintln!(
        "done: {rows} rows in {:.2?} -> {output_path}",
        started.elapsed()
    );

    Ok(())
}

fn parse_rows() -> Result<u64, String> {
    let rows = env::args()
        .nth(1)
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|_| format!("invalid row count: {value}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_ROWS);

    if !(MIN_ROWS..=MAX_ROWS).contains(&rows) {
        return Err(format!(
            "row count must be between {MIN_ROWS} and {MAX_ROWS}, got {rows}"
        ));
    }

    Ok(rows)
}

fn random_amount(rng: &mut impl Rng) -> String {
    let whole = rng.gen_range(1..=5_000);
    let fraction = rng.gen_range(0..10_000);
    format!("{whole}.{fraction:04}")
}

fn pick_open_deposit(rng: &mut impl Rng, open_deposits: &mut Vec<DepositRef>) -> Option<DepositRef> {
    if open_deposits.is_empty() {
        return None;
    }

    let index = rng.gen_range(0..open_deposits.len());
    Some(open_deposits.swap_remove(index))
}

fn pick_disputed_deposit(
    rng: &mut impl Rng,
    disputed_deposits: &mut Vec<DepositRef>,
) -> Option<DepositRef> {
    if disputed_deposits.is_empty() {
        return None;
    }

    let index = rng.gen_range(0..disputed_deposits.len());
    Some(disputed_deposits.swap_remove(index))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rows_defaults_to_ten_million() {
        assert_eq!(DEFAULT_ROWS, 10_000_000);
        assert!((MIN_ROWS..=MAX_ROWS).contains(&DEFAULT_ROWS));
    }
}

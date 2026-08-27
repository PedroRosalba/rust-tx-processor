# rust-tx-processor

A Rust CLI payment engine that ingests a CSV of transactions and writes final account balances to stdout.

```bash
cargo run --release -- transactions.csv > accounts.csv
```

Invalid rows and rejected transactions are logged to **stderr**; processing continues without panicking.

---

## Project layout

```
src/
├── lib.rs          # module exports
├── money.rs        # fixed-point Money type
├── types.rs        # Account, Transaction, TransactionType
├── engine.rs       # PaymentEngine + compute_tx logic
├── main.rs         # CSV streaming CLI
└── bin/
    └── generate_load.rs   # stress-test CSV generator
```

---

## Architecture decisions

### Fixed-point `Money` instead of floats

Amounts are stored as `i64` scaled by `10_000` (4 decimal places). All balance mutations go through `checked_add`, `checked_sub`, and `checked_neg` to avoid floating-point drift and to surface overflow as `EngineError::MathOverflow`.

CSV amounts are parsed from strings only (no `f64` path). Inputs with more than 4 fractional digits are rejected.

### Domain model (`types.rs`)

**Account**

| Field       | Meaning                                      |
|-------------|----------------------------------------------|
| `client`    | `u16` client identifier                      |
| `available` | spendable balance                            |
| `held`      | funds frozen during a dispute                |
| `total`     | `available + held` (constant during dispute) |
| `locked`    | set to `true` after a chargeback             |

**Transaction**

| Field         | Meaning                                                |
|---------------|--------------------------------------------------------|
| `kind`        | `deposit`, `withdrawal`, `dispute`, `resolve`, `chargeback` |
| `client`      | `u16`                                                  |
| `tx`          | `u32` unique transaction id                            |
| `amount`      | `Option<Money>` — empty in CSV for dispute lifecycle rows |
| `is_disputed` | in-memory flag on stored history entries               |

`Account` derives `Serialize` so the CLI can emit CSV rows with `writer.serialize(account)`.

### Payment engine (`engine.rs`)

`PaymentEngine` holds two hash maps:

- `client_ledger: HashMap<u16, Account>`
- `tx_history: HashMap<u32, Transaction>`

All state changes flow through `compute_tx(&mut self, tx: Transaction) -> Result<(), EngineError>`.

**Transaction rules**

| Type         | Behavior |
|--------------|----------|
| **Deposit**  | Add to `available` and `total`; store in history. Reject if account is locked or amount is missing. |
| **Withdrawal** | Subtract from `available` and `total` if funds suffice; store in history. Reject with `InsufficientFunds` otherwise — no partial state change. |
| **Dispute**  | Look up original tx. Reject if missing, already disputed, wrong client, or not a deposit. Move original amount from `available` → `held`; `total` unchanged; set `is_disputed = true`. |
| **Resolve**  | Look up disputed tx. Reject if missing or not disputed. Move amount from `held` → `available`; set `is_disputed = false`. |
| **Chargeback** | Look up disputed tx. Reject if missing or not disputed. Subtract from `held` and `total`; set `locked = true`, `is_disputed = false`. |

**Fraud-debt behavior:** disputing a deposit after the funds were fully withdrawn is allowed — `available` may go negative while `held` reflects the original deposit amount and `total` stays consistent.

### CLI layer (`main.rs`)

- Reads input path from `args[1]`.
- Streams CSV row-by-row with `csv::ReaderBuilder` (`flexible(true)` for ragged rows).
- Serde/validation errors → log to stderr, skip row.
- Engine errors → log to stderr, skip row.
- Final ledger written to stdout via Serde (order of rows is undefined — hash map iteration).

**`CliError`** wraps `MissingArgument`, `Io`, and `Csv` with `From` impls and `Display`.

---

## Testing

```bash
cargo test
```

### `money.rs`

Unit tests cover parsing (`"1.2345"`, `".5"`, `"10."`, negatives), rejection of >4 decimal places, overflow boundaries, and parse/display round-trips.

**proptest** properties verify that `checked_*` ops mirror raw `i64` arithmetic and that generated decimal strings normalize correctly.

### `engine.rs`

Integration-style tests drive `compute_tx` with native `Transaction` values and assert final balances or expected `EngineError` variants:

- basic deposit / withdrawal math
- overdraft protection
- dispute lifecycle (available → held → available, total constant)
- chargeback + account lock
- locked account rejection
- fraud debt (negative available after full withdrawal + dispute)
- phantom / duplicate dispute traps

---

## Load generator & benchmarking

Generate a stress-test CSV (default **10M rows**, range **10 – 50M**):

```bash
cargo run --release --bin generate_load
cargo run --release --bin generate_load -- 25000000 massive_load.csv
```

**Row mix:** 40% deposit · 30% withdrawal · 10% dispute · 10% resolve · 10% chargeback  
**Clients:** random `1..=1000`  
**I/O:** 8 MB `BufWriter`

Open and disputed deposit pools use `Vec::swap_remove` instead of `Vec::remove` so picking a random entry stays **O(1)** as those vectors grow into the millions.

Benchmark the processor:

```bash
cargo build --release
hyperfine --warmup 3 './target/release/rust-tx-processor massive_load.csv'
```

---

## Future: TCP stream instead of CSV

The current design is **synchronous and file-backed**: read a row, apply it, move on. That keeps ordering trivial and avoids concurrency overhead.

If the input were a **TCP stream** (or many concurrent streams), the tradeoffs change:

| Today (CSV)              | TCP / network                          |
|--------------------------|----------------------------------------|
| Sync `std::io`           | Async runtime (e.g. **Tokio**)         |
| Single ordered iterator  | Framed messages over the wire          |
| No concurrency concerns  | Strict ordering still required         |

### Why ordering matters

Ledger updates are **sequential by design**: `(read tx → mutate internal state)` must happen in arrival order. Parallel workers that apply transactions out of order would corrupt balances and dispute state.

That rules out several naive async patterns:

- **Many Tokio tasks** each calling `compute_tx` — messages could be applied out of order.
- **Shared `RwLock` / `Mutex` around the engine** — correct but serializes everything; extra CPU cores sit idle while one task holds the lock.

### Approach we would take

1. **Tokio** as the async runtime.
2. **`mpsc` channel** — producers (TCP readers) send decoded transactions into a single channel; one consumer applies them. This scales when many streams feed one ordered pipeline.
3. **Single pinned consumer task** owning `PaymentEngine` — the engine must live at a stable address if the task is self-referential (e.g. read loop + ledger in one struct). Pin the task/future so internal pointers are not invalidated by moves.
4. **`FramedRead`** (from `tokio-util` / `futures`) on each TCP connection — buffers raw bytes and yields complete frames (length-delimited or newline-delimited records), turning a raw async read into a clean message stream before enqueueing to `mpsc`.

This is a high-level sketch only; the CSV CLI intentionally stays simple until network ingestion is a real requirement.

---

## Dependencies

| Crate    | Role                          |
|----------|-------------------------------|
| `serde`  | CSV (de)serialization         |
| `csv`    | streaming parse / write         |
| `rand`   | load generator                |
| `proptest` (dev) | property tests for `Money` |

---

## Example

**Input** (`transactions.csv`):

```csv
type,client,tx,amount
deposit,1,1,10.0
withdrawal,1,2,3.0
dispute,1,1,
resolve,1,1,
```

**Run:**

```bash
cargo run --release -- transactions.csv > accounts.csv
```

**Output** (`accounts.csv`):

```csv
client,available,held,total,locked
1,7.0000,0.0000,7.0000,false
```

After the deposit and withdrawal the total is 7. Dispute temporarily moves the original deposit amount into `held`; resolve restores it to `available` while `total` stays at 7 throughout.

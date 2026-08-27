use std::collections::HashMap;
use std::fmt;

use crate::money::Money;
use crate::types::{Account, Transaction, TransactionType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineError {
    InsufficientFunds,
    TransactionNotFound,
    TransactionNotDisputed,
    AlreadyDisputed,
    AccountLocked,
    MathOverflow,
    InvalidDisputeType,
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InsufficientFunds => write!(f, "insufficient funds"),
            Self::TransactionNotFound => write!(f, "transaction not found"),
            Self::TransactionNotDisputed => write!(f, "transaction not disputed"),
            Self::AlreadyDisputed => write!(f, "transaction already disputed"),
            Self::AccountLocked => write!(f, "account locked"),
            Self::MathOverflow => write!(f, "math overflow"),
            Self::InvalidDisputeType => write!(f, "invalid dispute type"),
        }
    }
}

impl std::error::Error for EngineError {}

pub struct PaymentEngine {
    pub client_ledger: HashMap<u16, Account>,
    pub tx_history: HashMap<u32, Transaction>,
}

impl PaymentEngine {
    pub fn new() -> Self {
        Self {
            client_ledger: HashMap::new(),
            tx_history: HashMap::new(),
        }
    }

    pub fn compute_tx(&mut self, tx: Transaction) -> Result<(), EngineError> {
        match tx.kind {
            TransactionType::Deposit => self.process_deposit(tx),
            TransactionType::Withdrawal => self.process_withdrawal(tx),
            TransactionType::Dispute => self.process_dispute(tx),
            TransactionType::Resolve => self.process_resolve(tx),
            TransactionType::Chargeback => self.process_chargeback(tx),
        }
    }

    fn process_deposit(&mut self, tx: Transaction) -> Result<(), EngineError> {
        let amount = tx.amount.ok_or(EngineError::InsufficientFunds)?;
        let account = self.account_mut(tx.client);
        ensure_unlocked(account)?;

        account.available = add_money(account.available, amount)?;
        account.total = add_money(account.total, amount)?;

        self.tx_history.insert(tx.tx, tx);
        Ok(())
    }

    fn process_withdrawal(&mut self, tx: Transaction) -> Result<(), EngineError> {
        let amount = tx.amount.ok_or(EngineError::InsufficientFunds)?;
        let account = self.account_mut(tx.client);
        ensure_unlocked(account)?;

        if account.available < amount {
            return Err(EngineError::InsufficientFunds);
        }

        account.available = sub_money(account.available, amount)?;
        account.total = sub_money(account.total, amount)?;

        self.tx_history.insert(tx.tx, tx);
        Ok(())
    }

    fn process_dispute(&mut self, tx: Transaction) -> Result<(), EngineError> {
        let (amount, client) = {
            let original = self
                .tx_history
                .get(&tx.tx)
                .ok_or(EngineError::TransactionNotFound)?;

            if original.is_disputed {
                return Err(EngineError::AlreadyDisputed);
            }

            if original.kind != TransactionType::Deposit {
                return Err(EngineError::InvalidDisputeType);
            }

            if original.client != tx.client {
                return Err(EngineError::TransactionNotFound);
            }

            let amount = original
                .amount
                .ok_or(EngineError::TransactionNotFound)?;

            (amount, original.client)
        };

        let account = self.account_mut(client);
        ensure_unlocked(account)?;

        account.available = sub_money(account.available, amount)?;
        account.held = add_money(account.held, amount)?;

        self.tx_history
            .get_mut(&tx.tx)
            .map(|original| original.is_disputed = true);

        Ok(())
    }

    fn process_resolve(&mut self, tx: Transaction) -> Result<(), EngineError> {
        let (amount, client) = {
            let original = self
                .tx_history
                .get(&tx.tx)
                .ok_or(EngineError::TransactionNotFound)?;

            if !original.is_disputed {
                return Err(EngineError::TransactionNotDisputed);
            }

            if original.client != tx.client {
                return Err(EngineError::TransactionNotFound);
            }

            let amount = original
                .amount
                .ok_or(EngineError::TransactionNotFound)?;

            (amount, original.client)
        };

        let account = self.account_mut(client);

        account.held = sub_money(account.held, amount)?;
        account.available = add_money(account.available, amount)?;

        self.tx_history
            .get_mut(&tx.tx)
            .map(|original| original.is_disputed = false);

        Ok(())
    }

    fn process_chargeback(&mut self, tx: Transaction) -> Result<(), EngineError> {
        let (amount, client) = {
            let original = self
                .tx_history
                .get(&tx.tx)
                .ok_or(EngineError::TransactionNotFound)?;

            if !original.is_disputed {
                return Err(EngineError::TransactionNotDisputed);
            }

            if original.client != tx.client {
                return Err(EngineError::TransactionNotFound);
            }

            let amount = original
                .amount
                .ok_or(EngineError::TransactionNotFound)?;

            (amount, original.client)
        };

        let account = self.account_mut(client);

        account.held = sub_money(account.held, amount)?;
        account.total = sub_money(account.total, amount)?;
        account.locked = true;

        self.tx_history
            .get_mut(&tx.tx)
            .map(|original| original.is_disputed = false);

        Ok(())
    }

    fn account_mut(&mut self, client: u16) -> &mut Account {
        self.client_ledger
            .entry(client)
            .or_insert_with(|| Account::new(client))
    }
}

impl Default for PaymentEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn ensure_unlocked(account: &Account) -> Result<(), EngineError> {
    if account.locked {
        return Err(EngineError::AccountLocked);
    }

    Ok(())
}

fn add_money(base: Money, amount: Money) -> Result<Money, EngineError> {
    base.checked_add(amount).ok_or(EngineError::MathOverflow)
}

fn sub_money(base: Money, amount: Money) -> Result<Money, EngineError> {
    base.checked_sub(amount).ok_or(EngineError::MathOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TransactionType;

    fn m(value: &str) -> Money {
        value.parse().expect("valid money literal")
    }

    fn tx(kind: TransactionType, client: u16, id: u32, amount: Option<&str>) -> Transaction {
        Transaction {
            kind,
            client,
            tx: id,
            amount: amount.map(m),
            is_disputed: false,
        }
    }

    fn account(engine: &PaymentEngine, client: u16) -> &Account {
        engine
            .client_ledger
            .get(&client)
            .unwrap_or_else(|| panic!("client {client} not found in ledger"))
    }

    fn assert_balances(
        engine: &PaymentEngine,
        client: u16,
        available: &str,
        held: &str,
        total: &str,
        locked: bool,
    ) {
        let account = account(engine, client);
        assert_eq!(account.available, m(available), "available mismatch");
        assert_eq!(account.held, m(held), "held mismatch");
        assert_eq!(account.total, m(total), "total mismatch");
        assert_eq!(account.locked, locked, "locked mismatch");
    }

    fn assert_err(result: Result<(), EngineError>, expected: EngineError) {
        assert_eq!(result.unwrap_err(), expected);
    }

    #[test]
    fn deposit_and_withdrawal_move_balances() {
        let mut engine = PaymentEngine::new();

        engine
            .compute_tx(tx(TransactionType::Deposit, 1, 1, Some("10.0")))
            .unwrap();
        assert_balances(&engine, 1, "10.0000", "0.0000", "10.0000", false);

        engine
            .compute_tx(tx(TransactionType::Withdrawal, 1, 2, Some("3.5")))
            .unwrap();
        assert_balances(&engine, 1, "6.5000", "0.0000", "6.5000", false);
    }

    #[test]
    fn withdrawal_rejects_overdraft() {
        let mut engine = PaymentEngine::new();

        engine
            .compute_tx(tx(TransactionType::Deposit, 1, 1, Some("5.0")))
            .unwrap();

        let result = engine.compute_tx(tx(TransactionType::Withdrawal, 1, 2, Some("5.01")));
        assert_err(result, EngineError::InsufficientFunds);
        assert_balances(&engine, 1, "5.0000", "0.0000", "5.0000", false);
        assert!(!engine.tx_history.contains_key(&2));
    }

    #[test]
    fn dispute_lifecycle_moves_available_to_held_and_back() {
        let mut engine = PaymentEngine::new();

        engine
            .compute_tx(tx(TransactionType::Deposit, 1, 1, Some("8.0")))
            .unwrap();
        assert_balances(&engine, 1, "8.0000", "0.0000", "8.0000", false);

        engine
            .compute_tx(tx(TransactionType::Dispute, 1, 1, None))
            .unwrap();
        assert_balances(&engine, 1, "0.0000", "8.0000", "8.0000", false);
        assert!(engine.tx_history.get(&1).unwrap().is_disputed);

        engine
            .compute_tx(tx(TransactionType::Resolve, 1, 1, None))
            .unwrap();
        assert_balances(&engine, 1, "8.0000", "0.0000", "8.0000", false);
        assert!(!engine.tx_history.get(&1).unwrap().is_disputed);
    }

    #[test]
    fn chargeback_deducts_held_and_total_and_locks_account() {
        let mut engine = PaymentEngine::new();

        engine
            .compute_tx(tx(TransactionType::Deposit, 2, 10, Some("5.0")))
            .unwrap();
        engine
            .compute_tx(tx(TransactionType::Dispute, 2, 10, None))
            .unwrap();
        assert_balances(&engine, 2, "0.0000", "5.0000", "5.0000", false);

        engine
            .compute_tx(tx(TransactionType::Chargeback, 2, 10, None))
            .unwrap();
        assert_balances(&engine, 2, "0.0000", "0.0000", "0.0000", true);
        assert!(!engine.tx_history.get(&10).unwrap().is_disputed);
    }

    #[test]
    fn locked_account_rejects_deposit_and_withdrawal_without_balance_changes() {
        let mut engine = PaymentEngine::new();

        engine
            .compute_tx(tx(TransactionType::Deposit, 3, 20, Some("4.0")))
            .unwrap();
        engine
            .compute_tx(tx(TransactionType::Dispute, 3, 20, None))
            .unwrap();
        engine
            .compute_tx(tx(TransactionType::Chargeback, 3, 20, None))
            .unwrap();
        assert_balances(&engine, 3, "0.0000", "0.0000", "0.0000", true);

        assert_err(
            engine.compute_tx(tx(TransactionType::Deposit, 3, 21, Some("1.0"))),
            EngineError::AccountLocked,
        );
        assert_err(
            engine.compute_tx(tx(TransactionType::Withdrawal, 3, 22, Some("1.0"))),
            EngineError::AccountLocked,
        );
        assert_balances(&engine, 3, "0.0000", "0.0000", "0.0000", true);
        assert!(!engine.tx_history.contains_key(&21));
        assert!(!engine.tx_history.contains_key(&22));
    }

    #[test]
    fn fraud_debt_allows_negative_available_after_full_withdrawal_dispute() {
        let mut engine = PaymentEngine::new();

        engine
            .compute_tx(tx(TransactionType::Deposit, 4, 30, Some("10.0")))
            .unwrap();
        engine
            .compute_tx(tx(TransactionType::Withdrawal, 4, 31, Some("10.0")))
            .unwrap();
        assert_balances(&engine, 4, "0.0000", "0.0000", "0.0000", false);

        engine
            .compute_tx(tx(TransactionType::Dispute, 4, 30, None))
            .unwrap();
        assert_balances(&engine, 4, "-10.0000", "10.0000", "0.0000", false);
    }

    #[test]
    fn phantom_and_duplicate_disputes_return_expected_errors() {
        let mut engine = PaymentEngine::new();

        assert_err(
            engine.compute_tx(tx(TransactionType::Dispute, 1, 999, None)),
            EngineError::TransactionNotFound,
        );

        engine
            .compute_tx(tx(TransactionType::Deposit, 1, 1, Some("2.0")))
            .unwrap();

        assert_err(
            engine.compute_tx(tx(TransactionType::Resolve, 1, 1, None)),
            EngineError::TransactionNotDisputed,
        );
        assert_err(
            engine.compute_tx(tx(TransactionType::Chargeback, 1, 1, None)),
            EngineError::TransactionNotDisputed,
        );

        engine
            .compute_tx(tx(TransactionType::Dispute, 1, 1, None))
            .unwrap();

        assert_err(
            engine.compute_tx(tx(TransactionType::Dispute, 1, 1, None)),
            EngineError::AlreadyDisputed,
        );

        engine
            .compute_tx(tx(TransactionType::Resolve, 1, 1, None))
            .unwrap();

        assert_err(
            engine.compute_tx(tx(TransactionType::Resolve, 1, 1, None)),
            EngineError::TransactionNotDisputed,
        );
        assert_err(
            engine.compute_tx(tx(TransactionType::Chargeback, 1, 1, None)),
            EngineError::TransactionNotDisputed,
        );
    }

    #[test]
    fn disputing_withdrawal_is_invalid_dispute_type() {
        let mut engine = PaymentEngine::new();

        engine
            .compute_tx(tx(TransactionType::Deposit, 1, 1, Some("5.0")))
            .unwrap();
        engine
            .compute_tx(tx(TransactionType::Withdrawal, 1, 2, Some("1.0")))
            .unwrap();

        assert_err(
            engine.compute_tx(tx(TransactionType::Dispute, 1, 2, None)),
            EngineError::InvalidDisputeType,
        );
    }
}

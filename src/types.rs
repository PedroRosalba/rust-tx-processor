use serde::{Deserialize, Serialize};

use crate::money::Money;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Account {
    pub client: u16,
    pub available: Money,
    pub held: Money,
    pub total: Money,
    pub locked: bool,
}

impl Account {
    pub fn new(client: u16) -> Self {
        Self {
            client,
            available: Money::zero(),
            held: Money::zero(),
            total: Money::zero(),
            locked: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Transaction {
    #[serde(rename = "type")]
    pub kind: TransactionType,
    pub client: u16,
    pub tx: u32,
    #[serde(default, deserialize_with = "deserialize_optional_money")]
    pub amount: Option<Money>,
    #[serde(default, skip_deserializing)]
    pub is_disputed: bool,
}

fn deserialize_optional_money<'de, D>(deserializer: D) -> Result<Option<Money>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw: Option<String> = Option::deserialize(deserializer)?;
    match raw {
        None => Ok(None),
        Some(value) if value.trim().is_empty() => Ok(None),
        Some(value) => value
            .parse::<Money>()
            .map(Some)
            .map_err(|_| serde::de::Error::custom(format!("invalid money value: {value}"))),
    }
}

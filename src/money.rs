use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Fixed-point money stored as cents × 10_000 (4 decimal places).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Money(i64);

const SCALE: i64 = 10_000;

impl Money {
    pub fn zero() -> Self {
        Self(0)
    }

    pub fn from_cents(cents: i64) -> Self {
        Self(cents * SCALE)
    }

    pub fn raw(&self) -> i64 {
        self.0
    }

    pub fn checked_add(self, other: Self) -> Option<Self> {
        self.0.checked_add(other.0).map(Self)
    }

    pub fn checked_sub(self, other: Self) -> Option<Self> {
        self.0.checked_sub(other.0).map(Self)
    }

    pub fn checked_neg(self) -> Option<Self> {
        self.0.checked_neg().map(Self)
    }

    fn from_scaled(value: i64) -> Option<Self> {
        Some(Self(value))
    }

    fn parse_decimal(input: &str) -> Option<i64> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return None;
        }

        let negative = trimmed.starts_with('-');
        let unsigned = trimmed.trim_start_matches('-');

        let (integer_part, fractional_part) = match unsigned.split_once('.') {
            Some((int_part, frac_part)) => (int_part, frac_part),
            None => (unsigned, ""),
        };

        if integer_part.is_empty() && fractional_part.is_empty() {
            return None;
        }

        if fractional_part.len() > 4 {
            return None;
        }

        let integer: i64 = if integer_part.is_empty() {
            0
        } else {
            integer_part.parse().ok()?
        };

        let mut fraction: i64 = if fractional_part.is_empty() {
            0
        } else {
            fractional_part.parse().ok()?
        };

        let padding = 4 - fractional_part.len();
        for _ in 0..padding {
            fraction = fraction.checked_mul(10)?;
        }

        let scaled = integer
            .checked_mul(SCALE)?
            .checked_add(fraction)?;

        Some(if negative {
            scaled.checked_neg()?
        } else {
            scaled
        })
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let negative = self.0 < 0;
        let abs = self.0.abs();
        let integer = abs / SCALE;
        let fraction = abs % SCALE;

        if negative {
            write!(f, "-")?;
        }

        write!(f, "{}.{:04}", integer, fraction)
    }
}

impl FromStr for Money {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Money::parse_decimal(s)
            .and_then(Money::from_scaled)
            .ok_or(())
    }
}

impl Serialize for Money {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Money {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Money::from_str(&value).map_err(|_| {
            serde::de::Error::custom(format!("invalid money value: {value}"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn parse(s: &str) -> Option<Money> {
        s.parse().ok()
    }

    #[test]
    fn parses_four_decimal_places() {
        let money = parse("1.2345").unwrap();
        assert_eq!(money.raw(), 12_345);
    }

    #[test]
    fn parses_leading_decimal_without_integer_part() {
        let money = parse(".5").unwrap();
        assert_eq!(money.raw(), 5_000);
        assert_eq!(money.to_string(), "0.5000");
    }

    #[test]
    fn parses_trailing_decimal_point() {
        let money = parse("10.").unwrap();
        assert_eq!(money.raw(), 100_000);
        assert_eq!(money.to_string(), "10.0000");
    }

    #[test]
    fn rejects_more_than_four_decimal_places() {
        assert!(parse("1.23456").is_none());
        assert!(parse("0.12345").is_none());
        assert!(parse("99.99999").is_none());
    }

    #[test]
    fn checked_add_overflow_returns_none_at_boundary() {
        let max = Money(i64::MAX);
        assert!(max.checked_add(Money(1)).is_none());
    }

    #[test]
    fn display_formatting_symmetry() {
        let cases = ["1.500", "1.2345", ".5", "10.", "0.0001", "-2.7500"];

        for case in cases {
            let money = parse(case).expect("valid input");
            assert_eq!(
                money.to_string(),
                money.to_string(),
                "to_string must be stable for {case}"
            );
            assert_eq!(
                parse(&money.to_string()).unwrap(),
                money,
                "parse(to_string(x)) must round-trip for {case}"
            );
        }

        assert_eq!(parse("1.500").unwrap().to_string(), "1.5000");
    }

    #[test]
    fn parses_negative_decimal_values() {
        assert_eq!(parse("-10.0000").unwrap().raw(), -100_000);
        assert_eq!(parse("-0.5000").unwrap().raw(), -5_000);
        assert_eq!(parse("-10.0000").unwrap().to_string(), "-10.0000");
    }

    #[test]
    fn checked_sub_underflow_returns_none_at_boundary() {
        let min = Money(i64::MIN);
        assert!(min.checked_sub(Money(1)).is_none());
    }

    #[test]
    fn checked_neg_overflow_returns_none_at_boundary() {
        assert!(Money(i64::MIN).checked_neg().is_none());
    }

    proptest! {
        #[test]
        fn checked_add_matches_i64_arithmetic(a in any::<i64>(), b in any::<i64>()) {
            let left = Money(a);
            let right = Money(b);

            match a.checked_add(b) {
                Some(sum) => prop_assert_eq!(left.checked_add(right), Some(Money(sum))),
                None => prop_assert_eq!(left.checked_add(right), None),
            }
        }

        #[test]
        fn checked_sub_matches_i64_arithmetic(a in any::<i64>(), b in any::<i64>()) {
            let left = Money(a);
            let right = Money(b);

            match a.checked_sub(b) {
                Some(diff) => prop_assert_eq!(left.checked_sub(right), Some(Money(diff))),
                None => prop_assert_eq!(left.checked_sub(right), None),
            }
        }

        #[test]
        fn checked_neg_matches_i64_arithmetic(a in any::<i64>()) {
            let money = Money(a);

            match a.checked_neg() {
                Some(value) => prop_assert_eq!(money.checked_neg(), Some(Money(value))),
                None => prop_assert_eq!(money.checked_neg(), None),
            }
        }

        #[test]
        fn scaled_values_round_trip_through_display(
            scaled in any::<i64>().prop_filter("must fit money scale", |v| *v / SCALE >= i64::MIN && *v / SCALE <= i64::MAX)
        ) {
            let money = Money(scaled);
            let rendered = money.to_string();
            let reparsed = parse(&rendered).expect("rendered money must reparse");
            prop_assert_eq!(reparsed, money);
        }

        #[test]
        fn valid_decimal_strings_parse_and_normalize(
            integer in -1_000_000i64..1_000_000,
            fraction in 0u32..10_000u32,
        ) {
            let input = format!("{integer}.{fraction:04}");
            let money = parse(&input).expect("generated decimal must parse");
            prop_assert_eq!(money.to_string(), format!("{integer}.{fraction:04}"));
        }
    }
}

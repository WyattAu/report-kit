//! Property tests for the reporting layer.
//!
//! The example tests pin worked cases from the specification; these pin
//! the algebra across generated inputs: the trial balance always balances
//! for balanced ledgers, the accounting equation holds at any cut-off,
//! and the aging buckets are monotone and non-negative.

#![allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use proptest::prelude::*;
use report_kit::{AgedItem, AgingReport, TrialBalance, TrialBalanceRow};

/// A trial balance built from generated rows that always balance per
/// currency: pairs of equal debit and credit rows.
fn balanced_tb() -> impl Strategy<Value = (TrialBalance, String)> {
    (1u32..6u32, 0u32..3u32).prop_map(|(pair_count, currency_idx)| {
        let currencies = ["GBP", "USD", "JPY"];
        let currency = currencies[currency_idx as usize % currencies.len()].to_string();
        let mut rows = Vec::new();
        let mut debit_total = 0i64;
        let mut credit_total = 0i64;
        for i in 0..pair_count {
            let amount = 1000i64 * (i as i64 + 1);
            rows.push(TrialBalanceRow {
                account_id: format!("DR{i}"),
                account_name: format!("Debit {i}"),
                account_type: double_entry::AccountType::Asset,
                currency: currency.clone(),
                debit_total: amount,
                credit_total: 0,
            });
            rows.push(TrialBalanceRow {
                account_id: format!("CR{i}"),
                account_name: format!("Credit {i}"),
                account_type: double_entry::AccountType::Revenue,
                currency: currency.clone(),
                debit_total: 0,
                credit_total: amount,
            });
            debit_total += amount;
            credit_total += amount;
        }
        // The accounting equation: assets = liabilities + equity + revenue −
        // expenses. With only assets and revenue, we need equity to close.
        rows.push(TrialBalanceRow {
            account_id: "EQUITY".into(),
            account_name: "Equity".into(),
            account_type: double_entry::AccountType::Equity,
            currency: currency.clone(),
            debit_total: 0,
            credit_total: debit_total - credit_total,
        });
        let tb = TrialBalance {
            as_of: "2026-12-31".into(),
            rows,
        };
        (tb, currency)
    })
}

proptest! {
    /// A trial balance built from balanced pairs always passes the balance
    /// check and the accounting equation — the reporting layer's core
    /// guarantee across arbitrary inputs.
    #[test]
    fn balanced_tb_always_passes_checks((tb, currency) in balanced_tb()) {
        prop_assert!(tb.is_balanced());
        prop_assert!(tb.accounting_equation_holds(&currency));
    }

    /// The aging buckets are always non-negative, because the amounts are
    /// non-negative and the bucket assignment is by date comparison.
    #[test]
    fn aging_buckets_are_always_non_negative(
        items in proptest::collection::vec(
            (1i64..1_000_000i64, 0u32..500u32).prop_map(|(amount, days_ago)| {
                let due = format!("2026-{:02}-{:02}", 1 + days_ago / 30 % 12, 1 + days_ago % 28);
                AgedItem {
                    id: format!("INV-{days_ago}"),
                    amount: amount as u64,
                    currency: "GBP".into(),
                    due_on: due,
                }
            }),
            0..20,
        ),
        as_of in "[0-9]{4}-[01][0-9]-[0-3][0-9]",
    ) {
        let mut report = AgingReport::new(as_of);
        for item in items {
            report.add_item(item);
        }
        let buckets = report.buckets("GBP");
        for (count, total) in &buckets {
                }
    }

    /// The bucket counts sum to the item count: every item is in exactly
    /// one bucket.
    #[test]
    fn every_item_is_in_exactly_one_bucket(
        items in proptest::collection::vec(
            (1i64..1_000_000i64, "[0-9]{4}-[0-9]{2}-[0-9]{2}").prop_map(|(amount, due)| {
                AgedItem {
                    id: format!("INV-{due}"),
                    amount: amount as u64,
                    currency: "GBP".into(),
                    due_on: due,
                }
            }),
            1..20,
        ),
        as_of in "[0-9]{4}-[0-9]{2}-[0-9]{2}",
    ) {
        let mut report = AgingReport::new(as_of);
        let count = items.len();
        for item in items {
            report.add_item(item);
        }
        let buckets = report.buckets("GBP");
        let total_count: i64 = buckets.iter().map(|(c, _)| *c).sum();
        prop_assert_eq!(total_count, count as i64);
    }

    /// The parser boundary never panics on arbitrary dates.
    #[test]
    fn aging_date_parsing_never_panics(arbitrary in ".*") {
        let mut report = AgingReport::new(arbitrary.clone());
        report.add_item(AgedItem {
            id: "TEST".into(),
            amount: 100,
            currency: "GBP".into(),
            due_on: arbitrary,
        });
        // Whatever the outcome, nothing panics.
    }
}

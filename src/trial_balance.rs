//! The trial balance: every account a posted line names, per currency.
//!
//! # Why the rows are keyed `(account, currency)`
//!
//! A trial balance is a *per-currency* statement. Summing a GBP bank account
//! with a JPY fee account produces a number in no currency, and "debits equal
//! credits" is only meaningful within one denomination — the double-entry
//! invariant is stated per posting, and a posting names one currency. So the
//! balance check runs per currency, and a ledger that is balanced in GBP and
//! unbalanced in JPY is reported as exactly that, not as a single meaningless
//! total.
//!
//! # What counts
//!
//! Posted entries only, and only those effective on or before the cut-off.
//! Drafts contribute nothing — the ledger already guarantees that — and
//! reversed entries contribute nothing because their counter-entry does the
//! work. Cut-off is a `YYYY-MM-DD` string comparison, which is exact for that
//! format: no date parsing in the aggregation path.

use double_entry::{AccountType, Ledger};

/// One account's totals in one currency, at one cut-off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrialBalanceRow {
    /// The account's identifier.
    pub account_id: String,
    /// The account's name, for humans.
    pub account_name: String,
    /// The class, which decides the normal side.
    pub account_type: AccountType,
    /// The ISO 4217 code the lines were posted in.
    pub currency: String,
    /// Total debits, minor units.
    pub debit_total: i64,
    /// Total credits, minor units.
    pub credit_total: i64,
}

impl TrialBalanceRow {
    /// Debits minus credits. Positive means the account carries a net debit.
    #[must_use]
    pub fn net(&self) -> i64 {
        self.debit_total - self.credit_total
    }

    /// The side the account *should* sit on, from its class.
    ///
    /// The opposite is not an error to hide — a credit balance on a bank
    /// account is an overdraft, which is a real state — but it is a state a
    /// reader of the trial balance should be able to find, so it is a
    /// predicate rather than a normalization.
    #[must_use]
    pub fn is_on_normal_side(&self) -> bool {
        let normal_debit = self.account_type.increases_on_debit();
        (self.net() >= 0) == normal_debit || self.net() == 0
    }
}

/// The trial balance at a cut-off date.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrialBalance {
    /// The cut-off, `YYYY-MM-DD`: only entries effective on or before it count.
    pub as_of: String,
    /// One row per `(account, currency)` named by a posted line, ordered by
    /// account id then currency.
    pub rows: Vec<TrialBalanceRow>,
}

impl TrialBalance {
    /// Aggregate every posted entry effective on or before `as_of`.
    pub fn from_ledger(ledger: &Ledger, as_of: &str) -> Self {
        let mut order: Vec<(String, String)> = Vec::new();
        let mut totals: std::collections::HashMap<(String, String), (i64, i64)> =
            std::collections::HashMap::new();

        for entry in ledger.posted_entries() {
            // YYYY-MM-DD compares correctly as a string: the format is fixed
            // width and zero-padded, so lexicographic order is date order.
            if entry.date.as_str() > as_of {
                continue;
            }
            for line in &entry.lines {
                let key = (
                    line.account.as_str().to_string(),
                    line.amount.currency.alpha.to_string(),
                );
                let slot = totals.entry(key.clone()).or_insert((0, 0));
                match line.side {
                    double_entry::Side::Debit => slot.0 += line.amount.units,
                    double_entry::Side::Credit => slot.1 += line.amount.units,
                }
                if !order.contains(&key) {
                    order.push(key);
                }
            }
        }

        order.sort();
        let rows = order
            .into_iter()
            .filter_map(|(account_id, currency)| {
                let (debit_total, credit_total) =
                    totals.get(&(account_id.clone(), currency.clone()))?;
                let account = ledger.account(&double_entry::AccountId::new(account_id.clone()));
                Some(TrialBalanceRow {
                    account_name: account
                        .map(|a| a.name.clone())
                        .unwrap_or_else(|| "<unknown>".to_string()),
                    account_type: account.map(|a| a.account_type)?,
                    account_id,
                    currency,
                    debit_total: *debit_total,
                    credit_total: *credit_total,
                })
            })
            .collect();
        Self {
            as_of: as_of.to_string(),
            rows,
        }
    }

    /// Whether debits equal credits **in every currency present**.
    ///
    /// The double-entry invariant is stated per posting, so it is checked per
    /// currency: a ledger balanced in GBP and unbalanced in JPY is not
    /// balanced, and averaging the two would hide it.
    #[must_use]
    pub fn is_balanced(&self) -> bool {
        self.currencies()
            .into_iter()
            .all(|c| self.imbalance(&c) == 0)
    }

    /// Every currency the balance names, sorted.
    #[must_use]
    pub fn currencies(&self) -> Vec<String> {
        let mut seen: Vec<String> = self.rows.iter().map(|r| r.currency.clone()).collect();
        seen.sort();
        seen.dedup();
        seen
    }

    /// Total debits minus total credits in one currency. Zero means balanced.
    #[must_use]
    pub fn imbalance(&self, currency: &str) -> i64 {
        self.rows
            .iter()
            .filter(|r| r.currency == currency)
            .map(|r| r.net())
            .sum()
    }

    /// The rows in one currency, ordered by account id.
    #[must_use]
    pub fn rows_for(&self, currency: &str) -> Vec<&TrialBalanceRow> {
        self.rows
            .iter()
            .filter(|r| r.currency == currency)
            .collect()
    }

    /// The extended accounting equation, per currency:
    /// `assets + expenses == liabilities + equity + revenue`.
    ///
    /// This holds without closing entries precisely because the revenue and
    /// expense classes are on the right-hand side: at any cut-off, the period's
    /// result is still sitting in the revenue and expense accounts, and the
    /// equation describes exactly that. It is the reporting-layer restatement
    /// of "every posting balances", so a violation means the trial balance was
    /// built over something that is not a balanced ledger — never a cause to
    /// fudge a row.
    #[must_use]
    pub fn accounting_equation_holds(&self, currency: &str) -> bool {
        let mut left = 0i64;
        let mut right = 0i64;
        for row in self.rows_for(currency) {
            let normal_side_net = if row.account_type.increases_on_debit() {
                row.net()
            } else {
                -row.net()
            };
            match row.account_type {
                AccountType::Asset | AccountType::Expense => left += normal_side_net,
                AccountType::Liability | AccountType::Equity | AccountType::Revenue => {
                    right += normal_side_net
                }
            }
        }
        left == right
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use double_entry::money::currencies;
    use double_entry::{Account, JournalEntry, Line};

    fn chart() -> Ledger {
        let mut ledger = Ledger::new();
        for (id, name, kind) in [
            ("1000", "Current account", AccountType::Asset),
            ("1100", "Trade debtors", AccountType::Asset),
            ("2100", "VAT output", AccountType::Liability),
            ("4000", "Consulting revenue", AccountType::Revenue),
            ("5000", "Bank fees", AccountType::Expense),
        ] {
            ledger.add_account(Account::new(id, name, kind));
        }
        ledger
    }

    fn post(ledger: &mut Ledger, id: &str, date: &str, lines: Vec<Line>, version: u64) -> u64 {
        ledger
            .post(&JournalEntry::draft(id, date, "2026-07", lines), version)
            .expect("the fixture posts")
    }

    #[test]
    fn a_posted_month_balances_per_currency_and_satisfies_the_equation() {
        let mut ledger = chart();
        let mut v = 0;
        v = post(
            &mut ledger,
            "JE-1",
            "2026-07-02",
            vec![
                Line::debit(
                    "1000",
                    double_entry::Amount::minor(144_000, currencies::GBP),
                ),
                Line::credit(
                    "4000",
                    double_entry::Amount::minor(120_000, currencies::GBP),
                ),
                Line::credit("2100", double_entry::Amount::minor(24_000, currencies::GBP)),
            ],
            v,
        );
        post(
            &mut ledger,
            "JE-2",
            "2026-07-09",
            vec![
                Line::debit("5000", double_entry::Amount::minor(1_500, currencies::GBP)),
                Line::credit("1000", double_entry::Amount::minor(1_500, currencies::GBP)),
            ],
            v,
        );
        let tb = TrialBalance::from_ledger(&ledger, "2026-07-31");
        assert!(tb.is_balanced());
        assert!(tb.accounting_equation_holds("GBP"));

        // The bank account: 144,000 in, 1,500 out.
        let bank = tb
            .rows_for("GBP")
            .into_iter()
            .find(|r| r.account_id == "1000")
            .expect("the bank row");
        assert_eq!(bank.debit_total, 144_000);
        assert_eq!(bank.credit_total, 1_500);
        assert_eq!(bank.net(), 142_500);
        assert!(bank.is_on_normal_side());
    }

    #[test]
    fn drafts_and_future_entries_are_excluded_by_construction() {
        let mut ledger = chart();
        let v = post(
            &mut ledger,
            "JE-1",
            "2026-07-02",
            vec![
                Line::debit("1000", double_entry::Amount::minor(50_000, currencies::GBP)),
                Line::credit("4000", double_entry::Amount::minor(50_000, currencies::GBP)),
            ],
            0,
        );
        // A draft never enters the ledger at all — `post` is the only way in,
        // and it refuses non-drafts — so the structural guarantee is that the
        // aggregation only ever sees Posted entries. The cut-off is what this
        // test exercises: a posted entry dated after it.
        post(
            &mut ledger,
            "JE-FUTURE",
            "2026-08-01",
            vec![
                Line::debit("1000", double_entry::Amount::minor(77_777, currencies::GBP)),
                Line::credit("4000", double_entry::Amount::minor(77_777, currencies::GBP)),
            ],
            v,
        );
        let tb = TrialBalance::from_ledger(&ledger, "2026-07-31");
        let bank = tb
            .rows_for("GBP")
            .into_iter()
            .find(|r| r.account_id == "1000")
            .expect("bank row");
        assert_eq!(
            bank.debit_total, 50_000,
            "drafts and future entries are out"
        );
    }

    #[test]
    fn multi_currency_balances_are_checked_separately() {
        let mut ledger = chart();
        let mut v = 0;
        v = post(
            &mut ledger,
            "JE-GBP",
            "2026-07-02",
            vec![
                Line::debit("1000", double_entry::Amount::minor(10_000, currencies::GBP)),
                Line::credit("4000", double_entry::Amount::minor(10_000, currencies::GBP)),
            ],
            v,
        );
        post(
            &mut ledger,
            "JE-JPY",
            "2026-07-02",
            vec![
                Line::debit(
                    "1000",
                    double_entry::Amount::minor(100_000, currencies::JPY),
                ),
                Line::credit("4000", double_entry::Amount::minor(90_000, currencies::JPY)),
                Line::credit("1100", double_entry::Amount::minor(10_000, currencies::JPY)),
            ],
            v,
        );
        let tb = TrialBalance::from_ledger(&ledger, "2026-07-31");
        assert_eq!(tb.currencies(), vec!["GBP".to_string(), "JPY".to_string()]);
        assert!(tb.is_balanced());
        assert_eq!(tb.imbalance("GBP"), 0);
        assert_eq!(tb.imbalance("JPY"), 0);
        // And the two never merge: the rows stay apart, because a total in no
        // currency is not a total.
        assert!(tb.rows_for("GBP").iter().all(|r| r.currency == "GBP"));
        assert_eq!(tb.rows_for("JPY").len(), 3);
    }
}

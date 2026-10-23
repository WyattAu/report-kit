//! The P&L and the balance sheet, derived from the trial balance.
//!
//! # The retained result is the hinge
//!
//! A balance sheet drawn mid-period — which is when everyone draws one — has a
//! problem: the period's profit is still sitting in the revenue and expense
//! accounts, so `assets == liabilities + equity` does **not** hold yet. The
//! honest statement adds the profit where it actually is:
//!
//! ```text
//! assets = liabilities + equity + retained result
//! ```
//!
//! where the retained result is revenue minus expenses, exactly what the
//! income statement computed. The two statements are therefore not two views
//! of two datasets; they are one trial balance read two ways, and a profit on
//! the P&L that does not appear on the balance sheet is a bug this structure
//! cannot express. Closing entries at period end move the result into equity,
//! the retained result becomes zero, and the same equality reads as the
//! classic form.
//!
//! # Per currency, because everything here is
//!
//! The same rule as the trial balance: a P&L in two currencies is a P&L in
//! no currency. Both statements take the trial balance and a currency, and
//! refuse nothing — but total only the rows in that currency.

use double_entry::AccountType;

use crate::TrialBalance;

/// Every account of one class, in one currency, with its total.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatementClassTotals {
    /// The class.
    pub class: AccountType,
    /// `(account id, account name, net in normal direction)` per account.
    pub lines: Vec<(String, String, i64)>,
}

impl StatementClassTotals {
    /// The class total in minor units.
    #[must_use]
    pub fn total(&self) -> i64 {
        self.lines.iter().map(|(_, _, net)| net).sum()
    }
}

/// The income statement: revenue, expenses, and the result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncomeStatement {
    /// The currency.
    pub currency: String,
    /// Revenue accounts, each shown at its normal-side net.
    pub revenue: StatementClassTotals,
    /// Expense accounts, each shown at its normal-side net.
    pub expenses: StatementClassTotals,
}

impl IncomeStatement {
    /// Derive the P&L for `currency` from the trial balance.
    #[must_use]
    pub fn from_trial_balance(tb: &TrialBalance, currency: &str) -> Self {
        let mut lines_for = |class: AccountType| StatementClassTotals {
            class,
            lines: tb
                .rows_for(currency)
                .into_iter()
                .filter(|r| r.account_type == class)
                .map(|r| {
                    let net = if r.account_type.increases_on_debit() {
                        r.net()
                    } else {
                        -r.net()
                    };
                    (r.account_id.clone(), r.account_name.clone(), net)
                })
                .collect(),
        };
        Self {
            currency: currency.to_string(),
            revenue: lines_for(AccountType::Revenue),
            expenses: lines_for(AccountType::Expense),
        }
    }

    /// Total revenue, minor units.
    #[must_use]
    pub fn total_revenue(&self) -> i64 {
        self.revenue.total()
    }

    /// Total expenses, minor units.
    #[must_use]
    pub fn total_expenses(&self) -> i64 {
        self.expenses.total()
    }

    /// The period's result: revenue minus expenses. Positive is profit.
    #[must_use]
    pub fn result(&self) -> i64 {
        self.total_revenue() - self.total_expenses()
    }
}

/// The balance sheet, including the period's retained result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BalanceSheet {
    /// The currency.
    pub currency: String,
    /// Asset accounts at normal-side net.
    pub assets: StatementClassTotals,
    /// Liability accounts at normal-side net.
    pub liabilities: StatementClassTotals,
    /// Equity accounts at normal-side net.
    pub equity: StatementClassTotals,
    /// The period's result, still sitting in the revenue and expense
    /// accounts — the income statement's `result`, carried here because
    /// mid-period it is not in equity yet.
    pub retained_result: i64,
}

impl BalanceSheet {
    /// Derive the balance sheet for `currency` from the trial balance and the
    /// P&L of the same cut-off.
    #[must_use]
    pub fn from_trial_balance(tb: &TrialBalance, currency: &str) -> Self {
        let pl = IncomeStatement::from_trial_balance(tb, currency);
        let mut lines_for = |class: AccountType| StatementClassTotals {
            class,
            lines: tb
                .rows_for(currency)
                .into_iter()
                .filter(|r| r.account_type == class)
                .map(|r| {
                    let net = if r.account_type.increases_on_debit() {
                        r.net()
                    } else {
                        -r.net()
                    };
                    (r.account_id.clone(), r.account_name.clone(), net)
                })
                .collect(),
        };
        Self {
            currency: currency.to_string(),
            assets: lines_for(AccountType::Asset),
            liabilities: lines_for(AccountType::Liability),
            equity: lines_for(AccountType::Equity),
            retained_result: pl.result(),
        }
    }

    /// Total assets, minor units.
    #[must_use]
    pub fn total_assets(&self) -> i64 {
        self.assets.total()
    }

    /// Total liabilities, minor units.
    #[must_use]
    pub fn total_liabilities(&self) -> i64 {
        self.liabilities.total()
    }

    /// Total equity as posted, before the period's result.
    #[must_use]
    pub fn total_equity(&self) -> i64 {
        self.equity.total()
    }

    /// Whether the sheet balances: assets equal liabilities plus equity plus
    /// the retained result.
    ///
    /// This is the trial balance's per-currency balance restated in the
    /// reporting vocabulary, so it holds whenever the underlying ledger does —
    /// and when it does not, the correct response is to stop, not to plug a
    /// figure into an "unexplained difference" line.
    #[must_use]
    pub fn is_balanced(&self) -> bool {
        self.total_assets() == self.total_liabilities() + self.total_equity() + self.retained_result
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use double_entry::money::currencies;
    use double_entry::{Account, JournalEntry, Line};

    fn ledger_with_trading_month() -> Ledger {
        let mut ledger = Ledger::new();
        for (id, name, kind) in [
            ("1000", "Current account", AccountType::Asset),
            ("1100", "Trade debtors", AccountType::Asset),
            ("2100", "VAT output", AccountType::Liability),
            ("3100", "Owner's equity", AccountType::Equity),
            ("4000", "Consulting revenue", AccountType::Revenue),
            ("5000", "Bank fees", AccountType::Expense),
            ("5100", "Software", AccountType::Expense),
        ] {
            ledger.add_account(Account::new(id, name, kind));
        }
        let mut v = 0u64;
        // Opening capital £5,000.
        v = ledger
            .post(
                &JournalEntry::draft(
                    "JE-OPEN",
                    "2026-07-01",
                    "2026-07",
                    vec![
                        Line::debit("1000", double_entry::Amount::minor(500_000, currencies::GBP)),
                        Line::credit("3100", double_entry::Amount::minor(500_000, currencies::GBP)),
                    ],
                ),
                v,
            )
            .expect("opening posts");
        // Sale £1,200 + VAT.
        v = ledger
            .post(
                &JournalEntry::draft(
                    "JE-SALE",
                    "2026-07-02",
                    "2026-07",
                    vec![
                        Line::debit("1100", double_entry::Amount::minor(144_000, currencies::GBP)),
                        Line::credit("4000", double_entry::Amount::minor(120_000, currencies::GBP)),
                        Line::credit("2100", double_entry::Amount::minor(24_000, currencies::GBP)),
                    ],
                ),
                v,
            )
            .expect("sale posts");
        // Fees £15 and software £40.
        ledger
            .post(
                &JournalEntry::draft(
                    "JE-COSTS",
                    "2026-07-09",
                    "2026-07",
                    vec![
                        Line::debit("5000", double_entry::Amount::minor(1_500, currencies::GBP)),
                        Line::debit("5100", double_entry::Amount::minor(4_000, currencies::GBP)),
                        Line::credit("1000", double_entry::Amount::minor(5_500, currencies::GBP)),
                    ],
                ),
                v,
            )
            .expect("costs post");
        ledger
    }

    #[test]
    fn the_pl_shows_revenue_costs_and_profit() {
        let ledger = ledger_with_trading_month();
        let tb = TrialBalance::from_ledger(&ledger, "2026-07-31");
        let pl = IncomeStatement::from_trial_balance(&tb, "GBP");
        assert_eq!(pl.total_revenue(), 120_000);
        assert_eq!(pl.total_expenses(), 5_500);
        assert_eq!(pl.result(), 114_500);
        assert_eq!(pl.revenue.lines.len(), 1);
        assert_eq!(pl.expenses.lines.len(), 2, "both cost accounts, itemised");
    }

    #[test]
    fn the_balance_sheet_balances_with_the_retained_result_in_its_place() {
        let ledger = ledger_with_trading_month();
        let tb = TrialBalance::from_ledger(&ledger, "2026-07-31");
        let sheet = BalanceSheet::from_trial_balance(&tb, "GBP");

        // Assets: bank 494,500 + debtors 144,000.
        assert_eq!(sheet.total_assets(), 638_500);
        // Liabilities: the VAT collected.
        assert_eq!(sheet.total_liabilities(), 24_000);
        // Equity as posted: the opening capital.
        assert_eq!(sheet.total_equity(), 500_000);
        // The month's profit is not in equity yet — it is the retained result.
        assert_eq!(sheet.retained_result, 114_500);
        assert!(
            sheet.is_balanced(),
            "assets {} vs L {} + E {} + R {}",
            sheet.total_assets(),
            sheet.total_liabilities(),
            sheet.total_equity(),
            sheet.retained_result
        );
    }

    #[test]
    fn the_pl_result_is_the_balance_sheets_retained_result() {
        // One trial balance, two statements: if the profit on the P&L did not
        // appear on the balance sheet, the sheets would disagree, and this is
        // the bug the shared derivation makes inexpressible.
        let ledger = ledger_with_trading_month();
        let tb = TrialBalance::from_ledger(&ledger, "2026-07-31");
        let pl = IncomeStatement::from_trial_balance(&tb, "GBP");
        let sheet = BalanceSheet::from_trial_balance(&tb, "GBP");
        assert_eq!(pl.result(), sheet.retained_result);
    }

    #[test]
    fn a_loss_is_a_negative_retained_result_and_still_balances() {
        let mut ledger = Ledger::new();
        for (id, name, kind) in [
            ("1000", "Current account", AccountType::Asset),
            ("3100", "Owner's equity", AccountType::Equity),
            ("4000", "Revenue", AccountType::Revenue),
            ("5000", "Fees", AccountType::Expense),
        ] {
            ledger.add_account(Account::new(id, name, kind));
        }
        let mut v = ledger
            .post(
                &JournalEntry::draft(
                    "JE-OPEN",
                    "2026-07-01",
                    "2026-07",
                    vec![
                        Line::debit("1000", double_entry::Amount::minor(100_000, currencies::GBP)),
                        Line::credit("3100", double_entry::Amount::minor(100_000, currencies::GBP)),
                    ],
                ),
                0,
            )
            .expect("opening posts");
        v = ledger
            .post(
                &JournalEntry::draft(
                    "JE-SALE",
                    "2026-07-02",
                    "2026-07",
                    vec![
                        Line::debit("1000", double_entry::Amount::minor(20_000, currencies::GBP)),
                        Line::credit("4000", double_entry::Amount::minor(20_000, currencies::GBP)),
                    ],
                ),
                v,
            )
            .expect("sale posts");
        ledger
            .post(
                &JournalEntry::draft(
                    "JE-COSTS",
                    "2026-07-09",
                    "2026-07",
                    vec![
                        Line::debit("5000", double_entry::Amount::minor(45_000, currencies::GBP)),
                        Line::credit("1000", double_entry::Amount::minor(45_000, currencies::GBP)),
                    ],
                ),
                v,
            )
            .expect("costs post");

        let tb = TrialBalance::from_ledger(&ledger, "2026-07-31");
        let pl = IncomeStatement::from_trial_balance(&tb, "GBP");
        assert_eq!(pl.result(), -25_000, "a loss");
        let sheet = BalanceSheet::from_trial_balance(&tb, "GBP");
        assert_eq!(sheet.retained_result, -25_000);
        assert!(sheet.is_balanced(), "a loss balances too, or nothing does");
    }

    #[test]
    fn a_foreign_currency_row_never_reaches_the_statement() {
        let mut ledger = Ledger::new();
        for (id, name, kind) in [
            ("1000", "Current account", AccountType::Asset),
            ("4000", "Revenue", AccountType::Revenue),
        ] {
            ledger.add_account(Account::new(id, name, kind));
        }
        ledger
            .post(
                &JournalEntry::draft(
                    "JE-GBP",
                    "2026-07-02",
                    "2026-07",
                    vec![
                        Line::debit("1000", double_entry::Amount::minor(50_000, currencies::GBP)),
                        Line::credit("4000", double_entry::Amount::minor(50_000, currencies::GBP)),
                    ],
                ),
                0,
            )
            .expect("posts");
        ledger
            .post(
                &JournalEntry::draft(
                    "JE-USD",
                    "2026-07-02",
                    "2026-07",
                    vec![
                        Line::debit("1000", double_entry::Amount::minor(60_000, currencies::USD)),
                        Line::credit("4000", double_entry::Amount::minor(60_000, currencies::USD)),
                    ],
                ),
                1,
            )
            .expect("posts");

        let tb = TrialBalance::from_ledger(&ledger, "2026-07-31");
        let pl = IncomeStatement::from_trial_balance(&tb, "GBP");
        assert_eq!(pl.total_revenue(), 50_000, "the USD sale stays out of the GBP P&L");
        let sheet = BalanceSheet::from_trial_balance(&tb, "USD");
        assert_eq!(sheet.total_assets(), 60_000);
    }
}

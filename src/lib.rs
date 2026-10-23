//! Reporting over a `double-entry` ledger: the trial balance and aging.
//!
//! # The rules the reports obey
//!
//! **Per currency, by construction.** A trial balance is a per-currency
//! statement: summing a GBP bank account with a JPY fee account produces a
//! number in no currency. The double-entry invariant is stated per posting,
//! so "debits equal credits" is checked per currency, and a ledger balanced in
//! GBP and unbalanced in JPY is reported as exactly that.
//!
//! **The accounting equation is a runtime check, not an article of faith.**
//! `assets + expenses == liabilities + equity + revenue`, per currency, at any
//! cut-off — it holds without closing entries because the period's result is
//! still sitting in the revenue and expense accounts. A violation means the
//! report was built over something that is not a balanced ledger; the correct
//! response is to stop, never to fudge a row.
//!
//! **Aging buckets by how overdue, in the item's own currency.** "Current"
//! means not yet due — the due date, not the issue date, is what ages. Mixing
//! currencies into one grand total is a category error the bucket structure
//! refuses.
//!
//! **Drafts and future entries are invisible.** Only posted entries effective
//! on or before the cut-off count; reversed entries contribute nothing because
//! their counter-entry does the work. The cut-off is a `YYYY-MM-DD` string
//! comparison, which is exact for that fixed-width format.

mod trial_balance;

pub use trial_balance::{TrialBalance, TrialBalanceRow};

mod aging;

pub use aging::{AgedItem, AgingBucket, AgingReport};

mod statements;

pub use statements::{BalanceSheet, IncomeStatement, StatementClassTotals};

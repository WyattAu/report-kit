# Changelog

All notable changes to this project are documented here. Format: [Keep a
Changelog](https://keepachangelog.com/) — versions follow [semver](https://semver.org).

## [Unreleased]

## [0.2.0] - 2026-10-07

### Added

- `IncomeStatement` — revenue and expenses itemised per account, and the
  period's result.
- `BalanceSheet` — assets, liabilities, posted equity, and the period's
  **retained result** carried where it actually is. A balance sheet drawn
  mid-period does not satisfy `assets == liabilities + equity` because the
  profit is still sitting in the revenue and expense accounts; the honest
  statement is `assets == liabilities + equity + retained result`, and the
  retained result is the P&L's result by construction — one trial balance,
  two statements, so a profit on one that is missing from the other cannot be
  expressed.
- `is_balanced` refuses to plug an "unexplained difference" line: the sheet
  balances or it does not.


## [0.1.0] - 2026-10-07

Initial release.

### Added

- `TrialBalance::from_ledger` — every `(account, currency)` pair named by a
  posted line, at a cut-off date. `is_balanced` checks per currency; the
  accounting equation (`assets + expenses == liabilities + equity + revenue`)
  is a runtime check, per currency.
- `TrialBalanceRow::is_on_normal_side` — abnormal balances (overdrafts) are
  findable, not hidden.
- `AgingReport` — receivables bucketed by how overdue, aging from the *due
  date* (net-60 terms are current for two months), totals per currency.
- `days_between` via Hinnant's `days_from_civil`, pinned against leap years
  and century boundaries.

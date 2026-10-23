# Changelog

All notable changes to this project are documented here. Format: [Keep a
Changelog](https://keepachangelog.com/) — versions follow [semver](https://semver.org).

## [Unreleased]

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

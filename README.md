# report-kit

Trial balance and aging reports over a [`double-entry`](https://crates.io/crates/double-entry)
ledger. Per-currency by construction, with the accounting equation as a runtime
check.

## The rules the reports obey

**Per currency, by construction.** A trial balance is a per-currency statement:
summing a GBP bank account with a JPY fee account produces a number in no
currency. The double-entry invariant is stated per posting, so "debits equal
credits" is checked per currency, and a ledger balanced in GBP and unbalanced
in JPY is reported as exactly that.

**The accounting equation is a runtime check, not an article of faith.**
`assets + expenses == liabilities + equity + revenue`, per currency, at any
cut-off. It holds without closing entries because the period's result is still
sitting in the revenue and expense accounts. A violation means the report was
built over something that is not a balanced ledger; the correct response is to
stop, never to fudge a row.

**Aging buckets by how overdue, in the item's own currency.** The *due date*
ages, not the issue date: an invoice on net-60 terms is current for two months,
and aging it from issue would show a healthy customer as 60 days late.
"Current" means the promise is not yet broken. Buckets are the standard
escalation: current, 1–30, 31–60, 61–90, 90+.

**Drafts and future entries are invisible.** Only posted entries effective on
or before the cut-off count. Reversed entries contribute nothing because their
counter-entry does the work. The cut-off is a `YYYY-MM-DD` string comparison,
which is exact for that fixed-width format.

**Abnormal sides are findable, not hidden.** A credit balance on a bank account
is an overdraft; a real state. `TrialBalanceRow::is_on_normal_side` makes it a
predicate so a reader can find it, rather than a normalization that erases it.

## Dates

Day arithmetic is Howard Hinnant's `days_from_civil`, an exact integer
algorithm verified against known dates across leap years and century
boundaries; no float epoch differences.

## Licence

MIT OR Apache-2.0.

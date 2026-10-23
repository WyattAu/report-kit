//! Aging: receivables (or payables) bucketed by how overdue they are.
//!
//! The conventions that make an aging report comparable between businesses:
//!
//! - **The due date ages, not the issue date.** An invoice issued on net-60
//!   terms is current for two months; aging it from issue would show a healthy
//!   customer as 60 days late. Payment terms exist precisely so that the due
//!   date is the promise.
//! - **"Current" is not yet due.** It is not a bucket for "recently issued";
//!   it is a bucket for "no promise broken yet".
//! - **Totals are per currency.** A grand total across currencies is a number
//!   in no currency, and an exposure report in no currency is not an exposure
//!   report.
//!
//! Day arithmetic uses Howard Hinnant's `days_from_civil` — the same exact
//! integer algorithm the reconciliation side uses — pinned against known dates
//! across leap years and century boundaries rather than a float epoch
//! difference. (If date arithmetic grows again — week rules, durations — it
//! graduates to a shared crate; two pure functions with reference tests do not
//! justify one yet.)

/// How far past due an item is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AgingBucket {
    /// Not yet due: the promise has not been broken.
    Current,
    /// 1–30 days past due.
    Days1To30,
    /// 31–60 days past due.
    Days31To60,
    /// 61–90 days past due.
    Days61To90,
    /// More than 90 days past due: the bucket collection policy lives in.
    Over90,
}

impl AgingBucket {
    /// Every bucket, in escalation order.
    pub const ALL: [Self; 5] = [
        Self::Current,
        Self::Days1To30,
        Self::Days31To60,
        Self::Days61To90,
        Self::Over90,
    ];

    /// Which bucket an item falls in, given its due date and the report date.
    #[must_use]
    pub fn for_days_overdue(days_overdue: i64) -> Self {
        if days_overdue <= 0 {
            Self::Current
        } else if days_overdue <= 30 {
            Self::Days1To30
        } else if days_overdue <= 60 {
            Self::Days31To60
        } else if days_overdue <= 90 {
            Self::Days61To90
        } else {
            Self::Over90
        }
    }
}

/// One open item in the aging population.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgedItem {
    /// The item's identifier — the invoice number, typically.
    pub id: String,
    /// The open amount in minor units, never negative. Credits and payments
    /// reduce the open amount before it gets here; the aging population is
    /// what is still owed.
    pub amount: u64,
    /// The ISO 4217 code.
    pub currency: String,
    /// The date payment was promised, `YYYY-MM-DD`.
    pub due_on: String,
}

/// The aging report at a date.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgingReport {
    /// The report date, `YYYY-MM-DD`.
    pub as_of: String,
    items: Vec<AgedItem>,
}

impl AgingReport {
    /// An empty report as of `as_of`.
    pub fn new(as_of: impl Into<String>) -> Self {
        Self {
            as_of: as_of.into(),
            items: Vec::new(),
        }
    }

    /// Add an open item to the population.
    pub fn add_item(&mut self, item: AgedItem) {
        self.items.push(item);
    }

    /// The per-bucket totals in one currency, in escalation order.
    #[must_use]
    pub fn buckets(&self, currency: &str) -> [(i64, u64); 5] {
        let mut totals = [(0i64, 0u64); 5];
        for item in &self.items {
            if item.currency != currency {
                continue;
            }
            let days_overdue = days_between(&item.due_on, &self.as_of).unwrap_or(0);
            let bucket = AgingBucket::for_days_overdue(days_overdue);
            // `ALL` and the totals array are both fixed-length 5; the position
            // is computed from the same constant, so a failure to find it is a
            // programming error this crate refuses to turn into a panic inside
            // a report.
            if let Some(index) = AgingBucket::ALL.iter().position(|b| *b == bucket) {
                if let Some(slot) = totals.get_mut(index) {
                    slot.0 += 1;
                    slot.1 += item.amount;
                }
            }
        }
        totals
    }

    /// The open total in one currency: everything, regardless of bucket.
    #[must_use]
    pub fn total(&self, currency: &str) -> u64 {
        self.items
            .iter()
            .filter(|i| i.currency == currency)
            .map(|i| i.amount)
            .sum()
    }

    /// The item count in one currency.
    #[must_use]
    pub fn count(&self, currency: &str) -> usize {
        self.items.iter().filter(|i| i.currency == currency).count()
    }

    /// The currencies in the population, sorted.
    #[must_use]
    pub fn currencies(&self) -> Vec<String> {
        let mut seen: Vec<String> = self.items.iter().map(|i| i.currency.clone()).collect();
        seen.sort();
        seen.dedup();
        seen
    }
}

/// Days from `a` to `b` (`b - a`) as an exact integer, or `None` on a
/// malformed date.
#[must_use]
pub fn days_between(a: &str, b: &str) -> Option<i64> {
    Some(days_from_civil(parse_date(b)?) - days_from_civil(parse_date(a)?))
}

/// `(year, month, day)` from `YYYY-MM-DD`.
fn parse_date(s: &str) -> Option<(i64, u32, u32)> {
    let mut parts = s.split('-');
    let year: i64 = parts.next()?.parse().ok()?;
    let month: u32 = parts.next()?.parse().ok()?;
    let day: u32 = parts.next()?.parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || parts.next().is_some() {
        return None;
    }
    Some((year, month, day))
}

/// Days from 1970-01-01 (Hinnant's `days_from_civil`).
#[must_use]
pub fn days_from_civil((y, m, d): (i64, u32, u32)) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (i64::from(m) + 9) % 12;
    let doy = (153 * mp + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn item(id: &str, amount: u64, due_on: &str) -> AgedItem {
        AgedItem {
            id: id.into(),
            amount,
            currency: "GBP".into(),
            due_on: due_on.into(),
        }
    }

    #[test]
    fn the_due_date_ages_not_the_issue_date() {
        // Net-60 terms: issued June 1st, due July 31st. On July 15th the
        // customer is *current*, despite the invoice being six weeks old.
        let mut report = AgingReport::new("2026-07-15");
        report.add_item(item("INV-1", 100_000, "2026-07-31"));
        let [current, ..] = report.buckets("GBP");
        assert_eq!(
            current,
            (1, 100_000),
            "net-60 terms are current at six weeks"
        );
    }

    #[test]
    fn buckets_escalate_by_how_overdue() {
        let mut report = AgingReport::new("2026-10-01");
        // Due 2026-10-02: one day in the future → current.
        report.add_item(item("A", 1, "2026-10-02"));
        // Due 2026-09-30: 1 day late.
        report.add_item(item("B", 10, "2026-09-30"));
        // Due 2026-08-25: 37 days late.
        report.add_item(item("C", 100, "2026-08-25"));
        // Due 2026-07-15: 78 days late.
        report.add_item(item("D", 1_000, "2026-07-15"));
        // Due 2026-06-20: 103 days late.
        report.add_item(item("E", 10_000, "2026-06-20"));
        // Due 2026-05-15: 139 days late.
        report.add_item(item("F", 100_000, "2026-05-15"));

        let buckets = report.buckets("GBP");
        let expected: [(i64, u64); 5] = [(1, 1), (1, 10), (1, 100), (1, 1_000), (2, 110_000)];
        for (slot, (expected_count, expected_total)) in buckets.iter().zip(expected.iter()) {
            assert_eq!(slot.0, *expected_count, "bucket {expected_count:?} count");
            assert_eq!(slot.1, *expected_total, "bucket {expected_total:?} total");
        }
        assert_eq!(report.total("GBP"), 111_111);
    }

    #[test]
    fn currencies_never_merge_into_a_number_in_no_currency() {
        let mut report = AgingReport::new("2026-08-01");
        report.add_item(item("GBP-1", 5_000, "2026-07-01"));
        report.add_item(AgedItem {
            id: "JPY-1".into(),
            amount: 500_000,
            currency: "JPY".into(),
            due_on: "2026-07-01".into(),
        });
        assert_eq!(report.total("GBP"), 5_000);
        assert_eq!(report.total("JPY"), 500_000);
        assert_eq!(
            report.currencies(),
            vec!["GBP".to_string(), "JPY".to_string()]
        );
        // An unknown currency has an empty population, not a merged one.
        assert_eq!(report.total("USD"), 0);
    }

    #[test]
    fn day_arithmetic_survives_leap_years_and_century_boundaries() {
        assert_eq!(days_between("2026-07-01", "2026-07-01"), Some(0));
        assert_eq!(
            days_between("2024-02-28", "2024-03-01"),
            Some(2),
            "2024 is a leap year"
        );
        assert_eq!(
            days_between("1900-02-28", "1900-03-01"),
            Some(1),
            "1900 is not"
        );
        assert_eq!(days_between("2000-02-28", "2000-03-01"), Some(2), "2000 is");
        assert_eq!(
            days_between("2026-10-01", "2026-07-01"),
            Some(-92),
            "signed"
        );
        assert_eq!(days_between("nonsense", "2026-07-01"), None);
        assert_eq!(days_between("2026-13-01", "2026-07-01"), None);
    }

    #[test]
    fn bucket_boundaries_are_inclusive_on_the_named_edge() {
        assert_eq!(AgingBucket::for_days_overdue(0), AgingBucket::Current);
        assert_eq!(AgingBucket::for_days_overdue(-5), AgingBucket::Current);
        assert_eq!(AgingBucket::for_days_overdue(1), AgingBucket::Days1To30);
        assert_eq!(AgingBucket::for_days_overdue(30), AgingBucket::Days1To30);
        assert_eq!(AgingBucket::for_days_overdue(31), AgingBucket::Days31To60);
        assert_eq!(AgingBucket::for_days_overdue(60), AgingBucket::Days31To60);
        assert_eq!(AgingBucket::for_days_overdue(61), AgingBucket::Days61To90);
        assert_eq!(AgingBucket::for_days_overdue(90), AgingBucket::Days61To90);
        assert_eq!(AgingBucket::for_days_overdue(91), AgingBucket::Over90);
        assert_eq!(AgingBucket::for_days_overdue(365), AgingBucket::Over90);
    }
}

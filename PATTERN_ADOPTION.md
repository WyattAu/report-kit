# Pattern adoption

Declared per the pattern canon adoption format; every claim is verified against code in this repo.

| Pattern | Status | Evidence |
|---|---|---|
| T27 | partial | src/trial_balance.rs:175 (per-currency balance enforced at report build); the ledger itself is the double-entry crate |
| P12 | partial | src/lib.rs:5 (per currency by construction) + src/aging.rs:11 (cross-currency totals refused) |
| T13 | adopted | src/trial_balance.rs:72 (report is a pure function of ledger and cutoff) |

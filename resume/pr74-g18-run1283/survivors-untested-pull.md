# Pull survivors from the cases CI never tested (run 1283)

Run on the coordinator's machine at 969493e1 with CI's mutation flags, in place, so the uid
65534 tests run. The full pull run is DONE; the ledger is /tmp/claude-0/mut-untested/pull.done.

MISSED at 969493e1 (owner: Gate 18 rest fixer). Re-check each one on pr74/g18-rest: fea3659
says ist_day is killed (2/2), and 445ad02 targets check_day.
- crates/pull/src/daycheck.rs:67:42: replace + with - in ist_day
- crates/pull/src/ingest.rs:2504:5: replace check_day with ()
- crates/pull/src/ingest.rs:2509:23: replace match guard report.clean() with true in check_day

# Pull survivors from the cases CI never tested (run 1283)

Run on the coordinator's machine at 969493e1 with CI's mutation flags, in place, so the uid
65534 tests run. The run is in progress; the ledger is /tmp/claude-0/mut-untested/pull.done
plus pull/mutants.out.

MISSED so far (owner: Gate 18 rest fixer; send on RESUME at 12:12):
- crates/pull/src/daycheck.rs:67:42: replace + with - in ist_day

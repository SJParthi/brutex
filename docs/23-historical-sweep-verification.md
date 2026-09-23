# Actual historical sweep verification — 6 September 2026

The new commands ran successfully against copied real Zerodha index records
for May 2025. This is bounded historical execution and persistence evidence,
not an exhaustive Boolean-language run, institutional approval or cost-inclusive
profit claim. The subsequent backend activation is recorded separately below.

## Build and input provenance

The separate clean checkout `/private/tmp/brutex-sweep-verification-20260906`
is at commit `33ecdfba5285d4943816415edf20ddb1d06cf36a`. The snapshot helper
copied and compared 567 current source files, including necessary existing
shared changes without attributing them to this task. It committed only the
isolated checkout; the shared files/index were unchanged. Ordinary build
provenance passed. No commit override bypassed it.

The fixture is `target/sweep-audit-20260906/market-fixture`. Existing April/May
NIFTY/BANKNIFTY 1-minute, 5-minute and daily files plus CRC sidecars were copied
under each source month lock and compared. Ordinary stored readers applied
their existing header, record, lazy block-checksum and calendar checks. The
banner still correctly reports no separate independent checksum-scrub receipt.
No vendor request, production-store mutation, service restart or trade occurred
in these historical commands. Separately, the frontend build wrote workspace
static assets; the live service returned that asset version while the new
candidate/search endpoints returned 404. The later authorized backend activation
closed that route mismatch; no untouched-bundle claim is made.

## Actual results

| Check | NIFTY 1-minute | BANKNIFTY 5-minute |
|---|---|---|
| Signal / actual minute execution bars | 7,875 / 7,875 | 1,575 / 7,875 |
| Signals dropped by existing causal projection | 21 | 19 |
| Invocations | Eight candidates, then resume eight more | Eight candidates, then resume eight more |
| Saved expressions / catalogs | 16 / 16 | 16 / 16 |
| Signal rows | 122,800 | 22,000 |
| True / false / unknown rows | 52,487 / 70,313 / 0 | 9,413 / 12,587 / 0 |
| Support qualifiers at MIN_HITS=1 | 15 | 15 |
| Full trade files independently checked | 32: 16 long and 16 short | 32: 16 long and 16 short |
| Exact trade rows reconciled | 20,989 | 10,290 |
| Selected cells / explicit no-cell outcomes | 30 / 2 | 30 / 2 |
| Cell-rule-admitted outcomes | 0 | 0 |
| Grammar status | Paused, not exhausted | Paused, not exhausted |
| Independent reader | Five bounded pages, 18 exact links | Five bounded pages, 18 exact links |
| Writer / unfinished reservations observed | No / 0 | No / 0 |

Both searches use live alphabet `0,369`, horizon five execution minutes, one
grid rung and step 1,000 ppm. The complete existing cell policy is printed in
each log and bound into identity. Every report says cost-excluded, unvalidated
research. Zero unknowns in these two-bit examples does not imply every indicator
is always available.

Search identities:

- NIFTY: `d89f300660d6d423784ef09b578bd79041b200f361754f10ed44c9face3d0475`
- BANKNIFTY: `3aefdc68868e30f9269fb25d521095e45f0814d4f7e89342d4953ed20d14ccdc`

The native-minute AND run used MIN_HITS=7,000 and an explicit
8,388,608-candidate ceiling (`BRUTEX_CEILING` is a count, not a byte limit).
It swept 7,675 rows after 200 warm-up rows, found seven frequent singletons and
six frequent pairs, and reached an empty k=3 frontier. All 13 retained results
were below the statistical bar. Its repeat recovered checkpoint 3, preserved
depth counts and reused parent row 0 without duplicating the saved run.
Its identity is
`10df0283b61c84ff44dcd7e3a7243842fd7441ae4cdf8b0ef31ecd7320caf367`.

## Independent saved-evidence check

A standalone Rust inspector was compiled with warnings denied against that
clean CLI library. It walked learned checkpoint continuations, checked exact
descending ordinals and reconciled signal counts. It decoded/re-encoded full
fixed expression programs, compared catalog/program identities, opened both
complete side files and checked every trade's sequence, causal/exclusive
coordinates and financial reconciliation. It did not rerun market pricing or
mint institutional authority.

Its separate inspection digests are:

- NIFTY: `0f6c8591614ff668203181f5008b54740b95863b720f017ece8d12cf6b74cec6`
- BANKNIFTY: `46cc587e5cc86a1238af0dc8a02e85c5d8e7011ec89d313a4df03ae8bb6344d9`

These corruption-detection digests are not signatures or replacements for the
original search/catalog/trade identities.

Exact local logs under `target/sweep-audit-20260906/` are
`clean-source-snapshot.log`, `clean-historical-build.log`,
`market-fixture/copy-evidence.md`, `market-native-first.log`,
`market-native-resume.log`, `market-coarse-first.log`, `market-coarse-resume.log`,
`market-native-inspection.log`, `market-coarse-inspection.log`,
`market-and-first.log` and `market-and-resume.log`. The fixture's
`logs-and-resume/events.ndjson` records actual checkpoint recovery.

## Authorized live backend activation

On 6 September 2026 at 12:20 UTC, the operator-authorized service handoff
installed a byte-verified, separately named copy of the clean snapshot API
binary (17,582,024 bytes). The existing service label, runtime arguments, store,
masters, frontend directory and output logs were preserved. The original binary
remains available. The first immediate submission raced asynchronous launchd
removal and left no listener; resubmitting after removal completed started PID
99582. Successful submission alone was not counted as deployment verification.

| Live check after activation | Actual response |
|---|---|
| `/expression-search.json` with the fixture search ID | HTTP 200, explicit `missing`; the production store does not contain the separate fixture's search |
| `/candidate-trades.json` with the fixture ID/attempt/model | HTTP 200, explicit `missing`; absent evidence is not reported as measured zero |
| `/_app/version.json` | HTTP 200, version `42d5b294db871d0e`, matching the existing rebuilt workspace assets |
| `/autopilot.json` | HTTP 200, automatic scheduler remains `paused` |
| `/pull/run.json` | HTTP 200, existing 34,440-window recovery plan resumed; stored row count remained 282,845,758 at the observed checkpoint |

The recovery resume uses its existing durable journal and retry reservation
semantics. Process-local progress counters restart and are not durable retry
budgets. No new recovery plan or historical production sweep was requested.
These HTTP checks establish route activation and observed service continuity,
not a visual browser certificate or a completed production search. The real
fixture's full search/candidate/trade API-handler probe independently checked
10 search pages, 64 candidate pages, 282 trade pages and all 31,279 trades;
four wrong-pin requests returned HTTP 503 with no accepted rows.

Full institutional execution still requires the approved 37 admission values
and exact training/OOS/VIX evidence. Required coverage, mutation closure,
visual verification and customer-scale measurements remain separate. Finite success
does not guarantee every history, failure schedule or expression permutation.

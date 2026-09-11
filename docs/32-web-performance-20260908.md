# DB response costs, Backtest controls and NIFTY search scope

This report covers the real Brutex application at `http://127.0.0.1:8080`.
It records page work and execution admission separately from full historical
research clearance. No full sweep, vendor pull or trade was started for these
checks. The isolated source candidate is
`17e192301c4a049bbbf701812bae0f46d8746f53` for the native executable.
The subsequent frontend-only corrections are packaged separately; the final
delivery receipt records their build version and source identity.

## What the mode selector means

| Mode | Plain meaning | Parallel work for NIFTY alone | Exit settings |
|---|---|---|---|
| AND discovery | Find conditions that are true together | Selected timeframes and candidate pricing use workers | The selected ordinary grid and policy apply |
| Receipt-checked batches | Require source evidence and retain an outcome for each selected job | A bounded queue of instrument/timeframe jobs; not a promise that every job starts together | Native resolved grid and evidence requirements apply |
| AND / OR / NOT research | Explore Boolean expressions, then compare fixed settings on a later period | Timeframes are currently sequential; instrument-family parallelism provides one family worker for NIFTY | Both directions and the observed-data exit grid are included |

A work allowance can pause a Boolean search at a durable checkpoint. It is not
proof that every possible expression has been exhausted. The selector chooses
the rule language and evidence workflow; it is not a speed or profit selector.

## NIFTY input inventory and timing

The real Zerodha census captured on 8 September contains these NIFTY counts.
Every intraday rung has 82 monthly entries spanning December 2019 through
September 2026. These are stored inventory counts, not complete-session or
checksum certification. The partial current month remains a partial month.

| Timeframe | Stored candles |
|---|---:|
| 1 minute | 627,296 |
| 2 minutes | 314,315 |
| 3 minutes | 208,982 |
| 5 minutes | 125,385 |
| 10 minutes | 63,522 |
| 15 minutes | 41,787 |
| 30 minutes | 21,723 |
| 60 minutes | 11,692 |
| All eight resolutions | 1,414,702 |

Coarser candles overlap the same market history; the total is not a count of
independent observations. Another 1,681 daily candles provide reference context
and are not swept. The intraday 15:10 IST forced-exit rule remains unchanged.

There is **no measured full-history completion-time estimate**. For Boolean
research, evaluated coordinates equal the number of evaluated programs times
the sum of long and short exit cells across all eight rungs. The default
five-step exit policy has a source-derived unfiltered upper bound of 1,566
cells per direction, or 25,056 coordinates per program across eight rungs.
Actual observed ties and ratio checks can reduce that number; it is a sizing bound,
not an observed NIFTY grid or an elapsed-time estimate.

The engine must also prepare history, price entries/exits, perform later-period
replay and statistical checks, and durably save evidence. The total grammar,
actual cells and sustained processing rate are not measured. A new read-only
native descriptor provides a mathematical lower bound instead: the current
328-condition full Boolean alphabet includes more than **5 x 10^98** distinct
plain conjunctions per timeframe. The mixed grammar is larger still. It does
not use the AND ladder's support pruning, and its cumulative V1 counters cannot
represent that full population. Full exhaustion in hours is not feasible.
This lower bound counts syntax, including expressions that cannot yield a
viable trade; it is not a number of profitable setups or an execution benchmark.
The M4 Pro has 14 CPU cores and 48 GiB memory; its disconnected 8 TB drive adds
no current capacity. These specifications do not supply the missing runtime
measurements.

The operator subsequently chose a separate NIFTY/BANKNIFTY consistency
workflow with **one defined risk stop plus the 15:10 exit**, replacing a broad
exit-grid search in that workflow. The earlier 25,056-coordinate sizing bound
describes the existing grid workflow, not that requested single-stop design.
Its execution integration and resulting real workload must be verified before
the new workflow is run. Removing exit variants does not make all Boolean
expressions feasible to exhaust.

## Index consistency and candidate integration

| Requirement | Exact meaning | Current evidence boundary |
|---|---|---|
| Winning days | At least 60% of all eligible days; add all a setting's trades before classifying its day | A winning-trade percentage is not a winning-day percentage |
| Complete weeks | Each complete Mon-Fri five-session week needs at least 3 wins and at most 2 losses | Holiday-short and boundary-partial weeks are separately reported |
| Losing streak | At most 2 losing days; a third fails even across week boundaries | The conservative versioned rule requires a winning day to reset; flat/no-trade days cannot hide intervening losses |
| Multiple trades | Permitted; their daily total supplies one day result | No one-trade-per-day restriction or fixed rupee example is added |
| Institutional checks | Preserved and assessed separately; both result sets must pass | A new consistency result cannot replace failed or unmeasured institutional evidence |
| Source and execution | Real OHLCV, intraday only, gross amounts, 15:10 deadline | No synthetic market observations or net/account-profit claims |
| Historical records | Existing records remain visible with their original evidence | They are not retrospectively labeled as having passed the new policy |
| Selected timeframes | Identity and stored physical indices preserve the exact selected subset | The candidate API passed 50 tests; full combined release and live deployment remain separate |
| Top 0.1% | A rank needs an explicitly identified evaluated population | An incomplete grammar search cannot establish the strongest 0.1% of every possible expression |

This section records the active candidate work, not a claim that the running
8080 application already contains every new feature. The final delivery receipt
must identify the exact native and frontend source used by that application.

## Page changes and measured baseline

| Area | Before | Implemented behavior | Boundary |
|---|---|---|---|
| Zerodha census | 40,664,512 bytes and 148,222 rows per full response | Versioned lossless tuple transport; bytes and digest reused for an unchanged snapshot | Cold census work and retained bytes grow with inventory |
| Feed counts | Full inventories downloaded by the DB survey | Shared HEAD-only requests, at most two concurrent | A cold manifest read still has a cost |
| Small bar page | 109,944 bytes / 1,440 rows from a real stored month | The same eight OHLCV rows required 1,261 bytes using native addressed paging | Global sorting and extrema requests have their own larger work |
| Repeat DB navigation | About 1.46 seconds between navigation and the first bar request in the recorded baseline | Reuse one completed immutable preparation and avoid deep row proxies | Changed snapshots require preparation again |
| Departed pages | Some polls and queued work survived navigation | Owned requests, cancellation and stale-response rejection across the inspected routes | Up to six started Terminal shared-cache reads may finish |
| Disabled Backtest actions | An old unknown lifecycle result could disable a fresh request indefinitely | Historical outcome and current store execution admission are separate, with a visible reason | Active, unfinished, corrupt or unreadable evidence still refuses |
| App identification | A native app was labelled as a legacy server | Explicit native capability response | Capability is not release certification |

The eight-row comparison checked every timestamp, OHLCV and open-interest value
against the full native month response. Its single-request server timings were
0.676 ms and 1.125 ms respectively; they are observations, not percentile or
worst-case latency guarantees. Live measurements after deployment are recorded
separately below.

## Live measurements after the changes

The same 148,222 real Zerodha inventory rows were decoded from both native
formats and compared field by field. They were exactly equal. The compact
body was **9,298,287 bytes**, down from **40,664,512 bytes: 77.13% less data**.
Five already-cached local reads took 6.33–8.48 ms including transfer; a HEAD
count request returned no body in 2.02 ms and an unchanged conditional request
returned HTTP 304 with no body in 0.58 ms. These few observations are not a
latency percentile, cold-disk benchmark or worst-case guarantee.

The actual in-app browser also showed the following. These observations were
taken while native regression tests were running, and include browser overhead.

| Browser action | Observed result |
|---|---|
| Initial compact inventory read | 93.2 ms network transfer, 9,298,631 bytes including response headers |
| Cold feed-count survey | First two HEAD reads 89–97 ms; subsequent reads 0.6–0.8 ms, without downloading those inventories |
| Warm return to DB | No inventory GET; reused the completed preparation |
| Three-row stored-candle page | About 1.0–1.3 ms network transfer, 661–663 bytes including headers |
| Next DB page | Real rows 4–6 were rendered, with timestamp and OHLCV values |
| Ingest instrument list on a fresh load | 149.2 ms, 879 instrument records; the page rendered its 50 selected series |

An earlier isolated Ingest request exceeded its 15-second deadline. It did not
recur in the fresh browser or direct API checks, and its cause is not established.
The timeout message now reports that uncertainty; a deadline alone cannot prove
that the server accepted a connection, became wedged or encountered a locked
store. No timeout was increased and no automatic submission retry was added.

## Page and launch checks

| Page | Live evidence or correction | What this does not establish |
|---|---|---|
| Markets | Real NIFTY chart and stored month counts render | Data completeness or trading approval |
| Ingest | Instrument selector and stored coverage render; isolated timeout recorded above | Vendor recovery or missing history repaired |
| Autopilot | Native civil-date bounds are retained; counts select the exact feed, timeframe and window, keep empty months visible and withhold completion without exact target membership | Scheduler resumed or completed; partial-month bar counts beyond the census evidence |
| DB | Paged candles, next-page navigation and warm census reuse verified | Every sort or full-inventory operation is O(1) |
| Whole | Instrument/range controls render; source-minute-only check remains explicit | A full calendar audit was run |
| Audit | Stored generation and totals render; prose distinguishes store activity from execution state | A quiet store proves an idle or completed process |
| Mapping | 136 rows render; filtering for NIFTY works; 40 unresolved mappings remain visible | Unresolved master identities are repaired |
| Backtest | Research mode selector is available; ordinary launch and current admission are separate from historical unknown status | Boolean launch prerequisites or release gates are cleared |
| Terminal | Stored prices and 17 visible stock rows render; bounded request ownership was tested | A live price feed or unsupported ranking features exist |
| Logs | Durable events, sink health and explicit read refusals render | No other subsystem can fail |

The narrow in-app viewport also exposed navigation links hidden behind header
controls. The responsive header now gives navigation its own usable row. It
does not change the desktop navigation or research scope.

The native app's capability response identifies `mode: application` and permits
the application routes, while **`release_cleared` remains false**. The current
Boolean configuration has no production checksum-receipt location or its byte
and record limits. Search allowances, horizon and max-points settings remain
unresolved; policy resolution requires the chosen max-points. A later validation
period must follow training, while the initial training selection currently
spans the whole stored period. These are visible input checks, not reasons to
bypass them. Existing degraded master identity and recovery-state limitations
also remain. The ordinary saved-run ledger is absent; separate saved Boolean
research is not evidence that an ordinary sweep has run.

## Verification and limits

The native source passed **5,233 tests across 99 top-level targets, with zero
failures and 11 ignored tests**. Nested child-fixture summaries are excluded
from that count. Rust formatting, exact-source strict workspace Clippy and
dependency policy checks passed. The first complete frontend verification
passed 643 tests, its type check with zero errors and zero warnings, and its
production build. The final delivery receipt records the subsequent narrow
header, date projection and wording checks and final frontend totals.

New checks cover compact/expanded equality, vendor/format isolation, shared
cache identity, concurrent cold publication, corruption, HEAD behavior,
conditional reads, page cancellation, A-to-B-to-A changes, detached polling,
missing minute input, execution leases, incompatible store roots and ambiguous
submissions. Tests use private fixtures where fault injection is required;
those fixtures are not historical market observations.

Full touched-crate 100% line/branch coverage and complete mutation closure have
not been established. Entire search exhaustion, profitable selection and total
O(1) time or space have not been established. O(1) indexed operations and bounded
page reads do not make a growing inventory or exhaustive search constant-size
work. Historical release limitations remain in `docs/31-backtest-integration-20260908.md`.

Evidence is retained in `target/sweep-audit-20260908/web-performance/`, including
the NIFTY corpus table, source response digest, test logs and native build
receipts. Shared-page lifecycle findings are in
`target/sweep-audit-20260908/page-latency/shared-page-audit.md`.

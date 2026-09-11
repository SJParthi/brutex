# Zerodha pull reconciliation — 2026-09-06

This is an evidence report, not a trading assurance or a certificate that every
historical candle exists. The authorized scope is NSE spot, Zerodha, daily and
one-minute data, 2019-12-01 through 2026-09-04 inclusive. The frozen selection
contains 208 current F&O cash underlyings plus NIFTY and BANKNIFTY; FINNIFTY,
MIDCPNIFTY and NIFTYNXT50 are excluded. Current membership and cross-checked
identity are not point-in-time historical membership/identity proof.

## Measured results

| Check | Request/run | Result | What it establishes |
|---|---|---|---|
| Full daily replay | 1788662086032, full authorized date span | 210/210 reached; 337,082 rows matched; zero new commits; zero diagnostics | Returned daily records agree with storage |
| Recent minute replay before month-context fix | 1788663308052, August 22–September 4 | 210/210 reached; 756,389 rows matched; 1,666 diagnostics | Source acquisition worked; derived context was incomplete |
| ASIANPAINT corrected replay | 1788663993724, August 22–September 4 | 3,600 rows matched; zero new commits; zero diagnostics | Earlier-month eligibility context fixes the observed cash replay conflict |
| All210 corrected minute replay | 1788664019400, August 22–September 4 | 210/210 reached; 756,389 rows matched; zero new commits; 16 diagnostics, all DALBHARAT | Recent replay completed; remaining source gaps are explicit |
| DALBHARAT targeted repeat | 1788664646533, August 27–28 | 734 rows returned and matched; zero dropped; zero new commits; 16 diagnostics | Narrow retry did not recover the expected missing minutes |
| Actual saved NSE masters | Two explicitly invoked local-evidence tests | Both passed; 25 dated masters, exact identities for 208 cash stocks | Local schema, receipts and current identity checks pass |
| Physical census scrub | 146,752 entries | No missing files, count/bound disagreement or unreadable entries | Header/boundary agreement, not full interior or market-coverage proof |

The all210 corrected replay finished at 2026-09-06T03:16:54.453Z, telemetry
sequence 7784832, taking 595.051 seconds. The targeted repeat finished at
03:17:28.633Z, sequence 7784871, taking 2.102 seconds. Both returned HTTP 200
with an explicitly unclean coverage verdict; HTTP status alone is insufficient.

## Remaining dated gaps

| Instrument/date | Expected missing range (IST) | Repeat evidence |
|---|---|---|
| DALBHARAT, 2026-08-27 | 15:15–15:29, 15 minutes | 360 stored/returned minutes, 09:15–15:14 |
| DALBHARAT, 2026-08-28 | 12:43, one minute | 374 stored/returned minutes; adjacent observations 12:42 and 12:44 |

Its exact EQ/ISIN record in both dated NSE masters has closing-auction
eligibility zero, so the current schedule expects the regular continuous
session. No alias, fabricated candle, or other feed has been substituted.
The 16 diagnostics happen to accompany 16 absent minutes, but are **not** a
count of failed stocks: there are two request-gap reports and fourteen derived
bucket reports. One affected instrument must not render as sixteen members.

## Fixes and limits

- Calendar authority is extended only through September 4, backed by the
  primary sources in the charter and the measured index grids. Future dates
  remain unverified.
- A partial-month minute replay restores eligibility for actual committed
  source dates outside the request suffix, checking saved receipts and the
  exact symbol/ISIN. Missing/corrupt evidence refuses without rewriting bars.
- Failure diagnostics must distinguish missing source/context from proven
  complete-source byte conflicts. A versioned correction is not a substitute
  for obtaining missing evidence.
- The original source store is preserved. The additive revision facility is
  not automatically promoted into production readers.
- Full historical minute coverage remains uncertified. FORCEMOT's documented
  non-trading interval and IRFC's minute-only pre-daily months remain separate
  evidence issues; see the existing local reconciliation audit and charter.
- The backend changes are Rust. Indexed per-operation costs do not imply
  constant total time or space for downloading, storing or scanning more rows.

## Verification checkpoint

After the month-context fix, the full workspace run passed 4,458 tests with
zero failures and nine ignored. The two ignored local-master tests were then
invoked explicitly and passed. Dependency advisory, ban, license and source
checks passed. Final regression of subsequent diagnostic/status wording is
recorded separately when completed. No 100% line/branch coverage or complete
mutation/power-loss testing is claimed.

Receipts are preserved in the local store's `audit` directory as
`zerodha-daily-full-recheck-20260906.html`,
`zerodha-all210-month-context-20260906.html`, and
`zerodha-dalbharat-targeted-retry-20260906.html`.

## Final deployed checkpoint

The subsequent combined diagnostic/status fixes passed the full workspace:
**4,466 tests passed, zero failed, nine ignored**. The API build succeeded,
formatting and whitespace gates passed, and the existing idle local server
was restarted with the tested binary. Workspace Clippy passed for this tree.
No source data, credentials, or unrelated working-tree changes were removed.

Live DALBHARAT run 1788665574014 repeated the same two dates: 734 returned
and matched, no drops or new commits, and the same 16 diagnostics. The new
receipt correctly labels the count `Failure diagnostics` and asks for complete
source evidence before derivation, not a storage revision alone. During that
request, `/autopilot.json` showed `state: paused`, `pull_active: true`, and
the correct one-stock progress, proving automatic scheduling and independent
pull activity remain distinct.

Final NIFTY/BANKNIFTY September-4 run 1788665621965 reached 2/2, matched 750
minutes, added zero duplicates and finished with zero diagnostics. Afterwards
the server reported `pull_active: false` and `now: null`. The completed-run
monitor was paused, not left to retry a completed basket indefinitely.

The residual two DALBHARAT source gaps still require corrected provider
evidence; repeated requests did not supply it. Contacting the provider or
changing the authorized feed is a separate external action, not performed
here. Full historical minute coverage and point-in-time identity remain
uncertified; neither passing tests nor these recent-window replays resolve
those evidence limits. The report does not claim the entire requested history
is complete.

## Recovery and audit deployment — subsequent checkpoint

The following additional fixes are implemented and running on the existing
local API server. The server was idle before restart; broad autopilot remains
paused. This checkpoint did not launch a duplicate full-basket vendor request.

| Area | Earlier failure mode | Implemented behavior and proof |
|---|---|---|
| Retry scheduling | A failed minute request could repeatedly replay clean legs | Clean request receipts checkpoint their legs during the retry cycle; prerequisites, sibling failures/panics, stops and fresh full passes have coordinator regressions |
| Cash minute expectations | Generic or peer clocks could misclassify auction-eligible stocks | Exact dated stock/ISIN evidence selects the same continuous-session clock used by ingestion |
| Missing files | Opening failure could hide a known expected-minute obligation | The file error and independently known expected/lost minutes are both retained |
| Invalid stored timestamps | Flooring an off-grid timestamp could conceal a defect or create false later holes | Off-grid, duplicate and backward rows have an explicit invalid count and cannot certify coverage |
| Coverage page | Missing evidence or unusable totals could look complete | File faults, timestamp defects, evidence errors, truncation and unknown dates take precedence over green status |
| Source timeframe | Coarse bars could be supplied to a minute audit | Daily/coarse requests refuse; the page selects only one-minute source data |

Measured live comparisons after deployment:

| Stock/month | Expected continuous minutes | Stored rows read | Missing | Invalid/unreadable rows | Dated evidence |
|---|---:|---:|---:|---:|---|
| ASIANPAINT / August 2026 | 7,560 | 7,560 | 0 | 0 | Verified local receipts |
| DALBHARAT / August 2026 | 7,605 | 7,589 | 16 | 0 | Verified local receipts |

The two stocks legitimately have different session totals. The browser was
checked directly: DALBHARAT renders `SHORT`, naming August 27 15:15–15:29 and
August 28 12:43. The daily-as-minute request returned HTTP 400 with an explicit
explanation. Neither check writes source bars or obtains vendor data.

Verification: the integrated workspace run passed **4,483 tests, zero failed,
nine ignored**. A subsequent API-only run after the worker's last test addition
passed **883 tests, zero failed, one ignored**. Frontend regressions passed
**299 tests**, Svelte checks reported zero errors/warnings, and the production
frontend build passed. Rust formatting, whitespace, workspace Clippy, API build
and offline dependency advisory/license/ban/source checks passed. An earlier
concurrent worker run reported a timing-sensitive deduplication scaling test
failure (3.41×); the main workspace run and isolated rerun passed. No timing
threshold was relaxed to obtain those results. Full Rust branch/line coverage,
mutation completeness and physical power-loss guarantees were not measured.

Receipts: `zerodha-dalbharat-stored-audit-after-recovery-20260906.json`,
`zerodha-asianpaint-stored-audit-after-recovery-20260906.json`,
`recovery-workspace-tests-20260906.log`, and `recovery-api-tests-20260906.log`
in the local store's audit directory. Original records and earlier receipts
remain unchanged. The compiler's dirty-tree warning concerns canonical sweep
run persistence; it does not certify or invalidate the source-pull receipts.

Limits remain explicit: checkpoints are process-local, not restart-durable or
per-symbol chunk planning. An unchanged partial leg still retries under the
existing 400-pass ceiling; that mechanism is not being run against the known
unchanged source gap. Calendar checks cannot establish historical listing or
suspension intervals. The dated missing source records still require genuine
new source evidence, and the entire historical minute series is not certified
complete by these fixes or tests.

### Deployed-path source retry and bounded follow-up

A further two-date DALBHARAT request through the deployed server again read
734 matching rows, wrote zero new rows, dropped zero rows, and retained sixteen
failure diagnostics. The handler measured 2.362 seconds. Its HTTP-200 receipt
explicitly says `FAILED` and `Balances: NO`; transport success is not mistaken
for coverage success. The receipt is saved as
`zerodha-dalbharat-recovery-deployed-retry-20260906.html`. An initial missing-Origin
request was refused before any provider contact or source write; that separate
preflight response was also preserved, not counted as a vendor attempt.

The existing `monitor-brutex-data-pull` heartbeat was updated and verified
ACTIVE, every six hours. It is now limited to those two DALBHARAT dates, with
one idle-server request per check and a durable maximum of three unchanged
no-growth follow-ups before pausing and reporting the unresolved source
boundary. Terminal credential/refusal outcomes pause it sooner. It must not
repeat the entire basket, mint credentials, change feeds, restart an active
server or promote a storage revision. Any apparent correction must be checked
against actual committed source/derived evidence before reporting repair.
This follow-up is not a promise that the provider will correct its history.

## Full historical source-minute reconciliation

The deployed endpoint was read for the exact saved 210-symbol selection and
2019-12-01 through 2026-09-04. It returned all **17,220 symbol-months**. Per-day
gap summaries exclude September 5–30; file/record counters retain month
granularity. The read-only audit started at 04:09:36Z and its final sequential
rechecks ended at 04:19:24Z, 588 seconds later. Thirteen initial metadata-lock
contention responses cleared on recheck; both responses are preserved.

| Measurement | Result | Meaning |
|---|---:|---|
| Selected instruments reconciled | 210 / 210 | Exact selection matches the saved authorization |
| Stored one-minute records read | 125,479,603 | Successful record reads, not a certificate of market completeness |
| Invalid timestamps / unreadable records | 0 / 0 | No such defects reported by the complete read pass |
| Remaining dated-evidence errors / truncated reports | 0 / 0 | After the thirteen contention rechecks |
| Absent month files | 748 across 28 symbols | 735 precede the first stored month; not proof of listing dates |
| Other absent months | 13 | Three FORCEMOT months in its documented interruption; ten IRFC months before its stored daily history |
| Calendar-expected absences | 6,215,709 | Includes unproved listing lifetimes/halts; **not** 6,215,709 proved vendor omissions |
| Unmeasured symbol-days within authorized dates | 1,262 | Calendar/event evidence does not cover every requested date |

NIFTY has 11 and BANKNIFTY 56 calendar-expected absent minutes. Cash histories
also have interior absences. It would therefore be false to describe the
sixteen recent DALBHARAT minutes as the only unresolved point in the entire
history. Of the total calendar-expected absences, 5,798,980 fall in absent files;
those include leading histories that cannot be assumed to exist. The detailed
table deliberately does not convert these diagnostics into a blind pull queue.
Historical identity, listing lifetimes, actual halts and dated session evidence
are still required before a full-market-coverage certificate can be issued.

The complete per-stock table, CSV, date-level absences, metadata, original
responses and rechecks are preserved in the local store's audit directory:
`zerodha-fullspan-minute-audit-20260906-041936Z/`. Its `overall.json` reports
an exact selection match and all response-counter checks passing.

## Targeted historical index replays — 2026-09-06, subsequent follow-up

The remaining recorded index-gap dates were submitted individually through the
existing Rust `/pull/spot` handler, using Zerodha only, the explicit selected
members, `granularity=1min`, and inclusive one-day windows. This did not enable
the broader automatic scheduler or change the frozen basket or date range.

| Request date | Selected members | Returned rows | Newly written bars | Failure diagnostics | Receipt verdict |
|---|---|---:|---:|---:|---|
| 2022-03-07 | NIFTY, BANKNIFTY | 697 | 0 | 41 | FAILED |
| 2023-06-05 | NIFTY, BANKNIFTY | 748 | 0 | 42 | FAILED |
| 2023-06-14 | NIFTY, BANKNIFTY | 736 | 0 | 42 | FAILED |
| 2025-12-15 | BANKNIFTY | 374 | 0 | 8 | FAILED |

All four responses were HTTP 200 and all explicitly reported `Balances: NO`.
Each named its journal recording, including every failure diagnostic. The
diagnostics include month-wide derived-bucket checks, so their counts are not
counts of missing source minutes or failed instruments. The 2,555 returned
rows did not add any bars. These replays establish no repair and no global
claim that every provider response has been exhausted.

Independent read-back of `/gaps.json` still reports 47 expected absent minutes
for BANKNIFTY in March 2022 and eight for NIFTY in June 2023, with neither
response truncated. Those monthly counts are not a new market-calendar
certification: the endpoint retains its separately reported peer/calendar
evidence limits. Other unresolved history, including the recent DALBHARAT
gaps, is not erased by these narrower checks.

An initial malformed comma-joined member field was refused with HTTP 400
before contacting a vendor or writing source bars; its receipt is preserved
separately. The corrected requests used one `member` field per instrument.
The unrecognized initial `timeframe` field was also replaced by the actual
`granularity` field; the accepted receipts explicitly name `1min`.

The request receipts, HTTP headers, and read-back JSON are retained together
under `audit/zerodha-index-gap-replays-20260906/`. No source history was
overwritten or fabricated, and no revision was promoted. This follow-up does
not complete the outstanding historical eligibility or restart-safe recovery
work described above.

The parallel read-only coordinator review confirms that the bar store is
durable but recovery state is not. `pullrun::Checkpoints` is one in-memory bit
per submitted leg, which can cover a whole basket and date range. Startup
creates fresh progress. HTTP-200 FAILED/PARTIAL remains retryable and can
repeat unchanged through 400 passes; the three-clean-idle-pass rule does not
bound that failure path. The V1 audit journal lacks a typed instrument/feed/
timeframe work identity and committed-source progress, so its truncated
messages are not a safe restart queue. Neither a manifest row count nor its
last timestamp proves interior coverage.

A restart-safe correction therefore requires typed per-instrument/window work
identity, versioned durable attempt/completion records in the existing audit
machinery, a persisted per-work-unit no-growth limit, and replay through the
existing feed coordinator. Publishing that state after actual source/manifest
completion must be tested across the interrupted-write boundary. This remains
implementation work, not a capability supplied by this diagnostic follow-up;
even its completion cannot manufacture source observations not returned by the
provider. No code change or new test result is claimed in this follow-up.

The separate historical-evidence review establishes three absent FORCEMOT
month files (2023-11 through 2024-01) as within the documented NSE interruption
in charter §9. The other 745 absent files across 27 symbols remain unverified,
not automatically proved provider omissions or pre-listing exemptions. The
saved September 4 NSE security-master header was checked directly and contains
`ListgDt`, `RmvlDt`, and `RadmssnDt` at columns 35–37. The current
`cash_auction::DailyEligibility` parser retains identity and auction eligibility
but does not retain these lifecycle fields. Connecting dated lifecycle evidence
to historical obligations is unfinished application work. A current listing
date alone must not silently exempt earlier records or resolve rename,
readmission, or identity conflicts.

## Recovery implementation follow-up — 2026-09-06

The diagnostic findings above are preserved as the earlier measured state.
D-0522 now implements an explicit Rust `/pull/recovery` route instead of using
the legacy in-memory, whole-basket retry loop. The frozen launch request has
210 unique members and both source rungs over the exact authorized dates:
34,440 symbol/timeframe/month windows. This is the selected current snapshot,
not a claim to have reconstructed the historical F&O membership universe.

| Earlier limitation | Implemented recovery behavior | Remaining limit |
|---|---|---|
| Run state vanished on restart | Versioned, synced plan and activation journals | Torn/corrupt evidence refuses; it is not silently repaired |
| A new or overlapping plan could repeat requests | Shared exact-day reservation ledger, at most three coordinator attempts | Previous legacy receipts are not retrospectively counted as this new policy's reservations |
| A crash could lose Stop intent | Separate append-only Stop/explicit-clear history | Uncertain persistence returns an error rather than acknowledging a durable Stop |
| HTTP 200 looked like progress | Source readback, new-bar counts and diagnostics remain separate | The provider may return the same incomplete source again |
| Unproved missing history invited blind backfill | Requests require the symbol's own dated source evidence and qualified mapping/calendar evidence | Unknown listing/rename/halts and unsourced sessions remain unverified |
| A done banner could imply all data exists | Separate verified, not-applicable, unverified and missing/blocked window totals | Schedule verification is not full historical identity or coverage proof |

The dated NSE lifecycle reader now retains and binds the current snapshot's
listing/removal/readmission evidence, but does not turn that snapshot into a
historical certificate. It preserves contradictory older source observations.
The existing independently sourced FORCEMOT interruption stays narrowly bound
to its exact identity and dates. Source bars are neither deleted nor fabricated,
and storage revisions are never automatically promoted by this recovery path.

The recovery-focused suite passed 50 tests, including the journal's interrupted
writes and sync failures, shared budgets, stop-before-seed/restart, HTTP start
acknowledgement, mapping scope, source-session checks and escaped dashboard
output. This result is not a full-workspace gate, branch-coverage or mutation
claim. Deployment and actual acquisition results are recorded separately below
only after they occur.

## Tested deployment — 2026-09-06

The final sequential verification on this shared checkout completed with
4,579 passed tests, zero failed and ten ignored across 81 test summaries.
`cargo fmt --check`, strict workspace/all-target Clippy, the locked API build,
and the dependency advisory/license/source checks passed. This does not
establish 100% line/branch coverage or mutation closure; those remain
unmeasured for this delivery.

The tested API executable was copied before other tasks resumed Cargo work to
`/Users/parthi/brutex-local/bin/api-recovery-9b5f9a4e96a7ce0d`.
Its source and deployed SHA-256 both read
`9b5f9a4e96a7ce0df4b511c8fe1d6f45ad94cc3745b5bb2ccc25bb858829b3a7`.
Only the verified idle `com.brutex.codex.api` service was replaced. The new
process owns the sole localhost:8080 listener; broad autopilot and archive
suggestions remain paused, and the local store/master roots are unchanged.
The recovery HTML page and JSON endpoint were checked after startup.

The first frozen-plan submission was refused before parsing with HTTP 413:
the 10,154-byte double-encoded form exceeded the unchanged 8 KiB form limit.
The submission was compacted to 7,565 request bytes without changing either
decoded leg, its 210 explicit repeated member values, either timeframe, or
the inclusive dates. The original rejection and both forms are preserved;
the server limit was not widened. A run slot claimed during journal
preparation is not evidence of a vendor request or of completed data coverage.

Build/test logs and deployment request evidence are retained under
`/Users/parthi/brutex-local/store/audit/recovery-deployment-20260906-9b5f9a4e96a7ce0d/`.
The CLI dirty-checkout warning still disables stored **sweep** run identity
stamping. No commit stamp was invented or changed to bypass it; this
deployment concerns source ingestion and does not certify the separate
sweep work as ready.

### Frozen plan accepted and running

At 2026-09-06 07:51:44 UTC (13:21:44 IST), the compact submission returned
HTTP 202 with `started: true`, `windows: 34440`, and plan identity
`a531870c6182442cf88630f18bac395552842ac88bab17877765f9ad1c78ac0f`.
This acknowledgment follows the synced plan seal and activation pointer.
The first captured live status showed 3,725 completed scan windows, currently
auditing COALINDIA 1day for November 2022, `running: true`, no reported worker
error and no Stop request. The recent-event sample contained real Verified
and Unverified assessments, not a full-plan aggregate or historical coverage
certificate. Both source timeframes belong to the same saved plan; daily
windows are assessed before one-minute windows.

The initial status reported an unchanged process-wide census. That count
includes other stored series and is not the requested scope's source-bar
count. No newly downloaded bar is claimed from that initial observation.
The server-owned worker, not a browser loop, now owns the recovery. The
existing follow-up monitor is read-only and will not submit duplicate plans,
clear Stop, reset attempt budgets, switch feeds or mint a credential.

### One-minute stage entered automatically

The next captured status showed `legsDone: 17220` and
`Audit 360ONE · 1min · 2019-12-01..2019-12-31`: all daily scan windows had been
processed, and the same worker had entered the minute half without another
submission. This counts assessments, not 17,220 fully proved windows.
The shared attempt journal was non-empty. The first recorded exact-day
request, 360ONE/1min/2019-12-02, returned HTTP 200 with zero committed new bars,
five minutes still missing, `unchanged: 1`, and state Queued for its remaining
bounded attempts. Its 4,292 diagnostics are a separate reported quantity,
not new bars or additional vendor requests. The response does not establish
that the source gaps were filled. No credential or worker failure was reported
in that captured live status; recovery was still running.

The minute-stage live status and event receipt are retained alongside the
deployment evidence. The dashboard explicitly separates new bars, missing
observations, unverified obligations and diagnostics. Its recent 100 events
are not an aggregate completeness report; final reconciliation remains pending.

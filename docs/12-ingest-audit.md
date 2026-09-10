# Instrument and minute-ingest repair — 2026-09-05

This is an implementation and verification record, not a certification of vendor
history. Tests use fixtures, not vendor pulls. Metadata recovery and local server
deployment are recorded below. Current universe membership is not point-in-time
membership.

## Latest status — spot daily and minute scope

- D-0515 through D-0518 fixes are implemented in the local server. Explicit
  Zerodha native-token plus independently corroborated ISIN mapping passed for
  the requested 210 members. This does not establish historical identity or
  membership. The default strict vendor-ISIN policy has not been relaxed.
- Full workspace regression before final receipt-only corrections: 4,403
  passed, zero failed, seven ignored. Strict workspace Clippy, dependency and
  formatting checks passed. After those corrections: API 858 passed, zero
  failed, one ignored; API all-target Clippy and API build passed. Frontend:
  284 tests passed, type check zero errors/warnings, build passed.
- Live minute preflight passed mapping then returned 422 for unresolved dated
  cash CAS eligibility, before contacting a vendor. Daily is exempt from that
  intraday-phase guard. The receipt now uses the selected identity policy,
  actual timeframe and feed-specific range semantics, not stale default counts.
- At 23:47 IST on 2026-09-05, the server accepted and began the exact Zerodha
  daily-only request: 208 cash equities plus NIFTY and BANKNIFTY, 2019-12-01
  through 2026-09-04 inclusive. The three excluded indices stay excluded.
  Telemetry run 1788632232048 records committed daily writes, including ABB
  1,360 + 321 bars. Store census rose from zero while the job was running.
  This is progress, not full-window coverage certification. Minute not started.
- The existing five-minute thread monitor now follows this job read-only,
  with local telemetry and audit receipts. The unrelated broad autopilot stays
  paused. The coordinator is process-local, not a crash-resumable job ledger.
- Remaining limits: dated cash CAS eligibility, historical identity/listing
  history and final per-member coverage outcomes. `/health` remains DEGRADED
  for broader catalogue/default-policy discrepancies. 100% line/branch coverage
  and zero surviving mutants have not been demonstrated. No fabricated minutes
  or blanket O(1) total-runtime/space guarantee is claimed.

The sections below preserve the sequence of findings and earlier measurements;
their open statuses are superseded only by the specific fixes described above.

| Risk | Repair | Evidence / limit |
|---|---|---|
| Conflicting duplicate instrument rows silently selected | Preserve assertions and withhold ambiguous vendor IDs | `master::tests::duplicate_conflicting_assertions_survive_loading_and_refuse_lookup` |
| Alias resolution selects an ID by input order | Deterministic merge and explicit ambiguity | `merge` regression tests |
| A feed borrows another feed's ISIN | Per-vendor ISIN evidence and crawl export | `server::tests::crawl_rows_never_borrow_another_vendors_isin` |
| Damaged or missing feed replaces live catalogue | Preserve previous snapshot and report reload failure | `server::tests::master_reload_preserves_snapshot_on_a_damaged_or_missing_feed` |
| Concurrent refreshes publish in reverse order | Serialize the application's fetch-to-reload transaction | `mastersrun::tests::refresh_requests_wait_before_starting_the_next_fetch`; external writers are not serialized |
| Valid header conceals malformed data rows | Validate rows before replacement; use declared mirrors on invalid success | `mastersrun::tests::malformed_success_uses_the_mirror_without_retrying_the_bad_payload` |
| Missing minute looks like a complete derived candle | Check scheduled minute coverage before publication | `pull::fold::complete_minutes`; unknown calendar coverage is not invented |
| Split request commits an unfinished derived bucket | Derive from committed source month, withholding incomplete buckets | `resumed_partial_bucket_matches_one_shot_bytes` |
| Duplicate broker candles inflate volume | Identical duplicates counted once; conflicting duplicates refuse | `broker_duplicates_do_not_double_volume_and_conflicts_are_refused`; archive snapshots retain their separate semantics |
| Requested basket silently shrinks to mastered rows | Refuse before instrument attempts and name missing members | `mapping_preflight_never_silently_shrinks_the_requested_basket`, `mapping_preflight_refuses_before_any_instrument_is_attempted` |
| Cash ticker maps to a different exchange identity | Verify selected ID against vendor-specific exchange-ISIN join | `mapping_preflight_accepts_only_the_selected_verified_cash_identity` |
| Refresh mixes listing class and ID | Shared resolver obtains both from one snapshot at the fetch boundary | `mapping_preflight_rechecks_the_current_snapshot_and_vendor`; indices tested separately |
| Entire closing buckets disappear before a later observed day | Report the prior observed session's missing tail without inventing bars | `an_absent_closing_bucket_is_named_before_the_next_observed_day`, all eight intraday widths |

## Performance boundary

Backend changes are Rust. Fixed arrays and positional record access remain
constant-size operations. Hash probes have expected, not universal worst-case,
constant time. Loading and validating N rows costs O(N); deterministic sorting
costs O(N log N). Reconciliation reads a committed month at an ingest boundary,
not once per source bar. Storing N records requires O(N) storage. No claim of
constant total latency, unlimited scalability, or exhaustive testing is made.

## Existing data and unresolved coverage

Previously stored partial bars are not silently overwritten. Historical gaps or
conflicts requiring insertion/replacement need a separately verified versioned
repair path. A checksum certifies bytes, not complete market history. A log of
an incomplete bucket does not turn that bucket into a trustworthy candle.

Exceptional sessions are withheld from the fixed-grid derivation rather than
published with misleading opening timestamps. Entirely absent days cannot be
proved from a slice containing no rows for those days: request-span coverage
must establish them separately. The new minute check does not certify whole-day
coverage, historical gap insertion, or exceptional-session grid migration.

The inspected local service had Dhan and Groww masters, no Zerodha master,
208 F&O cash identities per available feed, and 747/750 Total Market ISIN
matches. Those are dated observations, not permanent constants or proof of
historical coverage. No missing identity is guessed to make coverage appear full.

## Verification status

Verified on 2026-09-05:

| Check | Result |
|---|---|
| `cargo fmt --check` | Passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed |
| `cargo test --workspace --locked --no-fail-fast` | Passed |
| Broker and census regression rerun after test-helper extraction | 32 passed |
| `cargo deny check` | Passed; duplicate-dependency warnings remain |
| Nightly branch-coverage attempt over API and pull libraries | Recovered previous profiles using matching LLVM: API 88.85% lines / 68.10% branches; pull 87.83% lines / 70.03% branches. Below 100%, and predates the readiness changes |
| Mutation verification | Earlier readiness-check revision: 9 tested, 7 caught, 2 survived. Resolver and distinguishing assertions subsequently strengthened; no current full touched-module certification |
| Deployment and live vendor pull | Not performed as part of this repair |

The running service has not been restarted with these changes. The dirty-tree
CLI build warning remains: sweep-result persistence is disabled for that build;
it does not mean the pull store itself is disabled. Existing unrelated changes
were preserved rather than committed to suppress the warning.

Until all AGENTS.md section 9 checks pass, this work is not fully certified or
ready to be described as a guaranteed ingest solution.

## Dated closing-session correction (D-0512)

The regular derivatives schedule is now shared by source filtering and derived
candle completeness: 375 source minutes before 2026-08-03, 385 afterward.
Opening-stamped closing minutes are 15:29 and 15:39 respectively. Direct
boundary tests cover all venues and intraday widths; the stored-futures test
checks conservation of volume in every derived timeframe. The 27 targeted
anchor/derive tests passed, followed by full strict Clippy, formatting, and
dependency checks. The subsequent full-workspace regression run passed:
4,371 tests passed, zero failed, seven ignored. Later edits require their own
verification and do not inherit that full-run result.

This fixes derivatives completeness only. Cash CAS eligibility and auction
phase separation remain unresolved: the generic cash timetable cannot attest
that every selected share continues ordinary trading until 15:30. No live
vendor pull or server deployment was performed, and no source gaps were filled
with invented minutes.

## Purchase/import readiness — 2026-09-05

Audience: operator deciding whether to start a full Zerodha pull or paid archive import.
Scope: local Rust ingestion and mapping; no vendor pull, deployment or trading assurance.

| Decision | Evidence | Remaining gap / action |
|---|---|---|
| Start a full Zerodha cash-stock pull | Current master schema has no ISIN; D-0511 requires vendor-specific cash identity evidence | Do not start under an assertion of verified identity. Obtain a separately attributable mapping or explicitly revise the assurance policy with supporting evidence |
| Treat missing ISIN as missing token | A real-shaped Zerodha master regression resolves the token, then names missing identity evidence | New explicit diagnostic; six targeted preflight tests passed |
| Import the attached TrueData package | Inspected actual futures CSV has five fields; email describes a separate nine-field bid/ask product | Separate explicit product schemas; do not infer listing class from field count |
| Drop a ZIP into the current importer | Existing entry point reads extracted directories | Safe nested-ZIP inventory and routing remain unimplemented |
| Preserve everything purchased | Current candle path does not preserve every original quote field | Raw record format and provenance/checkpoint architecture remain open |
| Guarantee all cases | Earlier full regression passed; coverage below 100% and limited mutation run had survivors | No complete certification and no live deployment |

Primary evidence: [Kite instruments specification](https://kite.trade/docs/connect/v3/market-quotes/)
(accessed 2026-09-05); local `server::spot_vendor_identity`,
`mapping_preflight_zerodha_cash_names_missing_evidence_not_missing_id`,
`archive::read_dir`, and `server::run_local`; the two operator-provided ZIP
samples and the vendor email screenshot. Sample inspection was not an audit
of every CSV row. No seven-year completeness or volume-semantics claim follows
from sample format compatibility.

Correction to the initial sample diagnosis: the active plain TrueData FNO
descriptor already selected five fields using the index-named physical shape.
D-0513 introduces the explicit FNO shape and wires the descriptor to it; it
does not newly enable a previously unsupported field count. Full pull-crate
regression after this wiring: 748 passed, zero failed. Six API identity
preflight tests, full strict Clippy, formatting and whitespace checks passed.
No new full-workspace or coverage/mutation certification is claimed.

The archive sidecar confirmed that `read_dir` accumulates all decoded member
rows before ingestion, contrary to its one-file-memory comment. Peak memory
is not bounded to one file. Native ZIP support and durable per-member import
checkpoints remain absent; current resume replays source input and verifies
stored overlap. These are open implementation tasks, not resolved guarantees.

Live Zerodha metadata recovery, 2026-09-05: the first refresh returned 502
because the credential-path configuration was missing after the local-store
move. A symlink restored the reference to the existing preserved configuration,
without changing contents or secret values. The retry returned 200,
`complete=true`, `reloaded=true`, no missing masters. The Zerodha instrument
endpoint changed from 503/empty to 200/879 listed rows with master-state read.
No historical candles were fetched; candle census remains absent. Metadata
recovery does not waive the unresolved cash identity or CAS checks, nor deploy
the new code into the already-running server.

## Minute matching audit — 2026-09-05, remaining boundaries

The latest workspace regression completed with 4,381 passed, zero failed and
seven ignored. Strict workspace Clippy and dependency-policy checks passed.
This does not certify full minute coverage, coverage percentages or mutants.

| Boundary | Evidence/status |
|---|---|
| Complete minute-source buckets and dated venue closes | Covered by current fold/derive regression tests |
| Broker timestamps off the requested minute grid | OPEN: `fold_in_place` can round them before completeness checking; needs rejection before folding, without rejecting legitimate archive snapshots |
| Derivation from a source coarser than one minute | OPEN: `derive` uses ordinary folding and bypasses minute completeness/reconciliation |
| Entire missing closing interval at a month boundary | OPEN: derivation reads one month; the next observed month does not provide the tail witness |
| Entire absent days and final requested endpoint | Not attested by observed-bucket matching alone |

These three new findings are source-traced, not newly reproduced test failures.
Required regressions include shifted broker minutes, two distinct timestamps
inside one minute, sparse/split five-minute inputs, and July-end missing tails
followed by August data without treating the intervening weekend as a gap.
No historical import or deployment was performed as part of this audit.

Operator scope clarification: Zerodha underlying spot only, daily and
one-minute sources; no futures/options contract pull. Consequently the expired
contract archive limitation does not block this request. Cash-equity CAS
eligibility still differs from index sessions.

D-0515 adds broker timestamp rejection before rounding and a named refusal of
derived output from non-minute sources. The coarse-source regression passed;
expanded broker regressions and pull-crate checks are being rerun. Month-end
request-span attestation remains open. No readiness certification or deployment
is implied by these implementation changes.

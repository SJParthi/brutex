# Historical sweep readiness — evidence comparison, updated 7 September 2026

**The AND sweep, resumable Boolean search, exact evaluated-candidate pricing,
and Selection V6/replay paths are implemented. Full release readiness still
depends on the verification results and explicit configuration below.** Passing
functional tests does not imply full coverage/mutation closure or complete
production-market validation. A complete research policy is now supplied under
the operator's explicit delegation; it is not institutional certification. The earlier
`33ecdfba` backend was activated; later `502d42dc` repairs and the current
follow-up repairs are not running in that service.

Open the [searchable comparison](../web/sweep-readiness/index.html) for an easy
area-by-area view and the latest repeatable check results. The
[Rust verifier](../web/sweep-readiness/README.md) saves actual output, exit codes
and source-change checkpoints. A green check certifies only its named scope.
The [37-setting explanation](22-research-policy.md) provides one editable profile
and a searchable plain-language table. No manual calculation of those settings
is required. Saved checks from 6 September remain historical checkpoints and do
not clear the new source changes made on 7 September.

## Scope

Four agents worked on the highlighted path: vocabulary, conditions, masks,
indicators and candle patterns; AND search and explicit expressions; stored
historical runs, identities and evidence; institutional filter boundaries; and
sweep drill-down/status UI. Separate recovery/ingestion work was preserved.
Shared builds were coordinated to avoid interference; that work is not claimed
as a sweep implementation.

Initial HEAD was `dd784de4bdb16f70966171d06c0bf3eff5f52e02`. The shared checkout
already contained unrelated changes and remained dirty. No unrelated work was
reverted, committed or silently incorporated into a claim of clean provenance.
The production data store was not modified by these tests, and no production
historical sweep or trading action was started. After explicit authorization,
the clean tested API binary replaced the old running backend while preserving
its configuration and existing recovery job; observed continuity and HTTP
responses are recorded in the execution report.
Generated fixtures remain explicitly generated evidence. A separate clean
source snapshot subsequently ran the bounded real historical checks documented
in [the exact execution report](23-historical-sweep-verification.md).

## Launch decision at this review

**The complete institutional brute-force campaign is not cleared to start.**
The table separates implemented research capability, observed evidence and
remaining release conditions. Service and market-run observations below are
dated 6 September 2026; the policy follow-up is dated 7 September. This is not a
live service monitor or a percentage of all possible correctness.

| Area | What has been demonstrated | What still needs to happen |
|---|---|---|
| Intraday and 3:10 PM exit | Priced paths use the fixed 15:10 IST deadline. Eleven public regressions cover off-grid timestamps, missing exact boundary minutes, foreign caches and late interior grid records; five selected faults tested the earlier clock/table defenses | The recorded fill model remains Open/PrintedExtreme on the final permitted one-minute interval. OHLCV cannot establish a tick-exact price at 15:10. |
| Indicators and patterns | All328 live conditions have owners and an explicit availability path; all62 patterns have named examples. Session, trend and crossing repairs plus truth-only handoff now pass455 indicator tests and38 scoped mutations | Missing references remain Unknown. Pattern conventions, complete touched-crate coverage and input completeness retain their documented limits. |
| Opening ranges across timeframes | Independent generated oracle: 846 signal rows across all eight timeframes. Separate clean174435aa actual-data oracle: all6,173 saved2/3-minute expression rows matched, with192 differences from the reconstructed coarse calculation | Generated fixtures and actual-data evidence are separate scopes. A single60-minute month refused for insufficient warm-up. The bridge uses observed minutes and does not certify complete exchange-calendar coverage. |
| Combination search | AND search, nested AND/OR/NOT evaluation, finite independent oracles and restart paths exist. Binary grammar batches connect to complete eight-timeframe campaigns; D-0549 adds one exact testing allowance across a declared search | The fixed Boolean grammar is not an ordered-event language or exhaustive semantic Boolean deduplication. The new path needs final source-stamped producer/observer and combined verification; old per-batch results are unchanged. |
| Real historical execution | Clean build 174435aa: 18 strict range cases plus 18 retries passed, covering both indices across all eight intraday rungs and two cash-stock cases | Recorded checkpoint from 6 September 2026, using high minimum support; it does not establish full low-support search, subsequent-source verification or institutional approval. [All 18 cases and exact result reuse](../target/sweep-audit-20260906/strict-range-matrix-174435aa/matrix-report.txt). |
| Saved trade drill-down | Earlier clean-library probe: 64 saved candidate manifests and 40,257 trade rows were reconciled across 352 API pages | Historical bounded real-data check from 6 September 2026, [saved probe log](../target/sweep-audit-20260906/equity-api-trade-reconciliation-final.log). A summary or completed search cannot certify an unopened candidate page or later source edits. |
| Strict input integration | CLI, explicit API, receipt-checked browser batch and institutional V6 loaders now use retained strict source receipts; V6 retains guards through empty families and later OOS | The API still needs explicit server-owned receipt location and physical limits; the live job has not been configured or replaced. Browser batches are page-owned queues, not a durable aggregate server authority. |
| Empty and mixed institutional results | Eight strict integration tests pass, including both families evaluated, either family naturally extinct and both naturally extinct, through genuine Selection V6 and later evidence rechecking | Generated stored fixtures prove wiring and refusal behavior, not approval of production policy or real-market admission. Zero strategies requires completed measured search and cannot be substituted for missing input. |
| Institutional admission | Selection V6 and later replay paths are built for the two indices. On 7 September, a complete 37-value research profile and automatic validation were added under the operator's delegation | The profile is not yet activated in the service. New Boolean/cash catalog evidence does not grant Selection V6 authority; successor integration and release verification remain separate. |
| Verification | Named finite tests and faults are recorded below; the saved check panel records the latest combined run with exit codes, source checkpoints and exact logs | Test passes apply to their recorded source. Required full touched-crate line/branch coverage and whole-module mutation closure remain incomplete; saved green checks do not supply missing policy or activation. |
| Active service | Earlier 33ecdfba backend is responsive; data acquisition continues; automatic sweeps are paused | Latest repairs are not activated. Health reports DEGRADED with master/universe identity notes; production results ledger is absent. |
| Speed and scale | Fixed-width operations and finite host benchmarks have been measured | Whole search, saved history, storage latency and millions of customers have no O(1) or load guarantee. |

## Requirement compared with the current implementation

| Area | What is implemented or repaired | Exact boundary |
|---|---|---|
| Intraday only, fixed 3:10 PM exit | Shared minute execution uses an exact 15:10 IST deadline. Actual clock checks reject sub-minute offsets, foreign caches and late/foreign-day records inside a grid path before they can supply an exit | Missing required execution evidence refuses affected trades. Open/PrintedExtreme remains the recorded OHLCV fill rule; no invented tick or exact 15:10 price. [Seven original deadline regressions](../target/sweep-audit-20260906/fixed-deadline-public-tests.log) and [four cached-facts regressions in the runner suite](../target/sweep-audit-20260906/foreign-slice-deadline-runner-tests.log). |
| One vocabulary | One Rust table: 370 allocated positions, 328 live, 3 retired, 39 void; all six words are used | 384 fixed positions; append-only identities. VIX remains reference-only. |
| Every indicator owner | All 328 live positions mapped to their emitters; every one of the 62 candle-pattern bits has a named exemplar | [Full family and pattern table](15-indicator-readiness.md). Pattern conventions remain reproducible conventions where the source labels them UNVERIFIED. |
| Complete VWAP mapping | All 20 existing positions have aligned truth/known masks, including false bands and near predicates; NOT preserves unavailable references | Enabled for eligible cash stocks, absent for spot indices; futures remain storage-only. Zero-volume, two-contributor, zero-sigma, session and integer boundaries are explicit. [All 20 positions and real-row evidence](26-vwap-mapping.md). |
| Opening ranges on coarse timeframes | Stored opening ranges use the exact one-minute stream and the signal candle's exact final minute, alongside the existing gap overlay | A 60-minute high or a straddling 2/3-minute candle no longer defines the opening 5-minute range. Missing exact closing-minute or mismatched-close evidence refuses; opening windows use observed minutes, without certifying every internal minute. [Family and boundary evidence](15-indicator-readiness.md). |
| Extreme candle inputs | Positive-price boundary, structural refusal precedence and transactional state preservation; cold rollover cannot admit a newly unready mask | Finite shape, engulfing, translation, session-reset and integer-boundary tests. Not every possible price history. |
| Missing data and NOT | Truth is stored alongside a separate known mask; unknown is never negated into a signal. Session/history, swing/break and crossing repairs complete the previously missing family availability paths | Known false uses the same causal references as truth. Missing prior history, opposite swing, real gap, valid tolerance or same-session crossing memory still remains Unknown. [Availability table](15-indicator-readiness.md) names the exact boundaries and finite proofs. |
| AND combinations | Existing Apriori joins, exact subset pruning, documented implication pruning and extinction-driven depth remain shared | Unordered combinations, not ordered event permutations. Always-true/false and nonlive singletons retain their existing exclusion semantics. |
| Independent search oracle | 378 finite cases, 11 live positions spanning all six words, more than 10,000 survivors compared and depth 10 reached | An independent finite oracle, not a proof over all market data. |
| Impossible worker requests | `usize::MAX` is treated as a scheduling upper bound and safely limited to available parallelism | Same answer as one worker. Scheduling is not part of strategy meaning. |
| Allocation and worker failure | Fallible engine input copies/reservations; explicit Memory/Workers halts | Indicator column and some standard-library allocations remain infallible; OS overcommit is not universally catchable. |
| AND / OR / NOT expressions | Shared parser, three-valued evaluator and resumable canonical grammar enumeration, including nested operators and repeated leaves | Fixed V1 language with 1,151 instructions; finite syntax uniqueness, not algebraic Boolean equivalence or ordered event sequences. Candidate/node budgets pause and never claim exhaustion. [Search contract](22-expression-search.md). |
| Resume an AND sweep | The ordinary stored sweep persists each completed level and resumes through the shared engine/ranking path; restored survivors cannot contain explicitly excluded bits or impossible nonhalted pair counts | Same exact input and policy required; actual resource halts and their outer-row pair-budget overshoot remain unchanged. Checkpoint history grows with retained survivors. [Resume contract](20-sweep-resume.md). |
| Price Boolean strategies | The priced stored command runs the shared minute execution and exit grid, retaining both directions' selected-cell trades for each evaluated expression | Actual grid controls and complete cell rules bind identity; unknown predicates cannot enter. Support and cell-rule results are distinct from institutional admission. |
| Exact exit-grid setting | The shared stored policy now honors the declared percentile resolution; absent and explicit five retain the original policy. Each directional grid is already bound into the source/search identity | Invalid resolution and grids exceeding the fixed cell admission refuse. Grid size is separate from risk thresholds and capture budgets. A smaller verification grid cannot clear a refused larger declaration. [Runtime contract](30-search-wide-qualification.md). |
| No ignored legacy controls | Explicit Boolean preparation rejects legacy range controls that it does not use, before reading policy or market sources. The positional horizon and admission profile remain the actual authorities | Ordinary range controls remain supported on their own path. Names are reported without exposing values or private paths. [Settings and failure contract](30-search-wide-qualification.md). |
| Resume Boolean search | Saved cursor, counters, full child expressions and exact receipts are reauthenticated; each semantic transition is replayed | Full cold history verification is real work. Corrupt latest state, false exhaustion and lost priced children refuse; no silent older-state fallback. |
| All eight timeframes together | A durable Rust campaign records each expected family, complete candidate catalog, statistics and training-admission receipt across all eight intraday rungs. Same-request restart validates saved children before skipping completed work; the dashboard follows acknowledged progress automatically | A complete campaign records computation, not a passing strategy. Family workers share explicit physical allowances; timeframes run sequentially. Overview checks fixed receipts while detail routes authenticate saved bodies. [Campaign contract](28-incremental-boolean-campaign.md). |
| Grammar to complete exit grids | Binary AND/OR/NOT batches feed the full catalog across all eight timeframes without narrowing the grammar to the text parser. The next batch waits for the exact prior campaign; node/program allowances pause without claiming exhaustion | Retained history and source guards are checked before completion. Cold recovery and whole grammar work grow with inputs. Each batch/timeframe has its own statistical population; no global multiple-testing correction is claimed. [Grammar contract](28-incremental-boolean-campaign.md). |
| Frozen exits on later history | A separate later-period command retains every original program, direction and exit coordinate, including zero outcomes. Later prices replay training exits without selecting new levels. Pinned browser pages compare both periods and expose later trades and sessions | Strictly later sessions and the 15:10 IST deadline are enforced. This is observed later research, not anchored-fold acceptance, institutional successor selection or live trading authority. Its production routes have not been activated. [Later comparison contract](28-incremental-boolean-campaign.md). |
| Every live condition in one expression | V1 supports 1,151 instructions, enough for all 384 possible signed leaves; current 328 live bits fit together using numeric IDs | Source text limited to 4,096 bytes and parser nesting below 32; exceeding a bound refuses. |
| Full identity | Program bytes extend the existing params term; stored search/preparation/probe identities precede their computation | Clean verified commit provenance still required. A dirty build cannot launch a recorded market run. |
| Monthly audit policy | Monthly audit uses the shared full policy identity and actual minute execution floor; actual execution bytes are bound separately | No invented cost or policy thresholds; no VIX identity term. |
| Automatic threshold search | Shared algorithm, durable outer orchestration and exact per-probe identities, depth evidence and terminal states | A failed/no-affordable search must refuse rather than silently fall back to a statistical threshold. |
| Save the search evidence | Append-only starts, exact per-depth accounting, retained signal rankings, durable frontier/cursor continuation and explicit terminal records | Final verification is O(saved rows); retained history and sync costs are explicit. No per-event hot-loop logging claim. [Lifecycle](16-sweep-evidence.md) and [checkpoints](22-expression-search.md). |
| Successful completion is earned | A terminal seal must agree with acknowledged child counts, identities, contents and generations; stored-month and priced-audit paths seal children before parent publication | Lost, replaced, torn or failed evidence cannot become a completed empty result. A failed parent append has an explicit report and nonzero command status. Completion attests the checked snapshot; unclean process death leaves an uncompleted attempt. |
| Concurrent result writers | Initial header/index/generation reads are locked; catch-up refuses partial tails/shrink and does not trust bad row seals | Initial open is O(history), refresh O(delta); lock/fsync latency is not constant. No destructive tail repair. |
| Incremental saved lookups | Reused parent/writer indexes, validated refresh and bounded child reads; cached top selection uses the canonical renderer | Expected hash cost; ordered selection maps have logarithmic lookup. Cold history reads remain real work. |
| Busy and malformed evidence files | Candidate/search readers refuse conflicting file locks; a common Rust open rejects final symlinks and FIFO paths without waiting for a peer | Verified native open flags apply to macOS and Linux x86_64/aarch64. Intermediate directories still require a trusted root; disk/device latency is not bounded by advisory locks. |
| Independent checksum receipts | Separate strict audit command, retained source/receipt locks and six-role manifest bind full source/header/sidecar digests to the common stored sweep kernel | Opt-in checksum integrity, not legacy ReferenceIntegrity, calendar completeness, vendor accuracy or institutional admission. Whole-file audit is linear; physical record caps are not a total memory guarantee. [Contract](24-checksum-admission.md); actual integration and activation are recorded separately. |
| Strict multi-month pricing | CLI and explicit API audit-audited-range command share the checked adapter, retained six-role receipts, exact minute execution and pricing/publication kernel; browser batches call that explicit adapter | API paths are server-owned; missing/invalid physical configuration refuses before claiming a slot. V6 additionally retains its own typed source and ledger authority. Cold audits are linear; no shortened range or missing-minute substitution. [Contract](24-checksum-admission.md). |
| Receipt-checked batch dashboard | The page queues selected symbols across the server's canonical intraday rungs and shows each request, exact attempt and saved report. Exact-attempt polling keeps its retained result visible despite unrelated CLI activity; unknown or failed status stops subsequent jobs | Up to 4,096 page-owned jobs, executed sequentially. Closing the page stops future launches; an accepted server job continues. Restart/replaced status remains Unknown. No ambiguous POST retry or durable aggregate batch/resume claim. Thirteen focused and 345 full frontend tests pass. |
| Complete frontier pages | Both browser consumers reconcile all pages before publishing a result | 256 rows/page, 4,096 rows/run, at most 16 pages. A failed later page does not publish a prefix. |
| Saved depth drill-down | `/sweep-evidence.json` plus selected-run comparison and level table | Exact identity and attempt on every page; legacy missing evidence is not reconstructed or called zero. |
| Saved Boolean progress | Read-only snapshot pages show recorded counters, bounded verified grammar links and each exact expression child's own trade action | Writer observation is instantaneous. Page verification does not reconstruct market inputs or certify unread older history; expired/spliced continuations refuse. |
| Exact browser values | Saved sweep-evidence counters/statistic bits and six-word masks use exact decimal strings | BigInt/strict decimal validation preserves those values. Legacy numeric endpoints retain their own precision guards and limits; this does not claim every browser field is a decimal string. |
| Reliable activity status | Whole-command start/finish, separate preparation/attempt events, bounded external observation and explicit unknown states | Latest observed command is not a census of all parallel jobs. Unreadable/stale evidence is not proof of idle or completion. |
| Five historical filters | Visible label is **5 rules pass**, using stored applied thresholds | It does not prove the remaining admission predicates, period consistency or institutional authority. |
| Institutional architecture | `ledger-v6` reaches authenticated Selection V6 for the two indices; `ledger-v6-replay` continues all eight retained selections through exact later OOS witnesses and Global Replay V4 | Boolean candidate captures do not grant V6 authority; mapping a program to its referenced AND bits would change its meaning. Institutional cash-stock expansion is also separate. Terminal prefixes may contain 0..25 winners and zero-stream replay does not prove market coverage. [Contract](21-institutional-sweep.md). |
| Genuine zero-strategy institutional families | Typed evaluated/extinct families retain actual completed search, Candidate, Base, Pre-Admission V2 and source evidence. Both evaluated, either mixed and all-extinct outcomes now pass through actual Statistics V3 to Selection V6 | No fabricated zero-row V1 or depth-one result. Fresh ledger/source rechecks remain required after projection and through later OOS. Counted partial input refusals remain visible; completion is not an all-market-data certificate. [Eight strict integration tests](../target/sweep-audit-20260906/strict-v6-final-tests.log). |
| Institutional run configuration | One runtime profile supplies all 37 delegated research settings; both authority routes validate and report all 39 configured rules before market loading. Existing explicit overrides remain visible and malformed values refuse | A configured rule is not a passing strategy. The profile is assistant-selected research policy, not external certification, and has not been activated in the live service. [Plain-language settings and source profile](22-research-policy.md). |
| Every candidate's exact trades | Each retained candidate actually priced in each visited screen pass has both directions' exact selected-cell trades or an explicit no-cell outcome; the browser exposes saved signals, refusal counts and exact selected exit settings | A summary cannot certify unopened trade evidence. No-cell pricing is distinct from not being priced. Actual caps and tiers are shown; skipped candidates, calibration, validation grids and all exit cells are not claimed captured. [Contract](19-candidate-trades.md). |
| Permanent observability | Durable attempt/result protocols plus boundary telemetry and drop-health reporting | Telemetry rotates and is not a permanent per-event fsync archive. Inner-loop logging is deliberately excluded. |
| Rust-only backend | Engine, vocabulary, store, CLI, API and verifier are Rust; browser work stays under web/ | Path and dependency gates are executable. Whole machine/toolchain independence must not be inferred solely from file extensions. |
| Current MacBook planning | Verified M4 Pro, 14 CPU cores and 48 GiB memory; parallel verification workers are shared across agents and the existing service | Current internal free space is rechecked. The operator's additional 8 TB drive is disconnected and contributes no present capacity. [Hardware baseline](25-macbook-runtime-budget.md). |
| Live backend routes | Earlier clean 33ecdfba backend activated; candidate/search endpoints answer with explicit missing/refused evidence instead of fabricated results | Later 502d42dc and current repairs are not active. Observed acquisition continues, scheduler is paused, health is DEGRADED with master/universe identity notes, and no production result ledger is present. This is a dated observation, not continuous monitoring. |
| Customer-scale assurance | Bounded operator-facing API and readable comparison surfaces | No million-user load, public-service security or latency SLA has been demonstrated. |
| Conservative zero-return statistics | A separate Rust procedure keeps every zero-return coordinate with exact probability one and retains negative variable series in the shared comparison. An independent1,152-family oracle and exact allocation boundary tests pass | Nonzero constant series refuse. The shared bootstrap assumptions remain; this is not universal statistical validity. [Successor contract](29-fixed-training-qualification.md). |
| Finite testing budget | The new qualification plan declares all eight timeframe shares before later pricing; interrupted and failed slots retain their original allocation | Covers the declared finite catalog only. Other catalogs and grammar batches are separate families. Unreachable draw resolution is reported rather than increased after outcomes. [Scope and limits](29-fixed-training-qualification.md). |
| Later institutional qualification | New wiring joins original selected exits, complete later windows, zero-inclusive family statistics and every common policy field in a separately versioned saved comparison | Combined integration and release checks must pass on the final source. These are fixed-training later windows, not expanding-prefix retraining or Selection V6 authority. Production activation remains separate. [Detailed comparison](29-fixed-training-qualification.md). |
| One allowance across grammar batches | A separately versioned declared search saves each batch before pricing, keeps its exact ordinal and allowance through failures, and projects all eight saved qualification results under a countable shared testing budget | New source requires its own final checks. This covers one immutable declaration, retains bootstrap assumptions and cannot certify arbitrary other research projects. Whole-history bytes and replay work are explicitly bounded. [Plain-language comparison](30-search-wide-qualification.md). |
| Exact shared probability limits | V2 caps four family probability ceilings, retains all35 other policy fields, and exposes original/effective values with exact row-policy binding. A writer-valid White/SPA counterexample is V1 admitted and V2 rejected | V1 retains its historical arithmetic and is labelled explicitly. New version markers bind identity; no old history is rewritten. This is comparison correctness, not a demonstrated global statistical-calibration guarantee. [Versioned contract](30-search-wide-qualification.md). |
| Mandatory release evidence | Strict deployment admission now requires exact-source raw whole-crate line/branch counts and complete module mutation outcomes, with physical limits and explicit compiler-invalid evidence | The retained85-artifact stage passes its artifact check but actually refuses missing mandatory evidence with exit3 before live observation. No activation is inferred from ordinary green checks. [Gate contract](30-search-wide-qualification.md). |
| Missing recovery plan | The actual missing-active-plan handler regression returns503 and visible unavailability while preserving exact pointer, STOP and attempt files and claiming no run slot | The live pointer's original plan was not found by the scoped read-only search. Attempts cannot reconstruct its durable scope. History remains intact and safe handoff is blocked. |
| Dependency process probes | Isolated pinned Rust prototypes demonstrate that compiler probes can be removed while preserving selected compiler/functional behavior | The full vendor approach conflicts with the explicit no-vendored-binding rule for system ABI packages. No shared dependency patch or rule exception was introduced. Native and optional-platform evidence remain separate. [Measured alternatives and limits](../target/sweep-audit-20260906/dependency-purity-20260907/remediation-matrix.md). |

## Institutional meaning, in plain language

There are several different questions, and one passing label cannot answer all
of them. A candle pattern can be implemented correctly without predicting the
market. A search can complete without finding an acceptable strategy. A result
can pass five historical filters without passing every admission gate. A
validation stack can be requested without its outcome being durably proven in
this view. An OHLC/volume feature does not identify institutional order flow.

The existing authority route requires explicit policy values. On 7 September
the operator delegated those choices, and this task supplied a versioned research
profile with all 37 missing settings. The profile does not invent missing cost
evidence, label cost-excluded cash research profitable, or turn a UI check into
an institutional admission receipt.
The [dashboard comparison](18-sweep-dashboard.md) names the distinctions that
are visible in the product.

## Verification evidence

The automatic panel and its archived logs record the checks actually run on
their named snapshot. They do not attest subsequent edits or the deployed
binary. Earlier measurements below are scoped checkpoints, not a substitute
for fresh checks on the source that will be launched.

| Evidence | Observed result and limit |
|---|---|
| Shared integration checkpoint | 4,579 tests passed, 10 ignored, no failures; format, strict workspace Clippy and API build green. This preceded the final new evidence/UI wiring. |
| New implementation preflight | 4,715 workspace tests passed, zero failed, ten ignored. This run preceded the last report-text assertion; the final automatic panel records the later stable snapshot. |
| Engine and indicators after final indicator fixes | 483 tests passed, no failures/ignores; scoped all-target Clippy green. Includes finite pattern/availability/search oracles. |
| Concurrent result stress | 27 Results tests and 11 Receipts tests passed; eight writers ×16 appends repeated 20 times, all passed. This is finite concurrency evidence, not a universal crash proof. |
| Expression semantics and codec | Independent three-valued truth table, all live bits, full signed vocabulary, exact capacity/refusal boundaries and 1,554 short wire programs. Current named tests are in vocab/tests/expression.rs. |
| Expression runner | Three focused tests passed for identity sensitivity, exact source alignment, unknowns and fallible evidence delivery. |
| Expression files | Nine strict persistence/integration tests passed, including every-byte mutation, every truncation of a fixed fixture, pending-path replacement, changed acknowledged bytes, per-row seals, an interleaved write, and generated-column → expression → saved rows → reopened evidence. |
| AND continuation | Independent restart oracle across 120 configurations; five stored CLI tests passed including actual sink failure, final checkpoint corruption and ranked-helper rerun. Final resume module: 540/540 lines, 134/134 branches; reconciled current mutations 213 caught and 20 unviable, no unresolved survivor. This is module closure, not whole-engine coverage. |
| Boolean cursor, earlier checkpoint | Seven focused tests and scoped Clippy passed. At that checkpoint: 120 mutations, 115 caught and five unviable. Coverage was 193/197 lines and 67/70 branches. The later D-0526 campaign below supersedes these measurements for its source. |
| Shared Boolean pricing | Three runner tests and seven CLI search tests passed: legacy grid/cell equivalence, mixed/unknown predicates, restart, forged transitions, missing trades, 164 grid/integer cases, malformed overrides, all 14 pricing identity terms and visible dropped-signal counts. Scoped CLI Clippy passed. |
| Candidate captures | Twelve storage integration tests passed, including real screen/audit paths, nonwinning traces, no-cell outcomes and expression model separation. Current API/frontend results belong to their archived checks. |
| Selection and replay | Selection V6/all-rung topology 13 tests and Global Replay V4 11 tests passed after pre-computation persistence, exact identity sensitivity and distinct plan/stream operations. Final Ledger V6 suite: nine passed, including both policy-before-market-sizing regressions. |
| Sweep evidence | All 14 public lifecycle/paging/concurrency tests passed, including acknowledged evidence loss/replacement before completion. |
| Saved evidence API | Three focused tests passed; the API library checkpoint passed 944 tests with one ignored, including status/top/cache regressions. The automatic panel identifies its own recorded snapshot. |
| Frontend | The earlier 64-test checkpoint used an isolated build. The final additional 14 candidate/search protocol tests, typecheck and build passed, but that final build wrote workspace web/build. The later authorized clean backend activation closed the observed 404 mismatch: both new routes now return HTTP 200 with explicit missing-evidence responses for IDs held only in the separate fixture. |
| Fresh engine speed checks | Per-bar support k=1 to k=384 ratio 0.941×; evaluator 1,000 to 200,000 bars 0.994×; column 20,000 to 200,000 bars 1.051×. These are regression measurements on one host. |
| Clean-snapshot speed suite | All engine, vocabulary, indicator and runner ratio benches passed at commit 33ecdfba5285d4943816415edf20ddb1d06cf36a. Support cost per bar at 10,000→1,000,000 bars was 1.254×; k=1→384 was 0.994×; indicator step 1,000→200,000 bars was 1.004×; column build 20,000→200,000 was 1.022×. These finite host measurements precede later checkpoint/read-refusal fixes; they do not certify whole-search O(1), I/O deadlines or million-user scale. Log: target/sweep-audit-20260906/clean-core-ratio-bench.log. |
| Indicator diff mutations | 38 caught across initial/follow-up runs, one unviable, no remaining tested diff survivor. Not whole-module mutation closure. |
| Engine diff mutations, earlier checkpoint | 42 caught, nine surviving, six unviable at that checkpoint. The later D-0526 repair campaign below replaced this list for the then-current diff; it does not certify subsequent edits. |
| Expression mutations, earlier checkpoint | 106 generated, 95 caught, seven unviable, three timeouts and one raw equivalent survivor at that checkpoint. The later 229-mutation campaign and exact timeout replay below supersede this list for their source. |
| Coverage, earlier checkpoint | Expression 362/371 lines and 71/75 branches; resume 540/540 lines and 134/134 branches. Later official expression coverage was 465/479 lines and 70/75 branches, search 242/245 and 68/72. Full 100% touched-crate coverage is not established. |
| Strict input and parent publication | Seven store, seven receipt, three native/coarse input, four shared-publication and four actual priced-audit finalization tests passed. Exact source/receipt locks, every receipt prefix, missing context, caps, FIFO refusal, late depth/rank loss and exact priced-capture loss are covered by finite fixtures. Real all-rung integration belongs to its separate execution log. |
| Real data and deployment | A separate clean commit ran two real May 2025 expression searches, each resuming 8→16 candidates. Independent readers verified 64 trade files / 31,279 trades. The AND run reached extinction with 13 results and reused its sealed checkpoint/parent. [Full evidence](23-historical-sweep-verification.md). Production stored bars were untouched by those tests. The authorized backend activation now serves the new routes, with the existing recovery job resumed and automatic scheduler still paused. No visual browser certificate is claimed. |
| Pricing admission mutations | 19 selected mutations: 17 caught, one unviable, one raw survivor changing >100000 to >=100000. Valid integer grid sizes jump from 87,542 to 127,155, so that boundary equality is unreachable; no raw zero-survivor certificate is claimed. |
| D-0527 indicator follow-up | 419 indicator tests and all-target Clippy passed after opening-range/Fibonacci availability and the combined exact-minute bridge. ORB13/13, current-day4/4 plus one same-day guard, previous-session Fib3/3 and combined bridge11/11 targeted mutants were caught. Gap scope: six caught and two invalid Rust mutations. These finite scopes do not establish full touched-crate coverage or mutations. |
| D-0527 dashboard follow-up | Nine sweep-summary tests and nine candidate-detail tests passed; typecheck reported zero errors/warnings. An isolated frontend build passed. The comparison renderer parsed, 41 detailed rows were generated, and 13 per-row evidence links resolved locally. Visual inspection was blocked by the locked Mac. |
| D-0527 strict API follow-up | Eleven API boundary tests and sixteen focused strict CLI tests passed; CLI/API all-target Clippy passed. Ten malformed-runtime subprocess cases refused before source receipts, preparation attempts or parent results. Missing configuration, invalid scalar/clamp values, exact attempt, source/child loss and unwritten terminal-event outcomes are explicit. Full workspace, coverage and mutation results remain separate gates. |
| V7 combined checkpoint | All seven checks passed with unchanged source checkpoints: 4,840 top-level Rust test passes, zero failures, ten ignored; eleven separately logged subprocess test executions also passed. Frontend regression scope: 44 passed; typecheck: zero errors/warnings. The original V6 cleanup-guard source-shape test failure remains archived; its correction and a new strict/ordinary shared-guard test passed, bringing the focused strict API suite to twelve. [Exact V7 workspace output](../target/sweep-readiness/1788711199-867614000/check-15.log). This predates D-0528. |
| Clean actual-data follow-up | Clean CLI/API build 174435aa640f7e6025ca63ddddaef06ca00d5672 passed. Eighteen strict April–May 2025 cases and eighteen retries completed with exact parent-byte reuse, at two concurrent commands. A deliberately high support threshold produced zero surviving candidates; no low-support or institutional conclusion follows. [Complete matrix](../target/sweep-audit-20260906/strict-range-matrix-174435aa/matrix-report.txt). |
| Actual opening-range oracle | On that same clean build, all 6,173 saved NIFTY May 2025 expression rows for !86 & !87 matched an independent actual-minute oracle: 3,748 at 2min and 2,425 at 3min, including 57 Unknown rows. The reconstructed coarse-candle calculation disagreed on 192 expression answers. A one-month 60min request correctly refused with 147 raw bars and no rows beyond the 200-bar warm-up. This is signal verification, without pricing or institutional admission. [Exact oracle output](../target/sweep-audit-20260906/independent-orb-oracle-final-v2.log). |
| D-0528 clock-window repair | Five new public tests and indicator all-target Clippy passed: all1,440 minutes over two days, four NOT windows, unavailable-price AND/OR combinations,375 warmed Column rows and malformed/repeated-bar refusal. The tests recover all315 not-early-morning signals in the generated day. This is a four-predicate repair, not completion of every remaining availability family. |
| D-0529 complete indicator checkpoint | All455 indicator tests and all-target Clippy passed. Selected mutation scopes:26 session,6 trend,5 crossing and1 truth-only handoff fault caught. Generated oracles exercise known false, unavailable references, causal history, touches, refusals and all85 crossing positions. These38 selected faults are not whole-module mutation closure. |
| D-0529 speed repair | The unchanged step budget initially failed at2,202 measured floors. Removing discarded availability work from the truth-only API reduced it to958 floors, about0.492 microseconds per generated bar, below the existing2,000 limit. Column20,000 to200,000 ratio1.048 passed. The initial failure is retained; this does not promise total-search or I/O latency. [Passing benchmark](../target/sweep-audit-20260906/session-trend-crossing-optimized-bench.log). |
| D-0529 coverage diagnostic | Nightly instrumented tests passed, but automatic object discovery failed. Native export reported200 mismatched functions and remaining branch gaps; the source predates the truth-only optimization. This is explicitly not a clean100% coverage certificate. |
| D-0530 strict authority integration | Eight actual stored integration tests passed in24.40s after the final stronger ledger rechecks; CLI and runner all-target Clippy passed. The four evaluated/extinct family shapes, three independent input ceilings, receipt-policy identity, source replacement and post-Selection V2 corruption are covered with generated stored fixtures. [Exact suite](../target/sweep-audit-20260906/strict-v6-final-tests.log). |
| D-0530 related authority regressions | The coherent strict and supporting suites passed 89 tests: 8 strict integrations, 22 Candidate, 14 Pre-Admission, 11 Observation, 6 completeness/shared-policy, 9 ledger preflight and 19 runner identity. Cold/no-data, actual resource halt, legitimate zero-depth evidence and unchanged nonempty V1 semantics have separate assertions. These counts overlap the eight-test row above and must not be added twice. |
| D-0531 deadline hardening | Seven public fixed-deadline regressions and24 trade unit tests passed; five selected timing/table faults were caught. The earlier full runner checkpoint passed543 tests before the final compatibility-table refinement. The combined panel records subsequent workspace evidence. |
| D-0532 browser and API correlation | Thirteen focused strict API tests preserve exact u64 acceptance/status tokens. Twelve receipt-batch tests and all344 frontend tests passed; typecheck reported zero errors/warnings and the final production frontend build passed. These are local implementation checks, not live activation or a visual browser certificate. |
| D-0533 exact-attempt recovery | Final review found that unrelated CLI activity could hide a retained browser terminal from the global status view. The batch now polls and rechecks its exact accepted token. Sixteen API tests and thirteen receipt-batch tests passed; all345 frontend tests, typecheck and production build passed. Missing/replaced attempts remain Unknown. [Exact API output](../target/sweep-audit-20260906/receipt-status-api-tests.log). |
| D-0533 actual execution clocks | Independent public regressions first reproduced actual15:20 forced exits and late/foreign-day interior grid exits using stale cached facts. Common endpoint/horizon checks and the existing checked crossing pass now reject those paths. All549 runner tests passed, including four new public regressions covering native/projected columns, both directions, masks/expressions, grids and preserved earlier horizons. [Runner output](../target/sweep-audit-20260906/foreign-slice-deadline-runner-tests.log). The combined panel and separate benchmark attest their own later source. |
| V9 combined checkpoint and D-0534 | Six gates passed, but four strict range tests correctly refused a temporary invalid grid-rung value from another test. The source checkpoints stayed unchanged. Isolated replay passed; shared test guards were added without changing production validation. This failed checkpoint remains archived and requires a later full workspace result. [Original V9 failure](../target/sweep-readiness/1788717819-867668000/check-15.log). |
| Clean 8c9c3e29 actual-data oracle | All 53,725 saved NIFTY May2025 expression rows matched independent integer calculations for body/range, prior-three history, running high and session-open crossings/ordinals, including unavailable-reference AND/OR/NOT behavior. All12 copied source files remained byte-identical. This is seven bounded signal programs, not pricing or institutional admission. [Actual oracle output](../target/sweep-audit-20260906/independent-session-crossing-oracle-8c9c3e29.log). |

Key logs are under `target/sweep-audit-20260906/` and each automatic run under
`target/sweep-readiness/<run>/`. Agent checkpoint logs are named in the linked
indicator, persistence and dashboard documents. Ignored tests, unviable mutants,
timeouts, missing coverage and missing provenance are never counted as passes.

## What O(1) can and cannot mean here

Let B be bars, F a frontier, C examined candidates, R saved rows and delta new
rows since an already-open handle refreshed.

| Operation | Honest bound |
|---|---|
| One condition lookup or one six-word mask test | Constant work at the fixed representation width |
| One bounded expression over one bar | Work bounded by its fixed program capacity; fixed auxiliary stack |
| One indicator step | Fixed state/work for the implemented finite families; measured separately |
| Support for one candidate | O(B) |
| Whole search and output | Potentially combinatorial; at least proportional to work/results actually emitted |
| Frontier sorting/grouping | O(F log F), with potentially quadratic pair work within groups |
| Canonical k>=2 joins | Unique construction avoids a candidate deduplication operation |
| HashMap/HashSet lookup | Expected O(1), not an adversarial worst-case guarantee |
| Cached result append/refresh | O(delta + 1) local work, plus locking and persistence costs; cold open O(R) |
| Terminal integrity and strict full expression read | O(R), bounded working buffers; not a constant-time whole-file audit |
| Live census and UI paging | Explicit bounded enumeration/rendering; cold work and requested rows are real costs |
| Materialized columns and retained history | O(B) and O(R) space; cannot retain unlimited distinct data in O(1) total space |

These are source-derived or mathematical bounds. Rust's primary
[collection documentation](https://doc.rust-lang.org/std/collections/index.html#performance)
distinguishes expected and amortized costs. None of these bounds guarantees a
constant wall-clock deadline under scheduling, filesystem stalls or resource
exhaustion. Universal O(1) total exhaustive search and unbounded retained history
are not achievable requirements.

## Remaining release conditions

### Later D-0526 repair evidence

The earlier measurements above are retained as historical observations. The
current engine diff was retested with112 passing tests and strict Clippy.
Its70 unique mutants reconcile to63 caught and7 unviable: two initial
timeouts were caught by exact focused replay after adding a deterministic
terminal-checkpoint assertion. This replaces the old nine-survivor list for
the current diff scope, not for every possible engine mutation. Clean
unit-binary coverage remains1,998/2,039 lines and122/154 branches in lib.rs;
column.rs has362/362 lines and19/22 branches.

The later expression campaign tested229 mutants. After exact prefix-underflow
replay,216 were caught,12 unviable and one raw equal-sibling rotation survivor
was proved byte-equivalent; no unresolved timeout remains. The final vocab
suite passed100 tests and strict Clippy. Official LLVM file coverage still
has gaps: expression465/479 lines and70/75 branches; search242/245 lines
and68/72 branches. Cross-instantiation custom unions are separately labeled
in the saved evidence and do not replace that official summary.

The VWAP repair and real-row oracle are in [the complete mapping](26-vwap-mapping.md).
The new strict span path has11 passing loader/kernel tests, including native
and coarse actual publication/retry and source/receipt/ranking replacement at
the final boundary. The expanded receipt suite has10 passing tests. Fresh
per-binary coverage without mismatch warnings remains372/414 receipt lines,
165/171 monthly-loader lines and216/221 range-loader lines; branch counts
are36/42,12/18 and17/22 respectively. These do not clear the100% gate.

Exact logs and exports are retained under
`target/sweep-audit-20260906/`, including
`engine-resource-closure-evidence.md`, `expression-repair-prefix-final-tests.log`,
`expression-repair-prefix-final-clippy.log`, and `checksum-range-fresh-coverage/`.
The repeatable verifier below the comparison carries later source-wide results
separately from these scoped measurements. The acquisition service is unchanged.

### Outstanding release conditions

1. Read the final repeatable check results for the current source; resolve any
   red functional/build gate instead of substituting an earlier green run.
2. Close or explicitly retain the required full coverage and mutation gaps.
3. Provide the approved institutional admission configuration and execute that
   exact compatible authority route with clean build and historical provenance.
   The bounded ordinary AND/Boolean historical paths have now run; this does
   not replace the full institutional route's separate authority requirements.
4. Verify the new candidate-trade, Boolean search/restart and Selection V6 /
   Global Replay V4 routes together. Ordered temporal sequence search remains
   outside the current Boolean grammar; syntactic enumeration is not semantic
   Boolean-equivalence deduplication or an attainable all-market certificate.
5. Deploy and verify the new local API/frontend together when the active data
   acquisition can be left undisturbed. A compiled route is not a deployed route.

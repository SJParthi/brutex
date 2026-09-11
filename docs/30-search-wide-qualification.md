# One declared search, one shared testing allowance

The new search command joins the existing Boolean grammar, real historical
catalog, eight intraday timeframes, frozen training exits, later-period
comparison and common research policy. It saves progress and a separately
corrected result across batches. This document describes the implementation;
the dated verification report records which checks actually passed.

## The comparison a person should see

| Area | What a finite batch alone establishes | What the declared search adds | What still must not be claimed |
|---|---|---|---|
| AND, OR and NOT | The exact submitted programs were evaluated | The existing fixed grammar supplies successive exact program batches | A work allowance means every combination has been exhausted |
| Eight timeframes | All eight declared rungs have saved results | Each rung also belongs to one immutable search batch | More timeframes than the engine actually supports |
| Exit-grid resolution | The exact declared stop, target and trail percentile levels are priced | Both directional grid policies bind the shared source declaration | A runtime setting can be accepted and silently replaced with five levels |
| Real prices | Retained OHLCV and the exact source identity produced the result | The same source declarations bind every batch | Generated test fixtures are real market observations |
| Intraday deadline | Existing execution limits entries and exits to the actual session and 15:10 IST deadline | The search uses the same execution path | An open-stamped 15:10 candle is an available pre-deadline exit price |
| VWAP | Instrument-kind mapping governs availability before bars are folded | Every program uses that same mapping | Index-spot volume can be invented, or stored futures become swept instruments |
| Training and later data | Original exits are frozen before later prices are observed | The exact training/later scope stays fixed across batches | Repeated exploration of the same later data is independent new confirmation |
| Lucky results | The finite family has its existing multiple-testing checks | A fixed allowance covers every successive batch in this declaration | A statistical filter guarantees profitability or removes all uncertainty |
| Policy settings | The original common policy evaluates every coordinate | V2 retains all35 other fields and caps four probability limits at the shared allowance | Missing measurements become passed checks, or an old V1 observation becomes a V2 admission |
| No trades | Zero-trade coordinates and zero-return sessions remain recorded | They remain in the complete search summary | Quiet periods may be dropped to improve a score |
| Refused results | Reasons remain in the saved original qualification | Refused counts and unchanged original reasons remain accessible | A completed computation means every candidate passed |
| Restart | The exact finite campaign can be reopened | The batch keeps its original ordinal, source binding and testing allowance | Retrying starts a fresh easier testing budget |
| Failure before completion | The child retains its acknowledged history | The parent saves the failure against the same reservation | Unacknowledged output is a completed batch |
| Lost evidence | A cold child reader refuses changed or missing files | Parent and campaign links stay checked through detail paging | Old success is silently substituted for missing latest evidence |
| Memory limits | Original readers use explicit bounds | Full history and declared program reservation have independent admission | A small file is safe merely because it claims a large allowed allocation |
| Work limits | A finite invocation bounds computation | Grammar replay has a separate server-owned work ceiling | Bounded output guarantees constant cold-read latency |
| Progress | Saved finite steps are visible | Declared ordinal, programs, grammar choices, checkpoint and refusal are shown | A fabricated percentage such as 2% or 5% measures unknown total work |
| Dashboard | Original statistics and reasons remain available | Each row compares original qualification with search-wide qualification | A rendered status is trading authorization |
| Costs | Current research excludes costs | This label stays visible on the new path | Returns are net profit or live trading performance |

## Why the testing allowance changes across batches

Testing many strategies creates many chances to see an apparently impressive
result by chance. The search therefore sets aside a smaller share of one
declared testing allowance for each successive batch. The shares are fixed
before the results are known. All eight timeframes receive equal shares within
each batch.

For zero-based batch `b`, the exact per-timeframe share is
`alpha / [8 × (b + 1) × (b + 2)]`. After `n` complete batch positions, the
assigned total is `alpha × n / (n + 1)`, which never exceeds alpha. The integer
implementation keeps exact reduced fractions and refuses arithmetic overflow.
Failed and empty positions keep their share; it is not handed to another result.

This allocation applies to one fixed declaration. Changing the data, grammar,
batch boundaries, policy or statistical procedure creates a different search.
Opening many independent research projects is not covered by the first
project's budget. The bootstrap calculations also retain their own validity
assumptions. These are research checks, not a promise about future trades.

The shared alpha is the minimum of the original FWER and Romano--Wolf
ceilings. V2 compares every search-adjusted family probability against an
effective policy whose four probability ceilings are capped at that alpha.
The original policy and qualification stay intact. The dashboard shows all39
original and effective values, with the exact policy digest on each comparison.

V1 remains readable under its original arithmetic. It kept White and SPA's
individual policy ceilings, which could exceed the shared alpha. These results
are labelled historical and are never silently upgraded to V2. New V2 records
use a distinct format discriminator, search identity and projection digest
domain; they cannot be appended to a V1 history. This fixes the stated
comparison contract without claiming validated bootstrap calibration or a
guarantee about trading loss.

## Durable work and honest progress

Each batch first saves a reservation containing its exact grammar cursors,
programs, source-bound plan and ordinal. Only then does pricing start. It ends
with either a recorded refusal or a complete record linked to all eight saved
qualifications. A retry must keep the same binding. A new ordinal is permitted
only after the old batch is complete.

The displayed counts mean:

| Label | Meaning |
|---|---|
| Reserved | Acknowledged plan exists; no parent completion is claimed |
| Refused | The attempt failed and retained its reason and original allowance |
| Complete batch | All eight saved qualification projections were reconciled |
| Paused at allowance | This invocation used its requested work; identical input resumes |
| Fixed grammar exhausted | The grammar cursor itself reports that no next program exists |
| Admitted | This coordinate passed the declared research policy at this observation |
| Rejected | At least one measured policy requirement failed |
| Unmeasured | A required measurement is unavailable |
| Refused coordinate | Required execution or evidence authority is unavailable/invalid |

Each acknowledged nonempty batch also links to the expected campaign's saved
timeframe progress. This prospective address does not itself prove that any
child work has started; that view independently checks its actual journal.

No percentage is derived from an unknown search total. A saved parent overview
authenticates history; opening a detail also authenticates the linked campaign,
qualification and complete projection. Missing evidence stays a visible refusal.
Both cold history and full-child validation grow with the retained work.

## Entry points and configuration

The Rust command is `boolean-qualified-search-stored`, with these17 explicit
arguments in order:

```text
VENDOR SYMBOLS FY FM TY TM BITS HORIZON MAX_POINTS
BATCH_PROGRAMS NODE_ALLOWANCE BATCH_ALLOWANCE OUTPUT
LATER_FY LATER_FM LATER_TY LATER_TM
```

`BITS` is `all` or an explicit condition-position list. Existing strict source,
policy, calendar, physical-capacity and clean-build checks still apply. Later
months must be wholly after training months. `BATCH_ALLOWANCE` is an invocation
pause allowance, not a maximum expression depth or part of the search identity.

`BRUTEX_GRID_RUNGS` selects the existing exit-grid resolution. Missing
configuration retains five levels for compatibility; an explicit setting must
satisfy the shared scalar bounds and the resolved grid's fixed cell admission.
It changes the source/search identity. `MAX_POINTS` remains a risk threshold.
A smaller verification grid does not prove that a larger one fits the same
capture budget. Source-only capacity estimates, actual captured work and process
memory measurements are separate observations.

Legacy range-screen controls that this explicit Boolean path does not use
refuse by name. Set this command's positional horizon and its declared admission
profile/overrides; an unrelated screen knob cannot silently replace those
authorities. If saving a refusal also fails, the diagnosis keeps the original
failure and states that refusal persistence is unconfirmed.

The dashboard URL is `/backtest?boolean_qualified_search=<identity>`. Its bounded
read endpoint is `/boolean-qualified-search.json`. Detail requires the exact
search checkpoint pin, declared batch, timeframe, child completion pin and page.
The server controls `BRUTEX_BOOLEAN_OBSERVATION_BYTES` and the required positive
`BRUTEX_BOOLEAN_SEARCH_REPLAY_NODES`; requests cannot override these limits.
Use the producer's printed settings for its declared evidence admission.

The command's source-record bound also caps charged grammar-history replay.
The producer reserves the planned and terminal journal capacity and one selected
batch replay before pricing. A finite host may reach this immutable admission
before grammar exhaustion. Existing evidence remains intact and the refusal
names the exhausted resource. This version does not migrate a sealed search to
a larger declaration while pretending the original identity is unchanged.

## Verification and activation

Generated fault fixtures exercise actual journal and qualification code, with
their generated provenance retained. Historical proof uses separate real retained
OHLCV and clean-source executables. The two evidence types must stay distinct.

The standalone readiness verifier records the full formatting, tests, lint,
dependency and frontend checks with source-stability checkpoints. Coverage,
mutation testing, actual historical reconciliation and deployment are separate
checks. The Rust staged-release preflight verifies pinned executable, frontend,
configuration and check artifacts; it never starts a second service or interrupts
an existing acquisition. Full-sweep launch remains blocked while required checks
are unproved or an active service cannot be safely handed over.

The strict deployment plan now also requires raw, complete, exact-source
line/branch coverage and mutation evidence. Missing measurements, surviving
mutations, timeouts and partial censuses refuse before live observation.
Compiler-invalid mutations require their exact compiler failure evidence.
Reports reject duplicate JSON keys, including nested and escaped spellings,
and exact coverage counts reject decimal/overflow values. A caught mutation
requires actual completed Build/Test phases. Retained input guards and bounded
inventories are rechecked after observation; this is not a filesystem lease.
`--offline --require-release-gates` checks artifacts without reading a live
service and returns a distinct refusal when mandatory proof is absent.
An active recovery pointer with no original plan is a separate handoff blocker;
the API reports it as unavailable and cannot reconstruct the plan from logs.

See `docs/29-fixed-training-qualification.md` for original qualification,
`docs/22-research-policy.md` for the policy explanation, and
`docs/14-sweep-readiness-20260906.md` for the dated readiness comparison.

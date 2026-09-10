# Durable sweep evidence, version 1

This document describes the implemented CLI evidence boundary. It does not
certify profitability, universal institutional admission, completion of every
Boolean expression search, or constant total runtime. Existing result, frontier,
trade, receipt and store formats are unchanged.

## What a reader can distinguish

| Recorded fact | Meaning | What it does not establish |
|---|---|---|
| `preparation` completed | The named signal/daily/exact-minute inputs produced a condition column | No candidate search or price evaluation is implied |
| `auto-search` completed | Threshold orchestration found a nonempty affordable probe | It is not the final strategy audit |
| `auto-probe` completed | One exact, identified threshold probe finished | Other thresholds can still refuse |
| `sweep` completed | The supported AND ladder finished under its recorded bounds | Not a generic exhaustive AND/OR/NOT search |
| `audit` completed | The AND audit path and its durable result publication completed | Requested validation is not proof that all institutional gates passed |
| `expression` completed | The separate versioned expression computation completed | It is not an AND mask or an institutional strategy winner |
| `ranked_available: false` | This attempt did not publish a signal-ranking file | It does not mean an explicitly measured zero candidates |
| `ranked_available: true`, zero rows | An empty signal ranking was explicitly published | No candidate trades are implied |
| `running` | A durable start exists without an accepted terminal | Age alone cannot turn it into success |
| `halted` | An actual recorded engine resource breach prevented completion | It is not extinction of the entire search universe |
| `refused` | A named input, execution or persistence boundary prevented completion | It is not a completed empty result |

`validation_requested` is `null` outside Audit, or the actual Audit boolean.
`false` means explicitly disabled. `true` means requested, not validated,
admitted or selected. Older ledgers without this evidence remain unrecorded at
this granularity; their missing data is not reconstructed from final totals.

Completion also requires positive swept samples, a first swept index, matching
sweep/census counts and a reconciled census; ranked outcomes additionally require
closure certification. Empty, cold, fully refused or inconsistent samples are
Refused, including a fabricated first index beside a zero sample. An outer
AutoSearch that finds no nonempty affordable probe is Refused. Individual probes
with an actual resource breach remain Halted.

## Identity and actual call sites

Every stored sweep identity still has the nine existing terms: mask, direction,
instrument, timeframe, params, data digest, vocabulary version, verified commit,
and feed. Attempts have a separate monotonically allocated token. A rerun can
share a RunId and receive a different attempt token. An undirected zero mask
names the aggregate search; a retained row's own six mask words identify its
candidate within that search. An attempt token is not a candidate RunId.

| Entry point or shared boundary | Before computation | Durable outputs |
|---|---|---|
| `sweep_stored_inner` | Verified commit, actual input digest, RunId and Sweep start precede the condition fold | Measured levels, retained signal ranking, legacy summary, terminal |
| `audit_stored_inner` / `screen_range_inner` | Full policy identity precedes the common Audit start; the stored fold runs inside that boundary | Levels, signal ranking, prepared priced frontier, exact selected trades when selected, receipt, summary, terminal |
| `audit_range_inner` | Preparation identity precedes any withholding fold; the later full Audit identity precedes candidate search | Each preparation attempt plus the common Audit evidence |
| `one_rung` | Preparation is recorded before support sizing; AutoSearch and every AutoProbe have their own identities | Actual probe level counters and terminals; no silent statistical fallback when no affordable nonempty probe exists |
| `auto_stored_inner` / `auto_recorded` | Outer identity precedes folding; each probe's actual Ladder is bound before it walks | AutoSearch and AutoProbe lifecycle, actual probe levels |
| `batch::one` | The actual batch Ladder identity and Sweep start precede the fold/walk | Per-level counters, legacy summary and terminal; a fold refusal retains its known identity |
| `audit_bars` | A recording target without an identity refuses; durable admission precedes evaluator preparation | Common audit lifecycle, including Refused on ordinary early return |

The shared runner API is `Sweeper::auto_prepared_reporting`, with structural
`AutoProbeEvent::Started(Ladder)`, `Level { frontier, admitted, pairs }` and
`Finished(&Sweep)`. The existing `auto_prepared` delegates to the same halving
and bisection implementation. `Started` refusal prevents that probe from
walking. A level-write refusal is remembered: the engine's current `()` reporter
does not cancel the active ladder immediately. It is returned when that walk
returns, before another probe or successful publication. The CLI's ranked and
batch callbacks have the same current-walk limit.

The streamed batch hook is `run_prepared_streamed_reporting`; it invokes the
same engine walk and exposes its actual level counters. No counters are inferred
from a selected winner or reconstructed with guessed pair counts.

The monthly Audit now uses the existing complete `policy_of` composition,
including the actual validation flag, ranking lens, derived rules, horizon and
grid widths. Its null floors are measured on the actual one-minute execution
series, using the same `floors_measured_on` rule as the range path. This fixes a
monthly identity collision without introducing a new trading policy.

Four independently priced stored paths use `stored_executed_digest`:
`sweep_stored_inner`, `audit_stored_inner`, `audit_range_inner` and
`screen_range_inner`. Its exact preimage is:

```
ASCII "brutex-stored-executed-inputs-v1" + one zero byte
32 bytes: existing stored_anchored_digest(signal, exact-minute, daily)
32 bytes: runner::identity::data_digest(actual execution slice)
```

The result is full BLAKE3. Explicitly binding the separately loaded execution
slice prevents a concurrent store change between input reads from giving two
different price inputs one identity. Native one-minute execution is explicitly
bound too. This strengthens the existing `data_digest` term; it introduces no
tenth term and no VIX input. A range prepared under one anchored digest refuses
if its reloaded anchored inputs differ before the Audit identity is published.

Preparation's `params` policy word is `0x464F4C442D563100` (`FOLD-V1` plus zero)
and orchestration's is `0x4155544F2D563100` (`AUTO-V1` plus zero), passed through
the existing ordered `Params::with_policy` composition. These are operation
domains, not claims about a chosen trading threshold. An actual probe uses the
actual Ladder's threshold, candidate ceiling and pair budget.

## Paths and publication ordering

All new files are under `<store>/results/sweep-evidence-v1/`.

| Path relative to that directory | Purpose |
|---|---|
| `attempts.bin` | Append-only global start/terminal journal |
| `<attempt>-start.bin` | Immutable `create_new` reservation; one start record |
| `<RunId hex>/starts.bin` | Ordered starts for one identity |
| `<RunId hex>/<attempt>-lifecycle.bin` | That attempt's start and terminal records |
| `<RunId hex>/<attempt>-levels.bin` | Actual per-depth accounting |
| `<RunId hex>/<attempt>-ranked.bin` | Retained signal evidence, in its original ranking order |

Allocation uses the next global physical journal ordinal, so terminal rows can
leave gaps between attempt tokens. An immutable reservation prevents a truncated
global journal from reusing an old token. No code truncates, deletes, renumbers
or overwrites an existing evidence record. A concurrent older start that arrives
after a newer start of the same identity is explicitly refused before it can
compute. An older attempt finishing later cannot replace the newest start.

Admission writes a global start, creates and syncs the immutable reservation,
writes the private lifecycle start and appends the identity's start index before
returning writer authority. Once reservation exists, an ordinary subsequent
admission refusal attempts a Refused terminal. A disk failure or process death
can prevent that terminal; missing/incomplete evidence remains a refusal or
Running, never invented completion.

Each level append has a file durability barrier. Ranked rows are published once,
including the legitimate empty case; a duplicate publication poisons the
attempt. The writer tracks acknowledged cardinalities and full incremental
BLAKE3 content digests. Before a terminal is sealed, it reads each acknowledged
child with a fixed-size buffer, verifying cardinality, row seal, identity,
attempt, complete content digest and unchanged file generation. Deletion,
truncation, substitution, duplicate/swap or corruption of acknowledged bytes
cannot be relabelled as a completed empty result.

Terminal events seal the expected cardinalities and ranking availability.
Subsequent metadata reads refuse deleted/shrunken children; page reads verify
each requested row and the held/path file generation. Incremental writer hashes
are used to validate publication and are not a separately stored authentication
key. Row seals detect corruption; they are not authentication against an actor
who can rewrite the storage and recompute seals.

The existing priced result set still publishes children first: frontier, exact
selected trades, receipt, then the result ledger parent. The no-admitted path
uses `record_unadmitted`: frontier and an explicit zero-trade receipt must both
succeed before `record_swept_run` can publish a parent. A failed child leaves no
new parent summary. A final result-set failure preserves the live evidence.

## Bytes on disk

All integer payloads are little-endian. Each file begins with 16 bytes: magic
`[u8;8]`, version `u32` equal to 1, and stride `u32`. Unknown magic, version,
stride, partial header or partial row refuses. Existing format versions are not
mutated.

| File family | Magic | Row bytes |
|---|---|---:|
| Global, reservation, starts and lifecycle events | `BRSWAT01` | 96 |
| Levels | `BRSWDP01` | 128 |
| Retained signal ranking | `BRSWRK01` | 200 |

Every row's last eight bytes are the first eight raw bytes of BLAKE3 over every
preceding byte of that row. Headers have explicit structural validation; they
are not part of the per-row seal.

Event row layout:

| Byte interval, end exclusive | Value |
|---|---|
| 0..8 | Attempt token, u64 |
| 8..40 | Full RunId, 32 bytes |
| 40..48 | Start epoch microseconds, i64 |
| 48..56 | Updated epoch microseconds, i64 |
| 56 | Operation: Sweep1, Audit2, AutoSearch3, AutoProbe4, Expression5, Preparation6 |
| 57 | Completion: Running0, Completed1, Halted2, Refused3 |
| 58..66 | Declared depth row count, u64 |
| 66..74 | Declared ranked row count, u64 |
| 74 | Ranking availability: 0 or 1 |
| 75 | Validation requested: 0 unknown/not applicable, 1 false, 2 true |
| 76..88 | Reserved, all zero |
| 88..96 | Eight-byte row seal |

Both child families begin with the full 32-byte RunId and eight-byte attempt
token. Depth then has ten u64 words, in order: `k`, `generated`, `duplicates`,
`pruned`, `infrequent`, `frequent`, `admitted`, `pairs`, `reconciles` (0/1),
`excluded`, followed by its seal.

Ranking has nineteen u64 words, in order: one-based `rank`; six `mask_words`;
`hits`; `observations`; `mean_bits`; `t_bits`; `refused`; `mismatched`; `wins`;
`win_sum_bits`; `loss_sum_bits`; `adverse_sum_bits`; `favourable_sum_bits`;
`losses`; then its seal. Statistical values preserve their full `f64::to_bits`
representations. Loss count is explicit; flat observations are not silently
counted as losses.

## Read contracts and visible limits

`CommittedParents::open_read_bounded`, `refresh` and `receipt` provide a cached
validated parent snapshot. Refresh reads the legacy ledger before receipts;
the API copies that receipt before refreshing and serving a child. Missing
parents stay absent; the reader creates no files. Shrink, replacement,
same-length mutation, corruption or a size limit is a refusal. There is no
implicit reopening into a different history.

`Results::with_shared_writer` is crate-private and holds one validated writer
for one root. It refreshes newly appended history before publication and verifies
an exact duplicate against its stored row. Changing roots drops that cached
index; a write/refresh refusal invalidates it without retrying the failed action.
Cold ledger/receipt initialization holds the existing file lock through header,
index and generation capture, then releases it before returning the handle.

The API endpoint is `/sweep-evidence.json?identity=<64 hex>&kind=depth|ranked`.
It accepts bounded `page`, `limit` and `attempt`. Page zero chooses the latest
start for that identity. Later pages require the returned positive canonical
attempt token and refuse a changed attempt. Page size is at most 256; the
existing detail admission limit is 4096 rows and 64 MiB per scanned file. The API
reads evidence before and after a page and refuses a changed snapshot. All u64
counts and statistical bit words are JSON strings, preserving exact values in
the browser. Bounded blocking-worker capacity refuses overload explicitly.

Whole-command lifecycle is separate from per-run lifecycle. `cli.lifecycle`
`command started`/`command finished` use their own explicit telemetry token and
phase `running`/`completed`/`refused`. They do not acquire or change the ambient
sink token. `is_sweep_command` is the shared classifier that prevents read-only
inspection from looking like a successful sweep. Legacy inner progress without
an explicit matching token is uncorrelated and cannot establish completion.

| Cost | Actual bound |
|---|---|
| Mask width, row address, per-row seal | Fixed-width work |
| Cached identity lookup | Expected hash-table cost; not a worst-case latency proof |
| Cold ledger/receipt index | O(history), with O(history) index memory |
| Refresh after other writers append | O(new rows) |
| Evidence append | Fixed row, locking and durability barrier; storage latency is not bounded by Big-O |
| Save or terminal-verify N ranking rows | O(N) time; terminal verification uses a fixed row buffer |
| Page of P rows | O(P) work and output, under explicit admission limits |
| Complete candidate search, ranking or validation | Data/candidate dependent; can halt or refuse |

Neither test success nor fixed strides establish guaranteed constant latency,
unlimited storage, O(1) total memory or exhaustive completion at arbitrary input
size. Full-disk and hardware power-loss behavior require actual fault-injection
measurements; deterministic file-obstruction, corruption and concurrency tests
are bounded evidence, not a claim that those physical failures were measured.

## Exact candidate drill-down boundary still required

This file's ranking is the retained **signal** ranking. It is not the final
priced frontier order. The legacy frontier can rank under realized cell
statistics and policy tiers. The currently persisted exact trade block belongs
to the one selected mask, direction and reproduced exit cell. It must never be
shown as another retained candidate's trades.

`trade_and_screen` has the authoritative selected rule tier, mask, side,
execution bars/column, horizon, stop ladder, grid levels and selected cell when
it rebuilds and calls `grid::materialize_cell`. Its earlier priced map retains
only `(Cell, Direction)` by mask and does not retain the complete per-candidate
policy/grid authority. The runner's level reporting callback sees frontier
counters, not that pricing context, and cannot reconstruct it safely.

Persisting exact drill-down for **every** priced candidate requires a typed
capture callback at the pricing/selection boundary while those exact inputs are
still in scope, plus a separate versioned manifest binding mask, side, cell,
policy, execution digest and trade-row cardinality. It must use the existing
grid/materialization implementation and must fail closed on replay mismatch.
No guessed policy, arbitrary reranking or winner-trade substitution fills this
gap. The existing institutional ledger generations also remain distinct from
the legacy AND audit and from expression signal research.

## Verification record

The latest diagnostic checkpoints recorded 14/14 public CLI evidence integration
tests, 3/3 API evidence tests and 9/9 expression CLI tests. These are scoped
diagnostics; they do not declare the final combined verifier green.
The earlier storage checkpoint passed 27/27 Results tests, 11/11 receipt tests,
the parallel explicit lifecycle test, and 20 repeated eight-writer runs (2560
append assertions). Those are scoped checkpoints, not measurements of every
later edit.

New final tests are named in `crates/cli/tests/sweep_evidence.rs`,
`crates/cli/src/sweep_wiring_tests.rs`, `crates/runner/tests/auto_reporting.rs`,
and the two shared-writer tests in `crates/cli/src/results.rs`. Final combined
gate results are published separately under `target/sweep-readiness/`; they must
be read with their unchanged-source flag. Earlier Results-only mutation
evidence under `target/sweep-audit-20260906/results-module-mutation` covers its
then-current seven mutants, not this new lifecycle and execution-binding code.

The terminal classification matrix is named
`empty_cold_refused_and_inconsistent_samples_cannot_be_completed_or_resource_halted`
and
`a_real_candidate_budget_halt_and_certified_completion_keep_opposite_terminal_states`.
It includes an actual candidate-budget engine walk, a completed walk, missing
closure proof, and zero samples with a fabricated first index.

The final focused boundary suite passed 8/8, including that classification
matrix and real child/receipt obstruction tests. Both exact stored-identity
wiring guards passed 2/2 on the same compiled test binary. Evidence is saved in
`target/sweep-audit-20260906/final-classification-tests.log` and
`target/sweep-audit-20260906/final-identity-guards.log`. The first budget test
fixture failed to create a real halt; it was replaced with the engine's known
climbing input, preserving the assertion that an actual Candidates breach must
exist. No production halt check was weakened.

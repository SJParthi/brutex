# Retained AND sweep resume — version 1

The retained AND search can save a completed depth boundary and continue from
that boundary through the same Apriori implementation. A saved resource halt
remains a resource halt. It never becomes extinction and its partial frontier
never seeds another depth.

This is a retained-history checkpoint, not a constant-space checkpoint. The
streamed retirement path still exists separately. Saving every earlier survivor
allows the ordinary ranker to score the complete restored history once, with the
same closure decisions and trial counts as an uninterrupted streamed run.

| Operator question | Implemented behavior | Evidence |
|---|---|---|
| Will restarting reset a resource budget? | No. The exact cumulative admitted-candidate and pair counters are restored. | `engine/tests/resume_readiness.rs::real_resource_halts_keep_exact_cumulative_budgets_and_never_advance` |
| Will restarting search a different depth range? | No caller depth field exists. Fresh and resumed runs share `first_level`, `continue_walk` and `next_level`; a resumed run skips rebuilding earlier levels. | `every_safe_depth_resumes_to_the_exhaustive_subset_oracle` |
| Can a failed save let the next depth run anyway? | A fallible callback runs before advancement. Its error exits the shared continuation immediately. | Every reached boundary is interrupted and resumed in the finite oracle matrix. |
| Can a partial frontier look complete? | Checkpoints preserve the halt; the rank adapter requires an extinction witness or a named halt. | Engine resource-halt regression; runner `checkpoint_rank_adapter_refuses_a_missing_terminal_or_mismatched_column` |
| Are high condition bits retained? | Each mask remains six little-endian `u64` words. Test positions cross 63/64 and 127/128 and reach 369. | Independent exhaustive-subset oracle and codec round trip. |
| Will reranking lose earlier combinations? | Every retained level feeds the existing `Accumulator::offer_retired` exactly once. A halted successor cannot prove its predecessor closed. | Runner `checkpoint_ranking_preserves_streamed_scores_trials_and_partial_closure` |
| Can another input or policy reuse a checkpoint? | Engine compares the supplied identity, row count, exact offered sequence, threshold, candidate ceiling, pair budget and requested lane policy. The caller must additionally bind and verify the exact signal/scoring/data bytes. | `resumed_identity_column_offers_and_every_configuration_term_must_match` |
| Can malformed bytes become a plausible answer? | The codec refuses unsupported versions, foreign identity, truncated/trailing bytes, invalid sizes, malformed masks/order, excluded survivor positions, non-reconciling counters and inconsistent halts. Durable cryptographic integrity is the caller's separate obligation. | `truncated_trailing_foreign_and_malformed_payloads_are_refused` and the public resume corruption regressions. |
| Does the stored command actually use recovery? | The stored single-month sweep calls the durable wrapper and the ordinary rank adapter. A fresh attempt restores prior rows and records the current boundary once. | Five passing `cli::and_checkpoint::tests`, including persisted restart and the actual ranked helper. |
| Is memory bounded independently of the answer size? | No. Retaining all survivor history costs O(total survivors). Encoding uses fixed scratch space; decoding allocates the declared bounded history fallibly. | Source contract, not an O(1) claim. |
| Does this prove all historical and future inputs? | No. The tests are finite differential, corruption and resource-boundary evidence. | The matrix explicitly contains 120 input/configuration cases. |

## Integration contract

`engine::Ladder::walk_checkpointed` accepts a prepared engine column, the exact
offered position sequence, a caller-recorded `[u8; 32]` identity and a
`FnMut(&CheckpointView) -> Result<(), String>` callback. It returns
`Result<Sweep, engine::resume::Error>`.

`Ladder::resume_checkpointed` adds the expected identity and an opaque decoded
`Checkpoint`. Its first callback repeats the already saved boundary. A durable
sink must therefore acknowledge an identical boundary idempotently. Earlier
levels are retained but not recomputed. Extinct or halted checkpoints replay one
terminal callback and return their original outcome.

`CheckpointView::write_to` streams the payload through `std::io::Write` without
cloning survivor vectors. It opens no file. `Checkpoint::read_from` accepts a
reader, maximum payload bytes and expected identity. The codec adds no crate
dependency: engine continues to depend on `vocab` alone.

The CLI owns the envelope, checksum/seal, exact input and configuration binding,
append-only checkpoint history, durable publication, recovery and lifecycle
events. A matching identity is not proof that a maliciously substituted column
with the same length contains the same masks. The caller must verify the full
input identity before resuming. Structural codec validation likewise cannot
prove the completeness of an invented frontier; sealed producer history is the
authority for that fact.

`runner::rank_checkpointed_sweep` converts a completed or resource-halted
retained `Sweep` into the ordinary `RankedRun`. It uses the same rank accumulator
and shared result assembly as the streamed ranked path. It performs no new
indicator fold or ladder search. Its caller must bind the signal column,
optional separate scoring column and forward series to the verified identity.

The actual `sweep-stored` operator now calls
`cli::and_checkpoint::run` after recording the exact stored `RunId`, building the
causal signal column and preparing the existing execution/forward series. The
new `Checkpoint::validate_for` precheck runs before any historical depth evidence
is copied into a new attempt.

The CLI wrapper starts with `BRTXAN01`, an exact depth-row count and an exact
engine-payload byte length. It then stores each original `DepthRow` as its ten
existing 64-bit fields and the engine payload. Thus earlier per-depth admitted
and pair counters survive exactly rather than being guessed from the final
counter. A fallible buffer limits the complete journal record to 64 MiB. Exceeding
that physical admission refuses the attempt before advancing; it is not called
extinction or silently truncated.

The shared immutable checkpoint journal seals and acknowledges the complete
wrapper before its corresponding attempt depth is appended. If that depth
append fails, the new checkpoint remains recoverable, the attempt refuses, and
no next depth starts. A new attempt restores only the prior depth rows; the
engine's first resumed callback appends the current row exactly once. Replaying
an unchanged terminal checkpoint adds no duplicate checkpoint reservation.
The final self-contained checkpoint is reopened and compared with its exact
acknowledged sequence and seal before a rankable run is returned. A test changes
the checkpoint after its final callback and requires a refused attempt.
Only this stored single-month sweep entry point is wired here; other entry
points do not inherit resumability from compiling the engine API.

## Exact version 1 payload

All integers below are unsigned little-endian 64-bit words on disk, including
positions and depths; narrowing into Rust types is checked. The mask remains
six words, never a single `u64`.

| Region | Bytes | Content |
|---|---:|---|
| Fixed header | 192 | Magic `BRTXCP01`; wire version 1; vocabulary version; identity; threshold, candidate ceiling, pair budget, requested lane policy; bar count, cumulative admitted candidates and pairs; halt presence and six halt fields; offered, excluded and level counts. |
| Offered positions | 8 each | Exact original sequence, including duplicates and invalid offers. |
| Exclusions | 24 each | Position, support payload, reason tag. Tag 0 is unmeasured/non-live and requires a zero support payload; tags 1/2 are always-false/always-true. |
| Level header | 56 each | Depth, survivor count, generated, duplicates, excluded, pruned, infrequent. |
| Survivor | 56 each | Six mask words, followed by exact support. |

Halt presence zero requires all six payload fields to be zero. Presence one
carries depth, cumulative candidates, ceiling, cumulative pairs, pair budget
and reason (`1` candidates, `2` pairs, `3` allocation, `4` worker creation).
Levels are consecutive from one; a zero-survivor level may only be last.
Survivor masks are strictly canonical and use live offered positions that are
not explicitly excluded. The
accumulated generated count from levels two onward must equal the checkpoint's
cumulative admitted count. These are validation rules, not a second search.

A checkpoint without a named halt requires `pairs < pair_budget`. The shared
join checks its budget once per outer row, including the final empty row. A
named halt can therefore retain an overshoot, including a candidate or memory
halt encountered within that row. Resuming preserves the original halt and
counters; this is not a per-pair hard work or latency bound.

## Resource and verification limits

The checkpoint callback happens at level boundaries. Work in an interrupted
in-progress level is repeated from the last durable completed level. A complete
checkpoint rewrite costs O(retained history); saving every level can repeatedly
write earlier levels. Neither serialization nor reading/ranking/searching a
growing result set is claimed to have O(1) time or space.

The requested support-lane policy is matched exactly. Its existing runtime CPU
upper bound can differ between hosts; successful support results remain
deterministic, but OS scheduling, allocation refusal, worker refusal and latency
are not identical-machine guarantees. Allocation refusal is explicit where
fallible reservation is used. OS overcommit termination and power-loss durability
remain outside an engine-only in-memory test.

The initial nine engine diff survivors were reviewed individually. Six spare
capacity arithmetic sites now use one shared table-size reservation. The
allocation-refusal frontier uses the same constructor as ordinary joined levels.
The duplicated spare-capacity check was removed in favor of `Vec::try_reserve`'s
own check. These are removed duplicate implementation sites, not nine newly
discovered output defects.

The remaining scheduling behavior now has an actual batch-handoff test. It
walks all live singleton positions on two rows, with candidate ceilings 100,
16,385 and 100,000. It checks full 8,192-candidate batches before handoff, retained
budget tails, repeated full batches and exact support accounting. The observer
is local and generic; production uses a zero-sized no-op. There are no global
fault flags or forced OS memory exhaustion.

Measured checkpoint before the final scheduling/refusal additions: all 630
engine and runner tests (including doc tests) passed. The subsequently added
engine batch test and runner refusal test passed separately; all-target scoped
engine/runner Clippy passed after them. The final workspace verifier remains the
authority for the complete combined change. Selected resume/shared-loop mutation
testing finished in an isolated copy: **23 tested, 20 caught, one unviable and
two timed out**, with no selected survivor. The two timeouts removed the shared
loop's extinction/halt stopping conditions. The original premature batch-handoff
mutation was caught by the new resource-behavior test. Evidence is under
`target/sweep-audit-20260906/resume-engine-mutation/`; the test and mutation logs
are named `brutex-resume-*` under `/private/tmp/`.

That earlier selected run did not cover the codec module's then-227 possible
mutations. The later whole-module measurement and its exact replay
reconciliation are recorded below; neither establishes whole-crate coverage.

After extracting the public preflight check so CLI recovery can refuse foreign
state before restoring audit rows, an independent scratch run caught **all 14
`validate_for` mutants**. Its evidence is in
`target/sweep-audit-20260906/resume-preflight-mutation/`. These logically overlap
the earlier policy tests and are not summed into a unique mutation percentage.

The durable CLI wrapper's five tests passed in 5.27 seconds after a 32.96-second
compile. They cover persisted interruption/restart across five threshold/resource
configurations, terminal replay without duplicate checkpoints/depth rows,
actual evidence-append failure, wrapper counter/length corruption, a bounded
writer refusing without truncation, actual ranked-helper rerun, unwarmed refusal
and post-callback final checkpoint corruption. The later test-only Clippy cleanup
is left to the combined final verifier; no broad workspace-green claim follows
from this focused run.

## Final measured verification — 2026-09-06

The final public resume suite has 13 passing tests, including the independent
120-case subset/resume oracle. Added cases exercise interrupted reads and
writes at every payload byte and the final EOF probe, all exclusion reasons,
count rejection before reading a declared payload, balanced but contradictory
accounting, empty frontiers, zero-row and zero-policy states, and halt identity.
One reproduced defect allowed an explicitly always-false bit 64 to survive in
an otherwise consistent checkpoint; excluding those positions from the allowed
survivor mask now refuses it. A second validation correction rejects an
unhalted checkpoint at or beyond its pair budget. Valid version 1 bytes remain
unchanged.

The resource regression uses actual shared-engine outcomes: pair budget 1
retains the outer-row overshoot, and candidate ceiling 2 with pair budget 1
produces a candidate halt after three pairs. Both decode and resume unchanged.
Memory and worker halt tags also have codec tests; those tags do not establish
that a physical allocator or operating-system thread failure was induced.

| Measurement | Final evidence for `crates/engine/src/resume.rs` |
|---|---|
| Line coverage | 540/540, 100% |
| Branch coverage | 134/134, 100% |
| Function coverage | 48/48, 100% |
| Region coverage | 798/815, 97.91%; 17 regions remain unexecuted |
| Reconciled mutation set | 233 unique mutations: 213 caught, 20 unviable; no unresolved survivor or timeout |
| Supporting crate tests | Isolated engine and vocab tests: 202 passed, zero failed or ignored |
| Scoped static checks | Engine/vocab all-target Clippy with warnings denied and scoped formatting passed |

The mutation result is a full pass plus exact replays, not a single raw
zero-miss full pass. The full snapshot had the inclusive no-halt comparison
before the final strictness correction: **233 tested, 209 caught, 20 unviable,
four missed**. Only `Checkpoint::validate` changed afterward. Its complete
current mutation subset was rerun: **31 tested, 30 caught, one missed**. A new
zero-policy regression caught that remaining mutation in a **one-tested,
one-caught** replay. Another **one-tested, one-caught** replay covered the
remaining `validate_level` empty-frontier accounting mutation. The current
31-mutation subset replaces the earlier subset; these runs must not be added
as 266 unique mutations. The effective current set is 213 caught plus 20
unviable.

The exact logs are:

- Full mutation pass: `/private/tmp/brutex-resume-closure-verified-mutation-20260906.log`.
- Current validator: `/private/tmp/brutex-resume-closure-invariants-mutation-20260906.log`.
- Exact remaining replays: `/private/tmp/brutex-resume-closure-zero-policy-mutation-20260906.log` and `/private/tmp/brutex-resume-closure-empty-level-mutation-20260906.log`.
- Coverage summary and lines: `/private/tmp/brutex-resume-closure-verified-summary-20260906.log` and `/private/tmp/brutex-resume-closure-verified-lines-20260906.log`.
- Coverage test execution: `/private/tmp/brutex-resume-closure-verified-coverage-20260906.log`.
- Final resume tests: `/private/tmp/brutex-resume-closure-checks-20260906.log`.
- Combined scoped tests and Clippy: `/private/tmp/brutex-engine-vocab-closure-all-tests-20260906.log` and `/private/tmp/brutex-resume-expression-closure-clippy-20260906.log`.

Raw mutation directories are under `target/sweep-audit-20260906/`, respectively
`resume-closure-verified-mutation/mutants.out/`,
`resume-closure-invariants-mutation/mutants.out/`,
`resume-closure-zero-policy-mutation/mutants.out/` and
`resume-closure-empty-level-mutation/mutants.out/`. The fresh coverage profile is
under `resume-closure-verified-coverage/`. The measured final source SHA-256 is
`902c558104d45e3081d954643411cd36df808bc734e8a6ec156aa81df9e911b4`.

The installed `cargo-llvm-cov` 0.8.4 wrapper failed to discover this nightly
toolchain's executables under `debug/build/CRATE/HASH/out/TEST-HASH` after tests
and profile merge succeeded. The reported measurements use those actual test
executables and the fresh merged profile through the bundled `llvm-cov`
directly, with no data-mismatch warnings. The wrapper's failed automatic report
is not represented as a passing automation gate.

These are module measurements. They do not establish 100% coverage of the
whole engine crate, arbitrary-frontier authenticity, physical OOM recovery,
power-loss durability, or all-input correctness. No real-market sweep or
ingestion operation was run for these checks. The combined workspace verifier
remains authoritative for integration readiness.

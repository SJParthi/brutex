Verdict: 3 findings (0 high, 2 medium, 1 low) in the `cli ledger-all` chain at 331b05c. The chain commits in a sound order inside one process, but it has three problems. An interrupted run can wedge the store-wide Candidate ledgers for every Step-3 verb. A successor check that can never pass runs after every ledger is already durably committed. And the support threshold is sized from a bar read that is taken earlier than, and separately from, the read that is actually swept.

# conc-pass2 / ledgerall: the `cli ledger-all` verb chain

Slice: `crates/cli/src/ledger_all.rs` (`run_chain`, `LedgerTree`, `build_sweepers`, `render_winners`), `all_rung_population_v5.rs`, `all_rung_selection_v5.rs`, and the ledgers they drive: Candidate, Base Evidence V2 and Pre-Admission V1/V2 under the store root; Observation V1/V2, Statistics V2, Search Lineage V4, Admission V3, Finalization V3 and Population V5 under `ROOT/authority/<rung>`; Execution V3 under `ROOT/execution/<rung>`; Selection V5 under `ROOT/selection/<rung>`. Source reading only; cargo was not run.

Reachability: the chain is live. `lib.rs:2275` `["ledger-all", ...] => ledger_all_arm(...)` → `ledger_all::ledger_all` → `run_chain` → `commit_all_rung_stored_population_v5` → `commit_all_rung_stored_execution_v3` → `commit_all_rung_stored_selection_v5`. `mod execution_v3;` (lib.rs:126) carries no `expect(dead_code)`.

---

## ledgerall-1 (medium): one interrupted Candidate, Base or Pre-Admission append in the shared STORE root wedges every later Step-3 run on that store (`ledger-all` and `ledger-v6`, any span) once any input changes

**Where**
- `crates/cli/src/all_rung_population_v5.rs:584-631`: phase one commits all 16 Candidate/Base/Pre-Admission families into `roots.source`, which is `crate::store_root()` (`ledger_all.rs:874`). It does not write them under the operator's ROOT.
- `crates/cli/src/candidate_universe.rs:3701-3709` (`append_complete_locked`):
  ```rust
  let (first_row, prefix) = match self.orphan {
      Some(orphan) => {
          if orphan.universe_id != receipt.universe_id() {
              return Err(format!(
                  "candidate row tail belongs to {}, not requested {}; no fallback may hide it", ...
  ```
- The same rule applies to the other store-root ledgers in the same transaction: `population_base_evidence_ledger_v2.rs:1212` ("physical Base tail belongs to another source"), and `pre_admission_data.rs:1328` and `:2550` ("trailing orphan belongs to …, not exact retry").
- `ledger_v6.rs:328` reaches the same files through `commit_strict_candidate_pre_admission_authority_v1` → `commit_family_with_inputs_v6`, the same function `ledger-all` uses (`step3_orchestrator.rs:3060`).

**Why it is wrong**

`append_rows` (candidate_universe.rs:6258+) writes the rows in chunks of `ROW_WRITE_CHUNK_ROWS` with one `write_all` per chunk. It then calls `sync_data` and only after that appends the receipt. A process that dies between two chunks, or between the row sync and the receipt, leaves a receipt-less whole-row orphan. `scan_orphan` accepts it, and from then on the writer accepts only a block whose `universe_id` equals the orphan's. That id is derived from the data digest, the feed, the **source commit**, the rung, the family and the span (`CandidateUniverseIdentitiesV1`, candidate_universe.rs:174-191; `derive_universe_id` :5504).

This repeats the class of pop2-4 and hunt-cli-a-5, but those reports cover per-rung roots. Three things make this case worse:
1. **Blast radius.** The ledger is one file per store, shared by every Step-3 run against that store. That covers every span and both verbs, and with them every ROOT an operator might use to keep runs apart. The usage text says ROOT holds these ledgers (lib.rs:543-550, "WRITES the ledgers -- candidate, pre-admission, … -- under ROOT"). They are written in the store.
2. **The trigger needs no rebuild.** If the TO month is the current month, the autopilot's appends change `data_digest`. The interrupted universe then cannot be prepared again by anyone, ever.
3. **There is no signal handling.** The cli installs no handler (no `ctrlc`/`signal_hook`/`sigaction` in crates/cli). An operator's Ctrl-C during a multi-hour `ledger-all` therefore lands wherever the process happens to be.

Universes that are already complete still reuse, because the `audits` lookup comes before the orphan check. Only new universes refuse. A Step-3 run whose span, build or data differs from the interrupted one can never commit again until someone truncates the store file by hand.

**Repro**
1. `cli ledger-all V 2025 1 2026 10 S P /x/ledgers` on 2026-10-03, during market hours. Press Ctrl-C while `append_rows` for 3min BANKNIFTY is between chunks.
2. The autopilot appends 2026-10 minutes.
3. Rerun the identical command. 1min NIFTY prepares a universe U' whose `data_digest` differs from the stored one. U' has no receipt, so it reaches the orphan branch and refuses: `all-rung 1min NIFTY refused: … candidate row tail belongs to <U>, not requested <U'>`.
4. `cli ledger-v6 V 2024 1 2024 12 S P /y` (another span, another ROOT) refuses the same way at its first universe not yet complete. So does every later run on that store.

**Minimal fix**

Under the exclusive writer lock, a receipt-less tail is provably unacknowledged. Choose one of these, and make the same change in the Base Evidence V2 and Pre-Admission V1/V2 writers:
- (a) Let the writer leave a fully valid foreign orphan in place as unreferenced evidence and start the new block after it (the Execution V1 EC-01 rule). `scan` then needs to index orphan ranges.
- (b) Let the writer truncate a foreign whole-row orphan to `committed`, `sync_all`, and emit a loud telemetry event that names the discarded universe.

At a minimum, the refusal must name the file and tell the operator the recovery step. The usage text at lib.rs:543 should also say these ledgers live in the store, not under ROOT.

---

## ledgerall-2 (medium): Selection V5 commits Top-*min(eligible, 25)* per rung, but the chain's own successor check requires exactly 25. The check runs after all three stages are durably committed, so every run where any rung admits fewer than 25 is reported as refused, and every rerun repeats it

**Where**
- `crates/cli/src/selection_v5.rs:634`, where the commit-time invariant is `self.selected_count != self.eligible_count.min(REQUESTED_TOP_U64)`, so fewer than 25 is a legal commit. `all_rung_selection_v5.rs:696-707` (`require_rung_receipt_proof`) also accepts `selected_count <= 25`.
- `crates/cli/src/all_rung_selection_v5.rs:362-372` (`preflight_successor_rung`):
  ```rust
  let winners = selection.successor_winners()?;
  if winners.len() != MAX_WINNERS_USIZE {
      return Err(format!(
          "all-rung {rung_name} successor requires exactly 25 winners, observed {}", ...
  ```
- `crates/cli/src/ledger_all.rs:917-933`. Here `stage_finished_event(.., SELECTION_STAGE, ..)` is emitted and "BLOCKS WRITTEN …" is printed. Then `render_winners(out, selection)?` → `selection.into_successor_set()?.visit_canonical(..)` (`:965`) runs the preflight.

**Why it is wrong**

The ordering is commit first, check second, and the second check is stricter than the first. `successor_winners()` returns `top_twenty_five()`, whose length is `selected_count = min(eligible, 25)`. On any span where one rung has fewer than 25 eligible (admitted) candidates, the following happens:
- Population V5, Execution V3 and Selection V5 for all eight rungs are already synced and their directories fsynced.
- The telemetry says the Selection stage finished.
- Then `ledger_all` prints `refused: all-rung Xmin successor requires exactly 25 winners, observed N`, emits `run_refused`, and the arm exits `MISUSED`.

None of the eight rungs' winners are shown, including the rungs that had 25. A rerun reuses every block byte for byte and refuses identically, forever, for that span. A durable commit reported as a failure is the success/failure confusion that CLAUDE.md §4 bans, here in the reverse direction.

With 39 admission gates, fewer than 25 admitted candidates on a rung is an ordinary outcome. The tightest and loosest rungs (1min/60min) are the likely cases.

**Repro**

Run `cli ledger-all …` on a span where 60min admits 7 candidates. All stages commit: "Population V5 8 Execution V3 8 Selection V5 8" is printed. The next line is `refused: all-rung 60min successor requires exactly 25 winners, observed 7`, and the exit code is non-zero. The rerun prints "Selection V5 0" written and then the same refusal.

**Minimal fix**

Make the successor's arity agree with the committed semantics. Either have `preflight_successor_rung` accept `winners.len() == receipt.selected_count() as usize` (≤ 25), or, if the Global Replay successor really needs 25, refuse inside `commit_selection_rung` *before* `commit_stored_selection_v5` writes. Either way, have `render_winners` render what was committed rather than gate the report on the replay successor's arity.

---

## ledgerall-3 (low): the support threshold is sized from one read of the store, and the bars swept come from a second, later read. A concurrent pull between the two makes the recorded run unreproducible from its recorded inputs

**Where**
- `crates/cli/src/ledger_all.rs:1400-1420` (`build_sweepers`): `let span = crate::stored::load_span(root, vendor, "NIFTY", rung, request.from, request.to)` → `let min_hits = crate::min_hits_for(span.bars.len(), request.support_ppm)` → `Sweeper::new(ladder)`, for all eight rungs, before any commit (`run_chain`, `:877`).
- `crates/cli/src/step3_orchestrator.rs:3068`: each family later reloads its bars with `load_bounded_stored_context_v1(&request, &root, config)`. This happens hours later for the 60min rung, because phase one sweeps 16 families in sequence.
- `step3_orchestrator.rs:2813`: `runner::identity::Params::of(self.sweeper_ladder)` folds the stale `min_hits` into identity. The candidate's `data_digest` (candidate_universe.rs:176) is taken from the *second* read.

**Why it is wrong**

Bar files for the current month are appended in place by the pull and autopilot path, and `ledger-all` holds no lock on the store. If minutes for the TO month land between T0 (sizing) and T1 (the family's own load), the sweep runs with `min_hits = floor(N0·ppm/1e6)` over N1 > N0 bars. The durable Candidate block then records {data_digest(N1), ladder(min_hits from N0)}, and no single read of the store produces that pair. Rerunning the same command on the same, unchanged store computes min_hits from N1, so it gets a different ladder and a different `universe_id`. It appends a second universe instead of reusing the first. This breaks §3 rule 5 (same inputs, same bytes) and §3 rule 3 (identity names what was computed). It also triggers ledgerall-1's wedge if that second attempt is interrupted.

`min_hits` changes whenever N crosses a multiple of `1e6/ppm`. At 1000 ppm on the 1min rung that is every 1,000 bars, under three trading days, so a run during market hours can cross it.

**Repro**

Start `cli ledger-all V 2026 1 2026 10 1000 P R` at 10:00 IST. `build_sweepers` sizes 1min at N0 = 69,999 bars, so `min_hits = 69`. The autopilot appends 1 minute. The 1min NIFTY family loads N1 = 70,000 bars, sweeps at 69, and commits. Rerun after 15:30 on a frozen store: `min_hits = 70`, a different universe, so the blocks are written again rather than reused, and the two runs disagree on the frontier.

**Minimal fix**

Derive `min_hits` from the bars the family actually loads, inside the commit and after `load_bounded_stored_context_v1`. Alternatively, pass the sized `span` (or its data digest) into the commit and refuse if the reloaded data digest differs. `ledger_v6` sizes each rung just before its commit (`size_sweeper`), which narrows the window but has the same two-read shape.

---

## Pass-1 verification (findings that touch this chain)

| pass-1 id | verdict | reason (from code at 331b05c) |
|---|---|---|
| pop2-1 Admission V3 mid-block crash wedge | CONFIRMED | `population_admission_v3.rs:3987-3989` writes one `append_raw` per 2,048-byte decision. `:3699-3720` indexes a whole-record proper prefix as `trailing`. `complete_trailing` (`:4071-4077`) demands `trailing.decisions == prepared.decisions`, so even the exact retry refuses. |
| pop2-2 Finalization V3 / Statistics V2 no write-error rollback | CONFIRMED in code. Finalization V3 is practically unreachable. | Both still use plain `seek(End)+write_all` (`population_finalization_v3.rs:2857-2861`, `population_statistics_v2.rs:5479-5486`). Finalization V3 files have no header, and its strides are 2,048/4,096 at aligned offsets, so each record lies inside one 4 KiB page/block. ENOSPC/EDQUOT on a 4 KiB-block filesystem then allocates the block whole or fails with 0 bytes, which leaves no ragged tail. That case is reachable only on block sizes below 2 KiB. Statistics V2 is fully confirmed: its 64-byte header (`:61-69`) shifts the 1,024-byte records, so every fourth one straddles a block. |
| pop2-3 SIGKILL mid-record leaves a ragged tail | REFUTED for Admission V3 and Finalization V3. CONFIRMED for Statistics V2. | One `write(2)` of ≤ 4,096 bytes that does not cross a page boundary is copied in a single `generic_perform_write` iteration. The fatal-signal check sits between page iterations, so it cannot split the write. Admission V3 (decisions 2,048, completions 4,096) and Finalization V3 (rows 2,048, completions 4,096) are headerless and stride-aligned. Statistics V2 has the 64-byte header (`record_count`, `:5435-5445`), so a torn straddling record is real. |
| pop2-4 receipt-less trailing prefix only accepts the exact retry, and the identity includes the build commit | CONFIRMED | Admission V3 `:4071`. Statistics V2 `resume_orphan` (`:4610-4616`, "trailing orphan belongs to …, not exact retry"). In `ledger-all` the same rule also holds in Execution V3 (`require_exact_prefix`, `execution_v3.rs:2325-2357`) and Selection V5 (`selection_v5.rs:2168`, "orphan tail is not the exact canonical retry prefix"). ledgerall-1 shows that a change in the data alone is a sufficient trigger and that the store-root ledgers carry the same rule. |
| pop2-5 Statistics V2 never fsyncs its directory | CONFIRMED | The only syncs in production code are file `sync_all`s (`:4582`, `:4591`, `:4636`, `:4645`, `:5404`). There is no root `File` and no directory sync. |
| pop2-6 Statistics V2 cannot recover a torn header (1-63 bytes) | REFUTED as stated | `ensure_header` (`:5395-5408`) writes the 64-byte header as one `write_all` at offset 0 of an empty file. That single-page write cannot be split by a signal, and `i_size` goes from 0 to 64 in one step, so a file of 1-63 bytes is not produced by kill. A power loss on a filesystem without data ordering could leave 64 bytes with stale contents. `verify_header` would refuse that as a bad seal, but it is a different mechanism and pass-1 did not show it. |
| pop2-7 nothing serialises runs on one ROOT; a concurrent append invalidates retained authorities | CONFIRMED for ledger-all | `ledger_all.rs` takes no lock or lease in production code. Execution V3's retained generation is a double full-content hash plus mtime and inode (`execution_v3.rs:3883-3960`), and it is re-checked by `require_live_topology` before Selection. |
| pop1-2 Population V5 short write leaves a ragged tail | REFUTED for V5 (the V6 and Observation parts are outside this slice) | `population_v5.rs` is headerless, with rows of 4,096 and completions of 1,024 (`:65-67`) at aligned offsets. The same alignment argument as pop2-2 and pop2-3 applies, so the tear cannot happen by ENOSPC on a 4 KiB block or by SIGKILL. |
| pop1-3 Observation authority ledgers never fsync their directory | CONFIRMED | `population_observations_v1.rs` production code has only file `sync_data` (`:2180`, `:2281`, `:2304`, `:3361`, `:3435`, `:3445`) and no directory sync. In `ledger-all` these files live in `ROOT/authority/<rung>`. |
| search-2 Pre-Admission V1/V2 append has no rollback | CONFIRMED (store root, written by ledger-all phase one) | `pre_admission_data.rs:3595-3599` uses `seek(End)+write_all` with no `set_len` anywhere in the file. The records are 740 and 812 bytes, so they straddle blocks. |
| sel-1, its reachability statement ("`execution_v3`, `selection_v5`, `all_rung_*_v5` are `expect(dead_code)`") | REFUTED | `mod execution_v3;` (lib.rs:126) has no attribute. All three are reached in production through `lib.rs:2275` → `ledger_all_arm` → `run_chain`. The `expect(dead_code)` on `all_rung_*_v5` is satisfied by other items in those modules, not by the commit doors. The Execution V3 copy of sel-1's hazard is still not reachable, for alignment reasons: headerless, strides 1,024/128 (`execution_v3.rs:66-72`). So no new finding follows from the refutation. Selection V5 has `append_with_rollback` (`:3028-3055`), as sel noted. |

## Checked and clean (no finding)

- **Commit order inside one run.** All eight rungs' store-root families are frozen before any successor is retained (phase one). Each rung then commits Observation → Statistics → Search V4 → Admission V3 → Finalization V3 → Population V5 into its own disjoint directory (`all_rung_population_v5.rs:640-688`). After that come Execution V3 for each rung (`:772-843`) and Selection V5 for each rung. Each writer syncs data before its Completion and syncs the Completion before the directory (for example `execution_v3.rs:2224-2268`, `:2438-2460`).
- **Rerun after a crash between ledgers**, with unchanged inputs: every committed block is reused through the `receipts`/`audits` lookup, which runs before any trailing logic. A whole-record trailing prefix of the same block completes in Execution V3, Finalization V3 and Candidate. Admission V3 is the exception (pop2-1).
- **Cross-ledger binding.** Execution V3 joins on `population_id` and the Population receipt (`all_rung_population_v5.rs:859-894`, `:1013-1053`). Selection V5 joins on the Execution completion id (`all_rung_selection_v5.rs:657-665`). Each retained authority re-derives its source after it persists (`execution_v3.rs:2997-3018`). I found no path where a downstream block can bind a divergent upstream digest without a refusal.
- **Retained store-root authorities.** `CommittedCandidatePreAdmissionV1` keeps only audits (step3_orchestrator.rs:219-225). Successor reads reopen under a shared flock and check the generation before and after, so a concurrent ledger-v6 append to the store-root ledgers does not invalidate a running ledger-all's retained Candidate authority.
- **Locks held across the run.** Observation and Statistics V2 readers release their flock when open returns (`population_observations_v1.rs:2383-2395`, `population_statistics_v2.rs:2192-2230`). Execution V3 and Candidate lock only for the duration of open and append. No lock is held across another ledger's lock, so no deadlock is possible between two ledger-all processes, or between ledger-all and ledger-v6.
- **ledger-all and ledger-v6 on the same ROOT.** Both use `ROOT/execution/<rung>` and `ROOT/selection/<rung>`, but the file names are disjoint (`*-v3.bin`/`execution-v3.lock` against `*-v4.bin`; `global-selection-*-v5` against V6), so neither corrupts the other.
- **Nondeterminism.** There is no rayon, thread or wall-clock value in the production code of the chain modules. HashMap/HashSet are used only for keyed lookup and duplicate checks (`execution_v3.rs:1928`, `population_v5.rs:1983`, `all_rung_selection_v5.rs:810-847`). Bootstrap draws use the fixed `BOOTSTRAP_SEED`.
- **`LedgerTree::create`** uses `create_dir_all` without a parent fsync. Each ledger fsyncs its own leaf when it creates files. A lost parent entry after power loss costs recomputation only, never a wedge, so I did not report it (the class is xcut-3).

Outside this angle (not counted): `AdmittedDirectoryV1::admit` requires the exact canonical spelling (all_rung_population_v5.rs:1255-1266), and `ledger_all` passes `request.root` unchanged. A relative or symlinked ROOT therefore refuses after `LedgerTree::create` has already made the 24 directories.

# conc-pass12: kill and resume, at 1f4de71

Head: `/home/claude/wt/zero3`, detached at origin/final/all-fixes-zero 1f4de71. Audit only. One throwaway test was run in a scratch worktree, which has since been removed.

**Counts:** 1 new finding (0 high, 1 medium, 0 low). 12 known rows re-verified: 2 FIXED, 1 PARTIAL, 9 NOT FIXED.

## Coverage: each interruptible operation against the five theme questions

| Operation | (1) byte-identical resume | (2) checkpoint vs data order | (3) identity re-derived on resume | (4) stale lock or pid | (5) month queue |
|---|---|---|---|---|---|
| `search_checkpoint::Journal` (all resumable searches) | yes. The marker is renamed into place (D-1909). A marker-less reservation is counted as interrupted. | The payload is written, synced and verified first. Then come the dir sync, the `complete.writing`, sync, rename and dir sync. The marker never precedes durable data. | The namespace is `<format>/<identity hex>`. The header carries the identity and is re-checked on every read (`read_saved`). | The owner lock is a `flock` on `owner.lock`, which the kernel releases on death. No pid. | n/a |
| expression search (`expression_search.rs`) | The state counters and cursor are identical. The report adds an "interrupted reservations" count, which is honest and not a defect. | The candidate evidence and attempt `finish` precede `journal.publish` (:563-575). A kill in between re-evaluates the candidate under a new attempt token. Nothing is skipped or double-counted, because `verify_history` follows only the pinned attempt. | `search_identity(Run)` holds all 9 terms, and VOCAB_VERSION sits in `identity.rs:810`. Changed data, commit or vocab means a new namespace, never a mix. | flock only | n/a |
| AND sweep checkpoint v2 (`and_checkpoint.rs`) | The engine resumes from the last boundary. Earlier depth rows are re-emitted into the new attempt (:174-178). | Level chunks, then the boundary record, then `attempt.level`. An orphan chunk names its boundary (`recover` :226-262). | The namespace is `attempt.identity()` (the 9-term run id). `validate_for(ladder, column, live, identity)` runs as well. | flock | n/a |
| Boolean grammar campaign (`boolean_grammar_campaign.rs`) | The PLAN is published before the campaign and DONE after it. A pending PLAN resumes the same batch (:178-185). | DONE follows `verify_complete` of the campaign. | The identity hashes the descriptor (sources and policy), the initial cursor and the budgets. `restore` replays every batch exactly (`Batch::decode` re-derives it). | flock | n/a |
| ledger-all (Population V5, Execution V3, Selection V5) | Byte-identical reuse on an exact rerun. | Rows are synced, then the Completion. | The block ids bind the verified commit. A foreign orphan is refused, not mixed (pop2-4 PARTIAL, below). | flock | n/a |
| ledger-v6 | as above. Selection V6 has the D-1569 quarantine. | | | | conc4-1 still open |
| sweep evidence (`sweep_evidence.rs`) | n/a | Journal allocation, then the reservation, then the lifecycle start, then the identity start, each behind its own barrier. Torn tails are healed by the writer (D-1901). | A superseded attempt is refused at `append_identity_start`. | A killed attempt stays `Running`, with no terminal (sweep-1 family). | n/a |
| pull ingest (`pull/src/ingest.rs`) | Overlap re-offers verify byte-for-byte and append only the suffix (`store/file.rs:2233-2330`). | The bars are on disk before the census install (:936). A kill in between leaves the census lagging, which self-heals: the next pull overlaps, `append` accepts it, and `count` reads the header. The module doc at :29 says "a store that correctly refuses the append", which is stale wording, not a defect. | n/a | flock | |
| api autopilot (`api/src/autopilot.rs`) | n/a | The store is the authority, re-probed after every tick (:3576-3595). | n/a | In memory only (doc :62-68). `serve.lock` is a flock. | **conc12-1** |
| cash-session cache | n/a | payload, then receipt | n/a | flock | pull2-3 still open |

There are no pid or lock files that are created and never cleaned up. Every production lock is an `flock`, and the kernel releases it when the process dies. `serve.lock` quotes a stale pid in text only, and only on `WouldBlock` (D-1911). Every production `create_new` site is a data or scratch file, never a lock sentinel.

## New findings

### conc12-1 (medium): every restart re-fetches, two dry rounds per rung, every complete month whose last calendar day is not a session day. The F&O press re-fetches each such contract month on every press.

**Where:**
- `crates/api/src/autopilot.rs:762-783` (`next_window`), `:700-706` (`month_span`), `:1429-1436` (dry-round retirement), `:2469-2509` (`frontier`) and `:2550-2560` (`drivable` starts every feed at its floor).
- `crates/pull/src/fnowork.rs:423-440` (`owed`) with `:459-470` (`span_of`).

```rust
// month_span: the "last askable day" is the CALENDAR month end
let last = first.end_of_month();
let to = if last < yesterday { last } else { yesterday };
// next_window: held through a Friday < a Saturday/Sunday month end => "behind"
Some(day) if day < to => { ... next }
// observe: a month is passed only after DRY_ROUNDS (2) empty ticks, in memory
self.dry = self.dry.saturating_add(1);
if self.dry >= DRY_ROUNDS { ... return Next::Advance; }
```

**Why it is wrong:**
- `month_span` measures "complete" against the calendar month end. A month that ends on a Saturday, a Sunday or a holiday therefore never reads as complete, however fully the store holds it.
- Within one process, the monotone hint moves past such a month after two dry ticks.
- On a restart the hint dies (doc :62-68: "A restart re-derives everything from the store"). Every feed starts again at its floor (`drivable`), so every such month is owed again.
- Each owed month costs `DRY_ROUNDS` = 2 ticks per rung (Day1 and Minute1). Each tick asks `SpotTarget::Everything`, about 765 instruments (:3448-3505), for a window that holds only non-session days.
- Over 2020-01..2026-09, 23 of 81 months end on a Saturday or Sunday (counted with `date`), before counting month-end holidays. That gives roughly 23 × 2 × 2 × 765 ≈ 70,000 vendor requests per restart that cannot return a bar.
- The ladder climbs oldest-first, so these requests also delay every real gap after them, including yesterday's day.
- This is the "repeat a month on resume" case of theme question 5. `docs/07-plan.md` §9.3 says EXPECTED is "trading days", and `pull::calendar::kind_of` is the canonical session authority CLAUDE.md §5 names. Neither path consults it.
- `fnowork::owed` has the same comparison against `opens.end_of_month()`. Its doc fixes the analogous case for an expiry ("refetched on every run, returning empty every time") but not for a non-expiry month that ends on a weekend. The F&O press has no dry-round retirement, so each such cell is re-asked on every press.

**Repro (ran):** throwaway test `conc12_restart_refetches_every_complete_month_ending_on_a_weekend`, added to the `autopilot.rs` test module in a scratch worktree at 1f4de71 and run with `cargo test -p api --lib conc12_`.
- Setup: a store holding NIFTY and BANKNIFTY for every month of 2020, each through its last Monday-to-Friday.
- Two simulated processes, each a `FeedState::new` at the floor, feed `frontier` and two dry `TickOutcome`s per owed month.
- Output: `run 0: 3 owed months, 6 dry ticks: [("2020-02","2020-02-29","2020-02-29",2), ("2020-05","2020-05-30","2020-05-31",2), ("2020-10","2020-10-31","2020-10-31",2)]`.
- `run 1` printed the identical line: the restart repeated all of it.
- Test result: ok, 1 passed. The F&O site was not run.

**Minimal fix:** clamp `to` in `month_span` (and `through` in `fnowork::span_of`) down to the last day in the span for which `pull::calendar::kind_of` does not answer `Closed`. That is the same authority `fold.rs:880` already uses. Keep the dry-round rule only for days the calendar reports as `Unmeasured`. Add the test above, asserting that `run 1` owes nothing.

## Verification of known rows this theme touches (state at 1f4de71)

| id | state | evidence |
|---|---|---|
| CE-3 | FIXED | `search_checkpoint.rs:283-298`: the marker is written as `complete.writing`, then `rename`. `:509-514`: a short marker counts as interrupted (D-1909). |
| pop2-4 | PARTIAL | D-1905's `discard_orphan` is not applied in the live `ledger-all` writers. `population_v5.rs:2232-2239` still refuses a foreign trailing block, and its test `foreign_orphan_..._fail_closed` pins that. `execution_v3.rs:2332` and `selection_v5.rs:2165` do the same. A kill followed by a rerun on a new commit wedges the rung. |
| pop2-1 | NOT FIXED in V5 | D-1906 covers Admission V3. Population V5's `complete_trailing` requires `trailing.population_id == prepared` (:2232), which still blocks any different retry (it accepts an exact prefix). Tracked under pop2-4. |
| conc4-1 | NOT FIXED | `selection_v6.rs:392`: the quarantine name is keyed by offset only, and a partial `abandoned-X` file is left behind on error. |
| conc4-2 | NOT FIXED | `population_v5.rs:2206-2216` / `:2250-2259`: plain `sync_data`, no `fixed_tail` failed-barrier memory. |
| engine-1 | NOT FIXED | `engine/src/lib.rs:1666-1668`: `sink.checkpoint` is called for every halt, including Workers and Memory, so a transient halt is resumed as terminal. |
| sweep-1 | NOT FIXED | `api/src/sweeprun.rs:2705-2719`: a stale `command started` marker maps to `unknown`. `:2748`: `launch_clear` holds only for completed or refused. A SIGKILLed CLI sweep blocks browser launches. |
| expr-2 | NOT FIXED | `boolean_candidate_persistence.rs:110`: resume still takes a non-blocking exclusive owner lock for children already complete. |
| recovery-2 | NOT FIXED | `api/src/recovery.rs:777`: `Journal::open(active_path)` creates an empty `active.bin` before the pointer append. A kill there refuses every later start. |
| pull2-3 | NOT FIXED | `pull/src/cash_session_cache.rs:500-507`: payload, then receipt. A kill in between leaves "incomplete cash-session cache" for good. |
| autopilot-4 | NOT FIXED | `server.rs` shutdown `flying.abort()` mid-tick, with no audit record. |
| CE-23 | FIXED | `autopilot.rs:2487-2491`: `frontier` clamps the hint to yesterday's month (D-1767). conc12-1 is a separate restart cost. |

Tally: FIXED 2 (CE-3, CE-23), PARTIAL 1 (pop2-4), NOT FIXED 9 (pop2-1-in-V5, conc4-1, conc4-2, engine-1, sweep-1, expr-2, recovery-2, pull2-3, autopilot-4).

## Checked and clean (this theme)

- **Expression search.** An exhausted rerun publishes nothing (:529-531). A kill between candidate evidence and the checkpoint re-evaluates the candidate under a new attempt token, and the superseded attempt cannot replace it (`append_identity_start` :1044-1049).
- **AND checkpoint v2.** A kill after a level's chunks but before its boundary resumes from the previous boundary. A depth-1 orphan with `previous == 0` restarts clean.
- **Grammar campaign.** A kill after the campaign completes but before DONE re-runs `run_prepared` on the same campaign identity and pins the same receipt. Changed sources produce a new descriptor and so a new namespace. Changed byte or record env ceilings refuse (`Batch::decode`) rather than mix, because budgets are hashed and bytes are not.
- **Sweep-evidence journals.** Torn header or tail are healed by the writer (D-1901). An empty file counts as no rows.
- **Pull ingest.** A census lagging behind the bars self-heals through the verified overlap append.

STATUS: IN PROGRESS

# R3-17 — one upkeep runner (cloud build report)

Branch `cloud/db-R3-17-20261017`, cut from `origin/feat/db-fix` at `50093fa3`; the newest
`origin/feat/db-fix`, `f9cdb584` (`Merge R3-14: …`), merged in on 17 Oct 2026 once R3-06 landed.

## Prerequisites

All three merged into `origin/feat/db-fix` (re-checked 17 Oct 2026 after the operator's CONTINUE):

```
$ git log origin/feat/db-fix --merges --format=%s | grep -E "^Merge (R3-06|R3-16|R3-36):"
Merge R3-06: Month summary kept beside each month (time-of-day index, extremes, day counts, gap result), and the gap audit reads it
Merge R3-36: Store steps as one checked list: every pending run with its command, its prerequisite and its proof
Merge R3-16: Shared safe file helpers: one way to create a temporary file, one way to open a file for reading
```

## Files

Owned files, as the tree spells them (the spec's names drifted; the operator's defaults):

| Spec name | File in the tree | What changed |
|---|---|---|
| `crates/pull/src/upkeep.rs`, `upkeep_tests.rs` | same (new) | the runner |
| `crates/cli/src/upkeep_command.rs` | same (new), with `upkeep_command_tests.rs` (new: its tests, the repo's `#[path]` idiom) | the verb |
| `crates/cli/src/srt_build.rs` | `crates/cli/src/srt_builder.rs` | `upkeep_month`, `symbol_id`; `vendor` and `month` made `pub(crate)` |
| `crates/cli/src/tix_rebuild.rs` | `crates/cli/src/tix_rebuilder.rs` | `upkeep_month` |
| `crates/cli/src/summary_build.rs` | `crates/cli/src/msum_builder.rs` (R3-06 named it) | `upkeep_month` |
| `crates/cli/src/census_recount.rs` | same | `upkeep_span`; `day` made `pub(crate)` |
| `crates/cli/src/gdfl_rekey.rs` | same | `upkeep_underlying` |

**Lines outside the owned files (registration and docs only):**

- `crates/pull/src/lib.rs`: `pub mod upkeep;` and its doc line.
- `crates/cli/src/lib.rs`: `mod upkeep_command;`, `"upkeep"` in `COMMANDS` (52 → 53), one dispatch arm, the
  usage lines.
- `crates/cli/src/store_write.rs`: `"upkeep"` in `WRITERS` (13 → 14), its `writes_the_store` arm
  (`run` and `resume` write), the doc sentence, and the row in the test table of every writer.
- `crates/cli/src/gdfl_import_command.rs`: one re-export line, `pub(crate) use guard::{Holder, RunLock, escape};`,
  so the upkeep takes the import's own run lock (no other change to the import).
- `docs/05-decisions.md` (D-4997, appended), `docs/04-invariants.md` (UK-01 to UK-14, appended),
  `docs/06-limits.md` (L-R317-01 to -09, appended), `docs/02-store-format.md` (§43, appended).

`crates/pull/src/gdfl_nfo.rs` is NOT touched.

## Done

- **`pull::upkeep`**: one tracked run of a builder over a plan of item keys. Journal
  `audit/upkeep.<vendor>.<kind>.log` (three strict line shapes; every item line carries the run's
  totals, so `status` is one 16 KiB tail read and one `flock` probe); one run of a vendor's kind at a
  time (the journal's `flock`); torn tail ended as a void line; run id = clock or newest + 1; Stop
  by the caller's flag or a request file naming the run (stale requests stop nothing, nothing is
  deleted); resume along a chain, skipping only what was closed CLEAN; N workers from one counter,
  outcomes in plan order; a journal that stops taking lines fails the run and counts only what it
  recorded. `months` walks a scope and says what each month file lacks (a three-bit set), beside
  the generation a reader opens.
- **`cli upkeep plan | run | resume | stop | status`**: per-month kinds `tix`, `srt`, `msum`
  (the months lacking that file, or `--all`), `recount` (one calendar month of `--from DAY --to DAY`
  an item), `rekey` (one GDFL underlying an item, applied only when its survey read every file and
  left nothing unresolved). Every item is built by the verb's own step; the receipt line is its
  reason. A GDFL run takes the import's run lock of every kind first (v3: the one store lock).
  `upkeep run|resume` joins the unstamped-build check.
- **DB-027**: the three impossible NIFTY `25JUN20` strikes are retired from the census by an
  upkeep re-key run (`the_three_impossible_strikes_are_retired_by_an_upkeep_rekey_run`). Their
  refusal at decode (`StrikeImplausible`, D-3331) was already in `gdfl_nfo.rs`.
- **DB-N14**: the store half: one run gives a whole feed its sort index
  (`a_whole_feed_gets_its_sort_index_in_one_run`). The request half (only months with no `.srt` are
  read row by row) is AF-05's, merged; `docs/06-limits.md` L-R317-09 says so.
- **The four named tests**: `an_upkeep_run_stops_and_resumes_where_it_stopped`,
  `upkeep_and_an_import_exclude_each_other_on_a_month`,
  `the_sort_index_is_the_same_bytes_at_1_and_8_workers`, `a_failed_index_open_is_retried`
  (the reader half of the last is already `api::bars_sort_tests::a_host_refusal_of_an_index_is_not_remembered`).
- **Docs**: D-4997, UK-01 to UK-14, L-R317-01 to -09, store format §43.

### Measured (cloud host, synthetic store — not the operator's private copy)

Release `cli` of this branch, `cli upkeep run KIND gdfl --workers N`, fresh copies of one synthetic
store (48 one-second spot months of 100,000 invented bars, no sidecars), 4-core Intel Xeon @
2.10 GHz, two rounds, months per second:

| builder | 1 worker | 4 workers | 8 workers | 8 ÷ 1 |
|---|---|---|---|---|
| srt | 10.3, 10.8 | 26.6, 20.9 | 26.1, 30.4 | 2.5×, 2.8× |
| tix | 83.1, 80.6 | 352.9, 266.6 | 358.2, 355.5 | 4.3×, 4.4× |
| msum | 104.5, 96.3 | 363.6, 335.6 | 309.6, 303.7 | 3.0×, 3.2× |

The pass line (≥ 4× at 8 workers) is met here by `tix` only; eight workers on four cores cannot pass
4×, so the host's cores are what holds `srt` and `msum` back here. Each stage's share inside a
builder is NOT MEASURED (it needs timing inside the store's doors, not R3-17's files). L-R317-01.

## Left

- Coverage (line and branch) and mutation testing of the changed lines, then this report's final
  gate lines.

## Gate results (so far)

- `cargo fmt --all --check`: clean.
- `cargo clippy -p cli -p pull --all-targets --locked -- -D warnings`: clean (workspace run pending).
- `cargo test -p pull --lib upkeep`: 21 passed. `cargo test -p cli --lib upkeep_command`: 13 passed.
  The workspace run is in progress.
- `cargo deny check`: **advisories FAILED; bans, licenses and sources ok** — exactly the base's five
  (RUSTSEC-2026-0344 ruzstd 0.8.2; RUSTSEC-2026-0346/0347/0348/0349 rustls 0.23.45), on the base
  (`f9cdb584`, a worktree) and on this branch alike. `Cargo.toml`, `Cargo.lock` and `deny.toml` are
  unchanged by this branch (DENY-1 owns the bumps).
- CI gates run locally with the repo's own tools: gate 27 (invariant ids) and 27b (decision numbers)
  OK. Gate 10 fails on the base and on this branch alike, on rows that are not R3-17's (R3-01's web
  rows at lines 7365–7408, GI-08's missing test); every UK row resolves. Gate 11 fails on the base
  and on this branch with the same 106 refusals; none is in an R3-17 file.

## NEEDS THE MAC

1. **Decision and section numbers at landing**: D-4997 (head was D-4996; D-4990..D-4995 are on
   R4-17's branch) and store format §43 — renumber if taken (amendment A6).
2. **Store steps** (`config/store-steps.toml`, R3-36's gate 30): add R3-17 as a `[[package]]` and
   one `[[step]]` per kind the operator wants run, with the rehearsal figures gate 30 asks of a
   merged package, from a private copy (`BRUTEX_STORE=<private copy>`):
   ```
   cli upkeep plan gdfl
   cli upkeep run srt gdfl NSE/INDEX/NIFTY --rung 1s --workers 8
   cli upkeep run srt gdfl NSE/INDEX/BANKNIFTY --rung 1s --workers 8
   cli upkeep run tix gdfl --workers 8
   cli upkeep run msum gdfl --workers 8
   cli upkeep run rekey gdfl --underlying NIFTY
   cli upkeep run recount gdfl --from 2019-11-14 --to 2020-09-15
   cli upkeep status srt gdfl
   ```
3. **The measured target, on a private copy**: months per second at 1, 4 and 8 workers for each
   builder, one options month set and one share — e.g. on a copy:
   `for w in 1 4 8; do cli upkeep run srt gdfl NSE/FNO/NIFTY --from 2019-03 --to 2019-03 --workers $w --all; done`
   and the same with `tix`, `msum`, and a share (`NSE/CASH/RELIANCE`). Each receipt's `rate` line is
   the figure. Write them into L-R317-01.
4. **DB-N14 on `:8080`** after the real `srt` run: AF-05's `/bars/window.json` rows (p50/p99/max,
   n = 200) with `sort=c`, `sort=v` and `sort=c&extremes=1`, 2018-09..2026-09, NIFTY 1s; the
   answer's `scan_rows` should read 0.
5. **DB-027 on the real store**: `cli upkeep run rekey gdfl --underlying NIFTY` on a private copy
   first, then the store on the operator's yes; check `/db`'s strike list for the 2020-06-25 NIFTY
   expiry no longer shows 19,12,500 / 19,12,700 / 19,13,000 CE.

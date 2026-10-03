Verdict: 3 new findings (0 high, 0 medium, 3 low). Each is a path where read_dir order or HashMap order reaches output bytes or refusal text. None of them reaches a digest, the run identity, or journal byte order, and none is listed in hunt-conc.md or pass 1.

# conc-pass2 / determinism (commit 331b05c)

Slice: workspace-wide, non-test code only. I looked for every HashMap/HashSet iteration, `read_dir` ordering, thread schedule and wall-clock value that can reach output bytes, digests, run identity, refusal text, journal byte order, or which work runs first before a stop. I read source only and ran no cargo.

Method:
- A script listed every non-test HashMap/HashSet binding and struct field, then every `.iter()`/`.values()`/`.keys()`/`.into_iter()`/`.drain()` and every `for (a, b) in map` over them, on one line or split across lines. Each hit was checked by hand.
- All 93 `read_dir` sites were checked; about 18 are in production code.
- I checked every `SystemTime::now` and `available_parallelism` site, and every rayon / `thread::scope` / `tokio::spawn` site outside tests.

## determinism-1 (low): the `/pull` folder picker's "capped" note can never print, and the 60 folders it offers are chosen by `read_dir` order

- **Where:** crates/api/src/render.rs:3142-3175 (`collect_csv_dirs`), 3097-3111 (`discover_folders`), 3036-3037 and 3053-3063 (`folder_input`).
- **Code:**
  ```rust
  fn collect_csv_dirs(dir: &Path, depth: usize, out: &mut Vec<String>) {
      if depth == 0 || out.len() >= MAX_FOLDER_SUGGESTIONS || !dir.is_dir() { return; }
      let Ok(entries) = std::fs::read_dir(dir) else { return; };
      ... for e in entries.flatten() { ... children.push(p) ... }
      if has_csv { out.push(dir.to_string_lossy().into_owned()); }
      for c in children { collect_csv_dirs(&c, depth - 1, out); }
  }
  ...
  found.sort(); found.dedup();
  ...
  let capped = suggestions.len() > MAX_FOLDER_SUGGESTIONS;
  ```
- **Why it is wrong:**
  - The guard runs before the push, so `out` stops at exactly 60. `dedup` cannot add entries, so `suggestions.len() > 60` is never true, and the ", capped at 60" clause in `folder_input` can never print.
  - Once the walk has more than 60 CSV-bearing folders, the 60 it keeps are the first 60 in depth-first `read_dir` order. That is filesystem order, which differs between machines, filesystems, and even the same directory before and after a copy. Sorting happens only after the walk has already truncated.
  - The page then says "60 folder(s) holding CSVs found" with no truncation note. The code's own doc at 3010-3012 and 3069-3071 calls that "a lie about completeness": *"A cap that silently truncates is a lie about completeness"*.
  - Same inputs give different page bytes (CLAUDE.md §3 rule 5). The truncation is hidden, which is the §4 "fallback that hides a failure".
- **Repro:** Put 61 or more subfolders, each holding one `.csv`, under `~/Downloads`, then start `api serve` and GET `/pull`. The datalist has 60 options and the footnote says "60 folder(s) … found" with no "capped" text. Copy the same tree to another filesystem (for example, tmpfs vs ext4 hash order) and restart: a different set of 60 folders is offered.
- **Fix:** Let the walk collect one more than the cap, so the check can fire: guard on `out.len() > MAX_FOLDER_SUGGESTIONS`, or keep a separate `truncated: bool` set when a push is refused. Then either sort each directory's `children` before recursing, which makes the truncated subset deterministic, or state on the page that the subset is filesystem-ordered.

## determinism-2 (low): folder-census `rejected` findings go out in `read_dir` order, and the strict walk's refusal names whichever bad member the filesystem lists first

- **Where:**
  - Production: crates/pull/src/archive.rs:703-794 (`descend` pushes `rejected` in `fs::read_dir` order), 613-665 (`walk` sorts `out` only: `sort_members(out)`), and 560-587 (`read_dir_reporting` returns `rejected` unsorted).
  - Census: crates/pull/src/folder.rs:380-394 (`census_of` sorts `instruments` and passes `rejected` through unchanged).
  - Output: crates/api/src/folder.rs:332-345 (writes `"rejected":[…]` in that order).
  - Strict path: archive.rs:787-789 (`Malformed::Refuse => return Err(ArchiveError::MemberMalformed { path, why })`), plus `read_bounded` (`MemberTooLarge { path }`) and `MemberNotText { path }` at 775-777. All return on the first offending entry met in directory order.
- **Code:**
  ```rust
  // archive.rs walk
  descend(dir, columns, out, passed, on_malformed, rejected, 0)?;
  sort_members(out);            // `rejected` is never ordered
  // folder.rs census_of
  instruments.sort_unstable(); ... Ok(Census { ..., instruments, rejected })
  ```
- **Why it is wrong:**
  - The module states twice (archive.rs:819-822 and 867-873; folder.rs:337-341) that `read_dir` order "is not stable between machines or between runs" and that §3 rule 5 requires sorting. The rule is applied to `out` and `instruments` but not to the sibling list that is also shipped on the wire.
  - A folder census with two or more undecodable members returns the same findings in a different order on another machine, or after the folder is copied.
  - On the ingest path (`read_dir`, `Malformed::Refuse`), a folder with two malformed or oversized members is refused with text naming member A on one machine and member B on another. That is "same bad input, different refusal text", the class hunt-conc-3 recorded for HashMap order, here reached through directory order.
- **Repro:** Create `vendor-data/gdfl/X/` with `a.csv` and `z.csv`, both with nine fields (one fewer than the declared layout), plus one good member. GET the folder census route for that path. The order of `"rejected"` is whatever `getdents` yields, so on ext4 with `dir_index` it follows the filename hash, not `a` before `z`. Copy the folder with `cp -r` to tmpfs and ask again: the order can flip. POST the same folder as an ingest and the refusal names `a.csv` or `z.csv` to match.
- **Fix:**
  - In `walk`, after `sort_members(out)`, add `rejected.sort_by(|a, b| a.path.cmp(&b.path))`.
  - For the strict refusal, collect directory entries into a `Vec<PathBuf>`, sort them, and then iterate, as `cli/build_provenance.rs:988-1001` already does. The first refusal is then the lexically first bad member everywhere. This also makes the `MAX_MEMBERS` cutoff point deterministic.

## determinism-3 (low): `absorb_rows` folds a `HashMap` into the index, so the duplicate-block refusal names a random population, and a retry on the same handle names one that is not duplicated at all

- **Where:** crates/cli/src/population.rs:3538-3552 (`absorb_rows`), with `index_rows_range` (4491-4560) returning `HashMap<[u8; 32], BlockFacts>`. Reached from `append_complete_locked` / `_v3_locked` / `_v4_locked` (3116, 3186, …), which run on a reusable `&mut self` ledger handle.
- **Code:**
  ```rust
  let new_blocks = index_rows_range(&mut self.row_file, &self.row_path, self.row_scanned, len)?;
  reserve_map(&mut self.raw_blocks, new_blocks.len(), "population-block index")?;
  for (identity, facts) in new_blocks {
      if self.raw_blocks.insert(identity, facts).is_some() {
          return Err(format!("{} gained a duplicate/non-contiguous block for population {}", ..., hex(&identity)));
      }
  }
  self.row_scanned = len;
  ```
- **Why it is wrong:**
  1. This is the hunt-conc-3 class at a site that report does not list (it names 4644/4670/4706 only). When the newly appended range repeats more than one already-indexed population, the identity printed is whichever the per-process `RandomState` yields first. Same bytes on disk give different refusal text from one process to the next.
  2. The loop inserts into `self.raw_blocks` before it refuses and never rolls back. `row_scanned` is not advanced, so the next call on the same handle re-indexes the same range. The entries the failed call already inserted now collide with themselves, and the refusal names a population whose block is *not* duplicated in the file. Which population it names depends on HashMap order again. The handle's in-memory index also now holds a seed-dependent subset of blocks from beyond `row_scanned`, which `contains_key(&receipt.population_id)` (3153) and `raw_blocks.get` (3165) read.
- **Repro:**
  1. Ledger L holds blocks P and Q, already absorbed by handle H.
  2. A foreign writer appends a second, non-contiguous block for P, then one for Q, then a fresh block R.
  3. `H.append_complete(...)` → `absorb_rows` → `new_blocks = {P, Q, R}`. Iteration order varies by process. The refusal names P in some processes and Q in others. In a process where the order is R, P, …, R is inserted before the refusal.
  4. Call `H.append_complete(...)` again. The first entry iterated may now be R, so the error reads "gained a duplicate/non-contiguous block for population R", and R is not duplicated in the file.
- **Fix:** Validate first and commit after: check every `identity` with `!self.raw_blocks.contains_key(&identity)` in a sorted pass (collect `new_blocks` into a `Vec` and sort by identity, or have `index_rows_range` return a `Vec` in file order, which it already walks). Insert only once all checks pass. That fixes both the random name and the poisoned retry.

## Pass-1 verification (findings that touch this slice)

- **recovery-3 (HashMap order in `reconcile_pending`): CONFIRMED.**
  - api/src/recovery.rs:1046-1056 still builds `pending` from `attempts.latest.values()`, where `latest` is a `HashMap<[u8;32], Record>` (recovery_journal.rs:630).
  - It then calls `assess`/`append_attempt` and checks `stopping(site)` per item in that order (1057-1060). The deterministic `attempts.order` (recovery_journal.rs:631) is unused here.
- **hunt-conc-3 (refusal names the first bad identity in HashMap order): CONFIRMED.**
  - admission_store.rs:1385-1406 `reconcile_all` still iterates `&HashMap` with early `return Err(... hex(population_id))`.
  - population.rs:4644 (and 4670/4706) still iterate `receipts: &HashMap` with `?` refusals.
- **hunt-conc-1 (durable writes inside rayon workers): CONFIRMED.**
  - batch.rs:357-360 is still `wanted.par_iter().map(|held| one(root, held, min_hits, commit))`.
  - `one` still calls `sweep_evidence::begin` (631) and `record_swept_run` (724) on the worker.
- **cli1-4 (thread-local invocation lost under `sweep_rungs`' `par_iter`): CONFIRMED.**
  - operation_audit.rs:438-441 `enter` sets the thread-local `CURRENT` on the calling thread only.
  - `completed_boundary` (493-506) does nothing when `CURRENT` is `None`.
  - lib.rs:15220 runs `one_rung` under `par_iter` with no re-entry. Called from a non-pool thread, rayon runs every closure on pool workers.
- **The pass-1 "clean" claims I re-checked hold:**
  - HashMap walks that reach output are sorted: server.rs:1196-1222 (total key on an NSE-only merge), 2875-2878, 6608-6613, 7363-7393, 7539-7540, 7783 (`targets.sort_unstable()` on the full `InstrumentKey`), 32126-32141, 33372; merge.rs:278-291, 562-564; autopilot.rs:2300-2320; indexmap.rs:175-176; census.rs:820-896; trades.rs:2411-2412.
  - Order-independent folds: merge.rs:237, 328; server.rs:5749; universe.rs:456/521; manifest.rs:2516; nseindex.rs:217-224 (ambiguity count), 296; recovery.rs:269 (constant refusal text), 464, 695 (unique attempt sequence), 787 (max).
  - Recovery seal ordering: `scope_order` sorts by the full body (recovery.rs:199-217), which is unique per key.

## Checked and not a finding

- **Other production `read_dir` sites:**
  - store/catalog.rs:291: `held` is sorted at 317, and the census holds only counts.
  - cli/live.rs:609: sorted at 678. live.rs:888 goes into a `BTreeMap`.
  - cli/search_checkpoint.rs:429: refusal texts are constants; `latest`/`next` are max folds.
  - cli/build_provenance.rs:408 and 988: both sort before use.
  - api/server.rs:8714: only tests emptiness.
  - api/assets.rs:387 and 626: counts, and the newest-mtime file; only a tie on mtime or more than 100,000 web/src entries makes the order matter, which is a dev-only banner.
  - telemetry/lib.rs:416: scratch cleanup.
- **Wall clock:**
  - cli/research.rs:59 resolves "today" once and prints it as the stated cutoff of an inventory report; nothing is hashed.
  - sweep_evidence.rs:1104 and lib.rs:17438 are ledger timestamps, not identity terms (as hunt-conc already noted).
  - The screen budget calibration at lib.rs:10106-10140 is refused on recording runs (D-0685).
- **Thread scheduling:**
  - The capture writes in the lib.rs:12064 screen `par_iter` go to per-(tier, rank, direction) files. The catalog is sealed in slot order (candidate_trades.rs:610-640), and `capture.check()?` after the collect (12205-12207) refuses the screen rather than publishing a schedule-dependent subset.
  - The only schedule dependence left is which concurrent failure's text is latched first by `refuse`'s `get_or_insert` (candidate_trades.rs:425-429). That needs two different simultaneous failures, so I did not raise it.
- **`serde_json`:** no `preserve_order`; its Map is a BTreeMap, as hunt-conc noted.

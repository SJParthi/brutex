STATUS: IN PROGRESS

NEXT: (HOLD lifted at 12:52 IST, `0521e9e1`) gate 1e (building), then the final report. Done this leg: the 149 (0 missed), the coverage pass (100% of the added lines and arms), the new code's 46 mutants (2 missed, both fixed), two base merges, workspace clippy, the CI shell gates (the base's exit codes, nothing new), bench C-RP-02 (ratio 0.897).

## Session of 27 Oct 2026, fifth leg (a new cloud session, continuing from `bcac42ad`)

The fourth leg was cut by the usage limit before any of the 149 left a saved result, so all 149
run again; nothing it lists as done is redone.

- **Seventh base merge.** `origin/feat/db-fix` moved to `a32722c4` (Merge R4-19, Merge R3-12, Merge
  BASE-DENY-1024, Merge R3-15); merged as `9a186f42`. `.github/gates_tree.rs` conflicted on
  `DECLARED`: R4-19's group first, IA-17b's after, the length 78 -> 79; gate 0's tool tests pass on
  it (`rustc --edition=2024 --test -D warnings .github/gates_tree.rs`: 49 passed). `docs/04`, `05`
  and `06` conflicted only where both sides appended at the end: feat/db-fix's file is the exact
  byte prefix (`cmp -n`), IA-17b's block after it, each block checked byte for byte against its own
  side's tail. Nothing the base moved under `crates/pull` (`gaps.rs`, `repair_plan.rs`,
  `cash_auction.rs` and their tests) is this part's: `git diff 5901ec77 0c18505b -- crates/pull/src`
  and `git diff origin/feat/db-fix 9a186f42 -- crates/pull/src` are byte-identical (`cmp`), so the
  head's mutant list is `0c18505b`'s, line for line. `Cargo.lock` is the base's byte for byte
  (tokio 1.53.3, BASE-DENY-1024).
- **The 149, rebuilt from the branch and matched, never restarted.** In a worktree of `8954122f`:
  `cargo mutants --list --in-diff <git diff bf9b1c8f cf6034d7 -- crates/pull/src> -p pull` prints
  264, and `--shard k/20 --sharding round-robin --list` for k = 0..19 partitions them (no name
  twice; shard 0 is the first session's 14 exactly). At the head, `--list --in-diff <git diff
  origin/feat/db-fix HEAD -- crates/pull/src>` prints 255. Matched by file and description, line and
  column dropped, the k-th of a key to the k-th: **106** done (shards 0..6, 9, 10, 11), **107** of
  shards 7, 8, 12..19, **39** new, **3** rerun (`>` -> `==`, `<`, `>=` at `gdfl_cm_auction.rs:1041`
  in `write_revision`, whose description's count went 2 -> 1): the fourth leg's numbers exactly.
  The 149 split alternately (75 and 74) into two `examine_re` lists, each anchored and escaped;
  `--list` with each prints exactly the intended names (`sort | cmp`).
- **How they run.** As the fourth leg: `cargo mutants` 26.2.0 `--in-place --in-diff <the head diff>
  -p pull --cargo-arg=--lib --config <the list> -- -- gdfl_import:: gdfl_cm:: ingest::`, the same
  three root tests skipped by name, `CARGO_PROFILE_DEV_OPT_LEVEL=1 CARGO_PROFILE_DEV_INCREMENTAL=true`
  in the environment only, `TMPDIR` on a world-writable tmpfs, each in its own worktree of
  `9a186f42` under `/home/user`, started detached from the session so no session timeout can stop
  one with a mutation applied. Baselines: `ok Unmutated baseline in 0s build + 26s test`, test
  timeout auto-set to 133 s and 135 s.
- **The 149, every one with a result: 0 missed.** `75 mutants tested in 2h: 56 caught, 17
  unviable, 2 timeouts` and `74 mutants tested in 2h: 55 caught, 19 unviable`: **111 caught, 36
  unviable, 2 timeouts, 0 missed**; every name of the 149 has exactly one outcome (`comm` of the
  list and the four outcome files: empty), and both worktrees were clean after. The two timeouts
  are infinite loops the mutation makes, which no test survives (each hit the 133 s limit):
  `gdfl_import_day.rs` `-=` -> `/=` in `name_in_order` (`at /= 1` never moves back) and
  `gdfl_import_filing.rs` `+=` -> `*=` in `give_added` (`cursor *= 1` never moves on). With the
  106 of the finished shards, **all 255 mutants of the head's diff have a saved result and none
  survived.**
- **The coverage pass (the arms no test reached).** Coverage at `9a186f42` (`cargo llvm-cov`
  0.8.4 `-p pull --lib --branch`, matched to `git diff --diff-algorithm=histogram -U0
  origin/feat/db-fix`): of this part's added executable lines, **34 never ran and 18 branch arms
  were never taken**. Each is now run, or its condition is gone because nothing could make it
  true (`f39f40ce`, `8adad281`, gate 1d `2abd7a43`); the details are under "The coverage pass,
  27 Oct" below. Coverage at `f39f40ce`: every added line run and every arm taken in the five
  production files; the three items left were test code, rewritten at `8adad281`.
- **Eighth base merge.** `origin/feat/db-fix` moved to `f01d7a0e` (Merge R4-26); merged as
  `3f2388c9`. `.github/gates_tree.rs` conflicted on `DECLARED`: R4-26's group first, IA-17b's
  after, the length 79 -> 80; gate 0's tool tests pass (50 passed). The three ledgers merged
  cleanly, feat/db-fix's file the exact byte prefix of each. Nothing R4-26 moved under
  `crates/pull` (`corporate_actions.rs`, `day_universe.rs`, `reference_data.rs`, `lib.rs`) is this
  part's; `Cargo.lock` is the base's byte for byte.
- **Mutants, the new code (the coverage pass's own lines).** `git diff 9a186f42 HEAD` of the files
  the pass touched (`new.diff`; unchanged by the eighth merge, `cmp`) lists **46** mutants. Run at
  `3f2388c9` as the 149 were, two worktrees of 23 (the filter adding
  `session::month_boundary::a_window`). A first try was cut twice (the kernel's out-of-memory
  killer took a build while a coverage build ran beside them; then the container restarted, which
  left one worktree with a mutation applied, restored from git before anything else ran, and one
  incremental cache corrupt, cleared): 2 results were saved (unviable) and the other 44 ran again.
  **Result: 38 caught, 6 unviable, 2 missed**, both in `widen_key` and both fixed at `66a53c71`:
  - `|=` -> `&=` on the new `widened.deferred |= waits(..)`: nothing read the count of a widened
    day whose member key's close waits for the month. The widened-member test
    (`scope_reopen::a_new_member_holding_only_a_later_day_takes_its_earlier_done_days`, outside
    ownership, the file the earlier legs already touched) now asserts `days_imported == 2`
    (IA-RP-45), shown failing on the mutant by hand (`left: 0, right: 2`).
  - `+=` -> `*=` on `widened.closed += 1`: WP-12f's line, which the pass's rewrite had only
    re-indented into this part's diff. `widen_key` keeps the base's `if let` shape again, so the
    line is the base's byte for byte and the pass's change is the `else` arm alone.
  - The fix's own mutants (`git diff 3f2388c9 66a53c71`, 5, at `66a53c71`): `|=` -> `&=` caught,
    `widen_key` -> `Ok(true)`/`Ok(false)` caught, `closed +=` -> `-=` caught, and **`closed +=` ->
    `*=` missed**: that line is unchanged against `feat/db-fix`, so it is not this part's mutant;
    it is a base finding, named for the Mac under NEEDS THE MAC (the `members` count of a widened
    day's summary event is read by no test; the summaries capture keeps only text fields).
  With the 149 and the 106: **every mutant of this part's diff against `feat/db-fix` has a saved
  result, and none survived.**
- **HELD at 11:5x IST.** `HOLD` returned on `cloud/db-base-20261024` at `b34f6a16` (10:29 IST,
  `GO-IA-17b` removed for it, lifting about 12:50 IST). The step in flight (these mutants and their
  fixes) was finished, committed and pushed; nothing new starts until GO.

## Session of 26 Oct 2026, fourth leg (a new cloud session, continuing from `b847a810`)

The third leg's worktrees did not survive the container; its fixes did. Its log after `e66351a3`
names MISSED mutants of shards 12, 14 and 15, each fixed (`78630e22` IA-RP-34, `2317072e`
IA-AUC-09, `307aad47` IA-RP-35, `b847a810` IA-RP-36), and two coverage tests (`4097b033` IA-RP-32,
`fca1d9f1` IA-RP-33). No shard left a record of finishing, so every shard of 7, 8, 12..19 is
counted as not done.

- **Sixth base merge.** `origin/feat/db-fix` moved to `5901ec77` (Merge IA-10c, Merge R4-22, Merge
  BASE-FIXTURE-1025); merged as `0c18505b`. `.github/gates_tree.rs` conflicted on `DECLARED`:
  IA-10c's group first, IA-17b's after, the length 77 -> 78. `docs/04`, `05` and `06` conflicted
  only where both sides appended at the end: feat/db-fix's file is the exact byte prefix
  (`cmp -n`), IA-17b's blocks after it (each side's block checked byte for byte against its own
  tail). `ingest.rs`, `gdfl_import.rs`, `gdfl_import_tests.rs` and `gdfl_scope_tests.rs` merged
  cleanly. `cargo test -p pull --lib --locked -- gdfl_import:: gdfl_cm:: ingest::`: 601 passed, 3
  failed (the three root-permission tests named below). BASE-FIXTURE-1025 made
  `gdfl_cm::tests::fixtures_are_built_not_pasted` green (1 passed), so it is no longer skipped.
  `Cargo.lock` is the base's byte for byte.
- **The mutant list, rebuilt and matched, never restarted.** The 264 were listed again in a
  worktree of `8954122f` (`cargo mutants --list --json --in-diff <git diff bf9b1c8f cf6034d7 --
  crates/pull/src> -p pull`: 264), and each round-robin shard listed with `--shard k/20 --sharding
  round-robin`; shard 0 names exactly the 14 mutants the first session's shard 0 reported. The
  head's list (`git diff origin/feat/db-fix HEAD -- crates/pull/src` at `0c18505b`) is **255**. Each
  head mutant was matched to the old list by file and description with its line and column
  dropped, the k-th of a description to the k-th: **106** match a mutant of a finished shard (0..6,
  9, 10, 11) and are not run again; **107** match a mutant of shards 7, 8, 12..19; **39** are new
  (the fixes' own lines: `span_of_day`, `name_in_order`, `Gather::take`'s new return, and others);
  **3** are rerun because their description's count changed (`>` in `write_revision`, 2 -> 1). So
  **149** run, the 106 do not.
- **How they run.** `cargo mutants` 26.2.0 `--in-place --in-diff <the head diff> -p pull
  --cargo-arg=--lib --config <an examine_re list naming the 149 exactly, anchored>` `-- --
  gdfl_import:: gdfl_cm:: ingest::`, skipping by name the three tests root defeats
  (`gdfl_import::tests::ia16::an_immutable_month_stops_the_run_by_name`,
  `ingest::census_store::link_tests::a_read_only_partition_directory_is_refused_with_the_host_s_words`,
  `ingest::census_store::link_tests::an_idempotent_requirement_opens_nothing_for_writing`), split
  alternately across two worktrees of `0c18505b` under `/home/user` (75 and 74; each list checked
  with `--list` against the intended names, exact). `--cargo-arg=--lib` puts `--lib` on the build
  as well as the test: without it the build step was `cargo test --no-run --package=pull` and
  linked every pull integration-test binary on every mutant, which is why earlier legs measured
  3.3 min a mutant. Built with `CARGO_PROFILE_DEV_OPT_LEVEL=1 CARGO_PROFILE_DEV_INCREMENTAL=true`
  in the environment for these runs only (nothing committed; `Cargo.toml` untouched): the same
  code and the same tests, compiled faster. Each baseline: 601 passed, 0 failed, 19.6 s; test
  timeout auto-set to 100 s.

## Session of 25-26 Oct 2026, third leg (a new cloud session, continuing from `7d401a3a`)

The previous leg's mutant worktree did not survive the container; its fixes did. Its commit log
names the shards it finished after the section below was written: 5 (`6336404d` IA-AUC-06,
`9a09997d` IA-RP-31, `25437274` an equivalent mutant restructured away), 6 (`91f10357`), 9
(`4ef253d5` IA-AUC-07) and 11 (`7d401a3a` IA-AUC-08). Shards 7 and 8 left no record, so they run
again.

- **Fifth base merge.** `origin/feat/db-fix` moved to `13dcf5e5` (Merge R4-28, Merge CLOUD-MUTAPIB);
  merged as `33a5f5ba`. `.github/gates_tree.rs` conflicted on `DECLARED`: R4-28's group first,
  IA-17b's after, the length 76 -> 77, and gate 0's tool tests pass on it (49 passed). `docs/05`
  conflicted only at the end: feat/db-fix's file is the exact byte prefix (`cmp -n`), IA-17b's
  blocks after it; `docs/04` and `docs/06` merged cleanly with the same prefix property. Nothing the
  base moved under `crates/pull` (`fno*`, `master_snapshots*`, `masters.rs`, `vendor.rs`, `lib.rs`)
  is this part's; `Cargo.lock` unchanged.
- **Gates after it:** `cargo fmt --all --check` green; `cargo clippy --workspace --all-targets
  --locked -- -D warnings` green; `cargo deny check` (0.19.0) `advisories ok, bans ok, licenses ok,
  sources ok`.
- **The mutant run resumed.** The same 264-mutant list (`cargo mutants --list --in-diff <git diff
  bf9b1c8f cf6034d7 -- crates/pull/src>` prints 264 in a worktree of `8954122f`, whose code is
  `cf6034d7`'s), shards 7, 8, 12..19 of 20, round-robin, the same skips, `TMPDIR` on a world-writable
  tmpfs. Beside it, every mutant of the fixes' own lines (`git diff cf6034d7 7d401a3a --
  crates/pull/src`: 68 mutants) in a worktree of `33a5f5ba`.

## Session of 25 Oct 2026, second leg (a new cloud session, continuing from `075dc877`)

- **Fourth base merge.** `origin/feat/db-fix` moved to `bf9b1c8f` (Merge R4-18: quarantine);
  merged as `cf6034d7`. `docs/04` and `docs/06` conflicted only where both sides appended at the
  end: feat/db-fix's file is the exact byte prefix (checked with `cmp -n`), IA-17b's blocks after
  it; `docs/05` merged cleanly. Under `crates/pull` R4-18 touched only `ingest_failkind.rs` and its
  tests, neither of this part's; `Cargo.lock` unchanged. The mutant diff
  (`git diff origin/feat/db-fix HEAD -- crates/pull/src`) still lists the same 264 mutants, and
  shard 0/20 names the same first mutants as before.
- **Gates after it:** `cargo fmt --all --check` green; `cargo clippy --workspace --all-targets
  --locked -- -D warnings` green (exit 0); `cargo deny check` (0.19.0) `advisories ok, bans ok,
  licenses ok, sources ok`.
- **The mutant run, shard by shard** (`cargo mutants` 26.2.0 `--in-place --in-diff <git diff
  origin/feat/db-fix -- crates/pull/src at cf6034d7> --shard k/20 --sharding round-robin -p pull
  -- --lib -- gdfl_import:: gdfl_cm:: ingest::`, skipping by name the 4 tests root defeats or the
  base owns: `gdfl_cm::tests::fixtures_are_built_not_pasted`,
  `gdfl_import::tests::ia16::an_immutable_month_stops_the_run_by_name`,
  `ingest::census_store::link_tests::a_read_only_partition_directory_is_refused_with_the_host_s_words`,
  `ingest::census_store::link_tests::an_idempotent_requirement_opens_nothing_for_writing`), in a
  worktree of `8954122f` under `/home/user` (a worktree under the scratch `/tmp` folder failed 3
  more tests: their uid-65534 child cannot reach the binary there). **Every MISSED mutant is a
  missing test, and each was fixed on the branch the same hour** (each test shown failing on the
  mutant by applying it by hand, and passing on the code):
  - shard 1: `gdfl_import_filing.rs` `&&`→`||` in `Gather::is_empty`. A real defect behind it:
    `Gather::take` made a new month's offer before judging `MAX_ROWS`, so a refused first file left
    an empty offer the flush would repair. Fixed (the judgement first; `is_empty` the one test of
    the waiting days), IA-RP-27, `b7f1e8b3`.
  - shard 2: `gdfl_import_day.rs` `-`→`+` in `record_close` (a day closed twice the same way was
    never tested): the insert is `name_in_order`, IA-RP-28, `fe52ea99`. `note_gathered_repair` →
    `()` (the audited event was never read): a test capture and IA-RP-29, `623b06e5`.
  - shard 3: `ingest.rs` the busy guard → `false` in `judge_back_fill` (base code moved, never run):
    IA-RP-30, `875fd56f`. `give_added` `<`→`<=` (no stamp on a file's first second): a test of
    both edges, IA-RP-26, `9b4eb459`. Two more MISSED there (`-`→`/` in `record_close`, `==`→`!=`
    in `note_gathered_repair`) are the same lines, caught by IA-RP-28 and IA-RP-29.
  - shard 4: `close_deferred`'s `stopped` guard → `false` (the stop test never read
    `days_imported`): asserted, `9f35d7b9`.
  - Gate 1d: the two new test folders declared (`ia17b-busy`, `ia17b-told`), `83a27e22`; its
    undeclared set equals the base's again.
  The worktree keeps the code the list was made from, so every shard stays a partition of the same
  264; the new code's own mutants are run after (NEXT).

## Session of 25 Oct 2026 (a new cloud session, continuing from `381ea29c`)

`GO-IA-17b` is back on `cloud/db-base-20261024`, so this part builds under its own GO.

- **Third base merge.** `origin/feat/db-fix` moved to `666ddf7f` (Merge DATA-1w, after Merge R3-13);
  merged as `9947dc2e`. The three ledgers (`docs/04`, `05`, `06`) conflicted only where both sides
  appended at the end; both kept whole, feat/db-fix's text the exact byte prefix (checked with
  `cmp`), IA-17b's blocks after it. Nothing under `crates/pull` moved on the base
  (`git diff d39e789e -- crates/pull/src` equals `git diff origin/feat/db-fix -- crates/pull/src`
  byte for byte), so the coverage and mutant tables below still describe the diff. R3-13's new
  `api::dayflags_rules` calls `gdfl_cm::auction::path` and `append`, whose signatures this part kept.
- **The Mac's spec-check note, checked.** "auction::append writes through path(); it must resolve
  the served generation without writing inside a served revision." `append` (`append_typed`,
  `crates/pull/src/gdfl_cm_auction.rs`) does not write through `path()`: it asks
  `store::generation::served` first, refuses by name (`NotFiled::Revision`) any month not served as
  the original, refuses a pointer it cannot read (never guessing the original), and only then writes,
  through `original_path`, which never names a revision root. `path()` (the reader's resolution) and
  `write_revision` (which writes only under the NEW, not-yet-served revision root, before promotion)
  are the only other places that name a generation. Proved by
  `revision_tests::a_revision_served_month_is_never_appended_to_and_its_readers_read_the_served_sidecar`
  and `revision_tests::a_month_whose_served_generation_cannot_be_told_takes_no_print`. No code change.
- **Gates after the merge** (this host: root, 4 cores; `TMPDIR` on a tmpfs):
  - `cargo fmt --all --check`: green.
  - `cargo clippy --workspace --all-targets --locked -- -D warnings`: green (exit 0).
  - `cargo deny check` (cargo-deny 0.19.0, CI's pin, installed fresh): `advisories ok, bans ok,
    licenses ok, sources ok`. RUSTSEC-2026-0386 (tokio 1.53.1) is the base's item BASE-DENY-1024;
    `Cargo.lock` untouched.
  - `cargo test -p pull --locked --no-fail-fast`: **2,545 passed, 9 failed, 49 ignored**: the same
    9 root/base failures as before (listed under Gates); all 35 of this part's tests
    (`gather`, `revision_tests`, `gather_unit_tests`) pass.
  - `cargo test -p api --lib --locked -- dayflags` (R3-13's tests that call `auction::append` and
    `path`): 116 passed, 0 failed.
  - `cargo test -p cli --locked --no-fail-fast -- gdfl_import tick_second scrub_blocks
    watchdog_command gdfl_run`: 166 passed, 3 failed (the same three chmod tests root ignores).
  - CI shell gates touching this part's files, each step's `run:` body run as written, against a
    worktree of `666ddf7f` for comparison: gate 0 green; gate 14 green (pull pin 39 with `C-RP-02`);
    gate 12 red on both, no new refused block (767 claim blocks, 165 name a test, 41 an invariant
    row); gate 1d red on both, **the set of undeclared words identical** to the base's.

# IA-17b: D-4875 remainder: the once-per-month gather and the CASH .auc writer

**Where it stands (27 Oct 2026).** The part's code is complete and pushed, on the newest
`feat/db-fix` (`f01d7a0e`). Green here: `cargo fmt --all --check`; `cargo clippy --workspace
--all-targets --locked -- -D warnings`; `cargo deny check` (CI's 0.19.0); gate 0 and gate 6c;
**100% of the added lines run and every added branch arm taken** (1,273 lines, 140 arms, six
files); **no surviving mutant** of this part's diff (the 106, the 149 and the new code's 46, every
one with a saved result). What is not green here is the host's and the base's, each named under
"Gates, 27 Oct" and NEEDS THE MAC: the workspace tests root, 4 cores and the free-space floor
defeat (red on the base alike), and the CI shell gates red on `feat/db-fix` the same way, with no
new finding inside them. That is the README's `STATUS: BLOCKED` with the code complete.

**The first leg (24 Oct 2026), as it was written.** Cloud session, branch `cloud/db-IA-17b-20261024`, cut from `origin/feat/db-fix` at `e851da1c`
(Merge DATA-2). `origin/feat/db-fix` moved twice during the session, and both were merged in:

- `d39e789e` (Merge IA-4, R4-27, CLOUD-MUTSTORE) as `af3746f8`. The three ledgers conflicted only
  where both sides appended at the end; both sides are kept whole, the base's first.
- `7b397bbe` (Merge CLOUD-MUTCLI: `crates/cli` and two ledgers) as `3a22e4b5`, cleanly.

After the second merge these ran again: fmt, clippy, deny, every CI shell gate (against a worktree of
`7b397bbe`: the same 23 green and 9 red, the same findings), and the cli tests that drive the import
(`TMPDIR` on the tmpfs, including CLOUD-MUTCLI's `srt`, `tix` and `readonly` tests): 211 passed, the 3
chmod tests failed as before. The tables below give `d39e789e` as the base where they were measured
against it; `7b397bbe` changes nothing under `crates/pull`.

Per-part switch: `GO-IA-17b` exists on `cloud/db-base-20261024` at every save point checked, so the
base branch's `HOLD` does not apply to this part (README, per-part GO).

## Prerequisites (README "Before you build")

`git log origin/feat/db-fix --first-parent --merges --format=%s | grep -E '^Merge (IA-17|WP-12d|IA-16):'`
printed all three: `Merge IA-16: ...`, `Merge IA-17: ...`, `Merge WP-12d: ...`. Not blocked.

## What is done

1. **The once-per-(instrument, month) gather (D-4875 (3), D-4899).** The back-fill pass no longer
   repairs a file at a time. `ingest::gather_rows` asks the back-fill door's own judgement
   (`judge_back_fill`, split out of `back_fill_or_append` with its behaviour unchanged) and publishes
   nothing: a file its month takes only as a new generation is answered `Filed::Gathered` and its bars
   join the run's offer for that (instrument, month) (`gdfl_import_filing::Gather`). The offers are
   repaired once each, through `ingest::repair_gathered` and the one repair authority
   (`store::repair::repair_month_with`), when the run leaves the month (the top of the next month's day
   pass), when it ends, and when it is stopped (asked once per check point).
2. **The day close waits for the month (`gdfl_import_day.rs`).** A day with gathered files is not
   closed when it is read: its close is deferred (`defer_day`, event code `gather_deferred`) with its
   own failures and counts (`rebased` marks), and written in day order when its months are repaired
   (`close_deferred`), by the failures its months' repairs gave its files. `months_repaired` counts
   months.
3. **Stop, crash, full disk, census (D-4899 (5) to (7), (9)).** A stop offers what is gathered and
   closes those days before the run ends `stopped`. A crash leaves them begun (D-2821); the next run
   gathers them again and publishes, or meets `Published::Reused` and promotes. A full disk (a store
   gone, a store that takes no writes) at the repair ends the run with the gathered days left begun;
   months repaired earlier in the same flush stay served. A census that refuses the repaired rows
   ends the run `stage=census`, closes the first waiting day `incomplete stage=census` and leaves the
   others begun (with no day waiting the run is refused by name all the same); the next run converges.
4. **Bounded (D-4899 (8)).** An offer past `store::repair::MAX_ROWS` is refused `repair_refused` by
   name, never cut; a second gathered twice alike is kept once, twice differently refused
   `bars_differ_held`; a refused merge leaves the offer as it was.
5. **The CASH `.auc` of a repaired month (D-4875 (5), D-TBD-IA-17b-1).** `gdfl_cm::auction::path`
   resolves the served generation (one pointer probe; signature unchanged, so `api` and `cli` need no
   edit). `append` refuses a month served from a revision by name. `file_or_carry` carries a day's
   prints (`Carried`, in `FileFacts.auction`) when the sidecar already holds a later day or the month
   is served from a revision, never refuses them; the gathered repair passes `write_revision` as the
   `before_promote` step, which merges the served sidecar and the carried days in day order into the
   new generation's whole version-2 `.auc` (temporary through `store::staging::create`, synced,
   renamed, directory synced), refuses a carried day the served sidecar holds differently, and is
   repeatable (writes nothing for nothing).
6. **The successor head kept.** A sealed or revision-served month's successor, which reaches the
   back-fill door without a reason, is gathered too, and its repair journal keeps the head the lone
   back-fill wrote ("sealed-month successor ...", "revision successor ..."). A month gathered from one
   file keeps the old reason words exactly; several files say `{first}..{last} {n} files`.
7. **Audited (D-4893).** "gathered month repair finished" at `info` reaches the audit stream
   (`GATHERED_REPAIR_FINISHED`); a deferred close (`gather_deferred`) and a deferred day with no close
   (`gather_unclosed`, warn) are events.
8. **The arms no first test reached, tested (coverage pass).** Fourteen more tests: the flush ending
   `store_not_writable` and `store_lost`; a day that gathered and whose own pass ends the run; a repair
   journal that takes no line; two readings of one gathered second; a mid-day stop after a gathered
   file; a deferred close behind a later day's; the gather's helpers at the unit; the `.auc`'s
   orphaned print and unreadable pointer (IA-RP-19 to IA-RP-26, IA-AUC-04, IA-AUC-05). The CASH step
   lost a no-op arm: it now exists only for a share whose month is named. The inject seam gained
   `plant_gathered_lost` (test-only).
9. **The coverage pass, 27 Oct: every added line run and every arm taken.** Thirteen more tests
   (IA-RP-37 to IA-RP-44, IA-AUC-10 to IA-AUC-13, and IA-RP-33's front-door proof) and six
   conditions removed because nothing could make them true; each item is in "The coverage pass,
   27 Oct" below. `Filed::Gathered` carries the month the door addressed; `Gather::take` makes a
   new month's offer aside and joins it only with its file; one pair compare says which day waits
   (`Deferred::is`); `widen_window` uses the total `Window::holding`; `flush_gather` takes its
   first waiting day up front; the `.auc` writer's torn-tail cut and directory sync have per-thread
   test seams beside `FAIL_AFTER`; the gathered-repair seam can plant a failure no door names. The
   one front-door path to a gathered day with no close waiting (a source that says it holds no
   file of a day yet hands one) is named in the code, in IA-RP-33 and in D-4899 (9), and tested.

## What is left

Nothing of this part's code (27 Oct). The whole mutant run and 100% coverage of the added lines,
left on 24 Oct, are done (fifth leg, above). What remains is the runs this host cannot make: as a
non-root user, on 8 or more cores, with more than 10 GiB free, and the shadow-server and
real-store checks of spec §6 (M0 to M2, (a) to (j)), each under NEEDS THE MAC; and the base's own
red CI shell gates, which are red the same way on `feat/db-fix`.

Not built here (BACKTEST-FIRST, spec §4b): the plan's generation and byte count per month (FM-092's
first sentence) and the "rewrites used of the 1,024" view (RA-266) are `plan.json`'s (IA-10a) and
R3-28's. The rules audit's IA-17 gap (RA-266, "IA-17's once-per-month gather gets a test id and a
bound") is closed by IA-RP-03 and C-RP-02; its view half is those owners'.

## Tests that fail before and pass after

**Before.** `crates/pull/src/gdfl_gather_tests.rs` was copied into a worktree of the unmodified base
`d39e789e` and mounted there, with the five tests that name a type this part adds
(`filing::Gather`, `GatheredFile`, `GATHERED_REPAIR_FINISHED`, `inject::plant_gathered`) compiled out
by `#[cfg(any())]` and every other line unchanged; then
`cargo test -p pull --lib --locked gdfl_import::tests::gather` on the base:
`0 passed; 11 failed`, each on the behaviour it names, not on a helper:

| Test | Base answered | Rule needs |
|---|---|---|
| `a_repair_reason_run_repairs_each_month_once` (IA-RP-03) | 12 generations | 4 (one per instrument-month) |
| `a_hole_early_in_a_month_costs_at_most_one_generation` (FM-092) | "NIFTY: at most one generation" failed | ≤ 1 |
| `special_sessions_cost_one_generation_per_month` (FM-098) | (2, 4) | (2, 2) |
| `a_month_holding_a_day_between_the_days_it_lacks_takes_them_in_one_repair` | 4 | 1 |
| `a_crash_between_publication_and_promotion_resumes_as_the_same_revision` | 4 | 1 |
| `a_stop_or_a_crash_mid_gather_never_strands_a_begun_day` (IA-RP-04) | 4 | 2 |
| `a_run_across_the_years_end_repairs_each_month_once` | 6 | 4 |
| `a_one_bar_day_and_an_empty_day_cost_no_generation_of_their_own` | 2 | 1 |
| `a_stop_on_a_months_first_day_closes_the_month_it_left` | 36 | 2 |
| `a_census_that_refuses_the_repaired_rows_ends_the_run_and_the_next_converges` | no day left begun (closed before its month's repair) | the two later days begun |
| `a_cash_months_gathered_repair_writes_its_auction_file_before_it_is_served` | "auction sidecar refused: ... the month holds a day after 2026-09-22; 2026-09-22 is a new generation" | carried into the new generation's `.auc` |

The five compiled out, and the five `revision_tests`, call functions the base does not have, so on
the base they fail to compile. `gdfl_scope_tests`'s existing assertion of `months_repaired == 2`
(base) measured the per-file repair, and is 1 under D-4899.

**After.** On the branch head, `TMPDIR=<tmpfs> cargo test -p pull --locked --no-fail-fast`:
**2,545 passed, 9 failed** (the 9 root and base failures, shown red on the base's own binary under
Gates), 49 ignored. Of this part's tests: all 25 in `gdfl_import::tests::gather`, all 6 in
`gdfl_cm::auction::revision_tests`, and all 4 in `gdfl_import::filing::gather_unit_tests` pass. The cli
tests that drive the import (`cargo test -p cli --locked --no-fail-fast -- gdfl_import tick_second
scrub_blocks watchdog_command gdfl_run`, `TMPDIR` on the tmpfs): 152 passed, 3 failed (the chmod
tests root ignores).

## The coverage pass, 27 Oct

Measured at `9a186f42`: `TMPDIR=<tmpfs> CARGO_LLVM_COV_TARGET_DIR=<disk> RUSTC_BOOTSTRAP=1 cargo
llvm-cov -p pull --lib --branch --locked --no-report --ignore-run-fail`, then `cargo llvm-cov report
--branch --lcov` (0.8.4; `RUSTC_BOOTSTRAP=1` for `--branch` on the stable 1.97.1, for the
measurement only, nothing committed). The lines this part adds are `git diff
--diff-algorithm=histogram -U0 origin/feat/db-fix` of `ingest.rs`, `gdfl_import_filing.rs`,
`gdfl_import_day.rs`, `gdfl_cm_auction.rs` and `gdfl_import.rs` (histogram, because the default
diff paired `gathered_repair`'s new `if added_at.is_empty()` with the base's identical one in
`back_fill_or_append` and so blamed the base's line), matched to the lcov's `DA` and `BRDA`
records by an `awk` script. **34 lines never ran and 18 arms were never taken.** Each, and what
closed it:

| Where | Never run or taken | Closed by |
|---|---|---|
| `gdfl_import_filing.rs` `Filing::file` | a gathered file whose day has no month (8 lines) | removed: `Filed::Gathered` now carries the month the door addressed the bars to, so nothing works it out again |
| `gdfl_import_filing.rs` `Filing::file` | the provenance check's `landed == Appended` false arm | removed: after the gathered return every landing is an append (the judge door yields `Appended` or `Gathered` only) |
| `gdfl_import_filing.rs` `Filing::file` | a carried print with a door refusal (the `door_kind.is_none()` false arm) | IA-AUC-13: `a_carried_print_whose_bars_the_door_refused_waits_for_the_read_that_lands_them` |
| `gdfl_import_filing.rs` `DayFiler::fold` | `Ok(false) => {}` | `self.gathered += usize::from(gathered)`: one statement for both |
| `gdfl_import_filing.rs` `Gather::take` | `get_mut` after a push (2 lines, 1 arm); the day compare's false arm under an equal key | the new month's offer is made aside (`Option::insert`) and joins the gather with its file; `Deferred::is`, one pair compare (a key names its day) |
| `gdfl_import_filing.rs` `widen_window` | `Window::new` failing on `min..max` (never backwards) | `Window::holding` (`session.rs`), total; IA-RP-44 |
| `gdfl_import_filing.rs` `repair_gathered_months` | a repair failure no door names (5 lines); `landed == NewGeneration` false | IA-RP-41 (`plant_gathered_unnamed`, the address step's refusal planted) and IA-RP-42 (another writer lands the day first: `NothingNew`) |
| `gdfl_import_filing.rs` `name_on_each_file` | a file whose day is not waiting | IA-RP-43, at the unit (as `give_added`'s was) |
| `gdfl_import_filing.rs` `inject::gathered` | the probe's `&&` | one pair compare |
| `gdfl_import_day.rs` `run_days` | a stop first seen at a day's top | IA-RP-37 |
| `gdfl_import_day.rs` `widen` | a member's pass that closed nothing and waits for nothing | `widened.deferred \|= waits(..)` |
| `gdfl_import_day.rs` `defer_day` | a lost source, a store that takes no writes, the held-back cap (6 lines, 4 arms) | IA-RP-38 |
| `gdfl_import_day.rs` `flush_gather` | a deferred day with no close at a repair that ends the run; a census refusal with no waiting day (6 lines); its first day with no close (1 line) | `Option::map_or_else`; the flush takes its first waiting day up front (none: nothing was gathered); IA-RP-39 and IA-RP-40, the one front-door path (a source that disowns a day after handing a gathered file) |
| `gdfl_cm_auction.rs` `file_or_carry` | the sidecar's other refusals | IA-AUC-10 |
| `gdfl_cm_auction.rs` `write_day_held` | a torn tail the host will not cut (2 lines); a directory sync the host refuses (2 lines) | `seam::FAIL_CUT`, `seam::FAIL_SYNC` (per thread, beside `FAIL_AFTER`); IA-AUC-11, IA-AUC-12 |
| `ingest.rs` `gathered_repair` | `added_at.is_empty()`: a repair that adds nothing | IA-RP-42 |
| `ingest.rs` tests | a `matches!` guard's false arm | the pattern holds the kind; then read off the `Debug` rendering, with no arm of its own |

At `f39f40ce` (the same command, the same five files plus `session.rs`): **0 lines never run and
0 arms never taken in the five production files**; three items were test code (an `&&` inside a
new test's `assert!`, and the rewritten `matches!` whose no-match arm never runs), rewritten at
`8adad281`. The final measurement at the head is under "Gates, 27 Oct" below.

**Fails before, honestly.** These are coverage tests: most pin behaviour the part already had
and no test pinned, so on the code before this pass they would pass too, and only the four that
need a new seam or API (`plant_gathered_unnamed`, `FAIL_CUT`, `FAIL_SYNC`, `Window::holding`)
fail there, by not compiling. Their teeth are shown by mutants instead: every mutant on the lines
they reach is caught (the 149 at `9a186f42`, and the new code's own, below).

## Gates, 27 Oct (the head, on `feat/db-fix` `f01d7a0e`)

This host: root (uid 0), 4 cores, 15 GB of memory, 16 GB of tmpfs. Code at `66a53c71` (every
later commit is the report).

| Gate | Command | Branch | Base `f01d7a0e` |
|---|---|---|---|
| fmt | `cargo fmt --all --check` | green | — |
| clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | green (exit 0) | — |
| deny | `cargo deny check` (0.19.0, CI's pin) | green: `advisories ok, bans ok, licenses ok, sources ok` (after the seventh merge; `Cargo.lock` has been the base's since) | — |
| tests, pull | `TMPDIR=<tmpfs> cargo test -p pull --lib --locked` (inside the coverage run below) | 2,230 passed, 9 failed: `alerts` 1, `gdfl_dropbox::repair_r1` 2, `ia16::an_immutable_month_stops_the_run_by_name`, `census_store::link_tests` 2, `zerodha_verify` 2, and IA-10c's `gdfl_run::ia2_tests::every_id_the_runs_ledger_gives_is_written_down_twice` (a read-only file root writes anyway); every one a permission root ignores, none this part's | the same tests are the base's, unchanged by this part |
| tests, workspace | gate 1e's test phase, below | running at this save point | — |
| gate 1e | the step as `ci.yml` writes it (`gates-tree gate-1e prepare`: a worktree of the head with `web/` deleted, every toolchain, interpreter and shell shadowed by a stub; `cargo build --workspace --locked --all-targets`, then `cargo test --workspace --locked`, each read by `gate-1e verdict`), with three differences for this host: the outputs kept in files, the test phase `--no-fail-fast`, and the two tests that hang on 4 cores skipped by name under a two-hour bound | **build phase green**: `cargo build --workspace --locked --all-targets` finished (`Finished dev profile ... in 34m 41s`, exit 0) with web/ deleted, `gate-1e verdict build` exit 0, and no stub of the 21 ever invoked (no stub left a log); a first attempt with 4 build jobs was killed by the host's out-of-memory killer (`api`'s optimised test build beside three others on 15 GB), so the run that counts set `CARGO_BUILD_JOBS=2` in the environment, the command unchanged. Test phase: running at this save point | — |
| gate 0 | `rustc --edition=2024 --test -D warnings .github/gates_tree.rs` and the rest of the step | green: 50 passed | — |
| gate 6c | `rustfmt --edition 2024 --check` and `clippy-driver -D warnings` (with and without `--test`) over every tool `gates-jobs tools` lists | green | — |
| CI shell gates | every step of the language-purity job, each tree's own `ci.yml`, run as written (extracted by an `awk` script, `RUNNER_TEMP` and `GITHUB_ENV` as on a runner), but 1e (above) | **the base's exit code at every step**: green 0, 1, 1b, 1g, 1f, 2, 9, 9b, 10b, 7, 13, 15, 16, 17, 21, 22, 24, 28, 25, 27, 26, 14, 29b; red 1c, 1d, 23, 27b, 30, 19, 10, 11, 12, 29 | the same: red 1c, 1d, 23, 27b, 30, 19, 10, 11, 12, 29 |
| coverage | `cargo llvm-cov -p pull --lib --branch` (as "The coverage pass, 27 Oct") | **100%**: 1,273 added executable lines all run and 140 added arms all taken (`ingest.rs` 303 and 12, `gdfl_import_filing.rs` 440 and 48, `gdfl_import_day.rs` 241 and 50, `gdfl_cm_auction.rs` 250 and 30, `gdfl_import.rs` 16 and 0, `session.rs` 23 and 0) | — |
| mutants | `cargo mutants` 26.2.0 `--in-diff`, as above | **none survived**: the 264 of the first list matched to the head's 255 (106 done by the first four legs, 149 here: 111 caught, 36 unviable, 2 timeouts), and the coverage pass's own 46 (38 caught, 6 unviable, 2 missed and fixed, the fix's own caught) | — |

**Inside the red shell gates, nothing new.** Each red step's output on the branch and the base,
line numbers stripped and lines sorted, differs only in counts (files walked, headings read,
lines in scope, blocks with a cost claim, test timings): gate 1d's set of undeclared words is the
base's exactly (with this part's labels declared in `IA17B_LITERAL`), gate 12's refused claims are
the base's (179 claims name a test that exists, the base 178), gate 11's banned constructs are the
base's, and gate 29 fails the same `tests::every_route_has_a_limits_row_and_a_growth_bench` on
both (a base red since R3-15 or R3-12; this part adds no route). Gate 14 was red on the base at
`a32722c4` only because the base already pinned this part's bench row (C-RP-02) and was green
on the branch; at `f01d7a0e` it is green on both.

**Whole files, the branch** (the same coverage run; the README asks 100% of the touched modules,
and the earlier legs read that as the lines a part adds, the reading kept here): `ingest.rs`
84.18% of lines, 70.99% of branches; `gdfl_import_filing.rs` 95.34%, 89.89%; `gdfl_import_day.rs`
93.44%, 86.01%; `gdfl_cm_auction.rs` 92.61%, 96.36%; `gdfl_import.rs` 91.95%, 87.96%;
`session.rs` 94.57%, 86.84%. The rest of each file is base code this part does not change; on the
first leg's base `d39e789e` the four files it measured were 78.11% to 94.38% of lines.

## Gates, 24 Oct (the first leg's)

Run on the branch after merging `d39e789e`. Where a gate is red, the same command was run on a
worktree of the unmodified base `d39e789e` and the outputs compared with line numbers stripped and
lines sorted, so a new finding inside a red gate cannot hide.

| Gate | Command | Branch | Base `d39e789e` |
|---|---|---|---|
| fmt | `cargo fmt --all --check` | green | — |
| clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | green (exit 0; the one `warning:` line is `cli`'s build-script notice that the git index cannot be proved mid-change, not a lint) | — |
| tests | `cargo test --workspace --locked --no-fail-fast -- --skip server::gaps_route::cloud_mutant_tests::a_full_calendar_pool_refuses_the_census_and_the_audit` (49 min, at `178d9940`) | 10,974 passed, 57 failed, 90 ignored, 1 skipped by name, 1 stopped hung; every failure is this host's (below), none is this part's. After the coverage tests (`81319109`), `pull` (all of it) and the `cli` import tests ran again: 2,545 and 152 passed, the same root failures only | each class shown red on the base or independent of the diff (below) |
| deny | `cargo deny check` with CI's pinned cargo-deny 0.19.0 | green: `advisories ok, bans ok, licenses ok, sources ok` | green, same |
| deny (newer tool) | the same with cargo-deny 0.20.2 | red: RUSTSEC-2026-0386 (unsound, `tokio` 1.53.1 `AsyncFd::new`) | red, same advisory; `Cargo.lock` is identical on both, left alone (CONTEXT §4.10) |
| CI shell gates | every `Gate` step of the `language-purity` job, run as written (`ci.yml`, through a script that extracts each step's `run:` body) | 23 green; 9 red | the same 9 red, nothing new in any |
| gate 6c | `rustfmt --edition 2024 --check` over every tool `gates_jobs tools` lists; `clippy-driver --edition=2024 -D warnings .github/gates_tree.rs`, with and without `--test` | green | — |
| coverage | `cargo llvm-cov` 0.8.4 `-p pull --lib --branch` (below) | **not 100%**: of the 1,110 executable lines this part adds, 56 never run (95.0%); of the 160 branch arms on them, 24 never taken (85.0%) | the base runs none of the 13 moved base lines either |
| mutants | `cargo mutants` 26.2.0 `--in-place --in-diff <diff against d39e789e> --shard 0/20` and `--shard 10/20 --sharding round-robin -- --lib -- gdfl_import:: gdfl_cm:: ingest::` (skipping the three root-only tests), `TMPDIR` on a tmpfs | **a sample only: 27 of 264 mutants, 19 caught, 8 unviable, 0 missed, 0 timeouts** (45 min a shard; baseline 10 s of tests); the full run is under NEEDS THE MAC | — |

**The workspace test failures, every one, by cause.** This host runs as root (uid 0), has 4 cores,
and by the end of the session had 5.7 GB free on its one disk against the import's 10 GiB
`DEFAULT_FREE_FLOOR`. Nothing was deleted to make room (CONTEXT: delete nothing).

| Where | Failed | Cause | Shown how |
|---|---|---|---|
| `cli` lib | 25 of 28 (`gdfl_import_command::*` 14, `tick_second_check::*` 11) | the host's free space under the 10 GiB floor: each run is refused `disk_full` ("run ... refused disk_full") | rerun with `TMPDIR` on a 16 GB tmpfs (`/dev/shm`): **25 passed** |
| `cli` `tests/gdfl_run_logs.rs` | 2 | the same floor | rerun on the tmpfs: 4 of 4 passed |
| `pull` lib | 3 of 12 | the same floor | rerun on the tmpfs: passed |
| `pull` lib | 9 of 12 (`alerts` 1, `gdfl_dropbox::repair_r1` 2, `ia16::an_immutable_month_stops_the_run_by_name`, `census_store::link_tests` 2, `zerodha_verify` 2: permissions root ignores; `gdfl_cm::tests::fixtures_are_built_not_pasted`: flags `crates/cli/src/tick_second_check_tests.rs`, not this part's) | root, and one base finding | the same 9 run on the base's own binary (built from `d39e789e`): **0 passed, 9 failed** |
| `cli` lib | 3 of 28 (`scrub_blocks` 1, `watchdog_command` 2) | each test makes a file or directory unwritable with `set_permissions` (0o444, 0o200, 0o555); root writes anyway | still red on the tmpfs; the tests' own code; `crates/cli` is not touched by this part |
| `store` lib 3, `telemetry` lib 4 | 7 | permissions root ignores (`settings::tests`, `sink::tests`) | neither crate is touched and neither depends on `pull`: their code is the base's byte for byte |
| `api` lib | 8 | 4 cores (`slots_under` clamps a cap of 8 to 4: "at most 4"); root (a 000 journal read anyway: "interrupted", not "not_known"; "this host reads a 000 directory (root?)"); 5 `head_deadline_body_tests` slow-reader timings | the same 8 on the base's own `api` binary: **0 passed, 8 failed**, with the same messages |
| `api` lib `server::gaps_route::cloud_mutant_tests::a_full_calendar_pool_refuses_the_census_and_the_audit` | hangs (child idle at 0% CPU, 22 min in the first run) | a full calendar pool held by `MAX_CALENDAR_CONCURRENT` barrier holders against a pool the 4-core clamp makes smaller | on the base's binary and the branch's, each stopped by a 300 s timeout, still running; skipped by name in the run above, **not silently** |
| `api` `tests/cloud_mutants_bars.rs` `spot_rs::spot_json_is_refused_by_name_when_every_store_read_slot_is_held` | hangs (0.1% CPU, 10 min), stopped by hand | a `Barrier::new(MAX_STORE_READ_CONCURRENT + 1)` with `MAX_STORE_READ_CONCURRENT = 8`, while the pool admits `slots_under(8)` = 4 holders on this host: the barrier never fills | the file is unchanged since CLOUD-MUTAPIA and `crates/api` is not touched; the other 58 tests of the binary passed |

This part's own tests all passed inside the run: the 16 `gather` tests, the 5 `revision_tests`, and
77 of 78 `backfill`/`sealed`/`ia16`/`scope_reopen` tests (the 78th is the root test above).

**Coverage, measured, and what is left.** Command, on the branch head, with `TMPDIR` and the
coverage target on a fresh world-writable tmpfs directory (the uid-65534 child tests write there):
`TMPDIR=<tmpfs> CARGO_LLVM_COV_TARGET_DIR=<tmpfs> RUSTC_BOOTSTRAP=1 cargo llvm-cov -p pull --lib
--branch --locked --no-report --ignore-run-fail`, then `cargo llvm-cov report --branch --lcov`.
`--branch` is unstable on the stable 1.97.1 toolchain and no nightly is installed, so
`RUSTC_BOOTSTRAP=1` let the stable compiler take it, **for this measurement only**: nothing of it
is committed. The lines this part adds are read from `git diff -U0 d39e789e` and matched to the
lcov's `DA` and `BRDA` records (`ingest.rs`, `gdfl_import_filing.rs`, `gdfl_import_day.rs`,
`gdfl_cm_auction.rs`, `gdfl_import.rs`; the test files are test code).

The first measurement found 124 of the 1,095 added executable lines never run and 39 of 160 arms
never taken. Fourteen tests were added for them (IA-RP-19 to IA-RP-26, IA-AUC-04, IA-AUC-05), and
the CASH step's no-op arm was removed (the step now exists only for a share whose month is named).
The final run:

| File | Added lines | Never run | Branch arms | Never taken | Whole file on the branch, lines (branches) | The same on the base `d39e789e` |
|---|---|---|---|---|---|---|
| `gdfl_import_filing.rs` | 441 | 15 | 68 | 11 | 94.98% (86.87%) | 94.11% (86.76%) |
| `gdfl_import_day.rs` | 241 | 25 | 58 | 11 | 91.76% (82.31%) | 92.00% (82.11%) |
| `gdfl_cm_auction.rs` | 232 | 3 | 26 | 0 | 95.01% (96.23%) | 94.38% (95.12%) |
| `ingest.rs` | 180 | 13 | 8 | 2 | 78.88% (63.39%) | 78.11% (62.98%) |
| `gdfl_import.rs` | 16 | 0 | 0 | 0 | 91.91% (87.74%) | 91.85% (87.74%) |

The 56 lines never run, by why:

- **Base code moved or touched, never run on the base either (13, `ingest.rs`).** The judgement split
  out of `back_fill_or_append` (the journal refusal, the unread month, the busy overlap guard,
  `Filed::Appended` on no stamps), the guard after `bars.is_empty()` in `rows_mode`, and the `None`
  argument added to `repair_or_append`'s `repair_after` call (the Zerodha door, which pull's own
  tests do not reach). A base coverage run of `d39e789e` shows the same statements at 0 hits.
- **Unreachable by construction, named rather than silent (35).**
  - `gdfl_import_filing.rs`: a gathered file whose day has no month (8; the door already addressed
    its month); `get_mut` on an offer just inserted (1); `defer_close` with no waiting day (1);
    repair failures the door did not type, for an offer whose address the gather already took (5).
  - `gdfl_import_day.rs`: a census refusal at the flush with no waiting day (6; an offer always has
    one); a waiting day with no close, and its `gather_unclosed` warning (13; a pass that takes a
    file always writes its close or ends the run).
  - `gdfl_cm_auction.rs`: the sidecar's other host errors (1).
- **Reachable, no seam to time them (8).** A source lost between two files of a day that gathered
  (`gdfl_import_day.rs`, 6: `DayRead` cannot say "source gone", and nothing times a vanishing tick
  store between two files); `set_len` failing on a regular file while a torn `.auc` tail is cut back
  (`gdfl_cm_auction.rs`, 2).

The README asks for 100% line and branch on the touched modules; that is **not met**, on the added
lines or on the whole files. The whole files were 78–94% (lines) on the base and stay within a point
of it: four rise, and `gdfl_import_day.rs`'s lines fall from 92.00% to 91.76% (its branches rise),
the 25 unreachable or untimeable lines above. Listed under NEEDS THE MAC.

**The mutant sample, each outcome** (`mutants.out/caught.txt`, `unviable.txt` of each shard):

- shard 0/20, caught (10): `gdfl_cm_auction.rs:732` `==`→`!=` in `file_or_carry`; `:844`
  `write_day` → `Ok((0, 1))`; `:1018` delete `!` in `write_revision`; `gdfl_import_day.rs:1934`
  `Widened::tell` → `()`; `:2780` `==`→`!=` in `record_close` (caught by IA-RP-25, added today);
  `gdfl_import_filing.rs:1668` `Gather::is_empty` → `false`; `:1743` `>`→`==` in `Gather::take`;
  `:1808` the `<` guard → `false` in `merge_into`; `:2025` `<=`→`>` in `give_added` (caught by
  IA-RP-26, added today); `:2070` `gathered_reason` → `String::new()`.
- shard 10/20, caught (9): `gdfl_cm_auction.rs:753` `+`→`*` in `records_of_day`; `:895` `+`→`*` in
  `write_day_held`; `:1038` `-=`→`+=` in `write_revision`; `gdfl_import_day.rs:2403` `defer_day` →
  `Ok(())`; `:2777` `>`→`<` in `record_close`; `gdfl_import_filing.rs:1688` `Gather::defer_close` →
  `true`; `:1775` `-`→`+` in `Gather::take`; `:1917` `+=`→`*=` in `repair_gathered_months`; `:2037`
  `+=`→`*=` in `give_added`.
- unviable, does not compile (8): `gdfl_import.rs:2639` `cm_read`; `ingest.rs:2814`
  `successor_of_served`; `ingest.rs:3235` `judge_back_fill`; `gdfl_import_day.rs:2524` `&&`→`||` in a
  `let` chain; `ingest.rs:1877` `rows_mode`; `ingest.rs:2949` `repair_after`; `ingest.rs:3290`
  `gathered_repair`; `gdfl_import_filing.rs:840` `Filing::through` (each replaced by a `Default`
  value its types do not have).

**The 9 red CI shell gates are base reds** owned by BASE-REDS-1023 (CONTEXT §4.10), each red the same
way on `d39e789e`:

- **1c**: `zerodha_spool.rs` and `docs/02-store-format.md`'s lock path (IA-9).
- **1d**: the base's undeclared literals. With this part's labels declared (`IA17B_LITERAL`), the
  branch's set of undeclared words equals the base's exactly. The rest of the difference is each word's sample of
  occurrences, which now names this part's files for words already undeclared on the base
  (`repair_impossible`, `repair_refused`, and a session word in the bench's messages).
- **10**: `R427-11` names no test.
- **11**: `tick_second_check.rs` and `tick_second_index.rs`.
- **12**: cost claims in `api`/`cli` that name no proof. Every claim this part adds names one: 764
  blocks make a cost claim on the branch against 762 on the base; 163 name a test that exists (162 on
  the base) and 41 an invariant row whose test exists (40 on the base); the refused set is the same.
- **19**: the base's three `ingest.rs` sites (the same three, shifted by this part's lines).
- **23**: the print surface.
- **27b**: D-6247 heads two entries.
- **30**: 20 steps whose command a package has not built yet.

Green on both: 0, 1, 1b, 1g, 1f, 2, 9, 9b, 10b, 13, 15, 16, 17, 21, 22, 24, 25, 26, 27, 28, 29, 29b,
14 (the gate 14 `pull` pin 39 with `C-RP-02`). Not run here: 1e (moves `web/` aside and builds the
engine; the tests above build with no front-end toolchain) and 7 (skips permanently, CLAUDE.md §5).

## Measurements

**C-RP-02 at the head (27 Oct).** `cargo bench -p pull --locked --bench ratio`, the code of
`66a53c71`, this host (4 cores; `TMPDIR` on a tmpfs), nothing else building but the CI shell gates'
small tools: 2 gathered days, one run, n = 5: p50 67,152,675 ns, p99 68,436,954 ns, max
73,190,417 ns; 22 gathered days, n = 5: p50 682,705,847 ns, p99 695,901,577 ns, max 776,534,707
ns; per gathered day, the fastest run, 31,209,315,000 ps at 2 days and 28,012,423,363 ps at 22:
**ratio 0.897**, gate 8's ceiling 3.0; peak resident set of the five 22-day runs 274,698,240 B
(printed, never gated); every other ratio of the pull bench within its ceiling (`all ratios
within the ceiling`, exit 0). The coverage pass changed nothing on that path but how a new
month's offer is made (aside, then pushed: the same allocations), so the numbers are the first
leg's within this host's spread. With n = 5, the p99 is the slowest sample but one.

**C-RP-02** (`crates/pull/benches/ratio.rs`, `gathered_repair_is_flat_per_day`). Method: `cargo bench
-p pull --locked --bench ratio` (the bench profile, release with debuginfo), this cloud host (Linux,
4 cores; `TMPDIR` on a tmpfs), on the branch after merging `d39e789e`. Each run is one repair-reason
import over January 2025 into a fresh store that holds only the 31st, 22,500 one-second NIFTY bars a
day, the store built outside the clock; every run must end with the month at generation 1 or the
bench refuses.

| Measured | n | p50 | p99 | max |
|---|---|---|---|---|
| 2 gathered days, one run | 5 | 72,294,034 ns | 73,745,634 ns | 74,948,032 ns |
| 22 gathered days, one run (495,000 bars offered) | 5 | 671,221,335 ns | 674,782,842 ns | 680,043,187 ns |

- Per gathered day, the fastest run: 34,624,487,500 ps at 2 days, 30,357,345,272 ps at 22: **ratio
  0.876**, gate 8's ceiling 3.0. The cost per day does not grow with the month's days.
- Peak resident set of the five 22-day runs, restarted before them (`VmHWM` after `clear_refs` 5):
  **274,386,944 B**. Printed, never gated.
- With n = 5, the p99 is the slowest sample but one. The pre-merge run (`docs/06-limits.md`, the
  numbers written there) gave 0.774, p50 749,859,448 ns at 22 days and 261,095,424 B: the same shape,
  this host's run-to-run spread.
- Every other ratio of the pull bench stayed within the ceiling (`all ratios within the ceiling`,
  exit 0).

**Per operation** (`docs/06-limits.md`, the 24 Oct section): one gathered file is one hash probe for
its month's offer plus an append of its bars (a file out of order merges, O(offer + file)); the
month's leave test is one compare a day; the flush is one repair per month gathered plus one cursor
walk of the stamps it added; the `.auc` path adds one generation pointer probe per read. **Not
measured here**: the `/sources/day.json` p50/p99/max over n ≥ 200 GETs with and without the pointer
probe (NEEDS THE MAC (c)); the cloud has no store or server to time it against.

## Extreme cases

All in `pull::gdfl_import::tests::gather` (`crates/pull/src/gdfl_gather_tests.rs`) unless marked
`revision_tests` (`crates/pull/src/gdfl_cm_auction_revision_tests.rs`). Every fixture is invented at
run time; `gdfl_cm::tests::fixtures_are_built_not_pasted` walks both files.

| # | Extreme | Test | Kind |
|---|---|---|---|
| 1 | Empty input and one-record input: an empty day and a one-bar day cost no generation of their own | `a_one_bar_day_and_an_empty_day_cost_no_generation_of_their_own` (IA-RP-13) | empty, one-record |
| 2 | Maximum input: an offer past `store::repair::MAX_ROWS` refused by name, never cut; a second read twice alike kept once, twice differently refused | `the_gather_refuses_past_the_most_a_month_holds_and_a_second_read_twice_differently` (IA-RP-16) | maximum, duplicate rows |
| 3 | Year boundary: December repaired and closed when the run reaches 1 January, before 1 January is begun | `a_run_across_the_years_end_repairs_each_month_once` (IA-RP-12) | year boundary |
| 4 | Special sessions: the 2025-10-21 Muhurat and a regular day of its month, after the month's later days, in one generation | `special_sessions_cost_one_generation_per_month` (IA-RP-10, FM-098) | special session |
| 5 | A missing month mid-range of the held data: a hole early in a month costs at most one generation | `a_hole_early_in_a_month_costs_at_most_one_generation` (IA-RP-09, FM-092) | missing days mid-range |
| 6 | Late and out-of-order rows: a held day between the days a month lacks, each day judged alone, all lacked days in one repair | `a_month_holding_a_day_between_the_days_it_lacks_takes_them_in_one_repair` (IA-RP-11) | late rows |
| 7 | A kill mid-write: a crash between publication and promotion resumes as the same revision (`Published::Reused`), no second generation | `a_crash_between_publication_and_promotion_resumes_as_the_same_revision` | kill mid-write |
| 8 | A stop or a crash mid-gather never strands a begun day | `a_stop_or_a_crash_mid_gather_never_strands_a_begun_day` (IA-RP-04) | stop, kill |
| 9 | A full disk at publish leaves the gathered days begun for the next run, which converges | `a_full_disk_at_publish_leaves_the_gathered_days_to_the_next_run` (IA-RP-05) | full disk |
| 10 | **Combination**: a full disk at the run's END plus a month boundary already crossed plus two instruments: April (left earlier) stays closed `done`; May's NIFTY repair lands before BANKNIFTY's meets the full disk, every May day stays begun; the next run makes no second NIFTY generation and closes every day | `a_full_disk_at_the_runs_end_spares_the_month_it_already_left` | full disk × month boundary × two instruments |
| 11 | **Combination**: a stop pressed on a month's FIRST day: the left month repaired and closed at the boundary, the stopped day writes nothing | `a_stop_on_a_months_first_day_closes_the_month_it_left` (IA-RP-15) | stop × month boundary |
| 12 | **Combination**: two instruments sharing days, one refused for good (`repair_impossible`), the other landed: each shared day closes once, `incomplete`, never `done` and never left begun | `one_instruments_refused_repair_closes_the_shared_days_once_and_spares_the_other` (IA-RP-14) | refusal × shared days × two instruments |
| 13 | **Out of the box**: the census refuses the rows of a month already published and served: the run ends `stage=census`, the first waiting day closes `incomplete`, the rest stay begun; the next run repairs nothing and closes every day | `a_census_that_refuses_the_repaired_rows_ends_the_run_and_the_next_converges` (IA-RP-18) | refusal after publication |
| 14 | A CASH month of the closing auction's era: the `.auc` of the new generation is written whole before it is served, older days' prints carried | `a_cash_months_gathered_repair_writes_its_auction_file_before_it_is_served` (IA-RP-02b) | CASH sidecar |
| 15 | A late day older than the sidecar holds: carried, never refused, nothing written to the original | `revision_tests::a_day_older_than_the_sidecar_holds_is_carried_never_refused` (IA-AUC-02) | out-of-order row |
| 16 | A revision-served month is never appended to, and readers read the served sidecar | `revision_tests::a_revision_served_month_is_never_appended_to_and_its_readers_read_the_served_sidecar` (IA-AUC-02) | append-only history |
| 17 | Two writers' copies disagree: a carried day the served sidecar holds differently is refused by name | `revision_tests::the_step_refuses_a_carried_day_the_served_sidecar_holds_differently` (IA-AUC-03) | two writers |
| 18 | Repeat and nothing: the step is repeatable and writes nothing for nothing; a carried day with no print carries nothing | `revision_tests::the_step_is_repeatable_and_writes_nothing_for_nothing` (IA-AUC-03, IA-AUC-05) | empty, rerun |
| 19 | A store that takes no writes, and a store lost, at the gathered repair: the run ends by that name, nothing published, the next run converges | `a_store_that_takes_no_writes_at_the_gathered_repair_ends_the_run_with_its_days_begun`, `a_store_lost_at_the_gathered_repair_ends_the_run_and_the_next_converges` (IA-RP-19) | unwritable volume, unplugged volume |
| 20 | **Combination**: a day that gathered one instrument's file while another's append met a full disk: the day stays begun | `a_day_that_gathered_and_whose_pass_ends_the_run_stays_begun` (IA-RP-20) | gather × full disk × two instruments |
| 21 | A repair journal that takes no line (a directory at its name): refused before anything is published, every day `incomplete` | `a_repair_journal_that_takes_no_line_refuses_the_gathered_repair_by_name` (IA-RP-21) | torn or unwritable ledger |
| 22 | **Combination**: two files of one instrument and day reading a second differently: the second refused by name, the first repaired | `a_second_file_reading_a_gathered_second_differently_is_refused_and_the_first_kept` (IA-RP-22) | duplicate rows × gather |
| 23 | **Combination**: a stop mid-day after a gathered file: the day closes `incomplete reason=stopped`, the gathered month repaired | `a_stop_mid_day_after_a_gathered_file_closes_the_day_by_what_was_written` (IA-RP-24) | stop × mid-day × gather |
| 24 | **Combination**: a later day closed before an earlier day waiting for its month, both incomplete: the report's day lists stay in order | `a_deferred_close_after_a_later_days_close_keeps_the_report_in_day_order` (IA-RP-25) | out of order × refusal |
| 25 | **Out of the box**: a sidecar holding a later day whose bars never landed: the carried print is named, never dropped | `a_print_carried_for_a_generation_its_bars_never_needed_is_named_never_dropped` (IA-AUC-04) | inconsistent store |
| 26 | A month whose served generation cannot be told (its pointer a directory): the print refused, nothing written | `revision_tests::a_month_whose_served_generation_cannot_be_told_takes_no_print` (IA-AUC-05) | garbage store state |
| 27 | **Combination**: a stop first seen at a day's top, the day before it gathered: the months repaired once each, the gathered day closed `done`, nothing of the next day read | `a_stop_seen_at_a_days_top_closes_the_gathered_days_before_it_and_reads_no_more` (IA-RP-37) | stop × month repair × day boundary |
| 28 | **Combination**: a gathered day whose own pass ends the run by a store that takes no writes, by the held-back set at its cap, and by the source's root gone: begun each time, nothing repaired, the next run converges | `a_gathered_day_whose_pass_ends_the_run_stays_begun_whatever_ends_it` (IA-RP-38) | gather × unwritable volume; gather × maximum (the cap); gather × unplugged source |
| 29 | **Out of the box**: a source that says it holds no file of a day yet hands one, which is gathered, and a full disk at the month's repair: the run ends `disk_full`, the day's one close is its pass's | `a_day_the_source_disowns_after_a_gathered_file_keeps_its_own_close_when_the_repair_ends_the_run` (IA-RP-39) | inconsistent source × full disk |
| 30 | **Out of the box**: the same disowned day and a census that refuses the repaired rows: closed `incomplete stage=census` after its `refused` line, read again, converges | `a_disowned_day_that_waited_first_is_closed_by_a_census_refusal_and_read_again` (IA-RP-40) | inconsistent source × census refusal |
| 31 | The same disowned day, its month repaired: the pass's `refused` line stands, no second close | `a_day_the_source_disowns_after_a_gathered_file_keeps_its_own_close_when_its_month_is_repaired` (IA-RP-33) | inconsistent source |
| 32 | A gathered repair's failure no door names: named on each gathered file under its own day, the month not counted, the next run repairs it once | `a_gathered_repairs_failure_no_door_names_is_named_on_each_gathered_file` (IA-RP-41) | an unaddressable batch |
| 33 | **Two writers**: another writer repairs the gathered day into the month before the run's end: no second generation, nothing counted, each bar served once | `a_gathered_day_another_writer_lands_first_costs_no_second_generation` (IA-RP-42) | two writers |
| 34 | A carried print whose bars the door refused (no repair reason): neither filed nor named dropped; the read that lands the day carries it into the new `.auc` | `a_carried_print_whose_bars_the_door_refused_waits_for_the_read_that_lands_them` (IA-AUC-13) | refusal × CASH sidecar |
| 35 | **Two writers**: a day the original sidecar holds differently is refused, never carried | `revision_tests::a_day_the_original_holds_differently_is_refused_never_carried` (IA-AUC-10) | two writers |
| 36 | **Combination**: a torn `.auc` the host will not cut back: refused by name, the file unchanged, the next append cuts and lands | `revision_tests::a_torn_tail_the_host_will_not_cut_back_refuses_the_append_and_keeps_the_file` (IA-AUC-11) | torn file × unwritable volume |
| 37 | A fresh sidecar whose directory the host will not sync: named, and a retry files nothing twice | `revision_tests::a_fresh_sidecar_whose_directory_will_not_sync_is_refused_by_name` (IA-AUC-12) | crash durability, empty store |
| 38 | A refusal named on each gathered file under its own day, a stray file under none | `gather_unit_tests::a_refusal_is_named_on_each_gathered_file_under_its_own_day` (IA-RP-43) | out-of-order index |
| 39 | A window widened to hold a day across a month and a year boundary, and never for a day inside it | `session::month_boundary::a_window_widens_only_as_far_as_the_day_it_must_hold` (IA-RP-44) | month and year boundaries |
| 40 | A widened day whose member key's close waits for its month is counted once | `scope_reopen::a_new_member_holding_only_a_later_day_takes_its_earlier_done_days` (IA-RP-45) | renamed / new member × gather |

## Files outside ownership

Each kept minimal (spec §4b):

- `crates/pull/src/ingest.rs` — the one seam the spec allows: `gather_rows` (the back-fill door's
  judgement with nothing published, `Filed::Gathered`), `repair_gathered` (the repair with the
  `before_promote` step through `repair_month_with`), `judge_back_fill` split out of
  `back_fill_or_append` with its behaviour unchanged, and `repair_after` taking the step (every other
  caller passes `None`, so their bytes on disk are unchanged).
  On 27 Oct (the coverage pass): `Filed::Gathered` gained `month` (the month the door addressed),
  and one test's `matches!` became a pattern read off its `Debug` rendering.
- `crates/pull/src/session.rs` — 27 Oct: `Window::holding`, a pure total addition (the window
  widened to hold a day), and its test. No caller but `widen_window`.
- `crates/pull/src/gdfl_import.rs` — minimal, as the spec allows: `FileFacts.auction` and `cm_read`
  calling `file_or_carry`.
- `crates/pull/src/gdfl_ia16_tests.rs` — one line: `auction: None` in a `FileFacts` literal.
- `crates/pull/src/gdfl_scope_tests.rs` — the `Ctx` literal's `gather` field, and `months_repaired`
  2 → 1 with a D-4899 comment (that test measured the old per-file repair; one generation is the new
  rule); 27 Oct: the same test asserts `days_imported == 2` (IA-RP-45, the mutant run's find).
- `crates/pull/src/gdfl_import_tests.rs` — the `#[path = "gdfl_gather_tests.rs"] mod gather;` mount.
- `.github/workflows/ci.yml` — gate 14's `pull` pin 38 → 39 and `C-RP-02` added (the shared exact
  line, recounted after merging `d39e789e`, which did not change it).
- `.github/gates_tree.rs` — gate 1d's `IA17B_LITERAL` group (event codes, test folder labels,
  `clear_refs`), `DECLARED` 75 → 76 (80 after the eighth merge, each base group first); 27 Oct: the
  coverage tests' fourteen folder labels. None names an account, an environment, a vendor
  credential or a field of a parameter path.

Not touched: `crates/store/**` (the step is only called), `crates/api/**` (`auction::path` keeps its
signature), `crates/cli/**`, `web/**`, `Cargo.toml`, `Cargo.lock`.

## D-numbers used

- **D-4899** (the spec's; heading added, `git grep -n -E '^#+ D-4899' origin/feat/db-fix --
  docs/05-decisions.md` printed nothing before it was written): one repair per (instrument, month) per
  run, its stop, crash, full-disk, bound and census rules; amends D-4875 (3), (6) and D-4836's V-4 line.
- **D-TBD-IA-17b-1**: a repaired CASH month's `.auc` follows its served generation, and a print is
  carried, never dropped. The Mac assigns its number at merge-back.

Cited, not re-headed: D-4875, D-4898, D-4889, D-4893, D-2821, D-3350, D-6247, D-7131.

## NEEDS THE MAC

The cloud has no store, no archive and no shadow server. Spec §6, with the commands as the spec gives
them (`W` and the store root are the spec's; the archive variables come from the operator's local
untracked configuration and are never written into a file):

- **M0. Prerequisites and build.**
  `git -C $W/wt/DBFIX log feat/db-fix --first-parent --merges --format=%s | grep -E '^Merge (IA-17|WP-12d|IA-16):'`
  (three lines), then `cargo build --release -p api -p cli` in the merge-back worktree, binaries to
  `$W/tgt/bin-IA-17b/release/`.
- **M1. Read-only shadow server on port 8414** (GET only): `$W/state/fix/stop.sh IA-17b;
  $W/state/fix/shadow.sh IA-17b`, then copy `bar-generations-v1` into the shadow (the spec's `cp -cR`
  line: without it a shadow serves every repaired month from its original), then
  `$W/state/fix/serve.sh IA-17b 8414 $W/tgt/bin-IA-17b $W/wt/P-IA-17b`, and
  `lsof -nP -iTCP:8414 -sTCP:LISTEN -t` must print the pid in `$W/shadow/IA-17b/test-server.pid`.
  - **(a)** `GET /sources/day.json?exchange=NSE&segment=CASH&symbol=RELIANCE&from=2026-09-24&to=2026-09-24`
    answers byte for byte what the head's binary on another port answers (`auction::path` resolving
    the served generation changes no answer for an original month; the store holds 0 `.auc` files).
  - **(b)** The same route for a share-day of a revision-served month (NSE-CASH-ACC 2024-05 is one):
    same answer as the head; the resolution adds one pointer read.
  - **(c)** p50, p99, max over n >= 200 sequential GETs of (a) and (b), against the head's numbers;
    expected: the added cost is one generation pointer read, O(1). Report n and the method.
- **M2. Write-path checks, only on a store root IA-17b owns entirely** (`BRUTEX_STORE=<that root>`),
  proving afterwards that no file under the real store changed (the months' mtimes, and
  `grep -c . <real store>/audit/gdfl.repairs` unchanged; take that count first as the baseline):
  - **(d)** NIFTY spot, a special-session month (the many-days shape):
    `cli gdfl-import indices zip <first day> <last day> <SYMBOL> --repair-reason "IA-17b merge-back check"`;
    expected ONE `repair` line in `<root>/audit/gdfl.repairs` for the (instrument, month), the month at
    generation 1, every gathered day closed in day order in `<root>/imports/gdfl.journal`,
    `months_repaired` counting months; run it again: no new generation, nothing written (IA-RP-03,
    FM-092).
  - **(e)** One NIFTY and one BANKNIFTY month of the 12 one-day-a-month index days (the month of
    2021-02-24): one generation each.
  - **(f)** NIFTY options, a contract-month with a lacked special session (FM-098): one generation per
    contract-month or a named refusal; run the critic's read-only check first,
    `find <real store>/bars/gdfl/NSE/FNO -name '*.grk' -o -name '*.ovl' | head` (a `.grk`/`.ovl`
    sibling makes every back-fill `repair_impossible (IndexedSibling)`). Then one BANKNIFTY month, one
    F&O stock's CASH month and one illiquid strike.
  - **(g)** CASH `.auc` of a repaired month, into an EMPTY private root:
    `cli gdfl-import stocks zip <a later day of 2026-09> 2026-09-30 RELIANCE`, then
    `cli gdfl-import stocks zip 2026-09-01 <the day before> RELIANCE --repair-reason "IA-17b merge-back check"`;
    expected one generation; `<root>/bar-revisions-v1/<n>/bars/gdfl/NSE/CASH/RELIANCE/1s/2026-09.auc`
    exists, parses (version 2), holds the repaired days' prints and every later day's in day order; the
    original `.auc` byte-identical; no closing-auction print is a 1s bar of the revision; a server over
    that root answers `/sources/day.json` for a repaired day with the auction read from the served
    `.auc`.
  - **(h)** Stop and crash mid-gather (IA-RP-04): during (d), `cli gdfl-stop indices`; expected the
    gathered days offered and closed, none left `begin`. A `kill -9` during (d): the days stay
    `begin`, and the next repair-reason run publishes or meets `Published::Reused` and closes them; no
    second generation.
  - **(i)** Full disk at publish (IA-RP-05), on a small private volume (a disk image of a few hundred
    MB): the run ends `disk_full` with the gathered days still `begin`; on a freed volume the next run
    converges.
  - **(j)** Memory: peak resident set of (d) and (g) by `/usr/bin/time -l`, one run per process; C-RP-02
    in release three times (`cargo bench -p pull --locked --bench ratio`; p50, p99, max, n); and a
    long-lived process doing many month repairs (verify-IA-17-r1 follow-up 1 measured about a 1 GB
    plateau for real-sized months).
- **Store writes this part makes NEED on the real store** (listed, never run): the V-5 back-fill of
  the alias months (IA-14's runbook step, on the operator's yes) is ready only once IA-17b merges;
  nothing prunes the superseded generations already kept (66.22 GiB on 14 Oct): a deletion, the
  operator's.

**Gate runs this cloud session could not finish, or could not make honestly:**

- **Mutants, the whole diff: done here (27 Oct), none survived** (Gates, 27 Oct). To confirm at
  merge-back, on the merge-back tree: `cargo mutants --in-diff <(git diff feat/db-fix HEAD --
  crates/pull/src) -p pull -- --lib -- gdfl_import:: gdfl_cm:: ingest:: session::month_boundary::`
  (26.2.0); expected: no `MISSED`, every mutant caught, unviable or timed out (two time out: the
  `name_in_order` and `give_added` loops a mutation makes endless). About 1.3 min a mutant on 4
  cores with `CARGO_PROFILE_DEV_OPT_LEVEL=1 CARGO_PROFILE_DEV_INCREMENTAL=true` and
  `--cargo-arg=--lib`.
- **A base finding the mutant run met (not this part's, not blocking):**
  `crates/pull/src/gdfl_import_day.rs` `widen_key`, `widened.closed += 1` -> `*= 1` survives. It is
  WP-12f's line, unchanged against `feat/db-fix`: the `members` count a widened day's summary event
  carries is read by no test (`tests::summary_emitted` keeps only the event's text fields). Its
  owner's fix: keep numeric fields in the summaries capture and assert `members` in a widened-day
  test.
- **Coverage: done here (27 Oct), 100% of the added lines and arms** (Gates, 27 Oct). To confirm
  on a nightly toolchain: `cargo llvm-cov -p pull --lib --branch`, its lcov matched to `git diff
  --diff-algorithm=histogram -U0 feat/db-fix HEAD` of `ingest.rs`, `gdfl_import_filing.rs`,
  `gdfl_import_day.rs`, `gdfl_cm_auction.rs`, `gdfl_import.rs` and `session.rs`; expected: every
  added line run, every added arm taken (this session used `RUSTC_BOOTSTRAP=1` on stable 1.97.1).
- **The tests root defeats**, run as a non-root user: `cargo test --workspace --locked`. Expected:
  the 9 `pull`, 3 `cli`, 3 `store`, 4 `telemetry` and the 2 root-dependent `api` permission tests
  green (they fail here because root reads and writes through 0o000, 0o444 and 0o555).
- **The tests the free-space floor refused** (25 `cli` and 2 `gdfl_run_logs` and 3 `pull`), on a
  volume with more than 10 GiB free: green here only with `TMPDIR` on a tmpfs.
- **The 4-core pool tests**, on a host with at least 8 cores (`MAX_STORE_READ_CONCURRENT` and
  `MAX_CALENDAR_CONCURRENT` are 8): `api` `detail::tests::a_refused_admission_names_what_its_bound_and_why`,
  `server::gaps_route::cloud_mutant_tests::a_full_calendar_pool_refuses_the_census_and_the_audit`
  (hung here, skipped by name), `tests/cloud_mutants_bars.rs`
  `spot_rs::spot_json_is_refused_by_name_when_every_store_read_slot_is_held` (hung here, stopped). And
  the five `head_deadline_body_tests` slow-reader timings, red here and on the base alike.
- **`cargo deny check` with a newer cargo-deny.** CI's pinned 0.19.0 is green; 0.20.2 reports
  RUSTSEC-2026-0386 (`tokio` 1.53.1, unsound `AsyncFd::new`; fixed in 1.53.3) on the base and the
  branch alike. The lock is not this part's (CONTEXT §4.10): a base change, if the pin moves.
- **The self-check's shadow re-runs** (README step 5b (iii)): every finding fixed here was proved
  by a test, not on a shadow server; (a) to (j) above are those runs.

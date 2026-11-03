STATUS: IN PROGRESS

# report-BASE-COV-PULL-PS3 (TOKEN BASE-COV-PULL-PS3-20261103-1)

Session BASE-COV-PULL-PS3 of the 3 Nov 2026 cloud wave: group G3 of `SHARDS.md` (BASE-COV-PULL, operator decision
D2 part A). Branch `cloud/cov-pull-ps3-20261103`, cut from `2e7a0b6d3b568190024e2e9b405e803805f3fdca` (PS1's
GREEN-TIP). Owner file `reports/base-cloud-20261103/OWNER-BASE-COV-PULL-PS3.md` (commit `5699dd48`). Code tip of
this report: **`a5ac2140fcac02e6b123b5661cab56ecbd731405`** (every figure below is for it unless named otherwise).

## Outcome

- **Route B, G3's 71 files, whole pull suite run** (the kit's `d3-route-b-cloud.sh`, pin `rustc 1.99.0-nightly
  (1a98b1e13 2026-08-07)`, LLVM 23.1.0):
  - start `2e7a0b6d` (61 files): lines 49,318/51,591 (**2,273 uncovered**), branches 3,865/4,792 (**927 sides**);
  - tip `a5ac2140` (71 files: ten new test files): lines 58,751/60,007 (**1,256 uncovered**), branches
    4,289/4,792 (**503 sides**).
- **By class** (`fu.mjs`, the same reader on both, checked by hand where this report says so): start 713 (F), 941
  inside `#[ignore]` tests (L-BCP-3), **1,546 gaps**; tip 753 (F), 898 L-BCP-3, 91 (U) with proofs, **17 gaps**.
  The 17: 11 sides of one test that depend on the git history present, 3 of one fuzz test's oracle, 1 race arm
  (fixed after this tip, not yet measured), and `tenor.rs:203-204`, which needs one test in G1's `vendor.rs`
  (BLOCKED OUTSIDE, the exact test below). So the bar is **not yet held** for G3.
- **251 new tests** (`tests-PS3.txt.md`), each shown red under a hand change of what it checks.
- **Three real defects fixed at their cause**, each with a test that fails first: the gdfl_dropbox staging lock
  and the tick store's day lock outlived their run in any child forked meanwhile (spurious `run_busy` /
  `day_locked`; 103/300 and 149/300 retakes refused under spawning, 0/300 after); rolling told a question about
  2101 that the day was before the regime. Also one test that wrote vendor captures into `$HOME/.brutex` (on the
  operator's machine, the real archive root) now uses an invented base URL.
- **Gates on `a5ac2140`** (stable 1.97.1, the repo's profile): see "Gates".

## Kit check

Every kit file used matches `MANIFEST.md` (`sha256sum` of the whole file, or of the fenced block extracted with the
manifest's own `awk`): CLOUD.md, CONTEXT.md, DECISIONS.md, SHARDS.md, repair-BASE-COV-PULL-progress.md,
d3-recipe.txt, d3-route-b-cloud.sh, d3-covset.mjs, deep-BASE-COV-PULL-c9d653d5f914-gaps.txt,
deep-BASE-COV-PULL-f98a899398f6-set.txt, base-cov-pull-fu.txt, verify-BASE-COV-PULL-r2.json, fix-BASE-COV-PULL.json;
`MANIFEST.md` hashes to `078a849c...c6479`; `jq -e .` accepts both json files. Decision id: D-7337 (my block), not
present in `docs/` at the start.

## Host

Linux x86_64 cloud container, uid 0 (root), 4 cores, 15 GiB memory, 20-30 GiB free on `/`. Stable `rustc 1.97.1
(8bab26f4f 2026-07-14)` (`rust-toolchain.toml`) for the gates. The Route B pin installed as `nightly-2026-08-08`:
`rustc -V` prints exactly `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`; its `llvm-cov` and `llvm-profdata` print
`LLVM version 23.1.0-rust-1.99.0-nightly`. `TMPDIR=/dev/shm/ps3-tmp` (tmpfs, mode 1777); `CARGO_PROFILE_DEV_DEBUG=0`.

## G3's files

The 55 files `SHARDS.md` lists under G3; the six test files repair 2 added for G3 production files
(`gdfl_dropbox_cov_tests.rs`, `gdfl_run_start_cov_tests.rs`, `http_cov_tests.rs`, `kite_cov_tests.rs`,
`masters_cov_tests.rs`, `zerodha_verify_cov_tests.rs`); and the ten test files this session added, each declared in
the G3 production file it tests: `masters_land_tests.rs`, `ingest_gap_tests.rs`, `zerodha_verify_cov_store_tests.rs`,
`archive_cov_tests.rs`, `gap_backfill_cov_tests.rs`, `gaps_cov_tests.rs`, `gdfl_import_journal_cov_tests.rs`,
`gdfl_import_lineage_cov_tests.rs`, `gdfl_run_scope_cov_tests.rs`, `gdfl_tickstore_cov_tests.rs`. 71 files. No file
outside G3 was edited (`git diff --stat 2e7a0b6d a5ac2140`: 49 files, all G3's, plus `reports/`).

## How the work was done

1. Route B baseline on the start tip (below), each item classified by where it sits (production, test code, (F)
   candidate, inside an `#[ignore]` test).
2. Six helpers (subagents of this session), each in a detached worktree of this branch with its own target dirs,
   one file family each, under one written brief (G3's files only; no git writes; red first; 8 extreme cases a file,
   2 combinations, 1 out of the box; root and telemetry notes): zerodha_verify; gdfl_dropbox; ingest and
   ingest_failkind; tickstore_write and http; the GDFL family (gdfl_import_lineage, gdfl_import_journal,
   gdfl_lineage, gdfl_tickstore, gdfl_archive, gdfl_run_scope, gdfl_run_start); the small files (gap_backfill,
   rolling, resolve, gaps, archive, session, totp, tenor, day_universe). I took masters and the test-only files.
   Builds shared two flock slots. For a quick check the helpers used an instrumented lib-only run (`cq.sh`, not
   Route B); every figure in this report is Route B's.
3. Each helper's diff was read here against the rules (production changes one by one, no real vendor row, no
   credential, no forbidden language), then its tests, fmt and clippy were run in this checkout before its commit.
4. Route B on the integrated tips (`b82b4f02`, `cd0cf688`, `2d8d91f7`, `a5ac2140`); the last is this report's.

Commits (`git log --oneline 2e7a0b6d..a5ac2140`): `5699dd48` owner; `d418a3ec` six root-only tests where the bits
bind; `f61e55d6` report; `c0230d8d` masters; `9341c620` ia16, nfo attack; `8e306353` no_scheduler_delete;
`e11fc694` derive, pipeline; `6d0c3a06` gdfl_cm admission; `454a6797` gdfl_dropbox; `35fbe045` report; `53e4b9a9`
ingest; `2a1fcb6b` split proof helpers; `d3d4bb2f` zerodha_verify; `b82b4f02` tickstore_write, http; `90d300f6`
gap_backfill, gaps, resolve, rolling, session, totp, archive; `194b5150` rolling defect; `cd0cf688` GDFL family;
`2d8d91f7` masters' plant lookup in both builds, wp12e helpers; `a5ac2140` dpn_09's known pre-cutover name. Each
commit message carries its tests' red-first evidence.

## Coverage (Route B)

Command, for each run: `bash rb.sh <tree at TIP> <tip12> <k> set-g3.txt` (scratch), which runs the kit's
`d3-route-b-cloud.sh` unchanged with `D3_W`, `D3_DIR`, `D3_NIGHTLY`, `D3_LLVM_BIN`, `D3_CARGO` set to this host's
nightly, `TMPDIR=/dev/shm/ps3-tmp`, `CARGO_PROFILE_DEV_DEBUG=0`, on a detached worktree at TIP; set = `reading
whole:pull` and one `coverage <file> whole` line per G3 file; package `pull`, no test filter. **Root host:** a
watcher makes the per-package profile folder mode 1777 within 50 ms of the script creating it, so
`where_permission_binds` children (uid 65534) write their profiles (35 of 101 profiles at the tip are theirs).

| run | build | tests | shows | tests passed / failed / ignored | profiles |
|---|---|---|---|---|---|
| start `2e7a0b6d` | 16:05-16:06 IST | 16:06-16:10 | to 16:16 | 2940 / 9 / 54 | 72 |
| tip `a5ac2140` | 18:51-18:52 | 18:52-18:56 | to 19:04 | 3197 / 3 / 54 | 101 |

Both: `ROUTE-B DONE ... every package measured and read (pull)`, exit 0, no NOT PROVED line. The failures are
root-only reds: at the start PS1's nine; at the tip G2's three (`alerts_tests`, two in `census_store_link_tests`).

Per file (lines a/b, branches c/d), start -> tip:

| file | start lines | start branches | tip lines | tip branches |
|---|---|---|---|---|
| `crates/pull/src/archive.rs` | 472/480 | 28/32 | 481/484 | 31/32 |
| `crates/pull/src/archive_cov_tests.rs` | - | - | 84/84 | 0/0 |
| `crates/pull/src/cash_session_cache_lock_tests.rs` | 291/302 | 15/22 | 291/302 | 15/22 |
| `crates/pull/src/corporate_actions.rs` | 1000/1000 | 166/166 | 1000/1000 | 166/166 |
| `crates/pull/src/corporate_actions_cov_tests.rs` | 289/291 | 9/12 | 289/291 | 9/12 |
| `crates/pull/src/corporate_actions_tests.rs` | 1279/1293 | 52/84 | 1279/1293 | 52/84 |
| `crates/pull/src/day_universe.rs` | 112/112 | 14/22 | 112/112 | 14/22 |
| `crates/pull/src/fnowork.rs` | 871/871 | 62/62 | 871/871 | 62/62 |
| `crates/pull/src/fnowork_cov_tests.rs` | 201/203 | 0/0 | 201/203 | 0/0 |
| `crates/pull/src/gap_backfill.rs` | 222/232 | 36/48 | 230/230 | 46/46 |
| `crates/pull/src/gap_backfill_cov_tests.rs` | - | - | 432/434 | 18/18 |
| `crates/pull/src/gaps.rs` | 714/720 | 96/104 | 728/728 | 82/82 |
| `crates/pull/src/gaps_cov_tests.rs` | - | - | 108/108 | 3/6 |
| `crates/pull/src/gdfl_archive.rs` | 1451/1520 | 77/86 | 1530/1597 | 81/86 |
| `crates/pull/src/gdfl_cm_admission_tests.rs` | 1282/1334 | 6/12 | 1295/1347 | 7/12 |
| `crates/pull/src/gdfl_dropbox.rs` | 1970/2133 | 367/450 | 2145/2145 | 447/448 |
| `crates/pull/src/gdfl_dropbox_cov_tests.rs` | 36/39 | 5/8 | 649/661 | 22/38 |
| `crates/pull/src/gdfl_dropbox_tests.rs` | 2627/2745 | 115/150 | 3689/3797 | 138/166 |
| `crates/pull/src/gdfl_ia16_tests.rs` | 2136/2187 | 84/136 | 2186/2188 | 87/132 |
| `crates/pull/src/gdfl_ia4_tests.rs` | 192/193 | 4/6 | 192/193 | 4/6 |
| `crates/pull/src/gdfl_import_ia2_tests.rs` | 449/551 | 11/22 | 449/551 | 11/22 |
| `crates/pull/src/gdfl_import_journal.rs` | 775/788 | 193/208 | 791/791 | 207/208 |
| `crates/pull/src/gdfl_import_journal_cov_tests.rs` | - | - | 323/323 | 2/2 |
| `crates/pull/src/gdfl_import_lineage.rs` | 367/385 | 64/82 | 380/380 | 78/78 |
| `crates/pull/src/gdfl_import_lineage_cov_tests.rs` | - | - | 265/265 | 0/0 |
| `crates/pull/src/gdfl_import_split_tests.rs` | 401/414 | 34/44 | 427/433 | 37/44 |
| `crates/pull/src/gdfl_lineage.rs` | 817/822 | 146/154 | 822/822 | 154/154 |
| `crates/pull/src/gdfl_lineage_tests.rs` | 898/1015 | 11/26 | 1116/1212 | 13/26 |
| `crates/pull/src/gdfl_nfo_attack_tests.rs` | 1264/1301 | 142/178 | 1302/1329 | 153/176 |
| `crates/pull/src/gdfl_run_ia2_tests.rs` | 766/942 | 38/74 | 782/946 | 40/74 |
| `crates/pull/src/gdfl_run_scope.rs` | 56/62 | 7/8 | 62/62 | 8/8 |
| `crates/pull/src/gdfl_run_scope_cov_tests.rs` | - | - | 157/157 | 2/2 |
| `crates/pull/src/gdfl_run_start.rs` | 70/71 | 5/6 | 71/71 | 6/6 |
| `crates/pull/src/gdfl_run_start_cov_tests.rs` | 90/90 | 2/2 | 130/130 | 2/2 |
| `crates/pull/src/gdfl_series_code_tests.rs` | 363/363 | 2/4 | 363/363 | 2/4 |
| `crates/pull/src/gdfl_tickstore.rs` | 1149/1167 | 111/120 | 1185/1186 | 115/116 |
| `crates/pull/src/gdfl_tickstore_cov_tests.rs` | - | - | 157/157 | 2/2 |
| `crates/pull/src/gdfl_wp12e_tests.rs` | 516/633 | 9/14 | 517/634 | 7/10 |
| `crates/pull/src/http.rs` | 4316/4463 | 324/416 | 4431/4501 | 339/394 |
| `crates/pull/src/http_cov_tests.rs` | 74/76 | 2/4 | 484/493 | 14/22 |
| `crates/pull/src/ingest.rs` | 3100/3268 | 243/296 | 3269/3286 | 270/284 |
| `crates/pull/src/ingest_cov_tests.rs` | 213/213 | 2/4 | 213/213 | 2/4 |
| `crates/pull/src/ingest_failkind.rs` | 124/125 | 10/10 | 124/125 | 10/10 |
| `crates/pull/src/ingest_failkind_tests.rs` | 574/574 | 7/10 | 583/583 | 8/10 |
| `crates/pull/src/ingest_gap_tests.rs` | - | - | 1064/1064 | 0/0 |
| `crates/pull/src/kite.rs` | 57/57 | 0/0 | 57/57 | 0/0 |
| `crates/pull/src/kite_cov_tests.rs` | 6/6 | 0/0 | 6/6 | 0/0 |
| `crates/pull/src/masters.rs` | 1643/1723 | 173/214 | 1742/1765 | 184/204 |
| `crates/pull/src/masters_cov_tests.rs` | 92/92 | 2/2 | 121/121 | 4/4 |
| `crates/pull/src/masters_land_tests.rs` | - | - | 245/248 | 7/10 |
| `crates/pull/src/repair_plan_not_owed_tests.rs` | 272/274 | 7/10 | 272/274 | 7/10 |
| `crates/pull/src/resolve.rs` | 610/639 | 28/34 | 667/680 | 23/24 |
| `crates/pull/src/rolling.rs` | 815/839 | 64/80 | 1030/1035 | 64/80 |
| `crates/pull/src/session.rs` | 692/703 | 108/116 | 738/747 | 106/112 |
| `crates/pull/src/tenor.rs` | 192/196 | 6/8 | 192/196 | 6/8 |
| `crates/pull/src/tickstore_write.rs` | 1003/1089 | 167/188 | 1121/1129 | 184/188 |
| `crates/pull/src/tickstore_write_tests.rs` | 1181/1351 | 44/68 | 1779/1931 | 49/68 |
| `crates/pull/src/totp.rs` | 301/306 | 29/34 | 370/372 | 24/26 |
| `crates/pull/src/zerodha_verify.rs` | 2625/2863 | 492/614 | 2859/2872 | 596/614 |
| `crates/pull/src/zerodha_verify_cov_store_tests.rs` | - | - | 1104/1106 | 10/12 |
| `crates/pull/src/zerodha_verify_cov_tests.rs` | 31/31 | 0/0 | 849/850 | 3/4 |
| `crates/pull/src/zerodha_verify_tests.rs` | 3715/3759 | 49/78 | 3762/3773 | 46/72 |
| `crates/pull/tests/anchor.rs` | 516/522 | 24/32 | 516/522 | 24/32 |
| `crates/pull/tests/census_v4.rs` | 549/628 | 4/6 | 549/628 | 4/6 |
| `crates/pull/tests/derive.rs` | 1396/1408 | 39/56 | 1396/1407 | 38/54 |
| `crates/pull/tests/futures_events.rs` | 130/131 | 3/6 | 130/131 | 3/6 |
| `crates/pull/tests/gdfl_bad_row_audit.rs` | 239/245 | 3/6 | 239/245 | 3/6 |
| `crates/pull/tests/gdfl_census_stop.rs` | 146/151 | 10/18 | 146/151 | 10/18 |
| `crates/pull/tests/integration.rs` | 237/238 | 0/0 | 237/238 | 0/0 |
| `crates/pull/tests/no_scheduler_delete.rs` | 295/300 | 96/112 | 318/318 | 112/112 |
| `crates/pull/tests/pipeline.rs` | 1048/1062 | 18/28 | 1047/1060 | 16/24 |

Every uncovered line and side at the tip, with its class: `gaps-PS3.txt.md`. The (F) and (U) items with their
reasons and proofs, and the L-BCP-3 items: `fu-PS3.txt.md`.

### The 17 gaps at the tip

- `gdfl_import_split_tests.rs` 910, 911, 912, 916, 921, 925, 930, 931, 936, 937, 954 (11): the split proof reads
  each of its six files from the tree while the bytes are the split's, else from the git history's blob, else
  skips; which sides run depends on the history present (here and on the Mac: files changed since, blobs found).
  The fix is a reader seam so both sides run in a test (the verifier's ssm env-door remedy). Not done yet.
- `gdfl_nfo_attack_tests.rs` 1440, 1443:32 False, 1448 (3): dpn_13's bent files never draw an
  `OpenInterestRefused` refusal, and no line refusal comes from a file with no bent line. Not done yet.
- `cash_session_cache_lock_tests.rs` 232: a race test's whole-master arm, which a loaded host's scheduling never
  took. Fixed after this tip (the post-race reader is judged by the same rule), not yet measured.
- `tenor.rs` 203-204: `TenorError::HoursUnverified`'s words need a `SessionRefusal`, whose fields are private to
  G1's `vendor.rs`. **BLOCKED OUTSIDE**, the exact test for `vendor.rs`'s `mod tests`:

      #[test]
      fn a_tenor_refused_for_unverified_hours_quotes_the_refusal() {
          let refusal = SessionRefusal { venue: Venue::NseDerivatives, day: day(2030, 1, 1),
              row_start: day(2029, 1, 1), verified_from: None, source: "TEST ROW" };
          let said = crate::tenor::TenorError::HoursUnverified(refusal).to_string();
          assert!(said.starts_with("the expiry day's trading hours are not verified"), "{said}");
          assert!(said.ends_with(&refusal.to_string()), "{said}");
      }

### (U) items (91 at the tip, `fu-PS3.txt.md`)

Each with its proof from the code, on any target (aarch64-apple-darwin included): masters' hostless URL (reqwest
refuses a URL without a host before sending); gdfl_dropbox's Body-kind reqwest error (built only for request bodies,
timeouts, blocking, http3); ingest's `year_month` of a `Day`, `derive_repair`'s rungs, `FileType`'s seven
predicates, `StoreError`'s 36 named variants; zerodha_verify's 31 (the session permit behind `Open(full)`, the const
`Paisa` scale and HTTP descriptor, slot and loop guards); tickstore_write's canonical stem, non-empty lines,
`parallel`'s guard, `ReadRefusal`'s two variants; http's 6-or-7-cell rows and bounded offsets; day_universe's slot;
resolve's `is_sound` and `constituent_link`; rolling's `TradeDay::MAX` (a Friday) and `Expiry::new` bounds;
session's recorded windows and verified tables; totp's 20-byte tag; gdfl_archive's slot; gdfl_import_journal's
`begun_early`; and the 11 lines of `a_killed_holder_releases_the_day`'s child, which run but only in a process the
test kills with SIGKILL (no exit handler, so no profile: the operator may prefer UNMEASURED for these).

### (F) items (753) and L-BCP-3 (898)

(F) only as recipe step 4 allows: a `panic!`/`unreachable!` line; an assert's message lines whose assert ran; a
condition side inside an assert; the else of a let-else or the body of an `if` that only panics; a closure or test
helper whose body is a panic. A production refusal is never (F). L-BCP-3: the bodies of G3's `#[ignore]` tests (real
data and measurements) and the seven helpers only they call (`ignored-helpers`: gdfl_wp12e `spread` and
`synthetic_ledger`, the two `pct`, gdfl_lineage_tests `joined_across_renames`, tickstore_write_tests `quantiles` and
`refuse_the_real_store`). The classifier's reasons are in `fu-PS3.txt.md`; I checked its rules on samples of each kind
and the helpers checked the TESTF items of their files by hand (a few they found were not true (F) were closed).

## Defects fixed (each with a test that fails first)

1. **gdfl_dropbox staging lock** (`454a6797`): an `flock` belongs to the open file description, and the lock was
   released only by closing the `File`; a child started meanwhile (any spawn; `where_permission_binds` children
   fork with `.uid()`) held a copy until its exec, so the next press was `run_busy` naming a press that had ended.
   Probe: 103 of 300 second presses refused with another thread spawning; 0 of 300 after `StagingLock`'s `Drop`
   calls `File::unlock()`. It is what made `a_duplicate_refused_by_its_sha256_stays_refused_on_every_later_press`
   red once under load (D1: found and made deterministic, not re-run). Test:
   `a_staging_lock_is_released_while_a_copy_of_its_description_lives`.
2. **tickstore_write day lock** (`b82b4f02`): the same shape; 149 of 300 retakes refused `day_locked`, 0 of 300 after
   `DayLock`. Test: `the_day_lock_is_released_by_its_run_not_by_the_last_copy_of_its_file`.
3. **rolling expiry words** (`194b5150`): a roll past the calendar's last day (weekly `OrdinalOutsideWindow`,
   monthly `YearOutsideWindow`) read "the day is before this regime was verified from". Test:
   `an_expiry_past_the_last_measured_day_is_filed_unchecked`, red on the old code with that sentence.
4. **A test writing into the archive root** (`b82b4f02`): `a_refusal_under_a_200_on_the_post_and_discovery_paths_is_
   never_a_success` built its source from the shipped Dhan row, so its clean answers were kept under
   `$HOME/.brutex/vendor-data/captures` (here 80 files under `/root`, written by test runs on this host). It now uses
   an invented base URL and asserts `feed == None`.

## Production changes (D-7337 draft below)

Behaviour unchanged outside tests unless named a defect above:

- **Planted refusals, test builds only** (the `gdfl_import_filing::inject` / `FULL_AT` pattern): masters `Step` and
  `planted()` for the write, sync, rename and cleanup of a replacement (one function in both builds since
  `2d8d91f7`); gdfl_dropbox `at_step("metadata" | "sync")` with `PART_FAULT`.
- **Seams keeping the live value**: gdfl_dropbox `proxy_named_by`; ingest `CensusLock::opened`; zerodha_verify
  `lock_answer`, `note_left`; tickstore_write `item_by`, `preflight_under` (live `u32::MAX`), `encode_all_under`
  (live `DAY_BLOCKS_CAP`), `write_temp`'s index encoder (live `zstd`), `lock_held`, `Faults::refused_with` (default
  `StorageFull`), `columnar_under` (live `u32::MAX`; replaces a helper's test that built a 4 GiB member); http
  `built_with` (live `pooled_client`); gdfl_tickstore `walk_within` (live `TICKSTORE_WALK_CAP`); gdfl_import_journal
  `make_leaf_dir_with` (live `create_dir`); archive `entry_path`.
- **Arms no input reaches, removed or merged with a proof**: masters `prime_now` takes the prime its callers checked;
  masters `holds_exactly` answers a refused status or a short read through the "not held" arm and splits with
  `split_at_mut`; gdfl_dropbox `status` skips with `skip()`; ingest `rows_mode`'s repeated emptiness check and
  `months_in`'s two let-else arms; gap_backfill `month_lines` opens with `Option::insert`; session `whole_months`
  loops to its own return; gdfl_import_lineage `day()` keeps each group's symbol with its first file and asks
  `stitched()` alone.

## Gates

On `a5ac2140`, in a detached worktree at that commit, stable 1.97.1, the repo's profile (opt-level 3),
`CARGO_PROFILE_DEV_DEBUG=0`, `TMPDIR=/dev/shm/ps3-tmp`:

- `cargo fmt --all --check`: rc 0.
- `cargo clippy -p pull --all-targets --locked -- -D warnings`: rc 0, no diagnostic (50 s).
- `cargo test -p pull --locked --no-fail-fast`: 32 binaries (the lib, 30 integration test files, the doc tests),
  **3197 passed, 3 failed, 54 ignored** (208 s, the lib and integration binaries rebuilt). Failed:
  `alerts::tests::create_fresh_returns_a_refusal_that_is_not_already_exists_as_the_host_gave_it`,
  `ingest::census_store::link_tests::a_read_only_partition_directory_is_refused_with_the_host_s_words`,
  `ingest::census_store::link_tests::an_idempotent_requirement_opens_nothing_for_writing`.
- The same three commands on `2d8d91f7` (one test file before): fmt rc 0, clippy rc 0, tests 3197 / 3 / 54 (268 s).

The failures are the three root-only reds of G2's files that PS1 listed with their proof (each passes as uid 65534);
none is in a G3 file. G3's six root-only reds of PS1's list now pass as root (`d418a3ec`). The Route B test run at
the tip (instrumented, opt-level 0) gives the same 3197 / 3 / 54.

## Benches

None run: this work adds no runtime step. The seams pass their live value or the host's own answer, and the two lock
guards add one `unlock` per run. Gate 8 and every p99 claim are the Mac's.

## Extreme cases

Each production file tested has at least 8 extreme cases, 2 combinations and 1 out of the box, marked by test in the
helpers' briefs and commit messages (masters: one byte past the 256 MiB ceiling streamed undeclared, `[`-opening
bodies, a blank row, every replacement step refused, a refused write AND a refused cleanup, a sync refused AND a
cleanup finding nothing, an untouchable unchanged master, an unreadable folder after the rename; out of the box: a
number among index names, read back from a sink).

## Process notes (for the record)

- **The forbidden language's name, three slips, nothing run with it and no file holding it:** in one of my shell
  commands a stray no-op variable name contained it (17:10 IST); helper `th` typed it twice in shell commands (a
  version probe and a variable name); helper `rs` ran a version probe of it once by mistake. Helper `zv` used
  `perl -pi` as an editor for edits and hand changes (an interpreter outside the brief's tools; nothing of it is in
  any file). All are against the operator's rules and must not recur.
- Instrumented quick builds left `default_*.profraw` files in worktree roots (proc-macros run without
  `LLVM_PROFILE_FILE`); each was removed by whoever made it; none was committed. The kit's Route B sets the path for
  its build, so its figures are not touched.
- Helper `gl` saw `llvm-profdata merge` refuse a truncated profile a test child left, once, under a filtered quick run.
  No Route B merge here failed; if the Mac's does, that child is the suspect (a test process ended mid-write).

## NEEDS THE MAC

Route B whole:pull on its pin (and G3's macOS lines, owed there, never claimed here); the shadow GET re-check;
macOS rows; non-root 14-core tests; the 54 ignored real-data tests (L-BCP-3); the D1 landing. And, from this work:

- `ingest`'s post-read cap arm of `read_regular_capped` is covered here by a Linux-only test (procfs
  `/proc/self/status`, fstat length 0); on aarch64-apple-darwin it stays uncovered unless the Mac finds a file that
  reads past its fstat length (or adds a seam).
- Not run on macOS: the two `RUST_MIN_STACK` children (zerodha_verify `no_worker_that_cannot_start_loses_a_series_or_
  a_spool`, gdfl_run_start's `unspawnable` gate case) rely on libtest running the test on its main thread when
  `pthread_create` refuses an unmappable stack; two ingest FIFO tests open the FIFO read-write to release a stuck
  writer (never blocks on Linux; POSIX undefined); gdfl_dropbox's `a_config_its_folder_owner_does_not_own_is_a_
  named_wait` needs a file another uid owns with no group or world bit (`/etc/master.passwd` on macOS);
  `a_block_device_is_named_as_one` takes the first block device under `/dev`.
- Slow tests added: two busy-month waits in zerodha_verify (about 6 s and 4 s), the 256 MiB streamed body in masters
  (under a second here).

## Follow-ups for other owners (not changed here)

- G1/MS1 `upkeep.rs:907`: the run-status probe holds `try_lock_shared()` on a raw `File` until close, the same lock
  shape as the two fixed above (a forked child's copy can make a run's `Flock::try_lock` refuse spuriously).
  `store::flock::Flock` unlocks explicitly, so its users are fine.
- store: `store::repair::drop_month` (pub) records nothing in the quarantine, so a back-fill puts a dropped bar back
  as a new generation; only `store::quarantine::apply_month` sets it aside.
- G3 wording, left for a test of its own: zerodha_verify `kept_window` names an undecodable kept body "the vendor was
  not reached: the vendor's answer is not JSON" (the self-contradicting words http.rs strips on the live path);
  gdfl_dropbox `transport_words` names an empty host for a cut body; `Config::load` of a bare file name refuses with
  ": No such file or directory"; gdfl_lineage `LineageRecord::parse` accepts a zero lot that `LotRecord::parse`
  refuses; `--stored` does not dedup names already in `only`.

## Decision draft D-7337

**D-7337 (draft, BASE-COV-PULL-PS3): G3's coverage seams, dead arms, and two lock defects.**

*Context.* BASE-COV-PULL (operator decision D2 part A) asks 100% line and branch coverage of crate `pull`; the 3 Nov
cloud wave split it by file (D6, `SHARDS.md`), and G3 is PS3's. Some G3 refusals are reached only by a host fault
no portable test can make on demand (a write, sync or rename refused after the file opened; a part's status read or
final sync; a listing that fails part way; an fstat of an open handle); some arms no input can reach; and one test
went red only under load.

*Decision.* In G3's files only:

1. Planted host refusals in test builds only, after `gdfl_import_filing::inject` (IA-16) and `FULL_AT`:
   `masters::Step` / `planted()` (one function in both builds; nothing planted outside tests) in `replace_locked` and
   `remove_partial_after`; `gdfl_dropbox::at_step("metadata" | "sync")` with a `cfg(test)` `PART_FAULT`.
2. Seams that pass the live value or the host's own answer: `gdfl_dropbox::Endpoints::proxy_named_by` (tests cannot
   set environment variables: `set_var` is `unsafe`, and `unsafe_code` is denied); `ingest::CensusLock::opened`;
   `zerodha_verify::lock_answer`, `note_left`; `tickstore_write::{item_by, preflight_under, encode_all_under,
   lock_held, columnar_under}`, `write_temp`'s index encoder, `Faults::refused_with`; `http::HttpSource::built_with`;
   `gdfl_tickstore::DayTable::walk_within`; `gdfl_import_journal::make_leaf_dir_with`; `archive::entry_path`.
3. Arms no input reaches, removed or merged, each with its proof in the report: `masters::prime_now` takes the prime
   its callers checked; `masters::holds_exactly` answers a refused status or a short read through the "not held" arm
   and splits with `split_at_mut`; `gdfl_dropbox::status` skips with `skip()`; `ingest::rows_mode` and `months_in`;
   `gap_backfill::month_lines`; `session::whole_months`; `gdfl_import_lineage::day`.
4. Defects fixed at their cause, each with a test that fails first: `gdfl_dropbox::lock_staging` returns
   `StagingLock` and `tickstore_write::lock_root` returns `DayLock`, each of whose `Drop` calls `File::unlock()`: an
   `flock` belongs to the open file description, and a child started while a run held it kept a copy until its exec,
   so the next run was refused (`run_busy`, `day_locked`) by a run that had ended (probes: 103/300 and 149/300
   refused; 0/300 after). `rolling::step_refused`: a roll past the calendar's last day says so instead of "before
   the regime". A test that kept vendor captures under `$HOME/.brutex` now uses an invented base URL.
5. Test side: six permission tests run through `where_permission_binds` (D-0995); test helpers lose branches only a
   failing or differently scheduled run took, or are driven directly over their edges.

*Coverage.* Route B, G3's files: start `2e7a0b6d` lines 49,318/51,591, branches 3,865/4,792; tip `a5ac2140` lines
58,751/60,007, branches 4,289/4,792. (F) and (U) items in `fu-PS3.txt.md`, every uncovered item in
`gaps-PS3.txt.md` (reports/base-cloud-20261103 of `cloud/cov-pull-ps3-20261103`).

*Rejected.* `#[coverage(off)]`, excluded files, weakened or skipped tests; a `cfg(not(test))` copy of production
code (it is a region only the integration tests' build holds, and none drives it: measured on `cd0cf688`); a
portable full disk (none exists: `/dev/full` is Linux-only); a 4 GiB member to reach a `u32` bound.

*docs/04 rows (drafts).* (a) A master replacement refused at its write, sync or rename leaves no `.partial` and the
held master whole, and a refused cleanup is named after the failure it followed: `masters::land_tests::*`. (b) A
replacement whose folder sync is refused after the rename is `Landed::Uncertain`, never `Written`:
`masters::land_tests::a_folder_that_will_not_sync_after_the_rename_is_uncertain`. (c) A run's lock is released by the
run, whatever copies of its open file description live:
`gdfl_dropbox::cov_tests::a_staging_lock_is_released_while_a_copy_of_its_description_lives`,
`tickstore_write::tests::the_day_lock_is_released_by_its_run_not_by_the_last_copy_of_its_file`. (d) A rolling expiry
past the calendar's last day is refused saying so: `rolling::tests::an_expiry_past_the_last_measured_day_is_filed_
unchecked`.

*docs/06 (draft).* A line that runs only in a child process a test kills with SIGKILL is never counted by Route B
(no exit handler runs, and the profile is written at exit): `cash_session_cache_lock_tests.rs`
`a_killed_holder_releases_the_day`'s child branch, listed (U) with that proof; the operator may prefer UNMEASURED.

## Not done

- The 17 gaps above (16 in G3's test files, mine; `tenor.rs` blocked outside).
- Mutants: deferred (operator 30 Oct).
- docs/04, 05, 06 rows: drafts here, for PSA (my block is D-7337 only).

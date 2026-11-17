# JOB APP-HEART: cargo-mutants on the app's heart crates (store, pull)

Read this whole file once, then do STEP 0 to STEP 4 in order. Labels: MEASURED = a number from a named
run; EXACT = a count; ESTIMATE = a forecast. You know K and N from your first message or from READY.txt
(this box is shard K of N).

## What

- Repository SJParthi/brutex (public). Code under test: the app commit
  `69aa8ffeb29c7c59a1585b45510c8658df00c7a2`, the tip of `feat/db-fix` on origin (the app's landed head
  since 9 Nov 2026). The box fetches exactly this commit (depth 1) and never a newer one.
- Crates: `store` and `pull` only. Each mutant is tested against the tests of its own crate
  (cargo-mutants' default).
- This box: shard K of N of the store+pull mutant list, cut by cargo-mutants' own default (slice) sharding.
- Results: ONLY the new branch `cloud-results/app-heart-mutants-K-of-N` (an orphan branch holding results
  only). This file sits alone on branch `cloud/jobs-app-heart-20261116`.
- Tools: rustc 1.97.1 (the app's `rust-toolchain.toml`), cargo-mutants 27.1.0 (the version of the
  16 Nov cloud wave), cargo-nextest 0.9.145 (the app's CI pin). The scripts install all three: Rust from the
  run user's own toolchain when it gives 1.97.1, else with the pinned rustup-init 1.29.3, sha256 checked
  before it runs (METHOD 1; never `curl | sh`); the two tools with `cargo install --locked`.

## Rules

1. Code only. Never fetch market data or anything beyond this repository, the Rust toolchain from
   static.rust-lang.org (rustup-init 1.29.3 and Rust 1.97.1) and crates from crates.io (those the app's
   `Cargo.lock` names and those of the two tools). Never use, ask for or print a signed link, a token, a key
   or any other secret.
2. Results only. Push only to `cloud-results/app-heart-mutants-K-of-N`. Never push code, never touch
   another branch, never force-push, never rewrite history. A rejected push stops the box (STEP 4).
3. Everything runs in the foreground. The only command of the work is `drive.sh` (STEP 2): one call
   lasts at most about 8.5 minutes, does one step of the work and leaves nothing running behind it.
   Call it again at once each time it returns, so your turn stays busy until FINAL or STOP (an idle box
   is paused and loses its work).
4. On any failure, stop and report (STEP 4). Never retry hoping for green, never edit the app's code or
   these scripts, never add a skip or an exclude beyond STEP 3's rule.
5. The scripts delete, stop or restore only these, all inside the run user's own folder `~/ah`: the step
   lock folder; the build output of the two tool installs once both are installed; the pinned rustup-init
   the setup downloads into `~/ah/tmp`, once it has run; test scratch in
   `~/ah/tmp` untouched for 10 minutes (failing tests leave it behind, and it filled earlier boxes'
   disks); on Linux, processes of the run user that a cut step left working inside `~/ah`; and any file
   of `crates/store` or `crates/pull` in the export `~/ah/work` that differs from the commit (a cut step
   can leave a mutated file), copied back from the commit. Nothing else.
6. Use shell, git, awk, Rust and cargo only. No other scripting language (never the six-letter one that
   starts with "Py").

## STEP 0: the box

Run once: `id -u; nproc; free -m; cat /sys/fs/cgroup/memory.max; df -h ~; git --version;
curl --version | head -n 1; command -v awk sha256sum timeout diff`. When `id -u` prints 0, also
`command -v useradd runuser su getent`. If one of git, curl, awk, sha256sum, timeout or diff is missing,
install it with the system package manager and say so in NOTES.md. Do not install Rust yourself: the
first steps do it (METHOD 1: the run user's own Rust when it gives 1.97.1, else the pinned rustup-init,
never `curl | sh`).

## STEP 1: the scripts

```sh
git clone -q --depth 1 --single-branch --branch cloud/jobs-app-heart-20261116 https://github.com/SJParthi/brutex.git ~/ahjob
cd ~/ahjob && git rev-parse HEAD && mkdir -p bin
awk 'BEGIN { f = ""; w = 0 } /^<!-- FILE [A-Za-z0-9._-]+ -->$/ { f = $3; next } w == 0 && f != "" && /^```/ { w = 1; next } w == 1 && /^```$/ { w = 0; f = ""; next } w == 1 { print > ("bin/" f) }' JOB-APP-HEART.md
cd bin && sha256sum -c SHA256SUMS
```

`git rev-parse HEAD` must print the job commit your first message or READY.txt names, and every file
must print OK; otherwise stop (STEP 4).

## STEP 2: the loop

Run `bash ~/ahjob/bin/drive.sh K N` (your K and N) in the foreground with the Bash tool's timeout set to
600000 ms. It returns within about 8.5 minutes and prints ONE line:

- `CONTINUE ...`: run the same command again at once, with nothing in between.
- `FINAL ...`: go to STEP 4.
- `STOP ...`: go to STEP 3.

If a call is cut by the tool's timeout, run it again: the next step cleans up after it. What one call
does: on the first call it makes the folders and the result repository (a clone of the result branch
when one exists, see RESUME); every call it sweeps old test scratch, checks the disk (stop below 4 GiB
free), runs ONE step of the work as the run user (`ah-step.sh`, at most 480 s), then commits and pushes
the results (so progress reaches the branch every 9 minutes or less) and prints the line. The steps
go: setup (Rust 1.97.1 as METHOD 1 says, then `cargo install` of cargo-mutants and cargo-nextest, which
resume across steps), the app commit and an export of it, the list, the baseline, then chunks of mutants.
The same installs took 4 min 49 s on a root cloud box (MEASURED 16 Nov by a box that ran the first version
of this job, commit c9aa530e); with the baseline, about 15-45 minutes of steps (ESTIMATE) before the first
mutant.

When the session's user is root, the work runs as a new ordinary user `mut`. Root reads every file, so
permission bits do not bind: the app's permission tests re-run themselves as another user when started by
root, and an earlier cloud pass saw 3 store tests fail as root. As an ordinary user they run as on the
app's CI and the operator's machine (the app's 17 Oct cloud pass of `pull` did the same). The run user's
folder is `~mut/ah` when root, else `~/ah`; the result repository is `~/ahres-K-of-N` of the session's
user. `mut` works in a login shell (`runuser -l`) without the session's proxy variables and has no Rust of
its own, so its setup takes the pinned rustup-init (METHOD 1); on a Default (limited) cloud network that
shell reached static.rust-lang.org, crates.io and github.com, while `sh.rustup.rs` is refused (MEASURED 16 Nov,
commit c9aa530e).

## STEP 3: STOP

Read the reason on the STOP line (also `<run user folder>/state/STOP.txt`).

- `BASELINE-RED`: an unmutated suite (store or pull) failed on this box. Read
  `state/baseline-failed-<crate>.txt` and, for each failing test, its output in
  `state/baseline-<crate>.log`. Only when EVERY failing test fails because it needs market data, the
  operator's live store or a Mac-only file or path (its own message names such a file, folder or
  `BRUTEX_*` variable): write their full test names (as nextest prints them after the binary name, e.g.
  `module::tests::name`), one per line, into `state/SKIPS.txt` (owned by the run user), rename
  `state/STOP.txt` to `state/STOP.handled-<UTC time>.txt`, and go back to STEP 2: that suite runs again
  with those names skipped, and every chunk skips them too. Any other failure, or skips that do not make
  the suite green: STEP 4.
- Anything else: STEP 4.

If CONTINUE lines keep saying `commit failed (git identity?)`, set the git name and e-mail your session
uses for its own commits, then keep calling STEP 2.

## STEP 4: finish

1. Write `NOTES.md` in `~/ahres-K-of-N/results/app-heart-mutants/shard-K-of-N/` (at most 80 lines):
   the SUMMARY line; the box (cores, RAM, cgroup limit, disk, run user); the versions
   (`logs/versions-*.txt`); the baseline (per crate: passed, failed, skipped, wall: MEASURED, from
   `logs/baseline-summary-*` and `logs/steps-*.log`); SKIPS (none, or each name with its reason); every
   STOP reason; anything unexpected; on a STOP, the last 20 lines of `state/steps.log`.
2. Run `bash ~/ahjob/bin/drive.sh K N push` (prints `PUSHED ...`; if it says push failed, wait 60 s and
   run it again, at most 5 times).
3. FINAL MESSAGE, short: the SUMMARY line; COMPLETE, or the STOP reason with the last 20 lines of
   `state/steps.log`; the 10 files with the most missed mutants
   (`cut -d: -f1 missed.txt | sort | uniq -c | sort -rn | head`); the result branch and its last commit.

## METHOD

The run user's environment for every cargo command: `TMPDIR=~/ah/tmp`, `CARGO_TERM_COLOR=never`, and the
dev profile overridden by environment to `CARGO_PROFILE_DEV_OPT_LEVEL=0`,
`CARGO_PROFILE_DEV_INCREMENTAL=true`, `CARGO_PROFILE_DEV_DEBUG=0`. Every long command runs under
`timeout --signal=INT` with the step's time left, so a cut command stops cleanly and the next step goes on.

1. Setup: Rust 1.97.1 (`--profile minimal`, components rustfmt, clippy, llvm-tools-preview, target
   wasm32-unknown-unknown, as `rust-toolchain.toml` lists), from the first source that applies:
   (a) the run user's own rustup, version 1.26 or newer: `rustup toolchain install 1.97.1 --profile minimal
   -c rustfmt -c clippy -c llvm-tools-preview -t wasm32-unknown-unknown` (it installs exactly 1.97.1);
   (b) no rustup at all, but a `rustc` and `cargo` that already print 1.97.1: used as they are (no
   components; the tests need none); (c) otherwise rustup-init 1.29.3, pinned: the file
   `https://static.rust-lang.org/rustup/archive/1.29.3/<target>/rustup-init` (target x86_64 or aarch64
   unknown-linux-gnu; any other platform STOPs), downloaded as a file (never `curl | sh`: a Default cloud
   network refuses `sh.rustup.rs`, and a pipe hides a failed download), checked against the sha256 pinned in
   `ah-common.sh` (the sums static.rust-lang.org publishes beside the file) and only then run as
   `rustup-init -y --profile minimal --default-toolchain 1.97.1`, then (a)'s toolchain install. The source
   used is written to `logs/versions-*.txt`, and `rustc` in the export must print 1.97.1. Then
   `cargo install cargo-mutants --version 27.1.0
   --locked` and `cargo install cargo-nextest --version 0.9.145 --locked`, both with a kept target folder
   so a cut build resumes; `git fetch --depth 1` of the app commit, a check that HEAD is that commit and
   the tree is clean; then `~/ah/work`, an export of the commit without `.git` (what a cargo-mutants copy
   looks like: the app's history tests skip their history checks there, as written for cargo-mutants).
2. List, in the export: `cargo mutants --list -p store -p pull --cargo-arg=--locked` (whole list) and the
   same with `--shard K/N`; the shard must be exactly lines K*c+1 .. (K+1)*c of the whole list
   (c = ceil(total/N)), hold no repeated name, and equal `SHARD-LIST.txt` already on the result branch,
   if any.
3. Baseline, once per box, in the export: `cargo nextest run --locked -p <crate> --no-run`, then
   `cargo nextest run --locked -p <crate> --no-fail-fast`, for store and then pull. Each suite's wall is
   MEASURED and sets that crate's test timeout: 3 times the wall, at least 60 s, at most what one step
   leaves after start-up and a rebuild (pull 310 s, store 370 s). A missed mutant runs the whole suite, so
   if that cap is under 1.5 times the suite's wall the box stops (STEP 4) instead of filing missed
   mutants as timeouts: the pull suite must take at most about 206 s on the box, the store suite 246 s.
4. Chunks: the next names of the shard (in order) that have no outcome yet, all of one crate, as many
   as the step's time left fits at the MEASURED rate (1 to 20); one exact anchored regex per name; then

   ```sh
   cargo mutants --in-place --baseline=skip --timeout <crate timeout> -p <crate> --cargo-arg=--locked \
     --test-tool nextest --cargo-test-arg=--max-fail=1:immediate -o <chunk folder> --re '^<name>$' ...
   ```

   in the export, whose `target` stays warm between steps. Before every chunk, any file of the two
   crates that differs from the commit is copied back from it. Exit 0, 2 or 3 is a finished chunk and
   its four outcome lists must hold exactly its names; a chunk cut at the step's end (exit 124) keeps
   the outcomes it finished (cargo-mutants records a mutant only once its test is done and puts the
   source back on an interrupt), and its other names go to the next chunk. Every outcome must be one of
   the chunk's names, and no name may get two outcomes. Any other exit stops the box. A name that is the
   first of a chunk cut with no outcome three times stops the box.

Changes from the 16 Nov wave (MUT-K: `cargo mutants --workspace --shard K/30 --jobs 2 --cargo-arg=--locked
-- -- --exact --skip ...`, cargo test, automatic timeout, results on the code commit), and why:

1. `-p store -p pull` instead of `--workspace`: this job's scope.
2. Foreground steps of at most 8.5 minutes instead of one long background run per shard: the wave-2
   boxes must never leave work in a background job and run long commands in the foreground in pieces of
   at most 9 minutes. So cargo-mutants runs in chunks, `--in-place` in an export without `.git` with a
   warm `target`, `--baseline=skip` (the box's own baseline of each suite stands for the unmutated tree,
   and the export is checked against the commit before every chunk), one job (`--in-place` cannot take
   `--jobs`; one mutant at a time uses the 4 cores for its build and tests). The shard is still
   cargo-mutants' own `--shard K/N` slice, and a box cut by the account's usage cap or a paused VM
   resumes from the result branch without repeating finished mutants.
3. cargo-nextest 0.9.145 with `--max-fail=1:immediate`, as the app's own CI gate 18 runs cargo-mutants:
   the first failing test ends the run (about half the time of cargo test per mutant, ESTIMATE), and the
   app's `.config/nextest.toml` orders its watchdog tests first so that hangs are caught instead of
   timing out. nextest runs no doc tests (as gate 18).
4. Timeouts: per crate, 3 times its MEASURED suite wall, at least 60 s and at most what one step allows
   (pull 310 s, store 370 s), instead of cargo-mutants' default 5 times the baseline (about 20 minutes a
   hang on pull, ESTIMATE); gate 18 uses 2 with a 900 s minimum. A timeout is never counted as caught.
5. Dev profile: opt-level 0, incremental, no debuginfo (the app's dev profile is opt-level 3 and not
   incremental, too slow to rebuild per mutant; the app's 16-17 Oct cloud passes did the same; outcomes do
   not depend on it, timings do).
6. No `--cap-lints`: gate 18 sets it, but in these two crates no rustc lint is above warn except
   `forbid(unsafe_code)`, which no mutant can trigger, and leaving it off keeps the warm `target` valid.
7. An ordinary run user when the box is root (STEP 2).
8. Results on an orphan branch (results only) under `results/app-heart-mutants/shard-K-of-N/`.
9. cargo-mutants pinned with `--version 27.1.0` (the wave installed the latest, which was 27.1.0).
10. Rust from the run user's own toolchain or the pinned rustup-init 1.29.3 with its sha256 checked, instead
    of the `sh.rustup.rs` script piped into `sh`: the wave's boxes worked as root, which had rustup already; this
    job's run user `mut` has none, and a Default cloud network refuses `sh.rustup.rs` (MEASURED 16 Nov,
    commit c9aa530e: every root box stopped at setup). rustup-init 1.29.3 is the file that c9aa530e's
    MEASURED run downloaded as the latest (static.rust-lang.org has served 1.29.3 as the latest since
    3 Nov 2026: the same sha256 and the same ETag as the archived 1.29.3 file).

Kept: cargo-mutants 27.1.0, rustc 1.97.1, `--cargo-arg=--locked`, each mutant tested against its own
crate's tests, cargo-mutants' four outcome lists and outcomes.json, progress commits and NOTES.md.

## RESULT LAYOUT

On branch `cloud-results/app-heart-mutants-K-of-N`, folder `results/app-heart-mutants/shard-K-of-N/`:

- `SUMMARY.txt`: one line, `SUMMARY app-heart shard K/N commit <app commit> cargo-mutants 27.1.0 nextest
  0.9.145 mutants <shard size> tested <n> caught <c> missed <m> timeout <t> unviable <u> chunk_wall_s <s>
  (MEASURED) boxes <b> status COMPLETE|PARTIAL|STOPPED`.
- `PROGRESS.txt`: counts, rate (MEASURED from the chunk walls), ETA (ESTIMATE), the last step.
- `caught.txt`, `missed.txt`, `timeout.txt`, `unviable.txt`: every finished mutant, chunk by chunk in
  shard order, cargo-mutants' own lines (`file:line:col: change`).
- `outcomes.json`: `{"job","shard","n","commit","cargo_mutants","cargo_nextest","chunks":[{"chunk",
  "outcomes": <cargo-mutants' outcomes.json of that chunk>}]}`.
- `SHARD-LIST.txt`: the shard's names in order. `chunks/c-NNNN/`: names.txt, STAT (crate, names, outcomes,
  counts, exit code, test timeout, start, end, wall), the four lists, outcomes.json, console.log (tail).
  `boxes.txt`: one line per box. `logs/`: steps, versions, setup and baseline tails, baseline summaries,
  STOP, per box. `SKIPS.txt` only after STEP 3. `NOTES.md` from STEP 4.

## TESTS THAT NEED DATA OR THE LIVE STORE

A read of every `.rs` file of `crates/store` and `crates/pull` at the app commit (before this job was
written): every test that reads market data, the operator's live store or a Mac path through a
`BRUTEX_*` variable or a fixed path is `#[ignore]`d, with the timing measurements: 62 `#[ignore]`
attributes. nextest never runs them unless asked (`--run-ignored`), so they need no skip. No other test
of the two crates reads such a variable or path. Expected SKIPS: none; the box's baseline is the proof
(STEP 3 if not). The 62, by file:

- `crates/pull/src/alerts_tests.rs`: `the_operators_older_steps_timed`
- `crates/pull/src/calendar.rs`: `calendar_of_record_costs`
- `crates/pull/src/cash_auction.rs`: `actual_dated_masters_match_every_current_fno_cash_identity`
- `crates/pull/src/cash_session_cache.rs`: `held_masters_give_every_cas_session_its_auction_band`, `actual_receipted_lifecycle_snapshot_never_claims_complete_history`, `official_25_dated_masters_install_and_validate_without_network`
- `crates/pull/src/census_store_kept_tests.rs`: `the_kept_publish_is_flat_measured`, `the_kept_publish_on_a_copy_of_a_real_epoch_measured`
- `crates/pull/src/census_store_tests.rs`: `legacy_drift_cost_per_call_is_measured`
- `crates/pull/src/disk_projection_tests.rs`: `real_delivery_sizes_project_through_the_module_and_meet_the_floor`
- `crates/pull/src/fnowork_futures_tests.rs`: `a_real_size_plan_does_not_grow_with_the_snapshots_held`
- `crates/pull/src/gdfl_archive.rs`: `the_real_containers_open_with_no_prefix_and_print_their_facts`
- `crates/pull/src/gdfl_cm_admission_tests.rs`: `the_match_band_costs_one_pass_a_file`
- `crates/pull/src/gdfl_dayzips_tests.rs`: `day_table_sizing_timing`
- `crates/pull/src/gdfl_dropbox_tests.rs`: `measure_the_journal_costs_of_a_press`
- `crates/pull/src/gdfl_ia3_tests.rs`: `ia3_lookup_costs`, `real_days_close_with_the_same_listing_from_two_sources`
- `crates/pull/src/gdfl_import_ia2_tests.rs`: `journal_append_cost_is_flat_with_a_second_writer`, `ia2_repair2_timings`
- `crates/pull/src/gdfl_lineage_tests.rs`: `the_unchanged_lot_arm_costs_one_pass_on_an_ordinary_day`, `the_real_names_are_one_key_per_contract_across_renames_and_redatings`
- `crates/pull/src/gdfl_nfo_tests.rs`: `the_real_archive_settles_every_name_and_refuses_only_the_impossible_strikes`, `the_real_names_decode_to_the_stored_contracts_less_the_refused_and_the_resettled`, `the_real_ex_dates_are_detected_from_the_data_and_ordinary_days_are_not`, `the_decoders_costs_on_real_days_are_measured`, `the_real_unchanged_lot_events_are_detected_and_ordinary_days_are_not`, `the_real_window_sweep_lists_every_event_the_detector_declares`, `the_fixed_array_decodes_real_days_as_the_field_vector_did`, `the_measured_bad_option_files_are_read_row_by_row`, `no_real_option_file_of_2018_ends_in_a_bare_cr`
- `crates/pull/src/gdfl_rekey_tests.rs`: `plan_cost_per_key_is_flat_in_the_number_of_keys`, `census_step_cost_is_one_load_per_plan_flat_per_entry`, `apply_guard_cost_per_operation`
- `crates/pull/src/gdfl_run_ia2_tests.rs`: `ia2_timings`, `ia2_repair_timings`
- `crates/pull/src/gdfl_run_tests.rs`: `source_cache_hit_timing`, `journal_load_and_refresh_costs`, `progress_and_failure_costs`
- `crates/pull/src/gdfl_v3_tests.rs`: `the_real_v3_ledger_agrees_with_the_real_year_zips`
- `crates/pull/src/gdfl_wp12e_tests.rs`: `the_ledger_load_and_a_resumed_day_are_timed`, `the_split_probe_is_timed`
- `crates/pull/src/landing_run_tests.rs`: `ia10c_timings`
- `crates/pull/src/master_snapshots_tests.rs`: `measure_the_hook_with_incomplete_entries_held`, `a_real_size_dump_is_measured`, `measure_the_catalog_listing_and_its_probes`
- `crates/pull/src/reference_data_tests.rs`: `replay_a_real_rate_table_under_the_gap_rule`
- `crates/pull/src/tickstore_write_tests.rs`: `measure_real_days_against_the_tools_files`, `measure_ruzstd_fastest_on_bar_months`
- `crates/pull/tests/census_v4.rs`: `the_audit_scale_census_migrates_and_compacts_with_equal_totals`
- `crates/pull/tests/store_evidence.rs`: `banknifty_2024_03_02_derives_from_its_stored_minutes`, `nifty_2026_09_derives_every_session`, `the_gap_ledger_names_each_short_day_of_a_stored_month`, `an_illiquid_share_folds_its_no_trade_minutes_into_partial_buckets`, `the_derive_fold_costs_per_month`
- `crates/store/src/file.rs`: `the_kernels_own_enospc_on_a_full_volume_is_disk_full_by_name`
- `crates/store/src/file_sealed_tests.rs`: `read_rows_at_timing`
- `crates/store/tests/sealed_codec_samples.rs`: `the_sample_months_encode_to_the_prototype_sizes_less_the_first_row_saving`
- `crates/store/tests/sealed_door.rs`: `seal_a_private_store_root_for_the_shadow_check`
- `crates/store/tests/sealed_samples.rs`: `real_months_seal_read_back_bit_for_bit_and_answer_like_the_raw_form`
- `crates/store/tests/sealed_write_door.rs`: `the_write_door_over_real_months_sealed_without_one_day`
- `crates/store/tests/tix_latency.rs`: `time_and_row_lookup_latency`, `small_month_rule_latency`

## EXPECTED WALL (ESTIMATE)

- Size: cargo-mutants 26.2.0 `--list` on the operator's machine at the same commit (no build): store+pull
  21,764 mutants (pull 16,343, store 5,421), so a shard of 2,177 at N=10 (27.1.0 may list a few more):
  shards 0-6 pull only, shard 7 pull 1,104 + store 1,073, shards 8-9 store only.
- Rate: from the app's earlier cloud passes (MEASURED there, on older commits): the pull suite took 62 s to
  build and 203-255 s to test (nextest, 2 threads, opt-level 0); 354 store mutants took 58 min. ESTIMATE
  per mutant, one at a time, step overheads included: pull 1.5-3 min, store 15-45 s.
- Wall per box, uncut: pull shards (0-6) about 55-110 h; shard 7 about 35-65 h; store shards (8, 9)
  about 10-30 h; plus 30-45 minutes of setup and baseline per box. Boxes stop earlier when the
  account's usage runs out; they resume (RESUME). More boxes for pull (a larger N) shorten this.
- `PROGRESS.txt` carries the MEASURED rate and the ETA from the first chunk on.

## RESUME

A box that ends early (usage cap, paused or reclaimed VM, account switch) loses at most the mutant it was
testing. To continue, start a fresh box with the same first message (same K and N). `drive.sh` finds the
result branch, takes every finished mutant from it (`chunks/c-NNNN/`), checks that its new shard list
equals the committed `SHARD-LIST.txt`, runs setup and the baseline again on the new box, and goes on with
the first name that has no outcome. Never run two boxes for the same K and N at once: the second push is
rejected and that box stops. A result branch that holds the first version's layout (`batches/`, job commit
8737b862) is never resumed or written: the box STOPs before it runs or pushes anything.

## FILES (STEP 1 extracts these into ~/ahjob/bin)

<!-- FILE ah-common.sh -->
```sh
# ah-common.sh: the settings every app-heart script shares. Sourced, never run on its own.
AH_C=69aa8ffeb29c7c59a1585b45510c8658df00c7a2   # the app commit tested (origin feat/db-fix)
AH_RUST=1.97.1                                  # the app's rust-toolchain.toml
AH_CMV=27.1.0                                   # cargo-mutants (the version of the 16 Nov wave)
AH_NXV=0.9.145                                  # cargo-nextest (the app's CI pin)
# rustup-init, pinned: used only when the run user has no rust $AH_RUST of its own (ah-step.sh, setup). The file
# https://static.rust-lang.org/rustup/archive/$AH_RUSTUP/<target>/rustup-init, checked against these sha256 sums
# (as published beside it in <...>/rustup-init.sha256) before it runs. Never a download piped into a shell.
AH_RUSTUP=1.29.3
AH_RUSTUP_SHA256_X86_64=b1c8cb14e30889e7e2999e6b69e62d9e9b9ffc0f4df8180564d306d9680e6c55    # x86_64-unknown-linux-gnu, 21347088 bytes
AH_RUSTUP_SHA256_AARCH64=9f608c142e0e38a78957d0661ba1dfaf0b6fb37184cfaa4b9a0aa57af6184282   # aarch64-unknown-linux-gnu, 20172120 bytes
AH_B=50                                         # mutants per batch
AH_REPO=${AH_REPO:-https://github.com/SJParthi/brutex.git}          # code (read only)
AH_RES_REPO=${AH_RES_REPO:-https://github.com/SJParthi/brutex.git}  # where results are pushed
AH_MINFREE_KIB=4194304                          # stop below 4 GiB free
export LC_ALL=C
ah_now() { date -u +%Y-%m-%dT%H:%M:%SZ; }
ah_lines() { if [ -f "$1" ]; then wc -l < "$1" | tr -d ' '; else echo 0; fi; }
```

<!-- FILE ah-esc.awk -->
```awk
# ah-esc.awk: one exact, anchored regex per mutant name, escaping the characters
# Rust's regex::escape escapes. < and > stay as they are (escaped they would be
# word boundaries). Input: names as `cargo mutants --list` prints them.
{
  s = $0
  out = ""
  for (i = 1; i <= length(s); i++) {
    c = substr(s, i, 1)
    if (index("\\.+*?()|[]{}^$#&-~", c) > 0) out = out "\\" c
    else out = out c
  }
  print "^" out "$"
}
```

<!-- FILE ah-step.sh -->
```sh
#!/bin/bash
# ah-step.sh K N BUDGET: ONE foreground step of the app-heart work, run as an ordinary (non-root) user,
# never longer than BUDGET seconds. Every part resumes where the last step stopped: setup (toolchain,
# cargo-mutants, cargo-nextest, the app commit, an export of it without .git), the list, the baseline
# (the unmutated store and pull suites), then chunks of mutants that cargo-mutants runs in place in the
# export. Exit 0 = continue with another step, 3 = STOP (STOP.txt says why), 10 = every mutant of the
# shard has its outcome.
set -u
K=$1; N=$2; BUDGET=$3
D=$(cd "$(dirname "$0")" && pwd)
. "$D/ah-common.sh"
AH=$HOME/ah; S=$AH/state; A=$AH/app; W=$AH/work
cd "$HOME" || exit 1   # never work from inside $AH: the leftover check below looks there
mkdir -p "$S/chunks" "$AH/tmp"
END=$(( $(date +%s) + BUDGET ))
left() { echo $(( END - $(date +%s) )); }
say() { echo "$(ah_now) $*" >> "$S/steps.log"; }
stop() { say "STOP $*"; echo "$(ah_now) $*" > "$S/STOP.txt"; exit 3; }
[ -e "$S/STOP.txt" ] && exit 3
[ -e "$S/ALL.done" ] && exit 10

# One step at a time: a lock folder holding the holder's pid.
if ! mkdir "$S/step.lock" 2>/dev/null; then
  op=$(cat "$S/step.lock/pid" 2>/dev/null || echo 0)
  if [ "$op" != 0 ] && kill -0 "$op" 2>/dev/null; then say "step: step $op still runs"; exit 0; fi
  rm -rf "$S/step.lock"
  mkdir "$S/step.lock" 2>/dev/null || exit 0
fi
echo $$ > "$S/step.lock/pid"
trap 'rm -rf "$S/step.lock"' EXIT

export CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}" RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}"
export PATH="$CARGO_HOME/bin:$PATH" TMPDIR="$AH/tmp" CARGO_TERM_COLOR=never
unset CARGO_TARGET_DIR CARGO_BUILD_TARGET_DIR RUSTFLAGS RUSTDOCFLAGS CARGO_ENCODED_RUSTFLAGS RUSTC RUSTDOC RUSTC_WRAPPER CARGO_BUILD_RUSTC_WRAPPER CARGO_BUILD_JOBS
# Rebuild time per mutant (the app's own cloud passes of 16-17 Oct did the same; outcomes do not depend on it):
export CARGO_PROFILE_DEV_OPT_LEVEL=0 CARGO_PROFILE_DEV_INCREMENTAL=true CARGO_PROFILE_DEV_DEBUG=0
DRY=${AH_DRY:-0}
command -v timeout >/dev/null 2>&1 || stop "setup: no timeout command (coreutils)"
# run CMD...: the command with SIGINT when the step's time is up (exit 124 then), SIGKILL 60 s later
run() {
  local t=$(( $(left) - 20 ))
  [ "$t" -lt 30 ] && return 124
  timeout --signal=INT --kill-after=60 "$t" "$@"
}

# Leftovers of a cut step (Linux only, never in a dry run): this user's processes whose working folder or
# command line lies inside $AH, never anything else.
if [ "$DRY" != 1 ] && [ -d /proc/self ]; then
  old=""
  for p in /proc/[0-9]*; do
    [ -O "$p" ] || continue
    q=${p#/proc/}
    [ "$q" = "$$" ] && continue
    c=$(tr '\0' ' ' < "$p/cmdline" 2>/dev/null)
    w=$(readlink "$p/cwd" 2>/dev/null)
    case "$c" in *ah-step.sh*|*drive.sh*|"") continue;; esac
    case "$w/ $c" in *"$AH/"*) old="$old $q";; esac
  done
  if [ -n "$old" ]; then
    say "cleanup: stopping leftover processes inside $AH:$old"
    kill $old 2>/dev/null; sleep 10; kill -9 $old 2>/dev/null
  fi
fi

# 1. SETUP
# Rust $AH_RUST comes from the run user's own toolchain when that is enough: a rustup of 1.26 or newer (it
# installs exactly $AH_RUST below), or, with no rustup at all, a rustc and cargo that already are $AH_RUST.
# Otherwise from rustup-init $AH_RUSTUP, pinned in ah-common.sh: downloaded as a file from static.rust-lang.org
# (never piped into a shell), checked against its pinned sha256, and only then run, with --default-toolchain $AH_RUST.
rustup_ok() {   # a rustup of 1.26 or newer on PATH
  command -v rustup >/dev/null 2>&1 || return 1
  rustup --version 2>/dev/null | head -n 1 | awk '{ split($2, v, "."); ok = ($1 == "rustup" && (v[1] + 0 > 1 || (v[1] + 0 == 1 && v[2] + 0 >= 26))) } END { exit !ok }'
}
if [ ! -e "$S/setup.done" ]; then
  if [ "$DRY" != 1 ]; then
    if rustup_ok; then
      [ -s "$S/rust-source.txt" ] || echo "rust: the run user's own rustup ($(command -v rustup), $(rustup --version 2>/dev/null | head -n 1))" > "$S/rust-source.txt"
    elif ! command -v rustup >/dev/null 2>&1 && [ "$(rustc --version 2>/dev/null | cut -d ' ' -f 1-2)" = "rustc $AH_RUST" ] \
        && [ "$(cargo --version 2>/dev/null | cut -d ' ' -f 1-2)" = "cargo $AH_RUST" ]; then
      [ -s "$S/rust-source.txt" ] || echo "rust: the run user's own rust $AH_RUST without rustup ($(command -v rustc), $(command -v cargo))" > "$S/rust-source.txt"
    else
      case "$(uname -sm)" in
        "Linux x86_64") RT=x86_64-unknown-linux-gnu; RS=$AH_RUSTUP_SHA256_X86_64;;
        "Linux aarch64") RT=aarch64-unknown-linux-gnu; RS=$AH_RUSTUP_SHA256_AARCH64;;
        *) stop "setup: no rust $AH_RUST for $(id -un) and no pinned rustup-init for $(uname -sm)";;
      esac
      RIU=https://static.rust-lang.org/rustup/archive/$AH_RUSTUP/$RT/rustup-init
      RI=$AH/tmp/rustup-init-$AH_RUSTUP
      say "setup: no rust $AH_RUST for $(id -un): rustup-init $AH_RUSTUP from $RIU"
      run curl --proto '=https' --tlsv1.2 --retry 3 -sSfL -o "$RI" "$RIU" >> "$S/setup.log" 2>&1
      rc=$?; [ "$rc" = 124 ] && exit 0; [ "$rc" = 0 ] || stop "setup: cannot download $RIU (curl rc $rc, setup.log)"
      got=$(sha256sum "$RI" 2>> "$S/setup.log" | awk '{print $1}')
      [ "$got" = "$RS" ] || stop "setup: the downloaded rustup-init has sha256 '$got', not the pinned $RS"
      echo "rust: rustup-init $AH_RUSTUP ($RT, sha256 $RS checked) from $RIU" > "$S/rust-source.txt"
      chmod u+x "$RI" || stop "setup: cannot make $RI executable"
      run "$RI" -y --profile minimal --default-toolchain "$AH_RUST" >> "$S/setup.log" 2>&1
      rc=$?; [ "$rc" = 124 ] && exit 0; [ "$rc" = 0 ] || stop "setup: rustup-init $AH_RUSTUP failed (rc $rc, setup.log)"
      rm -f "$RI"
      rustup_ok || stop "setup: rustup-init $AH_RUSTUP ran, but no rustup 1.26 or newer is on PATH ($CARGO_HOME/bin)"
    fi
    TC=""; rustup_ok && TC="+$AH_RUST"   # cargo +$AH_RUST only through rustup
    if [ -n "$TC" ] && [ ! -e "$S/toolchain.done" ]; then
      run rustup toolchain install "$AH_RUST" --profile minimal -c rustfmt -c clippy -c llvm-tools-preview -t wasm32-unknown-unknown >> "$S/setup.log" 2>&1
      rc=$?; [ "$rc" = 124 ] && exit 0; [ "$rc" = 0 ] || stop "setup: rustup toolchain install $AH_RUST failed (setup.log)"
      touch "$S/toolchain.done"
    fi
    if [ "$(cargo ${TC:+"$TC"} mutants --version 2>/dev/null)" != "cargo-mutants $AH_CMV" ]; then
      say "setup: cargo install cargo-mutants $AH_CMV (resumes across steps)"
      run cargo ${TC:+"$TC"} install cargo-mutants --version "$AH_CMV" --locked --force --target-dir "$AH/install-target" >> "$S/setup.log" 2>&1
      rc=$?; [ "$rc" = 124 ] && exit 0; [ "$rc" = 0 ] || stop "setup: cargo install cargo-mutants $AH_CMV failed (setup.log)"
    fi
    case "$(cargo ${TC:+"$TC"} nextest --version 2>/dev/null)" in
      "cargo-nextest $AH_NXV"*) ;;
      *)
        say "setup: cargo install cargo-nextest $AH_NXV (resumes across steps)"
        run cargo ${TC:+"$TC"} install cargo-nextest --version "$AH_NXV" --locked --force --target-dir "$AH/install-target" >> "$S/setup.log" 2>&1
        rc=$?; [ "$rc" = 124 ] && exit 0; [ "$rc" = 0 ] || stop "setup: cargo install cargo-nextest $AH_NXV failed (setup.log)";;
    esac
    if [ -e "$AH/install-target" ]; then rm -rf "$AH/install-target"; fi   # the two tools' own build output
  fi
  if [ ! -d "$A/.git" ]; then
    mkdir -p "$A" && git -C "$A" init -q && git -C "$A" remote add origin "$AH_REPO" || stop "setup: git init of the app folder failed"
  fi
  if [ "$(git -C "$A" rev-parse -q --verify HEAD 2>/dev/null)" != "$AH_C" ]; then
    run git -C "$A" fetch -q --depth 1 origin "$AH_C" >> "$S/setup.log" 2>&1
    rc=$?; [ "$rc" = 124 ] && exit 0; [ "$rc" = 0 ] || stop "setup: cannot fetch the app commit $AH_C (setup.log)"
    git -C "$A" checkout -q --detach "$AH_C" >> "$S/setup.log" 2>&1 || stop "setup: cannot check out $AH_C"
  fi
  [ "$(git -C "$A" rev-parse HEAD)" = "$AH_C" ] || stop "setup: the app HEAD is not $AH_C"
  [ -z "$(git -C "$A" status --porcelain)" ] || stop "setup: the app tree is not clean"
  if [ ! -f "$W/Cargo.toml" ]; then   # the export cargo-mutants mutates in place: no .git, as in its copies
    mkdir -p "$W" && git -C "$A" archive "$AH_C" | tar -x -C "$W" || stop "setup: export of $AH_C failed"
  fi
  ( cd "$W" && rustc --version && cargo --version && cargo mutants --version && cargo nextest --version ) > "$S/versions.txt" 2>&1
  [ -s "$S/rust-source.txt" ] && cat "$S/rust-source.txt" >> "$S/versions.txt"   # where rust came from (setup)
  if [ "$DRY" != 1 ]; then
    grep -q "^rustc $AH_RUST " "$S/versions.txt" || stop "setup: rustc in the app folder is not $AH_RUST (versions.txt)"
    grep -q "^cargo-mutants $AH_CMV\$" "$S/versions.txt" || stop "setup: cargo-mutants is not $AH_CMV (versions.txt)"
    grep -q "^cargo-nextest $AH_NXV" "$S/versions.txt" || stop "setup: cargo-nextest is not $AH_NXV (versions.txt)"
  fi
  touch "$S/setup.done"; say "setup: done: $(tr '\n' ';' < "$S/versions.txt")"
fi

# 2. LIST: the whole store+pull list, then shard K/N (cargo-mutants' own default slice sharding).
if [ ! -e "$S/list.done" ]; then
  ( cd "$W" && run cargo mutants --list -p store -p pull --cargo-arg=--locked ) > "$S/ALL-LIST.txt" 2> "$S/list.err"
  rc=$?; [ "$rc" = 124 ] && exit 0; [ "$rc" = 0 ] || stop "list: cargo mutants --list failed (list.err)"
  ( cd "$W" && run cargo mutants --list -p store -p pull --cargo-arg=--locked --shard "$K/$N" ) > "$S/SHARD-LIST.txt" 2>> "$S/list.err"
  rc=$?; [ "$rc" = 124 ] && exit 0; [ "$rc" = 0 ] || stop "list: cargo mutants --list --shard $K/$N failed (list.err)"
  T=$(ah_lines "$S/ALL-LIST.txt"); c=$(( (T + N - 1) / N ))
  sed -n "$((K * c + 1)),$(((K + 1) * c))p" "$S/ALL-LIST.txt" | cmp -s - "$S/SHARD-LIST.txt" || stop "list: shard $K/$N is not lines $((K * c + 1))..$(((K + 1) * c)) of the full list of $T"
  [ -s "$S/SHARD-LIST.txt" ] || stop "list: shard $K/$N is empty"
  [ "$(sort "$S/SHARD-LIST.txt" | uniq -d | wc -l | tr -d ' ')" = 0 ] || stop "list: the shard repeats a name"
  grep -q -v -E '^crates/(store|pull)/' "$S/SHARD-LIST.txt" && stop "list: a name outside crates/store and crates/pull"
  if [ -e "$S/SHARD-LIST.committed" ]; then
    cmp -s "$S/SHARD-LIST.committed" "$S/SHARD-LIST.txt" || stop "list: SHARD-LIST.txt differs from the one already on the result branch"
  fi
  say "list: store+pull $T mutants (pull $(grep -c '^crates/pull/' "$S/ALL-LIST.txt"), store $(grep -c '^crates/store/' "$S/ALL-LIST.txt")); shard $K/$N holds $(ah_lines "$S/SHARD-LIST.txt")"
  touch "$S/list.done"
fi

skipargs() {   # the SKIPS.txt names as nextest's own arguments (-- --skip NAME ... --exact), one per line
  [ -s "$S/SKIPS.txt" ] || return 0
  echo "--"
  while IFS= read -r t; do [ -n "$t" ] && printf -- '--skip\n%s\n' "$t"; done < "$S/SKIPS.txt"
  echo "--exact"
}

# 3. BASELINE, once per box, in the export: build the store and pull tests, then run each suite whole.
if [ ! -e "$S/baseline.done" ]; then
  for p in store pull; do
    [ -e "$S/built-$p" ] && continue
    if [ "$DRY" != 1 ]; then
      say "baseline: building the $p tests"
      ( cd "$W" && run cargo nextest run --locked -p "$p" --no-run ) >> "$S/baseline-build.log" 2>&1
      rc=$?; [ "$rc" = 124 ] && { say "baseline: the $p build goes on in the next step"; exit 0; }
      [ "$rc" = 0 ] || stop "baseline: the $p tests do not build (baseline-build.log)"
    fi
    touch "$S/built-$p"
  done
  for p in store pull; do
    [ -e "$S/suite-$p.s" ] && continue
    if [ "$DRY" = 1 ]; then echo 30 > "$S/suite-$p.s"; continue; fi
    [ "$(left)" -ge 420 ] || { say "baseline: the $p suite waits for a fresh step"; exit 0; }
    set --
    while IFS= read -r a; do [ -n "$a" ] && set -- "$@" "$a"; done <<EOF
$(skipargs)
EOF
    t0=$(date +%s)
    ( cd "$W" && run cargo nextest run --locked -p "$p" --no-fail-fast "$@" ) > "$S/baseline-$p.log" 2>&1
    rc=$?; t1=$(date +%s)
    grep -E '^ *Summary ' "$S/baseline-$p.log" | tail -n 1 | sed 's/^ *//' > "$S/baseline-summary-$p.txt"
    say "baseline: $p rc $rc in $((t1 - t0)) s: $(cat "$S/baseline-summary-$p.txt")"
    [ "$rc" = 124 ] && stop "baseline: the $p suite did not finish within one step ($((t1 - t0)) s)"
    if [ "$rc" != 0 ]; then
      grep -E '^ *(FAIL|TIMEOUT|SIGSEGV|SIGABRT|SIGKILL|SIGBUS|ABORT|LEAK-FAIL) ' "$S/baseline-$p.log" | sed 's/^ *//' | sort -u > "$S/baseline-failed-$p.txt"
      stop "BASELINE-RED: $p, nextest rc $rc; $(ah_lines "$S/baseline-failed-$p.txt") failing lines in baseline-failed-$p.txt (JOB STEP 3)"
    fi
    echo $((t1 - t0)) > "$S/suite-$p.s"
  done
  touch "$S/baseline.done"
fi

# 4. CHUNKS: the next names of the shard that have no outcome yet, all of one crate, as many as fit the
#    step, run in place in the export with that crate's timeout (3 x its MEASURED suite, 60 to 300 s).
restore() {   # copy back any file of the two crates that differs from the commit (a cut step may leave one)
  local c l f n=0
  for c in store pull; do
    diff -rq "$A/crates/$c" "$W/crates/$c" > "$S/restore.diff" 2>&1
    while IFS= read -r l; do
      case "$l" in
        "Files $A/"*" differ")
          f=${l#"Files $A/"}; f=${f%% and *}
          cp "$A/$f" "$W/$f" || return 1
          n=$((n + 1));;
        "Only in $A/"*) say "restore: $l"; return 1;;
        *) ;;   # files the tests wrote into the export do not change the source
      esac
    done < "$S/restore.diff"
  done
  [ "$n" -gt 0 ] && say "restore: $n mutated file(s) copied back from the commit"
  return 0
}
tmo_of() {   # crate $1, rebuild allowance $2: 3 x the MEASURED suite wall, at least 60 s, at most what one
             # step leaves after start-up and a rebuild; a timeout under 1.5 x the suite would turn
             # missed mutants into timeouts, so that stops the box instead
  local s t cap
  s=$(cat "$S/suite-$1.s"); cap=$(( BUDGET - 50 - $2 ))
  t=$(( s * 3 )); [ "$t" -lt 60 ] && t=60; [ "$t" -gt "$cap" ] && t=$cap
  [ $(( t * 2 )) -ge $(( s * 3 )) ] || stop "the $1 suite takes $s s (MEASURED): a test timeout of $t s, all a 9-minute step allows, is under 1.5 x that"
  echo "$t"
}
avg_of() {   # MEASURED seconds per finished mutant of crate $1 so far (pull 150 and store 40 before any)
  cat "$S"/chunks/c-[0-9][0-9][0-9][0-9]/STAT 2>/dev/null | awk -v c="$1" -v d="$2" '
    { for (i = 1; i <= NF; i++) { split($i, a, "="); v[a[1]] = a[2] }
      if (v["crate"] == c && v["done"] > 0) { n += v["done"]; w += v["wall_s"] } }
    END { if (n > 0) printf "%d", (w / n) + 1; else print d }'
}
for p in store pull; do   # can a whole mutant of each crate of this shard fit one step? (stops if not)
  grep -q "^crates/$p/" "$S/SHARD-LIST.txt" || continue
  b=120; [ "$p" = store ] && b=60; [ "$DRY" = 1 ] && b=5
  tmo_of "$p" "$b" > /dev/null
done
while :; do
  restore || stop "restore: the export lost a file of the commit (restore.diff)"
  cat "$S"/chunks/c-[0-9][0-9][0-9][0-9]/mutants.out/caught.txt "$S"/chunks/c-[0-9][0-9][0-9][0-9]/mutants.out/missed.txt \
      "$S"/chunks/c-[0-9][0-9][0-9][0-9]/mutants.out/timeout.txt "$S"/chunks/c-[0-9][0-9][0-9][0-9]/mutants.out/unviable.txt \
      2>/dev/null | sort -u > "$S/done.sorted"
  awk -v df="$S/done.sorted" 'FILENAME == df { d[$0] = 1; next } !($0 in d)' "$S/done.sorted" "$S/SHARD-LIST.txt" > "$S/remaining.txt"
  [ "$(( $(ah_lines "$S/remaining.txt") + $(ah_lines "$S/done.sorted") ))" = "$(ah_lines "$S/SHARD-LIST.txt")" ] || stop "chunks: outcomes and remaining names do not add up to the shard"
  if [ ! -s "$S/remaining.txt" ]; then
    touch "$S/ALL.done"; say "all $(ah_lines "$S/SHARD-LIST.txt") mutants of shard $K/$N have an outcome"; exit 10
  fi
  first=$(head -n 1 "$S/remaining.txt")
  case "$first" in crates/pull/*) crate=pull; bld=120; d0=150;; crates/store/*) crate=store; bld=60; d0=40;; *) stop "unexpected name: $first";; esac
  [ "$DRY" = 1 ] && bld=5
  T=$(tmo_of "$crate" "$bld") || exit 3
  avg=$(avg_of "$crate" "$d0")
  [ "$(left)" -ge $((30 + bld + T)) ] || exit 0   # room for one whole mutant: start-up, a rebuild, the test timeout
  M=$(( ($(left) - 40) / avg )); [ "$M" -lt 1 ] && M=1; [ "$M" -gt 20 ] && M=20
  cn=$(printf '%04d' $(( $(ls -d "$S"/chunks/c-[0-9][0-9][0-9][0-9] 2>/dev/null | wc -l) + 1 )))
  C=$S/chunks/c-$cn; mkdir -p "$C"
  awk -v p="crates/$crate/" -v m="$M" 'index($0, p) == 1 && n < m { print; n++; next } { exit }' "$S/remaining.txt" > "$C/names.txt"
  M=$(ah_lines "$C/names.txt")
  awk -f "$D/ah-esc.awk" "$C/names.txt" > "$C/names.re"
  set --
  while IFS= read -r r; do set -- "$@" --re "$r"; done < "$C/names.re"
  if [ -s "$S/SKIPS.txt" ]; then   # cargo-mutants passes what follows its own "--" to cargo nextest run
    set -- "$@" --
    while IFS= read -r a; do [ -n "$a" ] && set -- "$@" "$a"; done <<EOF
$(skipargs)
EOF
  fi
  echo "$cn $(date +%s)" > "$S/current"
  say "chunk $cn: $M $crate mutants, test timeout $T s, $(left) s left in the step"
  t0=$(date +%s)
  if [ "$DRY" = 1 ]; then
    mkdir -p "$C/mutants.out"
    for f in caught missed timeout unviable; do : > "$C/mutants.out/$f.txt"; done
    run sleep "${AH_DRY_SLEEP:-1}"; rc=$?; [ "$rc" = 124 ] || rc=2
    awk -v o="$C/mutants.out" -v h="$( [ "$rc" = 124 ] && echo $((M / 2)) || echo "$M")" 'NR <= h { f = (NR % 4 == 1) ? "caught" : (NR % 4 == 2) ? "missed" : (NR % 4 == 3) ? "unviable" : "timeout"; print >> (o "/" f ".txt") }' "$C/names.txt"
    printf '{"dry_run":true,"total_mutants":%s}\n' "$M" > "$C/mutants.out/outcomes.json"
    echo "dry run: cargo mutants not run" > "$C/console.log"
  else
    ( cd "$W" && run cargo mutants --in-place --baseline=skip --timeout "$T" -p "$crate" --cargo-arg=--locked \
        --test-tool nextest --cargo-test-arg=--max-fail=1:immediate -o "$C" "$@" ) > "$C/console.log" 2>&1
    rc=$?
  fi
  t1=$(date +%s)
  O=$C/mutants.out; mkdir -p "$O"
  for f in caught missed timeout unviable; do [ -f "$O/$f.txt" ] || : > "$O/$f.txt"; done
  sort "$C/names.txt" > "$C/names.sorted"
  cat "$O/caught.txt" "$O/missed.txt" "$O/timeout.txt" "$O/unviable.txt" | sort > "$C/got.sorted"
  [ -z "$(comm -13 "$C/names.sorted" "$C/got.sorted")" ] || stop "chunk $cn: an outcome that is not one of its names"
  [ -z "$(uniq -d "$C/got.sorted")" ] || stop "chunk $cn: a name with two outcomes"
  g=$(ah_lines "$C/got.sorted")
  case $rc in
    0|2|3) [ "$g" = "$M" ] || stop "chunk $cn: cargo-mutants ended (exit $rc) with $g outcomes for $M names";;
    124) say "chunk $cn: cut at the step's end after $g of $M";;
    *) stop "chunk $cn: cargo-mutants exit $rc (console.log)";;
  esac
  if [ "$rc" = 124 ] && [ "$g" = 0 ]; then
    echo "$first" >> "$S/cut-first.txt"
    [ "$(grep -c -x -F -e "$first" "$S/cut-first.txt")" -ge 3 ] && stop "chunk $cn: this mutant did not finish in 3 whole steps: $first"
  fi
  echo "chunk=$cn crate=$crate n=$M done=$g caught=$(ah_lines "$O/caught.txt") missed=$(ah_lines "$O/missed.txt") timeout=$(ah_lines "$O/timeout.txt") unviable=$(ah_lines "$O/unviable.txt") rc=$rc test_timeout_s=$T start=$t0 end=$t1 wall_s=$((t1 - t0))" > "$C/STAT"
  say "chunk $cn: $(cat "$C/STAT")"
  echo "none" > "$S/current"
done
```

<!-- FILE drive.sh -->
```sh
#!/bin/bash
# drive.sh K N [push]: one turn of an app-heart box, run IN THE FOREGROUND by the session's own user (root
# or not); one call lasts at most about 8.5 minutes and leaves nothing running behind it.
# First call on a box: makes the ordinary user (when the session is root), the folders, and the result
# repository (a clone of the result branch when it exists: the box then resumes from it).
# Every call: sweeps old test scratch, checks the disk, runs ONE step of the work as the ordinary user
# (ah-step.sh, at most 480 s), then commits and pushes the results and prints ONE status line starting
# with CONTINUE, FINAL or STOP. "push" only collects, commits and pushes.
set -u
K=${1:?K}; N=${2:?N}; MODE=${3:-loop}
J=$(cd "$(dirname "$0")" && pwd)
. "$J/ah-common.sh"
BUDGET=${AH_BUDGET_S:-480}   # AH_BUDGET_S exists for dry runs only
RB=cloud-results/app-heart-mutants-$K-of-$N
RD=results/app-heart-mutants/shard-$K-of-$N
RES=$HOME/ahres-$K-of-$N
MARK=$HOME/.ah-box-$K-of-$N

if [ "$(id -u)" = 0 ]; then
  RU=mut
  if ! id "$RU" >/dev/null 2>&1; then
    useradd -m -s /bin/bash "$RU" >/dev/null 2>&1 || { echo "STOP cannot create the ordinary user $RU (useradd)"; exit 3; }
  fi
  RH=$(getent passwd "$RU" 2>/dev/null | cut -d: -f6)
  [ -n "$RH" ] || RH=$(eval echo "~$RU")
  if command -v runuser >/dev/null 2>&1; then
    asru() { runuser -l -s /bin/bash -c "$1" "$RU"; }
  else
    asru() { su -l -s /bin/bash -c "$1" "$RU"; }
  fi
else
  RU=$(id -un); RH=$HOME
  asru() { bash -c "$1"; }
fi
AH=$RH/ah; S=$AH/state; R=$RES/$RD

# ---- first call on this box ----
if [ ! -f "$MARK" ]; then
  BOXID=$(date -u +%Y%m%dT%H%M%SZ)
  [ "$(asru 'id -un' 2>/dev/null)" = "$RU" ] || { echo "STOP cannot run commands as $RU"; exit 3; }
  asru "mkdir -p '$AH/bin' '$S/chunks' '$AH/tmp'" || { echo "STOP cannot make $AH"; exit 3; }
  cp "$J/ah-common.sh" "$J/ah-step.sh" "$J/ah-esc.awk" "$AH/bin/" || { echo "STOP cannot copy the scripts to $AH/bin"; exit 3; }
  if [ ! -d "$RES/.git" ]; then
    git ls-remote --exit-code --heads "$AH_RES_REPO" "$RB" >/dev/null 2>&1
    lr=$?
    if [ "$lr" = 0 ]; then
      git clone -q --single-branch --branch "$RB" "$AH_RES_REPO" "$RES" || { echo "STOP cannot clone the existing result branch $RB"; exit 3; }
    elif [ "$lr" = 2 ]; then
      git init -q "$RES" && git -C "$RES" checkout -q --orphan "$RB" && git -C "$RES" remote add origin "$AH_RES_REPO" || { echo "STOP cannot make the result repository"; exit 3; }
    else
      echo "STOP cannot reach the repository to look for $RB (git ls-remote rc $lr)"; exit 3
    fi
  fi
  if [ -d "$R" ]; then   # resume: the chunks already on the branch are not run again
    # the v1 job (8737b862) wrote the same branch and folder with batches/: never mix the two layouts
    [ -e "$R/batches" ] && { echo "STOP the result branch $RB holds the v1 layout (batches/): another writer; nothing run, nothing pushed"; exit 3; }
    for x in "$R"/chunks/c-[0-9][0-9][0-9][0-9]; do
      [ -f "$x/STAT" ] || continue
      y=$S/chunks/${x##*/}
      mkdir -p "$y/mutants.out" && cp "$x/STAT" "$x/names.txt" "$y/"
      for f in caught missed timeout unviable; do cp "$x/$f.txt" "$y/mutants.out/$f.txt"; done
    done
    [ -f "$R/SHARD-LIST.txt" ] && cp "$R/SHARD-LIST.txt" "$S/SHARD-LIST.committed"
    [ -f "$R/SKIPS.txt" ] && cp "$R/SKIPS.txt" "$S/SKIPS.txt"
    [ -f "$R/boxes.txt" ] && cp "$R/boxes.txt" "$S/boxes.txt"
  fi
  cores=$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null)
  ram=$(free -m 2>/dev/null | awk '/^Mem:/ {print $2 " MiB RAM, "}')
  cg=$(cat /sys/fs/cgroup/memory.max 2>/dev/null || cat /sys/fs/cgroup/memory/memory.limit_in_bytes 2>/dev/null || echo -)
  disk=$(df -Pk "$AH" | awk 'NR == 2 {printf "%.1f GiB free", $4 / 1048576}')
  echo "$(ah_now) box $BOXID: user $RU, $cores cores, ${ram}cgroup memory limit $cg, $disk, $(uname -sm)" >> "$S/boxes.txt"
  if [ "$(id -u)" = 0 ]; then chown -R "$RU" "$AH"; fi
  echo "$BOXID" > "$MARK"
fi
BOXID=$(cat "$MARK")

free_kib() { df -Pk "$AH" | awk 'NR == 2 {print $4}'; }
sweep() {   # test scratch untouched for 10 min in the worker's private TMPDIR (no step runs between steps)
  find "$AH/tmp" -mindepth 1 -maxdepth 1 -mmin +10 -exec chmod -R u+rwx {} + 2>/dev/null
  find "$AH/tmp" -mindepth 1 -maxdepth 1 -mmin +10 -exec rm -rf {} + 2>/dev/null
  true
}
stopfile() { echo "$(ah_now) $*" > "$S/STOP.txt"; }

collect() {
  local x cn f p first nm c m t u tn tw rate eta st nbox ck
  mkdir -p "$R/chunks" "$R/logs"
  for f in SHARD-LIST.txt SKIPS.txt boxes.txt; do [ -f "$S/$f" ] && cp "$S/$f" "$R/$f"; done
  for f in steps.log versions.txt baseline-summary-store.txt baseline-summary-pull.txt baseline-failed-store.txt baseline-failed-pull.txt STOP.txt; do
    [ -f "$S/$f" ] && cp "$S/$f" "$R/logs/${f%.*}-$BOXID.${f##*.}"
  done
  for p in store pull; do [ -f "$S/baseline-$p.log" ] && tail -n 300 "$S/baseline-$p.log" > "$R/logs/baseline-$p-$BOXID.log"; done
  [ -f "$S/baseline-build.log" ] && tail -n 60 "$S/baseline-build.log" > "$R/logs/baseline-build-$BOXID.log"
  [ -f "$S/setup.log" ] && tail -n 100 "$S/setup.log" > "$R/logs/setup-$BOXID.log"
  for x in "$S"/chunks/c-[0-9][0-9][0-9][0-9]; do
    [ -f "$x/STAT" ] || continue
    cn=${x##*/}
    [ -f "$R/chunks/$cn/STAT" ] && continue
    mkdir -p "$R/chunks/$cn"
    cp "$x/names.txt" "$R/chunks/$cn/"
    for f in caught missed timeout unviable; do cp "$x/mutants.out/$f.txt" "$R/chunks/$cn/$f.txt"; done
    [ -f "$x/mutants.out/outcomes.json" ] && cp "$x/mutants.out/outcomes.json" "$R/chunks/$cn/outcomes.json"
    [ -f "$x/console.log" ] && tail -n 200 "$x/console.log" > "$R/chunks/$cn/console.log"
    cp "$x/STAT" "$R/chunks/$cn/STAT"
  done
  for f in caught missed timeout unviable; do
    : > "$R/$f.txt"
    for x in "$R"/chunks/c-[0-9][0-9][0-9][0-9]; do [ -f "$x/$f.txt" ] && cat "$x/$f.txt" >> "$R/$f.txt"; done
  done
  {
    printf '{"job":"app-heart","shard":%s,"n":%s,"commit":"%s","cargo_mutants":"%s","cargo_nextest":"%s","chunks":[' "$K" "$N" "$AH_C" "$AH_CMV" "$AH_NXV"
    first=1
    for x in "$R"/chunks/c-[0-9][0-9][0-9][0-9]; do
      [ -f "$x/outcomes.json" ] || continue
      [ "$first" = 1 ] || printf ','
      first=0
      printf '{"chunk":"%s","outcomes":' "${x##*/}"; cat "$x/outcomes.json"; printf '}'
    done
    printf ']}\n'
  } > "$R/outcomes.json"
  nm=$(ah_lines "$R/SHARD-LIST.txt")
  c=$(ah_lines "$R/caught.txt"); m=$(ah_lines "$R/missed.txt"); t=$(ah_lines "$R/timeout.txt"); u=$(ah_lines "$R/unviable.txt")
  tn=$((c + m + t + u))
  set -- $(cat "$R"/chunks/c-[0-9][0-9][0-9][0-9]/STAT 2>/dev/null | awk '{ for (i = 1; i <= NF; i++) { split($i, a, "="); v[a[1]] = a[2] } k++; w += v["wall_s"] } END { print k + 0, w + 0 }')
  ck=$1; tw=$2
  if [ "$tw" -gt 0 ] && [ "$tn" -gt 0 ]; then
    rate=$(awk -v n="$tn" -v w="$tw" 'BEGIN { printf "%.1f", n * 3600 / w }')
    eta=$(awk -v n="$tn" -v w="$tw" -v r="$((nm - tn))" 'BEGIN { printf "%.1f", r * w / n / 3600 }')
  else rate=-; eta=-; fi
  st=PARTIAL; [ -e "$S/ALL.done" ] && st=COMPLETE; [ -e "$S/STOP.txt" ] && st=STOPPED
  nbox=$(ah_lines "$R/boxes.txt")
  {
    echo "app-heart shard $K/$N: app commit $AH_C, crates store + pull, cargo-mutants $AH_CMV, cargo-nextest $AH_NXV"
    echo "updated $(ah_now) by box $BOXID; status $st"
    echo "mutants with an outcome $tn of $nm, in $ck chunks"
    echo "caught $c, missed $m, timeout $t, unviable $u"
    echo "rate $rate mutants/h (MEASURED: chunk walls so far, start-up included); remaining $((nm - tn)); ETA $eta h (ESTIMATE from that rate)"
    echo "last step: $(tail -n 1 "$S/steps.log" 2>/dev/null)"
    [ -e "$S/STOP.txt" ] && echo "STOP: $(cat "$S/STOP.txt")"
  } > "$R/PROGRESS.txt"
  echo "SUMMARY app-heart shard $K/$N commit $AH_C cargo-mutants $AH_CMV nextest $AH_NXV mutants $nm tested $tn caught $c missed $m timeout $t unviable $u chunk_wall_s $tw (MEASURED) boxes $nbox status $st" > "$R/SUMMARY.txt"
  STATUS="shard $K/$N tested $tn/$nm caught $c missed $m timeout $t unviable $u rate $rate/h eta ${eta}h"
}

push_now() {   # $1 = commit message
  local out rc
  git -C "$RES" add -A -- "$RD" >/dev/null 2>&1
  if ! git -C "$RES" diff --cached --quiet 2>/dev/null; then
    git -C "$RES" commit -q -m "$1" >/dev/null 2>&1 || { PUSHNOTE="commit failed (git identity?)"; return 1; }
  fi
  git -C "$RES" rev-parse -q --verify HEAD >/dev/null 2>&1 || { PUSHNOTE="nothing to push yet"; return 0; }
  out=$(git -C "$RES" push -q origin "HEAD:refs/heads/$RB" 2>&1); rc=$?
  if [ "$rc" != 0 ]; then
    case "$out" in
      *rejected*|*non-fast-forward*|*"fetch first"*) stopfile "push rejected: another writer on $RB (never force)";;
    esac
    PUSHNOTE="push failed: $(echo "$out" | tail -n 1)"; return 1
  fi
  PUSHNOTE="pushed $(git -C "$RES" rev-parse --short HEAD)"
  return 0
}

finish() {   # $1 = FINAL or STOP
  collect
  push_now "app-heart $K/$N: $1, $STATUS"
  if [ "$1" = FINAL ]; then
    echo "FINAL $(cat "$R/SUMMARY.txt") | $PUSHNOTE | branch $RB"; exit 0
  fi
  echo "STOP $(cat "$S/STOP.txt" 2>/dev/null) | $STATUS | $PUSHNOTE | branch $RB"; exit 3
}

STATUS=""; PUSHNOTE=""
if [ "$MODE" = push ]; then
  collect; push_now "app-heart $K/$N: notes, $STATUS"
  echo "PUSHED $STATUS | $PUSHNOTE | branch $RB"; exit 0
fi

# ---- every call ----
sweep
if [ "$(free_kib)" -lt "$AH_MINFREE_KIB" ]; then stopfile "disk: less than 4 GiB free on $AH after the sweep"; fi
[ -e "$S/STOP.txt" ] && finish STOP
[ -e "$S/ALL.done" ] && finish FINAL
t0=$(date +%s)
asru "bash '$AH/bin/ah-step.sh' $K $N $BUDGET >> '$S/step.out' 2>&1 < /dev/null"
rc=$?
dt=$(( $(date +%s) - t0 ))
[ "$rc" = 3 ] && finish STOP
[ "$rc" = 10 ] && finish FINAL
[ "$rc" = 0 ] || echo "$(ah_now) step exit $rc after $dt s" >> "$S/steps.log"
if [ "$dt" -lt 20 ]; then sleep 60; fi   # a step that had nothing to do (another still held the lock): no busy loop
collect
push_now "app-heart $K/$N: progress, $STATUS" || true
echo "CONTINUE $STATUS | step ${dt}s rc $rc | $(tail -n 1 "$S/steps.log" 2>/dev/null | cut -c1-160) | $PUSHNOTE | disk $(( $(free_kib) / 1048576 )) GiB free"
exit 0
```

<!-- FILE SHA256SUMS -->
```text
6cc8ce351fcae780c47f797cf1a808fee48bc27edd0d575298dea97da6be5f00  ah-common.sh
0d9d57c772a0fc4f73cca118b195f50f9c2c8a1dcad60928d4410821b61c3a86  ah-esc.awk
7e230110534f8efb4f1942ee2ca1b1dbc5e8c583f805d6c4b8474ecc3d077cfe  ah-step.sh
c5350f485130f7d38be92fd43f42c263e0abd84e559b02900eaa050775338d36  drive.sh
```

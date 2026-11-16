# JOB APP-HEART: cargo-mutants on the app's heart crates (store, pull)

Read this whole file once, then do STEP 0 to STEP 4 in order. Labels: MEASURED = a number from a named
run; EXACT = a count; ESTIMATE = a forecast. Your first message gives you K and N (this box is shard K of N).

## What

- Repository SJParthi/brutex (public). Code under test: the app commit
  `69aa8ffeb29c7c59a1585b45510c8658df00c7a2`, the tip of `feat/db-fix` on origin (the app's landed head
  since 9 Nov 2026). The box fetches exactly this commit (depth 1) and never a newer one.
- Crates: `store` and `pull` only (`cargo mutants -p store -p pull`). Each mutant is tested against the
  tests of its own crate (cargo-mutants' default).
- This box: shard K of N of the store+pull mutant list, cut by cargo-mutants' own default (slice) sharding.
- Results: ONLY the new branch `cloud-results/app-heart-mutants-K-of-N` (an orphan branch holding results
  only). This file sits alone on branch `cloud/jobs-app-heart-20261116`.
- Tools: rustc 1.97.1 (the app's `rust-toolchain.toml`), cargo-mutants 27.1.0 (the version of the
  16 Nov cloud wave), cargo-nextest 0.9.145 (the app's CI pin). The scripts install all three.

## Rules

1. Code only. Never fetch market data or anything beyond this repository, the Rust toolchain and the
   crates the app's `Cargo.lock` names. Never use, ask for or print a signed link, a token, a key or any
   other secret.
2. Results only. Push only to `cloud-results/app-heart-mutants-K-of-N`. Never push code, never touch
   another branch, never force-push, never rewrite history. A rejected push stops the box (STEP 4).
3. Keep your turn busy until FINAL or STOP: an idle box is paused and loses its background work. The
   only long command is `drive.sh` (STEP 2); call it again at once each time it returns.
4. On any failure, stop and report (STEP 4). Never retry hoping for green, never edit the app's code or
   these scripts, never add a skip or an exclude beyond STEP 3's rule.
5. The scripts delete or stop only these, all inside the run user's own folder `~/ah`: the worker's lock
   folder; cargo-mutants copies a cut worker left in `~/ah/tmp`; the baseline export `~/ah/plain` after
   its run; test scratch in `~/ah/tmp` untouched for 60 minutes (failing tests leave it behind, and it
   filled earlier boxes' disks); and, on Linux only, processes of the run user that a cut worker left
   working inside `~/ah`. Nothing else.
6. Use shell, git, awk, Rust and cargo only. No other scripting language (never the six-letter one that
   starts with "Py").

## STEP 0: the box

Run once: `id -u; nproc; free -m; cat /sys/fs/cgroup/memory.max; df -h ~; git --version;
curl --version | head -n 1; command -v awk sha256sum`. When `id -u` prints 0, also
`command -v useradd runuser su getent`. If git, curl, awk or sha256sum is missing, install it with the
system package manager and say so in NOTES.md. Do not install Rust yourself: the worker does it.

## STEP 1: the scripts

```sh
git clone -q --depth 1 --single-branch --branch cloud/jobs-app-heart-20261116 https://github.com/SJParthi/brutex.git ~/ahjob
cd ~/ahjob && git rev-parse HEAD && mkdir -p bin
awk 'BEGIN { f = ""; w = 0 } /^<!-- FILE [A-Za-z0-9._-]+ -->$/ { f = $3; next } w == 0 && f != "" && /^```/ { w = 1; next } w == 1 && /^```$/ { w = 0; f = ""; next } w == 1 { print > ("bin/" f) }' JOB-APP-HEART.md
cd bin && sha256sum -c SHA256SUMS
```

Every file must print OK; otherwise stop (STEP 4).

## STEP 2: the loop

Run `bash ~/ahjob/bin/drive.sh K N` (your K and N) with the Bash tool's timeout set to 300000 ms. It
returns in about 4 minutes (at once on FINAL or STOP) and prints ONE line:

- `CONTINUE ...`: run the same command again at once, with nothing in between.
- `FINAL ...`: go to STEP 4.
- `STOP ...`: go to STEP 3.

If the tool call itself times out, run it again: `drive.sh` is safe to repeat and the worker runs on its
own. What it does: on the first call it makes the folders and the result repository (a clone of the result
branch when one exists, see RESUME) and starts the worker `ah-run.sh`, which installs the tools, fetches
the app commit, lists the mutants, runs the baseline and then the batches. Every call it restarts a dead
worker, sweeps old test scratch, checks the disk (stop below 4 GiB free) and a stalled batch (nothing
written for 90 minutes), and commits and pushes progress every 10-14 minutes (always within 15) and
after every finished batch. The first call takes about 20-30 minutes of setup before batches start (ESTIMATE).

When the session's user is root, the work runs as a new ordinary user `mut`: the app's permission tests
re-run themselves as another user when started by root, and that child cannot enter cargo-mutants'
private copies (mode 0700), so as root the baseline fails for the setup's reason. As an ordinary user they
run as on the app's CI and the operator's machine (the app's 17 Oct cloud pass of `pull` did the same).
The run user's folder is `~mut/ah` when root, else `~/ah`; the result repository is
`~/ahres-K-of-N` of the session's user.

## STEP 3: STOP

Read the reason on the STOP line (also `<run user folder>/state/STOP.txt`).

- `BASELINE-RED`: the unmutated store and pull suites failed on this box. Read `state/baseline-failed.txt`
  and, for each failing test, its output in `state/baseline.log`. Only when EVERY failing test fails
  because it needs market data, the operator's live store or a Mac-only file or path (its own message
  names such a file, folder or `BRUTEX_*` variable): write their full test names (as nextest prints them
  after the binary name, e.g. `module::tests::name`), one per line, into `state/SKIPS.txt` (owned by the
  run user), rename `state/STOP.txt` to `state/STOP.handled-<UTC time>.txt`, and go back to STEP 2: the
  baseline runs again with those names skipped, and every batch skips them too. Any other failure, or
  skips that do not make the baseline green: STEP 4.
- Anything else: STEP 4.

If CONTINUE lines keep saying `commit failed (git identity?)`, set the git name and e-mail your session
uses for its own commits, then keep calling STEP 2.

## STEP 4: finish

1. Write `NOTES.md` in `~/ahres-K-of-N/results/app-heart-mutants/shard-K-of-N/` (at most 80 lines):
   the SUMMARY line; the box (cores, RAM, cgroup limit, disk, run user); the versions
   (`logs/versions-*.txt`); the baseline (passed, failed, skipped, wall: MEASURED, from
   `logs/baseline-summary-*.txt` and `logs/steps-*.log`); SKIPS (none, or each name with its reason);
   the batches that ran with `--jobs 1` after a memory kill; every STOP reason; anything unexpected.
2. Run `bash ~/ahjob/bin/drive.sh K N push` (prints `PUSHED ...`; if it says push failed, wait 60 s and
   run it again, at most 5 times).
3. FINAL MESSAGE, short: the SUMMARY line; COMPLETE, or the STOP reason and the batch it stopped in; the
   10 files with the most missed mutants (`cut -d: -f1 missed.txt | sort | uniq -c | sort -rn | head`);
   the result branch and its last commit.

## METHOD

What the worker runs, in the app commit's folder, with `TMPDIR=~/ah/tmp`, `CARGO_TERM_COLOR=never` and the
dev profile overridden by environment to `CARGO_PROFILE_DEV_OPT_LEVEL=0`, `CARGO_PROFILE_DEV_INCREMENTAL=true`,
`CARGO_PROFILE_DEV_DEBUG=0`:

1. Setup: rustup 1.97.1 (`--profile minimal`, components rustfmt, clippy, llvm-tools-preview, target
   wasm32-unknown-unknown, as `rust-toolchain.toml` lists), `cargo install cargo-mutants --version 27.1.0
   --locked`, `cargo install cargo-nextest --version 0.9.145 --locked`, then `git fetch --depth 1` of the
   app commit and a check that HEAD is that commit and the tree is clean. Each install has a 40-minute limit.
2. List: `cargo mutants --list -p store -p pull --cargo-arg=--locked` (whole list) and the same with
   `--shard K/N`; the shard must be exactly lines K*c+1 .. (K+1)*c of the whole list (c = ceil(total/N)),
   hold no repeated name, and equal `SHARD-LIST.txt` already on the result branch, if any.
3. Baseline, once per box: `cargo nextest run --locked -p store -p pull --no-fail-fast --test-threads=2`
   in an export of the commit without `.git` (what a cargo-mutants copy looks like), `CARGO_BUILD_JOBS=2`.
4. Batches of 50 names in shard order. Each batch: one exact anchored regex per name; `cargo mutants
   --list` with those regexes must print exactly the batch's names; then

   ```sh
   cargo mutants -p store -p pull --cargo-arg=--locked --jobs 2 --cap-lints true \
     --test-tool nextest --cargo-test-arg=--max-fail=1:immediate --cargo-test-arg=--test-threads=2 \
     --timeout-multiplier 3 --minimum-test-timeout 60 -o <batch folder> --re '^<name>$' ...
   ```

   with `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 RAYON_NUM_THREADS=2`. cargo-mutants runs its own baseline
   in every batch and sets that batch's timeout from it. Exit 0, 2 or 3 is a finished batch; its four
   outcome lists must hold exactly its names. Exit 4 (baseline failed) or any other stops the box. If the
   cgroup's out-of-memory kill count rose during a batch, the batch runs again with `--jobs 1`, and so does
   every later batch on that box.

Changes from the 16 Nov wave (MUT-K: `cargo mutants --workspace --shard K/30 --jobs 2 --cargo-arg=--locked
-- -- --exact --skip ...`, cargo test, automatic timeout, results on the code commit), and why:

1. `-p store -p pull` instead of `--workspace`: this job's scope.
2. Batches selected by exact names instead of one run per shard: the shard is still cargo-mutants' own
   `--shard K/N` slice, but a box cut by the account's usage cap or a paused VM resumes from the result
   branch without repeating finished batches. The wave's single run could not resume.
3. cargo-nextest 0.9.145 with `--max-fail=1:immediate` and `--cap-lints true`, as the app's own CI gate 18
   runs cargo-mutants: the first failing test ends the run (about half the time of cargo test per mutant,
   ESTIMATE), and the app's `.config/nextest.toml` orders its watchdog tests first so that hangs are caught
   instead of timing out. nextest runs no doc tests (as gate 18).
4. Timeouts: multiplier 3 with a 60 s minimum instead of cargo-mutants' default 5 (about 20 minutes a hang
   on pull, ESTIMATE); gate 18 uses 2 with a 900 s minimum. A timeout is never counted as caught.
5. Dev profile: opt-level 0, incremental, no debuginfo (the app's dev profile is opt-level 3 and not
   incremental, too slow to rebuild per mutant; the app's 16-17 Oct cloud passes did the same; outcomes do
   not depend on it, timings do).
6. `--jobs 2` as the wave, inside gate 18's 4-vCPU shape (2 build jobs, 2 test threads each), falling back
   to `--jobs 1` after a memory kill (the box has a 13.4 GiB cgroup limit and no swap).
7. An ordinary run user when the box is root (STEP 2).
8. A baseline once per box with nextest, besides cargo-mutants' own baseline in every batch.
9. Results on an orphan branch (results only) under `results/app-heart-mutants/shard-K-of-N/`.
10. cargo-mutants pinned with `--version 27.1.0` (the wave installed the latest, which was 27.1.0).

Kept: cargo-mutants 27.1.0, rustc 1.97.1, `--cargo-arg=--locked`, `--jobs 2`, copy mode (not
`--in-place`), cargo-mutants' four outcome lists and outcomes.json, progress commits and NOTES.md.

## RESULT LAYOUT

On branch `cloud-results/app-heart-mutants-K-of-N`, folder `results/app-heart-mutants/shard-K-of-N/`:

- `SUMMARY.txt`: one line, `SUMMARY app-heart shard K/N commit <app commit> cargo-mutants 27.1.0 nextest
  0.9.145 mutants <shard size> tested <n> caught <c> missed <m> timeout <t> unviable <u> batch_wall_s <s>
  (MEASURED) boxes <b> status COMPLETE|PARTIAL|STOPPED`.
- `PROGRESS.txt`: batches done, counts, rate (MEASURED from the batch walls), ETA (ESTIMATE), the running batch.
- `caught.txt`, `missed.txt`, `timeout.txt`, `unviable.txt`: every finished batch in shard order,
  cargo-mutants' own lines (`file:line:col: change`).
- `outcomes.json`: `{"job","shard","n","commit","cargo_mutants","cargo_nextest","batches":[{"batch",
  "outcomes": <cargo-mutants' outcomes.json of that batch>}]}`.
- `SHARD-LIST.txt`: the shard's names in order. `batches/b-NNN/`: names.txt, DONE (counts, exit code,
  start, end, wall), the four lists, outcomes.json, console.log. `current/`: the running batch's partial
  lists (not counted). `boxes.txt`: one line per box. `logs/`: steps, versions, setup and baseline tails,
  STOP, per box. `SKIPS.txt` only after STEP 3. `NOTES.md` from STEP 4.

## TESTS THAT NEED DATA OR THE LIVE STORE

A read of every `.rs` file of `crates/store` and `crates/pull` at the app commit (before this job was
written): every test that reads market data, the operator's live store or a Mac path through a
`BRUTEX_*` variable or a fixed path is `#[ignore]`d, with the timing measurements: 62 `#[ignore]`
attributes. `cargo test` and nextest never run them unless asked (`--ignored`), so they need no skip.
No other test of the two crates reads such a variable or path. Expected SKIPS: none; the box's baseline
is the proof (STEP 3 if not). The 62, by file:

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
  per mutant with 2 jobs: pull 1-2 min, store 10-30 s; setup 20-30 min a box; 5-10 min a batch of overhead.
- Wall per box: pull shards (0-6) about 40-80 h; shard 7 about 25-50 h; store shards (8, 9) about 10-25 h.
  Boxes stop earlier when the account's usage runs out; they resume (RESUME).
- `PROGRESS.txt` carries the MEASURED rate and the ETA from the first finished batch on, within about
  1.5 h of a box's start.

## RESUME

A box that ends early (usage cap, paused or reclaimed VM, account switch) loses at most its running
batch. To continue, start a fresh box with the same first message (same K and N). `drive.sh` finds the
result branch, takes every finished batch from it (`batches/b-NNN/DONE`), checks that its new shard list
equals the committed `SHARD-LIST.txt`, runs setup and the baseline again on the new box, and continues
with the first unfinished batch. Never run two boxes for the same K and N at once: the second push is
rejected and that box stops.

## FILES (STEP 1 extracts these into ~/ahjob/bin)

<!-- FILE ah-common.sh -->
```sh
# ah-common.sh: the settings every app-heart script shares. Sourced, never run on its own.
AH_C=69aa8ffeb29c7c59a1585b45510c8658df00c7a2   # the app commit tested (origin feat/db-fix)
AH_RUST=1.97.1                                  # the app's rust-toolchain.toml
AH_CMV=27.1.0                                   # cargo-mutants (the version of the 16 Nov wave)
AH_NXV=0.9.145                                  # cargo-nextest (the app's CI pin)
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

<!-- FILE ah-run.sh -->
```sh
#!/bin/bash
# ah-run.sh K N: the app-heart worker. It runs as an ordinary (non-root) user, detached; drive.sh starts it.
# Steps, each skipped when its marker exists in ~/ah/state: setup, list, baseline, then the batches
# b-000, b-001, ... of AH_B mutants each, in shard order. On any failure it writes STOP.txt and exits 3.
set -u
K=$1; N=$2
D=$(cd "$(dirname "$0")" && pwd)
. "$D/ah-common.sh"
AH=$HOME/ah; S=$AH/state; A=$AH/app
cd "$HOME" || exit 1   # never work from inside $AH: the leftover check below looks there
mkdir -p "$S/batches" "$AH/tmp"
say() { echo "$(ah_now) $*" >> "$S/steps.log"; }
stop() { say "STOP $*"; echo "$(ah_now) $*" > "$S/STOP.txt"; exit 3; }
[ -e "$S/STOP.txt" ] && exit 3

# One worker at a time: a lock folder holding the holder's pid.
if ! mkdir "$S/worker.lock" 2>/dev/null; then
  op=$(cat "$S/worker.lock/pid" 2>/dev/null || echo 0)
  if [ "$op" != 0 ] && kill -0 "$op" 2>/dev/null; then exit 0; fi
  rm -rf "$S/worker.lock"
  mkdir "$S/worker.lock" 2>/dev/null || exit 0
fi
echo $$ > "$S/worker.lock/pid"
trap 'rm -rf "$S/worker.lock"' EXIT
echo $$ > "$S/runner.pid"

export CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}" RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}"
export PATH="$CARGO_HOME/bin:$PATH" TMPDIR="$AH/tmp" CARGO_TERM_COLOR=never
unset CARGO_TARGET_DIR CARGO_BUILD_TARGET_DIR RUSTFLAGS RUSTDOCFLAGS RUSTC RUSTDOC RUSTC_WRAPPER CARGO_BUILD_RUSTC_WRAPPER
# Rebuild time per mutant (the app's own cloud passes of 16-17 Oct did the same; outcomes do not depend on it):
export CARGO_PROFILE_DEV_OPT_LEVEL=0 CARGO_PROFILE_DEV_INCREMENTAL=true CARGO_PROFILE_DEV_DEBUG=0
TO=""; command -v timeout >/dev/null 2>&1 && TO="timeout 2400"
DRY=${AH_DRY:-0}
say "worker start: pid $$, shard $K/$N, user $(id -un)"

# Leftovers of a cut worker (this point is reached only while no other worker holds the lock): on Linux,
# this user's processes whose working folder or command line lies inside $AH (cargo-mutants, cargo,
# nextest and test processes in its copies), never anything else; then the leftover copies themselves.
ah_ours() {
  [ -d /proc/self ] || return 0
  for p in /proc/[0-9]*; do
    [ -O "$p" ] || continue
    q=${p#/proc/}
    [ "$q" = "$$" ] && continue
    c=$(tr '\0' ' ' < "$p/cmdline" 2>/dev/null)
    w=$(readlink "$p/cwd" 2>/dev/null)
    case "$c" in *ah-run.sh*|*drive.sh*|"") continue;; esac
    case "$w/ $c" in *"$AH/"*) echo "$q";; esac
  done
}
if [ "$DRY" != 1 ]; then
  old=$(ah_ours | sort -u)
  if [ -n "$old" ]; then
    say "cleanup: stopping leftover processes inside $AH: $(echo $old)"
    kill $old 2>/dev/null; sleep 10; kill -9 $old 2>/dev/null
  fi
  for x in "$AH"/tmp/cargo-mutants-*; do
    [ -e "$x" ] || continue
    chmod -R u+rwx "$x" 2>/dev/null; rm -rf "$x" && say "cleanup: removed leftover copy ${x##*/}"
  done
fi

# 1. SETUP: toolchain, cargo-mutants, cargo-nextest, the app commit.
if [ ! -e "$S/setup.done" ]; then
  say "setup: rust $AH_RUST, cargo-mutants $AH_CMV, cargo-nextest $AH_NXV, app $AH_C"
  if [ "$DRY" != 1 ]; then
    if ! command -v rustup >/dev/null 2>&1; then
      curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain "$AH_RUST" >> "$S/setup.log" 2>&1 || stop "setup: rustup-init failed (setup.log)"
    fi
    $TO rustup toolchain install "$AH_RUST" --profile minimal -c rustfmt -c clippy -c llvm-tools-preview -t wasm32-unknown-unknown >> "$S/setup.log" 2>&1 || stop "setup: rustup toolchain install $AH_RUST failed (setup.log)"
    if [ "$(cargo mutants --version 2>/dev/null)" != "cargo-mutants $AH_CMV" ]; then
      $TO cargo +"$AH_RUST" install cargo-mutants --version "$AH_CMV" --locked --force >> "$S/setup.log" 2>&1 || stop "setup: cargo install cargo-mutants $AH_CMV failed (setup.log)"
    fi
    case "$(cargo nextest --version 2>/dev/null)" in
      "cargo-nextest $AH_NXV"*) ;;
      *) $TO cargo +"$AH_RUST" install cargo-nextest --version "$AH_NXV" --locked --force >> "$S/setup.log" 2>&1 || stop "setup: cargo install cargo-nextest $AH_NXV failed (setup.log)";;
    esac
  fi
  if [ ! -d "$A/.git" ]; then
    mkdir -p "$A" && git -C "$A" init -q && git -C "$A" remote add origin "$AH_REPO" || stop "setup: git init of the app folder failed"
  fi
  if [ "$(git -C "$A" rev-parse -q --verify HEAD 2>/dev/null)" != "$AH_C" ]; then
    $TO git -C "$A" fetch -q --depth 1 origin "$AH_C" >> "$S/setup.log" 2>&1 || stop "setup: cannot fetch the app commit $AH_C (setup.log)"
    git -C "$A" checkout -q --detach "$AH_C" >> "$S/setup.log" 2>&1 || stop "setup: cannot check out $AH_C"
  fi
  [ "$(git -C "$A" rev-parse HEAD)" = "$AH_C" ] || stop "setup: the app HEAD is not $AH_C"
  [ -z "$(git -C "$A" status --porcelain)" ] || stop "setup: the app tree is not clean"
  ( cd "$A" && rustc --version && cargo --version && cargo mutants --version && cargo nextest --version ) > "$S/versions.txt" 2>&1
  if [ "$DRY" != 1 ]; then
    grep -q "^rustc $AH_RUST " "$S/versions.txt" || stop "setup: rustc in the app folder is not $AH_RUST (versions.txt)"
    grep -q "^cargo-mutants $AH_CMV\$" "$S/versions.txt" || stop "setup: cargo-mutants is not $AH_CMV (versions.txt)"
    grep -q "^cargo-nextest $AH_NXV" "$S/versions.txt" || stop "setup: cargo-nextest is not $AH_NXV (versions.txt)"
  fi
  touch "$S/setup.done"; say "setup: done: $(tr '\n' ';' < "$S/versions.txt")"
fi

# 2. LIST: the whole store+pull list, then shard K/N (cargo-mutants' own default slice sharding).
if [ ! -e "$S/list.done" ]; then
  ( cd "$A" && cargo mutants --list -p store -p pull --cargo-arg=--locked ) > "$S/ALL-LIST.txt" 2> "$S/list.err" || stop "list: cargo mutants --list failed (list.err)"
  ( cd "$A" && cargo mutants --list -p store -p pull --cargo-arg=--locked --shard "$K/$N" ) > "$S/SHARD-LIST.txt" 2>> "$S/list.err" || stop "list: cargo mutants --list --shard $K/$N failed (list.err)"
  T=$(ah_lines "$S/ALL-LIST.txt"); c=$(( (T + N - 1) / N ))
  sed -n "$((K * c + 1)),$(((K + 1) * c))p" "$S/ALL-LIST.txt" | cmp -s - "$S/SHARD-LIST.txt" || stop "list: shard $K/$N is not lines $((K * c + 1))..$(((K + 1) * c)) of the full list of $T"
  [ -s "$S/SHARD-LIST.txt" ] || stop "list: shard $K/$N is empty"
  [ "$(sort "$S/SHARD-LIST.txt" | uniq -d | wc -l | tr -d ' ')" = 0 ] || stop "list: the shard repeats a name"
  if [ -e "$S/SHARD-LIST.committed" ]; then
    cmp -s "$S/SHARD-LIST.committed" "$S/SHARD-LIST.txt" || stop "list: SHARD-LIST.txt differs from the one already on the result branch"
  fi
  say "list: store+pull $T mutants (pull $(grep -c '^crates/pull/' "$S/ALL-LIST.txt"), store $(grep -c '^crates/store/' "$S/ALL-LIST.txt")); shard $K/$N holds $(ah_lines "$S/SHARD-LIST.txt")"
  touch "$S/list.done"
fi

# 3. BASELINE: the unmutated store and pull suites, once per box, in an export without .git
#    (what a cargo-mutants copy looks like), with nextest.
if [ ! -e "$S/baseline.done" ]; then
  if [ "$DRY" = 1 ]; then
    say "baseline: skipped (dry run)"
  else
    P=$AH/plain
    if [ -e "$P" ]; then chmod -R u+rwx "$P" 2>/dev/null; rm -rf "$P"; fi
    mkdir -p "$P" && git -C "$A" archive "$AH_C" | tar -x -C "$P" || stop "baseline: export of $AH_C failed"
    set --
    if [ -s "$S/SKIPS.txt" ]; then
      set -- --
      while IFS= read -r t; do [ -n "$t" ] && set -- "$@" --skip "$t"; done < "$S/SKIPS.txt"
      set -- "$@" --exact
    fi
    t0=$(date +%s); m0=$(awk '$1 == "oom_kill" {print $2}' /sys/fs/cgroup/memory.events 2>/dev/null)
    ( cd "$P" && CARGO_BUILD_JOBS=2 cargo nextest run --locked -p store -p pull --no-fail-fast --test-threads=2 "$@" ) > "$S/baseline.log" 2>&1
    rc=$?; t1=$(date +%s); m1=$(awk '$1 == "oom_kill" {print $2}' /sys/fs/cgroup/memory.events 2>/dev/null)
    grep -E '^ *Summary ' "$S/baseline.log" | tail -n 1 | sed 's/^ *//' > "$S/baseline-summary.txt"
    say "baseline: rc $rc in $((t1 - t0)) s, oom_kill count ${m0:--} -> ${m1:--}: $(cat "$S/baseline-summary.txt")"
    chmod -R u+rwx "$P" 2>/dev/null; rm -rf "$P"
    if [ "$rc" -ne 0 ]; then
      grep -E '^ *(FAIL|TIMEOUT|SIGSEGV|SIGABRT|SIGKILL|SIGBUS|ABORT|LEAK-FAIL) ' "$S/baseline.log" | sed 's/^ *//' | sort -u > "$S/baseline-failed.txt"
      stop "BASELINE-RED: nextest rc $rc; $(ah_lines "$S/baseline-failed.txt") failing lines in baseline-failed.txt (JOB STEP 3)"
    fi
  fi
  touch "$S/baseline.done"
fi

# 4. BATCHES of AH_B mutants, selected by exact anchored names, each with cargo-mutants' own baseline.
#    The box's out-of-memory kill counter is read around every batch: a batch during which the kernel
#    killed a process (a killed rustc would read as unviable, a killed test as caught) is run again
#    with --jobs 1, and every later batch keeps --jobs 1.
NM=$(ah_lines "$S/SHARD-LIST.txt"); NB=$(( (NM + AH_B - 1) / AH_B ))
export CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 RAYON_NUM_THREADS=2
oomcount() {
  for f in /sys/fs/cgroup/memory.events /sys/fs/cgroup/memory/memory.oom_control; do
    if [ -r "$f" ]; then awk '$1 == "oom_kill" {v = $2} END {print v + 0}' "$f"; return; fi
  done
  echo -1
}
b=0
while [ "$b" -lt "$NB" ]; do
  bb=$(printf '%03d' "$b"); d=$S/batches/b-$bb
  if [ -f "$d/DONE" ]; then b=$((b + 1)); continue; fi
  [ -e "$S/STOP.txt" ] && exit 3
  if [ -e "$d" ]; then mv "$d" "$d.cut-$(date +%s)"; fi
  mkdir -p "$d"
  JOBS=2; [ -e "$S/jobs1" ] && JOBS=1
  o0=$(oomcount)
  sed -n "$((b * AH_B + 1)),$(((b + 1) * AH_B))p" "$S/SHARD-LIST.txt" > "$d/names.txt"
  awk -f "$D/ah-esc.awk" "$d/names.txt" > "$d/names.re"
  set --
  while IFS= read -r r; do set -- "$@" --re "$r"; done < "$d/names.re"
  ( cd "$A" && cargo mutants --list -p store -p pull --cargo-arg=--locked "$@" ) > "$d/selected.txt" 2> "$d/list.err" || stop "batch $bb: cargo mutants --list with its regexes failed (list.err)"
  sort "$d/names.txt" > "$d/names.sorted"; sort "$d/selected.txt" > "$d/selected.sorted"
  cmp -s "$d/names.sorted" "$d/selected.sorted" || stop "batch $bb: its regexes select $(ah_lines "$d/selected.txt") names, not exactly its $(ah_lines "$d/names.txt")"
  if [ -s "$S/SKIPS.txt" ]; then
    set -- "$@" -- --
    while IFS= read -r t; do [ -n "$t" ] && set -- "$@" --skip "$t"; done < "$S/SKIPS.txt"
    set -- "$@" --exact
  fi
  t0=$(date +%s)
  echo "$bb $t0" > "$S/current"
  say "batch $bb: start, $(ah_lines "$d/names.txt") mutants, --jobs $JOBS, oom_kill count $o0"
  if [ "$DRY" = 1 ]; then
    mkdir -p "$d/mutants.out"
    for f in caught missed timeout unviable; do : > "$d/mutants.out/$f.txt"; done
    awk -v o="$d/mutants.out" '{ f = (NR % 4 == 1) ? "caught" : (NR % 4 == 2) ? "missed" : (NR % 4 == 3) ? "unviable" : "timeout"; print >> (o "/" f ".txt") }' "$d/names.txt"
    printf '{"dry_run":true,"total_mutants":%s}\n' "$(ah_lines "$d/names.txt")" > "$d/mutants.out/outcomes.json"
    echo "dry run: cargo mutants not run" > "$d/console.log"
    sleep "${AH_DRY_SLEEP:-2}"; rc=2
  else
    ( cd "$A" && cargo mutants -p store -p pull --cargo-arg=--locked --jobs "$JOBS" --cap-lints true \
        --test-tool nextest --cargo-test-arg=--max-fail=1:immediate --cargo-test-arg=--test-threads=2 \
        --timeout-multiplier 3 --minimum-test-timeout 60 -o "$d" "$@" ) > "$d/console.log" 2>&1
    rc=$?
  fi
  t1=$(date +%s)
  o1=$(oomcount)
  if [ "$o0" -ge 0 ] && [ "$o1" -gt "$o0" ]; then
    if [ "$JOBS" = 1 ]; then stop "batch $bb: the kernel killed a process for memory (oom_kill $o0 -> $o1) even with --jobs 1"; fi
    say "batch $bb: the kernel killed a process for memory (oom_kill $o0 -> $o1): the batch runs again with --jobs 1"
    touch "$S/jobs1"; mv "$d" "$d.oom-$(date +%s)"; echo "none" > "$S/current"
    continue
  fi
  case $rc in
    0|2|3) ;;
    4) stop "batch $bb: cargo-mutants says the unmutated tree fails its tests (exit 4; console.log)";;
    *) stop "batch $bb: cargo-mutants exit $rc (console.log)";;
  esac
  O=$d/mutants.out
  for f in caught missed timeout unviable; do [ -f "$O/$f.txt" ] || : > "$O/$f.txt"; done
  cat "$O/caught.txt" "$O/missed.txt" "$O/timeout.txt" "$O/unviable.txt" | sort > "$d/got.sorted"
  cmp -s "$d/names.sorted" "$d/got.sorted" || stop "batch $bb: its four outcome lists do not hold exactly its names"
  echo "batch=$bb n=$(ah_lines "$d/names.txt") caught=$(ah_lines "$O/caught.txt") missed=$(ah_lines "$O/missed.txt") timeout=$(ah_lines "$O/timeout.txt") unviable=$(ah_lines "$O/unviable.txt") rc=$rc start=$t0 end=$t1 wall_s=$((t1 - t0))" > "$d/DONE"
  say "batch $bb: $(cat "$d/DONE")"
  echo "none" > "$S/current"
  b=$((b + 1))
done
touch "$S/ALL.done"
say "all $NB batches done"
exit 0
```

<!-- FILE drive.sh -->
```sh
#!/bin/bash
# drive.sh K N [push]: one turn of an app-heart box, run by the session's own user (root or not).
# First call on a box: makes the ordinary user (when the session is root), the folders, and the result
# repository (a clone of the result branch when it exists: the box then resumes from it).
# Every call: checks the disk, sweeps old test scratch, starts or restarts the worker (ah-run.sh),
# commits and pushes progress every 10-14 min (always within 15), waits until about 230 s have passed, and prints
# ONE status line starting with CONTINUE, FINAL or STOP. "push" only collects, commits and pushes.
set -u
K=${1:?K}; N=${2:?N}; MODE=${3:-loop}
J=$(cd "$(dirname "$0")" && pwd)
. "$J/ah-common.sh"
T0=$(date +%s); TURN=${AH_TURN_S:-230}; PUSHEVERY=${AH_PUSH_S:-600}   # the two AH_ variables exist for dry runs only
RB=cloud-results/app-heart-mutants-$K-of-$N
RD=results/app-heart-mutants/shard-$K-of-$N
RES=$HOME/ahres-$K-of-$N
MARK=$HOME/.ah-box-$K-of-$N
LASTPUSH=$HOME/.ah-lastpush-$K-of-$N
PUSHED=$HOME/.ah-pushed-$K-of-$N

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
  asru "mkdir -p '$AH/bin' '$S/batches' '$AH/tmp'" || { echo "STOP cannot make $AH"; exit 3; }
  cp "$J/ah-common.sh" "$J/ah-run.sh" "$J/ah-esc.awk" "$AH/bin/" || { echo "STOP cannot copy the scripts to $AH/bin"; exit 3; }
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
  if [ -d "$R" ]; then   # resume: the batches already on the branch are not run again
    for x in "$R"/batches/b-[0-9][0-9][0-9]; do
      [ -f "$x/DONE" ] || continue
      mkdir -p "$S/batches/${x##*/}" && cp "$x/DONE" "$S/batches/${x##*/}/DONE"
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
sweep() {   # test scratch untouched for 60 min in the worker's private TMPDIR, never a cargo-mutants copy
  find "$AH/tmp" -mindepth 1 -maxdepth 1 ! -name 'cargo-mutants-*' -mmin +60 -exec chmod -R u+rwx {} + 2>/dev/null
  find "$AH/tmp" -mindepth 1 -maxdepth 1 ! -name 'cargo-mutants-*' -mmin +60 -exec rm -rf {} + 2>/dev/null
  true
}
stopfile() { echo "$(ah_now) $*" > "$S/STOP.txt"; }

collect() {
  local x bn f first cur cs nm nb db tn tw c m t u rate eta st nbox
  mkdir -p "$R/batches" "$R/logs" "$R/current"
  for f in SHARD-LIST.txt SKIPS.txt boxes.txt; do [ -f "$S/$f" ] && cp "$S/$f" "$R/$f"; done
  for f in steps.log versions.txt baseline-summary.txt baseline-failed.txt STOP.txt; do
    [ -f "$S/$f" ] && cp "$S/$f" "$R/logs/${f%.*}-$BOXID.${f##*.}"
  done
  [ -f "$S/baseline.log" ] && tail -n 400 "$S/baseline.log" > "$R/logs/baseline-$BOXID.log"
  [ -f "$S/setup.log" ] && tail -n 100 "$S/setup.log" > "$R/logs/setup-$BOXID.log"
  for x in "$S"/batches/b-[0-9][0-9][0-9]; do
    [ -f "$x/DONE" ] || continue
    bn=${x##*/}
    [ -f "$R/batches/$bn/DONE" ] && continue
    mkdir -p "$R/batches/$bn"
    cp "$x/names.txt" "$x/console.log" "$R/batches/$bn/"
    for f in caught missed timeout unviable; do cp "$x/mutants.out/$f.txt" "$R/batches/$bn/$f.txt"; done
    [ -f "$x/mutants.out/outcomes.json" ] && cp "$x/mutants.out/outcomes.json" "$R/batches/$bn/outcomes.json"
    cp "$x/DONE" "$R/batches/$bn/DONE"
  done
  cur=$(awk '{print $1}' "$S/current" 2>/dev/null)
  if [ -n "$cur" ] && [ "$cur" != none ] && [ -d "$S/batches/b-$cur/mutants.out" ]; then
    cs=$(awk '{print $2}' "$S/current")
    echo "batch b-$cur running since $(( ($(date +%s) - cs) / 60 )) min (partial lists, not counted)" > "$R/current/CURRENT.txt"
    for f in caught missed timeout unviable; do
      if [ -f "$S/batches/b-$cur/mutants.out/$f.txt" ]; then cp "$S/batches/b-$cur/mutants.out/$f.txt" "$R/current/$f.txt"; else : > "$R/current/$f.txt"; fi
    done
  else
    echo "none" > "$R/current/CURRENT.txt"
    for f in caught missed timeout unviable; do : > "$R/current/$f.txt"; done
  fi
  for f in caught missed timeout unviable; do
    : > "$R/$f.txt"
    for x in "$R"/batches/b-[0-9][0-9][0-9]; do [ -f "$x/$f.txt" ] && cat "$x/$f.txt" >> "$R/$f.txt"; done
  done
  {
    printf '{"job":"app-heart","shard":%s,"n":%s,"commit":"%s","cargo_mutants":"%s","cargo_nextest":"%s","batches":[' "$K" "$N" "$AH_C" "$AH_CMV" "$AH_NXV"
    first=1
    for x in "$R"/batches/b-[0-9][0-9][0-9]; do
      [ -f "$x/outcomes.json" ] || continue
      [ "$first" = 1 ] || printf ','
      first=0
      printf '{"batch":"%s","outcomes":' "${x##*/}"; cat "$x/outcomes.json"; printf '}'
    done
    printf ']}\n'
  } > "$R/outcomes.json"
  nm=$(ah_lines "$R/SHARD-LIST.txt"); nb=$(( (nm + AH_B - 1) / AH_B ))
  set -- $(cat "$R"/batches/b-[0-9][0-9][0-9]/DONE 2>/dev/null | awk '{ for (i = 1; i <= NF; i++) { split($i, a, "="); v[a[1]] = a[2] } d++; n += v["n"]; w += v["wall_s"] } END { print d + 0, n + 0, w + 0 }')
  db=$1; tn=$2; tw=$3
  c=$(ah_lines "$R/caught.txt"); m=$(ah_lines "$R/missed.txt"); t=$(ah_lines "$R/timeout.txt"); u=$(ah_lines "$R/unviable.txt")
  if [ "$tw" -gt 0 ] && [ "$tn" -gt 0 ]; then
    rate=$(awk -v n="$tn" -v w="$tw" 'BEGIN { printf "%.1f", n * 3600 / w }')
    eta=$(awk -v n="$tn" -v w="$tw" -v r="$((nm - tn))" 'BEGIN { printf "%.1f", r * w / n / 3600 }')
  else rate=-; eta=-; fi
  st=PARTIAL; [ -e "$S/ALL.done" ] && st=COMPLETE; [ -e "$S/STOP.txt" ] && st=STOPPED
  nbox=$(ah_lines "$R/boxes.txt")
  {
    echo "app-heart shard $K/$N: app commit $AH_C, crates store + pull, cargo-mutants $AH_CMV, cargo-nextest $AH_NXV, $AH_B mutants a batch"
    echo "updated $(ah_now) by box $BOXID; status $st"
    echo "batches done $db of $nb; mutants done $tn of $nm"
    echo "caught $c, missed $m, timeout $t, unviable $u"
    echo "rate $rate mutants/h (MEASURED: batch walls so far); remaining $((nm - tn)); ETA $eta h (ESTIMATE from that rate)"
    echo "current: $(cat "$R/current/CURRENT.txt")"
    [ -e "$S/STOP.txt" ] && echo "STOP: $(cat "$S/STOP.txt")"
  } > "$R/PROGRESS.txt"
  echo "SUMMARY app-heart shard $K/$N commit $AH_C cargo-mutants $AH_CMV nextest $AH_NXV mutants $nm tested $tn caught $c missed $m timeout $t unviable $u batch_wall_s $tw (MEASURED) boxes $nbox status $st" > "$R/SUMMARY.txt"
  STATUS="shard $K/$N batches $db/$nb tested $tn/$nm caught $c missed $m timeout $t unviable $u rate $rate/h eta ${eta}h"
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
  date +%s > "$LASTPUSH"
  ls -d "$S"/batches/b-[0-9][0-9][0-9]/DONE 2>/dev/null | wc -l | tr -d ' ' > "$PUSHED"
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
if [ "$(free_kib)" -lt "$AH_MINFREE_KIB" ]; then
  stopfile "disk: less than 4 GiB free on $AH after the sweep"
fi
[ -e "$S/STOP.txt" ] && finish STOP
[ -e "$S/ALL.done" ] && finish FINAL

# stalled batch: nothing written in its mutants.out for 90 min
cur=$(awk '{print $1}' "$S/current" 2>/dev/null)
if [ -n "$cur" ] && [ "$cur" != none ] && [ -d "$S/batches/b-$cur" ]; then
  cs=$(awk '{print $2}' "$S/current")
  if [ $(( $(date +%s) - cs )) -gt 5400 ] && [ -z "$(find "$S/batches/b-$cur" -type f -mmin -90 2>/dev/null | head -n 1)" ]; then
    stopfile "batch b-$cur: stalled, nothing written for 90 min"
    finish STOP
  fi
fi

# the worker
wpid=$(cat "$S/runner.pid" 2>/dev/null || echo 0)
alive=0
if [ "$wpid" != 0 ] && kill -0 "$wpid" 2>/dev/null; then
  if [ -r "/proc/$wpid/cmdline" ]; then
    tr '\0' ' ' < "/proc/$wpid/cmdline" | grep -q 'ah-run.sh' && alive=1
  else
    ps -p "$wpid" -o command= 2>/dev/null | grep -q 'ah-run.sh' && alive=1
  fi
fi
WNOTE="worker $wpid running"
if [ "$alive" = 0 ]; then
  echo "$(ah_now) start ${cur:-none}" >> "$S/starts.txt"
  if [ -n "$cur" ] && [ "$cur" != none ] && [ "$(grep -c " start $cur\$" "$S/starts.txt")" -ge 3 ]; then
    stopfile "batch b-$cur: the worker died 3 times in it"
    finish STOP
  fi
  SS=""; command -v setsid >/dev/null 2>&1 && SS=setsid
  # only the worker goes to the background, with every stream redirected, so this call never waits for it
  asru "cd '$RH' || exit 1; $SS nohup bash '$AH/bin/ah-run.sh' $K $N >> '$S/run.log' 2>&1 < /dev/null &" > /dev/null 2>&1 < /dev/null
  sleep 5
  WNOTE="worker (re)started"
fi

# wait, then commit and push progress (10 min after the last push, and at once after a finished batch), then report
while [ $(( $(date +%s) - T0 )) -lt "$TURN" ]; do
  if [ -e "$S/STOP.txt" ] || [ -e "$S/ALL.done" ]; then break; fi
  sleep 15
done
[ -e "$S/STOP.txt" ] && finish STOP
[ -e "$S/ALL.done" ] && finish FINAL
collect
last=$(cat "$LASTPUSH" 2>/dev/null || echo 0)
pdone=$(cat "$PUSHED" 2>/dev/null || echo 0)
ndone=$(ls -d "$S"/batches/b-[0-9][0-9][0-9]/DONE 2>/dev/null | wc -l | tr -d ' ')
if [ $(( $(date +%s) - last )) -ge "$PUSHEVERY" ] || [ "$ndone" -gt "$pdone" ]; then
  push_now "app-heart $K/$N: progress, $STATUS" || true
fi
echo "CONTINUE $STATUS | $WNOTE | $(tail -n 1 "$S/steps.log" 2>/dev/null | cut -c1-160) | ${PUSHNOTE:-no push this turn} | disk $(( $(free_kib) / 1048576 )) GiB free"
exit 0
```

<!-- FILE SHA256SUMS -->
```text
c6fb3de3a61185765a1af7a5b3775c7a93dbd30f006afa639a27e02c51a5f8e6  ah-common.sh
0d9d57c772a0fc4f73cca118b195f50f9c2c8a1dcad60928d4410821b61c3a86  ah-esc.awk
b2062f861dfe820f1063ff22856ce71e5f30d725476fffe0e2595225b9902ffa  ah-run.sh
eb427bd39a7401ccdd49ef477995339830838c93e33338039b1a66b4663b11f3  drive.sh
```

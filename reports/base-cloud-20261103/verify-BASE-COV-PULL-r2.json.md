# verify-BASE-COV-PULL-r2.json.md

Source: `state/fix/round2/verify-BASE-COV-PULL-r2.json` on the Mac. One fenced block below holds it byte for byte; its sha256 is in MANIFEST.md.

`````json
{
  "name": "BASE-COV-PULL",
  "verdict": "fail",
  "head": "c9d653d5f9145c4d82b77a261b7614db6987f0b1",
  "held": false,
  "needs_operator": false,
  "tip_moved": false,
  "blocking": [
    {
      "what": "THE BAR IS NOT HELD (operator decision D2 part A, CLAUDE.md section 9, recipe v4 step 4), reading whole:pull. Measured by Route B on this tip: lines 151,638/158,587 (6,949 uncovered), branches 11,522/14,216 (2,694 uncovered sides), 172 of the 196 files of the set with a gap. Production-named files alone (not *_tests.rs, not tests/, not base_cov_*): 3,065 uncovered lines (2,878 of them no assert/panic text) and 1,347 uncovered sides. The 54 #[ignore] real-data tests (L-BCP-3) leave their bodies uncovered too, and those lines are never exceptions. The fixer reports the same (NOT DONE item 1).",
      "where": "crates/pull (whole); per-file table and every uncovered line and side with its text: /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fix/round2/deep-BASE-COV-PULL-c9d653d5f914-gaps.txt",
      "reproduce": "W=/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925; take a deep dir with $W/state/decisions-20261027/d3-deep-take.sh, then $W/state/decisions-20261027/d3-route-b.sh $W/scratch/deep-BASE-COV-PULL $W/tgt/deep-<k> <new empty profile folder> <new log folder> $W/state/fix/round2/deep-BASE-COV-PULL-c9d653d5f914-set.txt pull ; grep '^COVERAGE .* TOTAL' in its output (this run: scratch/verify-BASE-COV-PULL-r2-logs/routeb.log)",
      "expected": "every line and every branch side of every file of crates/pull covered, except (F)/(U) items listed by file:line with their text and proof"
    },
    {
      "what": "Gaps left in production files this part itself changed: gdfl_import.rs 33 lines + 2 sides (cm_present's Ok(None) and Err at 2653-2654 and the side 2656:21 False; lineage.day's Err at 3126-3130; the warm-up's begin closure 3243-3247 and thread-spawn Err arm 3259-3267, 3416-3417; the day ledger append Err 3660-3669; 2570:24 False, 2578, 2834); fnowork.rs 8 lines (802-806 the rebuilt first-listed index event, 848-850 the not-asked event); gdfl_import_causes.rs 9 lines (380-383 report() arms StoreLost/FreeSpaceUnknown/SourceLost, the ? at 418, 497, 502, 566, the causes at 962 calendar_window_not_landed and 967 auction_sidecar); corporate_actions.rs:2090:34 False (Some(product) outside [low, high]); gdfl_run.rs lines 1358/1410, branches 157/170; master_snapshots.rs lines 1620/1673, branches 254/272",
      "where": "crates/pull/src/gdfl_import.rs:2653, crates/pull/src/fnowork.rs:802, crates/pull/src/gdfl_import_causes.rs:380, crates/pull/src/corporate_actions.rs:2090, crates/pull/src/gdfl_run.rs, crates/pull/src/master_snapshots.rs",
      "reproduce": "grep -E '^  UNCOVERED (LINE|BRANCH) crates/pull/src/(gdfl_import|fnowork|gdfl_import_causes|corporate_actions|gdfl_run|master_snapshots).rs:' /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/scratch/verify-BASE-COV-PULL-r2-logs/routeb.log",
      "expected": "each refusal and error arm driven by a test that asserts its named fault (a production refusal is never (F))"
    },
    {
      "what": "Production files with no new test at all, still far from the bar (lines a/b, branches c/d on this tip): zerodha_verify.rs 2628/2863, 493/614 (e.g. :218 `_ => return None,`, :303 `Err(why) => return Err(format!(\"{}: {why}\", path.display())),`); gdfl_dropbox.rs 1945/2133, 370/450 (:245 LinkMalformed's sentence); census_store.rs 2115/2295, 299/370 (:153, :160); gdfl_rekey.rs 1354/1479, 212/266 (:334); masters.rs 1597/1723, 163/202 (:767 `async fn get`); tickstore_write.rs 1003/1089, 167/188 (:201 `Err(refuse())`); gdfl_import_day.rs 1379/1476, 218/246 (:308, :411); fno_membership.rs 1556/1681, 209/258 (:307); ssm.rs 853/937, 74/116 (:308 `pub fn discover()`); response_keep.rs 1203/1269, 166/194 (:298); also cash_session_cache.rs 190 lines, ingest.rs 168, http.rs 162, cash_auction.rs 76, gdfl_archive.rs 69",
      "where": "crates/pull/src/zerodha_verify.rs:218, crates/pull/src/gdfl_dropbox.rs:245, crates/pull/src/census_store.rs:153, crates/pull/src/gdfl_rekey.rs:334, crates/pull/src/masters.rs:767, crates/pull/src/tickstore_write.rs:201, crates/pull/src/gdfl_import_day.rs:308, crates/pull/src/fno_membership.rs:307, crates/pull/src/ssm.rs:308, crates/pull/src/response_keep.rs:298",
      "reproduce": "grep -E '^COVERAGE crates/pull/src/(zerodha_verify|gdfl_dropbox|census_store|gdfl_rekey|masters|tickstore_write|gdfl_import_day|fno_membership|ssm|response_keep).rs TOTAL' /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/scratch/verify-BASE-COV-PULL-r2-logs/routeb.log",
      "expected": "lines a/a and branches c/c on each, with tests that drive and assert each arm"
    },
    {
      "what": "Two uncovered sides in this part's OWN new test files that are no failing arm (the same class as the first check's blocking 3): reference_data_cov_tests.rs:45:8 False, `if !body.is_empty() {` in the helper file(): no caller passes an empty list, so the branch is dead in a helper; ssm_cov_tests.rs:55:9 True, `set(\"AWS_ACCESS_KEY_ID\") && set(\"AWS_SECRET_ACCESS_KEY\")`: which side runs depends on this process's environment, so on this Mac the second operand never runs and the test reaches only one answer of from_env",
      "where": "crates/pull/src/reference_data_cov_tests.rs:45, crates/pull/src/ssm_cov_tests.rs:55",
      "reproduce": "grep -E 'UNCOVERED BRANCH crates/pull/src/(reference_data_cov_tests.rs:45|ssm_cov_tests.rs:55)' /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/scratch/verify-BASE-COV-PULL-r2-logs/routeb.log",
      "expected": "both sides reached (drop the dead branch in file(); drive the identity lookup through its lookup closure with both a present and an absent variable instead of the real environment)"
    }
  ],
  "reproduced": [
    {
      "id": "D2-A-measure",
      "ok": true,
      "evidence": "The before figures are on disk (state/fix/round2/base-cov-pull-gaps-a1.txt, 1,049,822 bytes, 27 Oct; scratch/deep-BASE-COV-PULL-logs/0b8a21a5bf49-cov-v2). Not re-run on f01d7a0e4 or 0b8a21a5b; my own Route B on c9d653d5f gives lines 151,638/158,587 and branches 11,522/14,216, against the fixer's 0b8a21a5b figures 147,430/155,508 and 11,275/14,190: 1,129 fewer uncovered lines and 221 fewer uncovered sides, consistent with the claim of partial progress."
    },
    {
      "id": "D2-A-close-gaps (partial)",
      "ok": true,
      "evidence": "168 new #[test] functions counted in the diff, exactly as D-7333 item 4 lists them (145 in the new files, 19 more in gdfl_import_cov_tests.rs, 3 in rehearsal_tests.rs, 1 in expiry_alias.rs). Gate run: cargo test -p pull --locked --no-fail-fast rc=0: 32 binaries, 2901 passed, 0 failed, 54 ignored, the fixer's count. The red-first hand edits (7 behaviour lines) were not repeated here: NOT PROVED by me."
    },
    {
      "id": "V1-blocking-2 measure_tick_store_within error arms",
      "ok": true,
      "evidence": "Route B on c9d653d5f: no UNCOVERED LINE or BRANCH in gdfl_import.rs anywhere in the OriginEntry impl, yearly_zips, holds_a_live_contract, walk_tick_store, measure_zips or space_of (lines 720-1080); gdfl_import.rs TOTAL lines 1737/1770, branches 214/216, the 33 lines and 2 sides left all elsewhere (blocking 2). The six named tests pass in the gate run. L-BCP-2 closed holds."
    },
    {
      "id": "V1-blocking-3 own test files' sides",
      "ok": true,
      "evidence": "The two named items are gone: the words() 'shared' parameter is removed (base_cov_codes_tests.rs, lines 423/423 covered, no branch), and no new test file holds .any( (grep prints nothing). But other sides of the part's new test files are no failing arm: reference_data_cov_tests.rs:45:8 False and ssm_cov_tests.rs:55:9 True (blocking 4)."
    },
    {
      "id": "gates claim (fmt, clippy, tests on 6eeee6a0d, no .rs after)",
      "ok": true,
      "evidence": "git diff --stat 6eeee6a0d c9d653d5f: docs/04, 05, 06 only. On c9d653d5f: fmt rc=0, clippy -p pull --all-targets -D warnings rc=0, test -p pull rc=0 (2901 passed, 0 failed, 54 ignored)."
    },
    {
      "id": "shadow GET claim (fixer :9721 on 6eeee6a0d)",
      "ok": true,
      "evidence": "Mine on :9821 from tgt/bin-BASE-COV-PULL-v/release (release built on c9d653d5f, rc 0), pid 29620 = lsof listener = test-server.pid: /feeds.json, /ingest/status.json, /masters/status.json, /calendar.json, /logs.json, /ingest, /db, /logs all 200 (n=200 each); /ingest/refusals.json without feed 400 by name, with feed=zerodha&symbols=NIFTY 200."
    }
  ],
  "gates": {
    "fmt": "cargo fmt --all --check rc=0 on c9d653d5f9145c4d82b77a261b7614db6987f0b1 (wt/P-BASE-COV-PULL, every tracked file touched under the pool-4 lock right before; log scratch/verify-BASE-COV-PULL-r2-logs/gate-fmt.log)",
    "clippy": "CARGO_TARGET_DIR=tgt/pool-4 cargo clippy -p pull --all-targets --locked -- -D warnings rc=0 on c9d653d5f (gate-clippy.log)",
    "tests": "CARGO_TARGET_DIR=tgt/pool-4 cargo test -p pull --locked --no-fail-fast rc=0 on c9d653d5f: 32 binaries, 2901 passed, 0 failed, 54 ignored (gate-test.log). Under Route B's instrumentation (opt-level 0, machine load) one timing test failed: reference_data::tests::the_loader_reads_every_table_inside_its_budget, p99 251,398 us against its 250 ms bound; not this part's test (followup)",
    "web_build": "not applicable (web/ unchanged: the diff touches crates/pull and docs only)",
    "web_test": "not applicable (web/ unchanged)"
  },
  "p50_p99": [
    "/feeds.json n=200 sequential on :9821: p50 1.0 ms, p99 17.8 ms, max 22.1 ms",
    "/ingest/status.json n=200: p50 0.6 ms, p99 12.8 ms, max 35.4 ms",
    "/masters/status.json n=200: p50 0.6 ms, p99 11.4 ms, max 24.6 ms",
    "/calendar.json n=200: p50 0.8 ms, p99 18.0 ms, max 25.2 ms",
    "/logs.json n=200: p50 2.6 ms, p99 35.0 ms, max 159.3 ms",
    "/ingest n=200: p50 0.7 ms, p99 15.4 ms, max 70.4 ms",
    "/db n=200: p50 0.7 ms, p99 5.6 ms, max 18.9 ms",
    "/logs n=200: p50 2.7 ms, p99 14.4 ms, max 21.2 ms",
    "/ingest/refusals.json?feed=zerodha&symbols=NIFTY n=200: p50 1.0 ms, p99 7.1 ms, max 12.3 ms",
    "20 concurrent GETs each: /ingest/status.json max 1.2 ms, /masters/status.json 3.3 ms, /logs.json 3.2 ms, /calendar.json 1.8 ms, /feeds.json 1.9 ms, all 200",
    "No hot path changed (the fixer measured nothing new; the seams sit on origin walks at a run's start, capped by ORIGIN_WALK_CAP); the walk under 20 concurrent callers and a changing tree: 400 walks, no panic",
    "D3 coverage pull route B: 396 min wall (02:35-09:11 IST): build 338 min of which almost all was the cargo slot queue (the build itself 37 s), tests 28 min, reading 30 min; deep dir wait 77 min (01:18-02:35 IST)",
    "Queue: the probe build waited 5,150 s for a slot, the gates runner 2,555 s, Route B's build about 337 min"
  ],
  "attacks": [
    "E1 combination, real tree: a link cycle, a hidden directory, an upper-case .BTS, a named pipe called p.bts, a link called link.bts, a zero-length .bts and an empty directory, measured at exactly its cap (9 entries): bytes 3, no fault, digest; at cap 8: 'more than 8 entries; the walk stopped there'. ok (a pipe or link named .bts is uncounted with no fault: pre-existing)",
    "E2 a real chmod 000 directory mid-tree: fault '<root>/ts/a: Permission denied', no digest. ok",
    "E3 roots of every shape: a file, a missing path, /dev/null, a path with a NUL byte, an empty path: each a named fault, no digest; a root that is a link to a directory is walked. ok",
    "E4 OUT OF THE BOX: a named pipe at FORMAT.md, and a named pipe at <yyyy>.zip: Origin::measure did not return within 4 s in either case (std::fs::read of FORMAT.md, File::open of the year): a hang with no event. Pre-existing lines (not changed by this part): followup",
    "E5 combination: 20 concurrent measures x 20 each while a writer removes and recreates 200 .bts files and their directories: 400 walks, 63 (then 36 on the rerun) ended on a fault naming the directory, the rest with a digest; bytes never above the tree's size; no panic. ok",
    "E6 combination: a directory named 2019.zip, a good 2020.zip, an empty 2021.zip, a dangling 2022.zip and a five-digit 20230.zip, under 20 concurrent measures: all 20 answers equal, the first fault is 2019.zip ('Is a directory'), no digest. ok (bytes 175 count the directory's own metadata length: pre-existing, minor)",
    "E7 volume_space on /dev/null, a missing volume, a NUL path and an empty path: each named, no panic; space_of with fragment u64::MAX and blocks 2 refused, with blocks 1 answered. ok",
    "E8 an old symbol's directory the run may not list (chmod 000 TATAMOTORS): split_underlyings holds the run [(TATAMOTORS, TMPV)]: unread is not proof of absence. ok",
    "E9 the rewritten gdfl_run::day_of against the old body, copied verbatim, on 120,021 words (12 years incl. 0000 and 9999 x every two-digit month and day, plus non-ASCII digits, NUL, signs, spaces, a trailing newline and a 1 MB word): every answer and every message identical. ok",
    "E10 fnowork::the_day_after against the two removed arms of owed, on 1,600 (held, through) pairs incl. the first five and last five representable days: identical. ok",
    "E11 corporate_actions: rows of one share apart in the file are refused at the row by name ('rows must ascend by symbol, each share's rows together'), so the order list holds each share once and the new or_default() inserts nothing; grouped rows load (ACO 3, BCO 2, an unknown share 0); indexed_steps of zero and one run is empty. ok",
    "E12 two writers at once, eight in fact: 8 threads snapshot 8 different bodies on one day beside a directory named like a snapshot, a name with a line break and a dot file: ordinals exactly 1..8, none lost or shared; the newest body again is AlreadyHeld(8). ok (my first form expected AlreadyHeld for every repeat, which the code rightly gives only for the day's newest, master_snapshots.rs:922)",
    "Shadow: garbage, wrong-case, negative, huge and NUL parameters on 20 URLs: each a named 400, a 414 for a 70,000-byte path, or 200 where the route ignores the field (/calendar.json?from=, /logs.json?limit=-1, a 'filter ignored' warn); feed=ZERODHA accepted case-blind (pre-existing api lines)",
    "Shadow: 20 concurrent GETs on each of five routes: all 200, max 3.3 ms",
    "Write paths: the diff changes no write call (no rename, fsync, create or write in any production hunk), so no write path is new; the snapshot writer was exercised only in temp dirs (E12), twice per body; the real store was never written (the shadow links bars read-only under the kernel lock, the probes write only under the system temp dir)"
  ],
  "followups": [
    "pull / gdfl_import origin measure (pre-existing lines, inside this part's whole:pull set): a named pipe at FORMAT.md (crates/pull/src/gdfl_import.rs:951, std::fs::read) or at a <yyyy>.zip (crates/pull/src/gdfl_import.rs:656, File::open) blocks Origin::measure forever with no event; refuse a non-regular file by name before opening it",
    "pull / gdfl_import: a <yyyy>.zip that is a directory adds the directory's metadata length to archive_bytes (measure_zips) while its fault is named; a .bts that is a named pipe or a link is uncounted with no fault (the fixer named the link case)",
    "pull / reference_data tests (R3-30, last touched by BASE-GATE11): reference_data::tests::the_loader_reads_every_table_inside_its_budget fails under Route B's instrumentation at opt-level 0 (p99 251,398 us, bound 250 ms) while it passes in the gate; every D3 run of pull can hit it",
    "L-BCP-3 needs the operator: the 54 #[ignore] real-data tests' bodies count as uncovered under the recipe, so the bar cannot hold as they stand (synthetic twins, a counted real-data pass, or another rule)",
    "D6 (split) still undecided: the gaps are about 6,949 lines and 2,694 sides, over three times the 2,000-line mark",
    "pull / gdfl_import.rs: note_unattributed now formats the day for every file even at booked 0 (one small allocation per file)",
    "pull / corporate_actions.rs builds: by_symbol.entry(symbol.clone()).or_default() would quietly make an empty shelf if order ever named a key by_symbol lacks; today the row grammar makes that impossible (E11), but the code no longer says so",
    "api (pre-existing): /calendar.json answers 200 and ignores fields it does not read (from=, day=), where /calendar-days.json refuses them by name; /ingest/refusals.json accepts feed=ZERODHA case-blind",
    "Coverage ratchet D-0677 (main session): this measure is pull alone (lines 95.6%, branches 81.0%); CI's floor is workspace-wide (--fail-under-lines 90 --fail-under-regions 89), so it does not by itself allow raising it",
    "Mutants: deferred (operator 30 Oct); line appended to state/fix/round2/d3-mutants-deferred.txt for c9d653d5f with set deep-BASE-COV-PULL-c9d653d5f914-set.txt"
  ],
  "coverage": "Route B (d3-route-b.sh, rustc 1.99.0-nightly (1a98b1e13 2026-08-07), llvm-cov and llvm-profdata LLVM 23.1.0-rust-1.99.0-nightly, -C instrument-coverage -Z coverage-options=branch, opt-level 0, new profile folder scratch/deep-BASE-COV-PULL-prof/c9d653d5f914-4-B, deep-4), tip c9d653d5f9145c4d82b77a261b7614db6987f0b1, reading whole:pull, set state/fix/round2/deep-BASE-COV-PULL-c9d653d5f914-set.txt (196 files, every pull .rs file the tool reports; the 8 files under crates/pull/benches are in no test object): lines 151,638/158,587 (6,949 uncovered), branches 11,522/14,216 (2,694 uncovered sides); 172 of 196 files with a gap; ROUTE-B DONE, EXIT 0, no NOT PROVED line. Per-file table and every uncovered line and side: state/fix/round2/deep-BASE-COV-PULL-c9d653d5f914-gaps.txt (log scratch/verify-BASE-COV-PULL-r2-logs/routeb.log). No (F)/(U) list was written for this tip: the fixer's state/fix/round2/base-cov-pull-fu.txt classifies 0b8a21a5b's figures and is provisional; with every item it calls (F) or (U) excused, thousands of production lines are still GAPs.",
  "mutants": "deferred (operator 30 Oct)",
  "d3": {
    "step": "check",
    "tip": "c9d653d5f9145c4d82b77a261b7614db6987f0b1",
    "reading": "whole:pull",
    "set_file": "/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fix/round2/deep-BASE-COV-PULL-c9d653d5f914-set.txt",
    "set_size": 196,
    "coverage_proved": true,
    "mutants_proved": false,
    "gaps": 9643,
    "survivors": 0,
    "timeouts": 0,
    "not_proved": 0,
    "not_applicable": false,
    "empty_set_evidence": "",
    "shards": "deferred (operator 30 Oct)",
    "mutants": "deferred (operator 30 Oct)"
  },
  "notes": "DRAIN CHECK printed OPEN. No earlier r2 progress file, so nothing was resumed. Tip checked c9d653d5f9145c4d82b77a261b7614db6987f0b1; feat/db-fix f01d7a0e4 is its merge base, so no merge was owed. Reading whole:pull (set override of the launcher; Merge BASE-COV-PULL: is not on feat/db-fix). d3.gaps 9,643 = the 6,949 uncovered lines plus the 2,694 uncovered sides, before any (F)/(U) is taken out; at 0b8a21a5b the fixer's classifier called 1,828 items (F) and 18 (U) of 9,147, so the true GAP count is lower but in the thousands either way.\n\nStatic review (B): every production hunk read against the old code. OriginEntry, yearly_zips, holds_a_live_contract, walk_tick_store, space_of, names_a_split, the cached-day rewrite, unattributed_event, stop_said's arguments, the_day_after, gdfl_run::day_of, indexed_steps as a zip, or_default in both corporate_actions builds and Catalog::read's extend are each equivalent to what they replace (E9, E10 and E11 prove three of them by test). No loop grows: the seams sit on origin walks at a run's start, capped. No write, rename or fsync changed. No look-ahead, no float price, Rust only, no .github or web change, no real vendor row (symbols ZQ*, INVENTED*, ACO; TATAMOTORS/TMPV/LTM are rows of the compiled lineage table, not vendor data). Every new test asserts. Docs: D-7330..D-7333 used, D-7334..D-7339 unused; BCP-01..BCP-07 name tests that exist; L-BCP-1 and L-BCP-3 open, L-BCP-2 closed.\n\nOrder: D, then E, then the deep dir was asked for; the gates of C were started in one queue slot while the deep dir was still being asked for (after D and E, on the same fixed tip, so the order changes no result) and finished at 02:54 IST; Route B ended 09:11 IST. The probe build waited 86 minutes for a slot, the gates runner 43 minutes, the deep dir 77 minutes and Route B's build about 5.6 hours.\n\nLeft in place (delete nothing): scratch worktree /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/scratch/verify-BASE-COV-PULL-r2 (detached at c9d653d5f, with the probe files crates/pull/src/vprobe_*.rs and their mod lines in five files, plus five stray untracked files named like 'crates/pull/src/gdfl_import.rs vprobe_import' left by a zsh word-splitting slip of mine, harmless); logs /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/scratch/verify-BASE-COV-PULL-r2-logs (release.log, probe.log, probe2.log, probe3.log, runner.sh, runner.log, gate-*.log, routeb.log, t-*.txt); profile /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/scratch/deep-BASE-COV-PULL-prof/c9d653d5f914-4-B; show files /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/scratch/deep-BASE-COV-PULL-logs/c9d653d5f914-cov; release bins /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/tgt/bin-BASE-COV-PULL-v/release; shadow /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/shadow/BASE-COV-PULL-v; the deep tree scratch/deep-BASE-COV-PULL is now detached at c9d653d5f. Probe scratch dirs under the system temp dir (brutex-gdfl-vp-*), one with a FIFO. New state files: round2/deep-BASE-COV-PULL-c9d653d5f914-set.txt (the fixer's 6eeee6a0de6f set, copied: no .rs differs), round2/deep-BASE-COV-PULL-c9d653d5f914-gaps.txt, one appended line in round2/d3-mutants-deferred.txt. Slip: one curl wrote its body to /tmp/vp.out; nothing else outside my folders. Locks: pool-4 taken and released twice, deep-4 and the tree lock taken and released. The real store and the tick store were never written, nothing was pushed, :8080 was not touched, no POST was sent, no process I did not start was signalled."
}
`````

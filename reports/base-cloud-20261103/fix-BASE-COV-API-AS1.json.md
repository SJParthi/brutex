# fix-BASE-COV-API-AS1.json.md

Session BASE-COV-API-AS1, shard G1 of BASE-COV-API repair round 3 (the third continuation, token BASE-COV-API-AS1-20261105-4). One fenced block below holds the FIX object.

```json
{
  "name": "BASE-COV-API-AS1",
  "branch": "cloud/cov-api-as1-20261105",
  "head": "4325492338f45405d8ba4bea05f1e16fd4549a2f",
  "committed": true,
  "findings": [
    {
      "id": "R3-ITEM-3 eight extremes for each test G1's brief names (D-8660)",
      "status": "fixed",
      "how": "detail.rs 8/2/1; store_route.rs 10/2/1; facets_tests.rs (with facets.rs) 10/3/1; emitted.rs (BCA-19) 8/2/1; audit.rs 8/2/1; audit_lock_wait_measure.rs 8/2/1; import_journal.rs end-line 8/2/1 and field-count 9/2/1; pullrun_restart_tests.rs open-runs 8/2/1 and walks 8/2/1 (singles / at once / out of the box, each asserted and named on the Extremes: line). Small-size timing wrappers call the full-size entry at its smallest size first, then assert every extreme through a helper that returns what it measured. sweeprun_accepted_tests.rs press tests untouched (verify r3: met).",
      "evidence": "commits c4969e7a and 87e75385; red: ten hand behaviour changes in a scratch tree, ten tests red at their new assertions (test result: FAILED. 0 passed; 10 failed); report section Blocking item (3) in the first session's record"
    },
    {
      "id": "R3-ITEM-1 G1's GAP rows (D-8661)",
      "status": "partial",
      "how": "First session: 33 gap-closing tests over 11 new cover modules and existing test modules. First continuation (token -2, 5295f342 and b85ecf4d): 17 new tests, test-only failing forms, 41 (U) rows with proofs. Second continuation (token -3, test code only, 7bd8a33a..5f028785): 18 new tests. facets: deltas by expiry, an unread contract counted once through apply, the cache helpers driven directly, refusals through answer() (one contract under two spellings is rows_incomplete), a second builder held by the test pause, kick over refused reads. chain: a ledger record struck at 0 paisa (unread), an alias record, mark's arms, first_stored_from over month files, /chain.json over a whole and a non-UTF-8 ledger, a listed put, two instants of one spot month, render_instant past its strikes. settings_route: saves naming the key they move, a save nested in a read (the further reader kept), ist_text at i64::MAX, i64::MIN and 0, the feed table's window and the charter's credential fields asserted. audit_json: a torn journal, a folder at the journal's path, numbered and worded pages, committed_at over unstatable census paths. store_route: a delta at 9999-12, since over an absent and a two-part census, an unreadable census and quarantine ledger, a poisoned behind cache, a day refused for two kinds. recovery: assess past the identity check over invented fixtures (a Zerodha master with an INDICES row; a share mapped under zerodha_cross_checked by a Zerodha cash row and a Groww witness with the universe table's ISIN; source months by the store's own writer; sessions from the calendar of record), start over five windows with reconcile_pending, page_html in every state, latest_view, the seal, successor, envelope and pointer refusals, a share's withdrawn days. assets: content_addressed and newest_under's bound. Test code: census_cover_tests' three panicking match arms as let-else, one audit_json and two recovery test assertions reading their own failing arm. 31 new (U) rows with proofs (22 lines, 9 sides). Route B whole:api on 5f028785, G1's 87 files: lines 60,195/60,999 (804 uncovered), branches 4,293/4,986 (693 uncovered sides): GAP production 429 lines 172 sides (702/277 at b85ecf4d, 821/366 at 3f324ee8), GAP test 77/77 (86/86), GAP ignored 37/0, HOST 51/10, (F) 139/397, (U) 71/37. The bar is not met. Third continuation (token -4, test code only, 1e71e821..43254923): 15 new tests and one door added to served_routes' crowded doors. audit_json: a census whose every epoch will not stat and the store block over GDFL import journals; detail: with_verified's race held open by publishing from inside the refresh; indexmap: names of nothing, verbatim names the alias table did not choose, INDEX keys with contracts; credential_law: re-reads with nothing sent or a dead value remembered (a new cover module declared at the end of credential_law.rs); page_report: note driven whole in a child with no log; backtest: appendable over every reason; chain: an option first stamp and a spot read over month files whose non-tail checksum block was changed after writing, and /chain.json refused admission; facets: a kick while a build is pending; pullrun: run_pass over a document with no feed row; bars: the merge arms a direct call reaches and a spent budget refusing a page's extremes; recovery: the 100,000-unit bounds (plan over 477 legs, a seal of 100,001 scans, the shared index held at 100,000 units in memory), a unit of two symbols, minutes outside the audited shape, successor preparation and activation refusals, a start refused a slot or an activation, an unsealed plan, a pointer to a plan gone or no file, the page under a full store-read pool (in a child). 33 carried GAP rows proved (U) from the code (13 lines, 20 sides: bars.rs merge-plan locals and runs/slots written only by SeriesPlan::build, pullrun walk positions and zipped outcomes, recovery pointer rows after active_history, indexmap's per-vendor slot table). G1 production GAP 342 lines and 110 sides at 43254923 (429 and 172 at 5f028785): 74 lines and 42 sides closed by tests, 13 and 20 proved (U). The bar is not met: 225 lines and 51 sides are L-BCA-5, the rest race, fault and bound arms and recovery's remaining reachable arms, each named in gaps-AS1.txt.md.",
      "evidence": "reports/base-cloud-20261103/gaps-AS1.txt.md and fu-AS1.txt.md at code tip 43254923 (every row with its text and class); the Route B log of 7 Nov 15:54:32-16:23:36 IST in the session scratchpad (rb-20261107-102425/routeb.log and its figures files); report sections What the third continuation did, Coverage and Not done"
    },
    {
      "id": "R3-EXTREMES the second continuation's new tests: eight extremes per tested file",
      "status": "fixed",
      "how": "Each new test names its asserted cases on its Extremes: doc line; per file over the new tests (singles / at once / out of the box): facets.rs 35/8/4, chain.rs 24/6/3, settings_route.rs 12/3/2, store_route.rs 13/4/2, recovery.rs 61/10/5, assets.rs 8/2/1; audit_json.rs 7/2/1 in its one new test, beside the first session's four tests in the same module. Three tests sit below eight singles alone (4, 7 and 6); their files clear the rule and every case they hold is asserted. census_cover_tests names the four cases two of its tests asserted and did not name. Third continuation: every new test 8/2/1 on its Extremes: line (bars 10/2/1); per file recovery.rs 32/8/4 as written. Three lines over-name a case (the start test names a start asked twice that it does not run and an at-once plan kept it does not assert; the pointer test names a run state it does not assert; the bounds test's plan one window under is 40 under): counted by what each asserts, start 8/1/1, pointer 7/2/1, bounds 8/2/1, recovery.rs 31/7/4, so every file clears the rule; the wording is corrected in the follow-up commit.",
      "evidence": "report table New tests, each with its Extremes: doc line"
    },
    {
      "id": "R3-RED every new test shown red under a hand change",
      "status": "fixed",
      "how": "First session: 33 production hand changes at once in a scratch tree at 3f324ee8, all 33 gap-closing tests red at their targeted assertions, reverted. First continuation: 17 hand behaviour changes, all 17 new tests red, reverted. Second continuation: 16 hand behaviour changes at aed9488e (test result: FAILED. 0 passed; 16 failed) and 2 at ec865d10 (0 passed; 2 failed), built with Route B's flags and run over exactly the new tests, each red at the assertion its change targets; reverted by git checkout, git status empty. Later commits changed no assertion (Extremes: doc lines, three lint attributes, Duration::from_mins). Third continuation: 16 hand behaviour changes in four runs over 1e71e821, 9456e268, 759dc617 and 43254923's tests (6 of 6, 4 of 4, 2 of 2, 4 of 4 red at their targeted assertions; run 3 also the chain test's spot arm, which run 2's change did not reach), reverted, git status showing the tests alone.",
      "evidence": "report table Red under a hand change (test, file:line of the change, the change, where it went red)"
    },
    {
      "id": "R3-HOST the final runs' reds are this root 4-core host's",
      "status": "needs_mac",
      "how": "Second continuation: Route B 13 reds, the same 13 names as the first continuation's run, each among the first session's proven host reds (failing the same way at START, rb0-start); the stable run 12 lib reds, the same twelve; the first continuation's two other stable reds (the autopilot tick and the G3 millisecond collision in logs::r3_24_tests) passed this time. None is in a file the second continuation changed. Owed to the Mac (14 cores, non-root). Third continuation: Route B 13 reds and the stable run's 12 lib reds, the same names as both earlier continuations' runs; pool_load's red passed in the stable run.",
      "evidence": "report section Host reds of this continuation's runs; logs/rb-5f028785-2.log and logs/stable-test.log in the session scratchpad"
    },
    {
      "id": "R3-DECISIONS D-8660 and D-8661 have no ledger entry yet",
      "status": "needs_assembly",
      "how": "cargo test -p core --test decision_citations --locked names exactly D-8660 first cited at crates/api/src/audit.rs:5372 and D-8661 first cited at crates/api/src/aliases.rs:497; ASA writes both entries from the report's drafts (D-8661 gains items 5-9 for the first continuation and 10-14 for the second).",
      "evidence": "report section Decision drafts; every_cited_decision_heads_an_entry_or_is_named_pending output"
    },
    {
      "id": "R3-FINDING audit::Journal::look reads a folder at the journal's path as a held journal",
      "status": "needs_assembly",
      "how": "audit.rs:1448-1466 takes its state from one metadata call and never asks is_file(): a folder at the pull journal's path is Log::Held with records = len / 256 (16 for ext4's 4,096-byte folder) and no trouble sentence. The read that follows is refused (Is a directory) and /audit.json says so in feed_scope.error, last_runs_error and runs_error, so the failure is not hidden, but present true, records 16 and trouble null misstate a folder as a journal. One-arm fix: Ok(meta) if !meta.is_file() => Log::Unreadable { reason: \"not a regular file\" }, with a test that fails first. Not fixed: a production fix needs its own decision id and AS1's block (D-8660, D-8661) holds none spare; for the main session or ASA.",
      "evidence": "audit_json::cover_tests::a_torn_journal_is_said_a_numbered_page_is_silent_and_an_unstatable_census_is_named (asserts the refusal in all three places, not the folder's count); report section A finding, not fixed"
    },
    {
      "id": "R3-LISTS the gap and (F)/(U) lists read the 11 test files AS1 added",
      "status": "fixed",
      "how": "The first session's lists read SHARDS.md's 76 files only, so the failing arms of its 11 new *_cover_tests.rs files were never listed; the first continuation's lists and this continuation's read all 87.",
      "evidence": "gaps-AS1.txt.md per-file table (87 files); fu-AS1.txt.md"
    },
    {
      "id": "R3-FOLLOWUP the third continuation's own test-code forms",
      "status": "partial",
      "how": "Three Extremes: lines over-name a case (counted by what each test asserts in the report), and chain_route_tests.rs 1314:9 False is a test-code GAP of this continuation's own making (the named helper's && whose false side only a failing assertion reaches). The follow-up commit corrects the wording, asserts the start test's plan kept, reads the helper as an array, and is measured again by Route B.",
      "evidence": "report sections New tests and Not done; gaps-AS1.txt.md row GAP test BRANCH crates/api/src/chain_route_tests.rs:1314:9 False"
    }
  ],
  "files_changed": [
    "crates/api/src/aliases.rs",
    "crates/api/src/assets.rs",
    "crates/api/src/audit.rs",
    "crates/api/src/audit_json.rs",
    "crates/api/src/audit_json_commit_tests.rs",
    "crates/api/src/audit_json_cover_tests.rs",
    "crates/api/src/audit_lock_wait_measure.rs",
    "crates/api/src/backtest.rs",
    "crates/api/src/bars.rs",
    "crates/api/src/calendar_build.rs",
    "crates/api/src/candidatejson.rs",
    "crates/api/src/candidatejson_cover_tests.rs",
    "crates/api/src/census.rs",
    "crates/api/src/census_cover_tests.rs",
    "crates/api/src/chain.rs",
    "crates/api/src/chain_cover_tests.rs",
    "crates/api/src/chain_route_tests.rs",
    "crates/api/src/credential_law.rs",
    "crates/api/src/credential_law_cover_tests.rs",
    "crates/api/src/credential_law_tests.rs",
    "crates/api/src/detail.rs",
    "crates/api/src/detail_cover_tests.rs",
    "crates/api/src/emitted.rs",
    "crates/api/src/facets_tests.rs",
    "crates/api/src/feed_checks_files.rs",
    "crates/api/src/feed_checks_files_cover_tests.rs",
    "crates/api/src/feed_checks_tests.rs",
    "crates/api/src/folder.rs",
    "crates/api/src/import_journal.rs",
    "crates/api/src/indexmap.rs",
    "crates/api/src/isolated.rs",
    "crates/api/src/merge.rs",
    "crates/api/src/merge_cover_tests.rs",
    "crates/api/src/page_report_route_tests.rs",
    "crates/api/src/pullrun.rs",
    "crates/api/src/pullrun_cover_tests.rs",
    "crates/api/src/pullrun_restart_tests.rs",
    "crates/api/src/recovery.rs",
    "crates/api/src/recovery_cover_tests.rs",
    "crates/api/src/recovery_journal.rs",
    "crates/api/src/recovery_journal_cover_tests.rs",
    "crates/api/src/repair_route.rs",
    "crates/api/src/repair_route_cover_tests.rs",
    "crates/api/src/served_routes_tests.rs",
    "crates/api/src/settings_route_tests.rs",
    "crates/api/src/store_route.rs",
    "crates/api/tests/state_stream_emits.rs"
  ],
  "files_outside_ownership": [],
  "gates": {
    "fmt": "cargo fmt --all --check rc 0 at 43254923 (each code commit formatted and checked before it was made)",
    "clippy": "cargo clippy -p api --all-targets --locked -- -D warnings rc 0 (stable 1.97.1) on the tree of 43254923, the run after the last code change (it first found two slice clones and three lint expectations the shorter tests no longer needed; fixed before the commit)",
    "tests": "cargo test -p api --locked --no-fail-fast -- --skip a_full_calendar_pool_refuses_the_census_and_the_audit --skip spot_json_is_refused_by_name_when_every_store_read_slot_is_held (stable 1.97.1, dev profile, CARGO_PROFILE_DEV_DEBUG=0) on the clean tree at 43254923, 7 Nov 16:27:43-16:37:03 IST with the build: lib 3,074 passed, 12 failed (host reds), 16 ignored, 1 filtered out; bin unit 4, binary 4, cloud_mutants_bars 58 (1 filtered out), logs_r3_24 6, pool_load 5, state_stream_emits 4, store_step_forms 3, doc tests 1",
    "tests_route_b": "the same suite instrumented under Route B at 43254923: 3,158 passed, 13 failed (host reds, the same 13 names as both earlier continuations), 16 ignored",
    "red_new_tests": "16 hand behaviour changes in four runs over the third continuation tests: 6 of 6, 4 of 4, 2 of 2, 4 of 4 red at their targeted assertions; reverted, git status showing the tests alone",
    "decision_citations": "cargo test -p core --test decision_citations --locked at 43254923: red until ASA lands D-8660 and D-8661 (D-8660 first cited at crates/api/src/audit.rs:5372, D-8661 at crates/api/src/aliases.rs:497); nothing else",
    "gate_23_print_sites": "the diff 53d7eec7..43254923 adds no print, println, eprint, eprintln, dbg, stdout or stderr site (searched); every child proves it ran by a file its parent reads",
    "no_py_language": "cargo test -p vocab --test no_py_language --locked: 9 passed (stable 1.97.1) over the tree holding this report and both lists"
  },
  "p50_p99": [
    "D3 coverage api route B on 43254923 (the third continuation): 29 min (7 Nov 15:54:32-16:23:36 IST; build 2 min, tests 5 min, readout 21 min), this cloud container (Linux x86_64, 4 cores, root)",
    "D3 coverage api route B on 5f028785 (the second continuation): 28 min (6 Nov 10:45:37-11:13:57 IST; build 2 min, tests 5 min, readout 20 min), this cloud container (Linux x86_64, 4 cores, root)",
    "D3 coverage api route B on b85ecf4d (the first continuation): 34 min (5 Nov 20:45:57-21:20:04 IST; build 3 min, tests 8 min, readout 22 min), a cloud container (Linux x86_64, 4 cores, root)",
    "D3 coverage api route B on 3f324ee8 (the first session): 29 min (5 Nov 15:46:25-16:14:54 IST; build 2 min, tests 6 min, readout 19 min), a cloud container (Linux x86_64, 4 cores, root)",
    "no bench run: crates/api/benches/ is ASA's; the small-size timing tests' own lines below are smoke checks of the measurement paths, not bench figures",
    "smoke, not bench (the first session): one run each of the six small-size timing tests, <lib test binary> --exact <test> --nocapture --test-threads=1, stable 1.97.1 dev profile (optimized + debuginfo), a 4-core container as root, 11:05-11:12 UTC 5 Nov",
    "audit::tests::the_append_timing_runs_at_a_small_size, appends_timed at its smallest size: n 3, p50 6 us, p99 6 us, max 48 us",
    "audit::lock_wait_measure::the_lock_wait_measurement_runs_at_a_small_size, open_run at 1/2/8 writers: n 8 each, p50 25/79/463 us, p99 348/194/875 us, max 348/194/875 us (0 refused)",
    "server::import_journal::tests::the_end_line_timing_runs_at_a_small_size, end_lines_folded_beside(3): n 3, p50 2.662 us, p99 21.787 us, max 21.787 us",
    "server::import_journal::tests::the_field_count_timing_runs_at_a_small_size, 8 fields: n 2, p50 23.806 us, p99 23.806 us, max 23.806 us; 4,096 fields: n 2, p50 3.793 ms",
    "pullrun::restart_tests::the_open_runs_timing_runs_at_a_small_size, restart_scan 65 records 64 open: n 2, p50 244 us, p99 244 us, max 244 us; status_json 64 open: n 2, p50 277 us",
    "pullrun::restart_tests::the_walks_worst_case_timing_runs_at_a_small_size, restart_scan 864 run records every close a guess: n 2, p50 2,904 us, p99 2,904 us, max 2,904 us",
    "the continuations ran no bench and their tests time nothing (the second continuation's builder test bounds a second 1.5 s build under a 2.7 s ceiling, a behaviour bound, not a measurement); the first session's figures stand"
  ],
  "coverage": "Route B (kit d3-route-b-cloud.sh unchanged; rustc 1.99.0-nightly (1a98b1e13 2026-08-07), llvm-cov and llvm-profdata LLVM 23.1.0-rust-1.99.0-nightly; RUSTFLAGS -C instrument-coverage -Z coverage-options=branch, opt-level 0, llvm-profdata merge -sparse), reading whole:api on code tip 43254923, set = the kit 205 coverage files plus the 12 test files AS1 added (217), 7 Nov 2026 15:54:32-16:23:36 IST, exit 3: NOT PROVED exactly the 22 crates/api/benches/** files and crates/api/src/lib.rs (L-BCA-2). whole api 194 files read: lines 178,650/182,743 (4,093 uncovered), branches 13,072/15,594 (2,522). G1 88 files: lines 61,508/62,240 (732), branches 4,342/5,000 (658); the 76 SHARDS.md files alone 57,970/58,686 (716) and 4,288/4,910 (622). Classified: GAP production 342 lines 110 sides, GAP test 75/81, GAP inside ignored tests 37/0, HOST 51/10, (F) 143/400, (U) 84/57; the bar is not met",
  "mutants": "deferred (operator 30 Oct)",
  "store_writes_needed": [],
  "notes": "STATUS IN PROGRESS: G1 GAP rows remain that a cloud session can close (recovery.rs' remaining reachable arms, the test-code forms), besides the items owed to the Mac and the L-BCA-5 projections. THIRD CONTINUATION (token BASE-COV-API-AS1-20261105-4, 7 Nov 2026): took the branch at 53d7eec7 (owner 9bfdb6a0), changed test code only (1e71e821, 9456e268, 759dc617, 43254923; no production line moved); Route B whole:api on 43254923 7 Nov 15:54:32-16:23:36 IST, exit 3 with NOT PROVED exactly the 22 benches and lib.rs (L-BCA-2); G1's 88 files lines 61508/62240, branches 4342/5000; whole api 178650/182743 and 13072/15594. A follow-up is begun and not committed: three Extremes lines corrected, the start test asserting its plan kept, chain_route_tests.rs' named helper read as an array (its 1314:9 False side is a test-code GAP of this continuation's own making); then Route B again. One slip: one shell command carried a stray shell variable whose name held the banned language's name; nothing ran through it and no file holds it. settings_route.rs 967-971 is a collision arm (the temporary name carries the pid, a private sequence and the clock's nanoseconds), not the reachable save the second continuation's Not done took it for. STATUS IN PROGRESS: G1 GAP rows remain that a cloud session can close (recovery.rs' durable state machine, settings_route.rs' leftover-cleared save, chain.rs' first-stamp walk, bars.rs' merge-plan arms), besides the items owed to the Mac and the L-BCA-5 projections. FIRST SESSION: commit messages corrected in the report: c4969e7a says store_route 11 singles (it is 10); 3f324ee8 says pullrun.rs (7) rewritten match arms (it is 8, 15 in all); its edit tree checked START out once, to prove a host red, and returned to 3f324ee8; nothing of it was committed. FIRST CONTINUATION (token BASE-COV-API-AS1-20261105-2): took the branch at 3b58556d (owner file bf435421), test code only, code tip b85ecf4d. SECOND CONTINUATION (token BASE-COV-API-AS1-20261105-3, 6 Nov 2026): took the branch at e033f06e (owner file 12205098), test code only (every production-file hunk inside its mod tests), code tip 5f028785; Route B whole:api, the stable gates, both lists and the report are of 5f028785; the red proofs were taken at aed9488e and ec865d10. Slips, said plainly: one shell command of each continuation carried a stray version query of the interpreter the brief bans (output discarded, nothing ran through it, no file holds it); three Route B attempts of the second continuation were stopped and none of their figures is used (a worktree run whose test objects later builds in this checkout replaced, one before clippy's fix moved the tip, one whose uplifted debug/api binary the stable test gate replaced); the worktree it made was removed. Scratch left in place by the second continuation (never in the repository): /home/user/as1-k (its one build tree), /tmp/as1-prof-* profile folders, the session scratchpad.",
  "held": false
}
```

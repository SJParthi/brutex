# fix-BASE-COV-PULL-PSA.json.md

The launcher FIX shape for session BASE-COV-PULL-PSA; `head` is the code tip (`git rev-list -1 HEAD -- . ":(exclude)reports"`), every finding of the package listed.

```json
{
  "name": "BASE-COV-PULL-PSA",
  "branch": "cloud/cov-pull-psa-20261103",
  "head": "3f77a7b0fa6012c9982d29dc5833d0a2bc8ae534",
  "committed": true,
  "findings": [
    {
      "id": "V2-blocking-1 D2-A-bar",
      "status": "not_fixed",
      "how": "The bar is NOT held. Route B whole:pull (run 2) on the code tip's .rs (cef062ebf194): lines 175,299/179,878 (4,579 uncovered), branches 12,622/14,464 (1,842 sides), 255 files; every uncovered row classed: (F) 2,231, (U) 364, IGNORED 3,798 (L-BCP-3), UNMEASURED 11 (L-BCP-5), GAP 17 (listing_tests.rs:490 macOS only, the Mac's; ssm.rs 966/970/971 after a signed send reached AWS, the operator's; 13 rows of upkeep.rs and upkeep_tests.rs, MS1's files). BEFORE (the second check, c9d653d5f): 6,949 lines and 2,694 sides uncovered. D-7250's narrow reading does NOT end for pull.",
      "evidence": "reports/base-cloud-20261103/gaps-PSA.txt.md, reports/base-cloud-20261103/base-cov-pull-fu.txt.md; docs/05 D-7338, docs/06 L-BCP-1"
    },
    {
      "id": "V2-blocking-2 own production files",
      "status": "fixed",
      "how": "Every arm the second check named in the part's own production files is covered or listed (U) with its proof at the tip (run 2): gdfl_import.rs lines 1769/1770, branches 214/214 (its one row (U)); gdfl_import_causes.rs 349/349, 6/6; fnowork.rs 871/871, 62/62; corporate_actions.rs 1000/1000, 166/166; gdfl_run.rs 1419/1425, 166/170 and master_snapshots.rs 1677/1679, 263/270 (every row left (U) or (F)). Driven by PS1's and PS2's seams and tests (D-7335, D-7336).",
      "evidence": "reports/base-cloud-20261103/gaps-PSA.txt.md (per-file table, rows by class)"
    },
    {
      "id": "V2-blocking-3 production files with no new test",
      "status": "fixed",
      "how": "Every file the second check named has no GAP left (run 2): zerodha_verify.rs 2859/2872, 596/614; gdfl_dropbox.rs 2145/2145, 447/448; census_store.rs 2329/2340, 366/372; gdfl_rekey.rs 1500/1511, 258/268; masters.rs 1742/1765, 184/204; tickstore_write.rs 1121/1129, 184/188; gdfl_import_day.rs 1475/1482, 246/246; fno_membership.rs 1686/1692, 249/260; response_keep.rs 1317/1322, 191/200; ssm.rs 912/939, 86/114 (its three GAP rows the operator's, the rest (U) or (F) in its test module); cash_session_cache.rs 1455/1615 (IGNORED 168, (F) 27); ingest.rs 3269/3286; http.rs 4431/4501; cash_auction.rs 870/927 (IGNORED 57); gdfl_archive.rs 1530/1597 (IGNORED 71): every remaining row (U), (F), IGNORED or GAP as listed.",
      "evidence": "reports/base-cloud-20261103/base-cov-pull-fu.txt.md"
    },
    {
      "id": "V2-blocking-4 own test files' sides",
      "status": "fixed",
      "how": "reference_data_cov_tests.rs:45 and ssm_cov_tests.rs:55 are covered: ssm_cov_tests.rs 159/159, 2/2; reference_data_cov_tests.rs 418/418 lines, its two open sides (391:9 and 391:37 False) an assertion's failing sides, (F).",
      "evidence": "reports/base-cloud-20261103/gaps-PSA.txt.md"
    },
    {
      "id": "D2-A-measure",
      "status": "fixed",
      "how": "Route B whole:pull measured on the assembled code: run 1 on a52270b1f00a (lines 175,277/179,855, branches 12,614/14,454) and run 2 on cef062ebf194 (lines 175,299/179,878, branches 12,622/14,464), the kit's d3-route-b-cloud.sh unchanged on the pinned nightly, under the four conditions of D-7339.",
      "evidence": "reports/base-cloud-20261103/deep-BASE-COV-PULL-PSA-cef062ebf194-set.txt.md, reports/base-cloud-20261103/deep-BASE-COV-PULL-PSA-a52270b1f00a-set.txt.md"
    },
    {
      "id": "D3-coverage",
      "status": "not_fixed",
      "how": "coverage_proved: every file of the set measured and read (ROUTE-B DONE, exit 0, no NOT PROVED line, both runs); the bar is not held: 17 GAP rows (none a cloud session may close), IGNORED 3,798 and UNMEASURED 11 rows.",
      "evidence": "reports/base-cloud-20261103/base-cov-pull-fu.txt.md"
    },
    {
      "id": "D3-mutants",
      "status": "needs_operator",
      "how": "deferred (operator 30 Oct). No cargo-mutants ran. The set files keep their mutants lines (1,952 at cef062ebf194, the D2 set of f01d7a0e4...cef062ebf194) for the background pass.",
      "evidence": "reports/base-cloud-20261103/deep-BASE-COV-PULL-PSA-cef062ebf194-set.txt.md"
    },
    {
      "id": "L-BCP-3 real-data ignored tests",
      "status": "needs_operator",
      "how": "54 ignored tests; 3,798 rows (3,520 lines, 278 sides) inside their bodies or helpers only they reach, over 27 files. 12 of the 23 in G2's files are timings on invented data that a pass with --ignored would count. The operator chooses synthetic twins, a real-data pass that counts, or another rule.",
      "evidence": "docs/06-limits.md L-BCP-3"
    },
    {
      "id": "L-BCP-4 arms that need a seam",
      "status": "partial",
      "how": "Every arm the limit named is driven by the shards' seams and RUST_MIN_STACK children but ssm::get_parameter's send half: ssm.rs 966, 970, 971, which need an endpoint argument for the signed read, the operator's call (not added: a second endpoint for a credential read).",
      "evidence": "docs/06-limits.md L-BCP-4; docs/05 D-7335 item 7"
    },
    {
      "id": "L-BCP-5 lines of a child killed by SIGKILL",
      "status": "needs_operator",
      "how": "New limit: cash_session_cache_lock_tests.rs 377-388 (11 rows), the child of a_killed_holder_releases_the_day, run but write no profile (the kill is the property under test; flushing a profile needs a call unsafe_code forbids). PS3 listed them (U); the package's list holds them UNMEASURED.",
      "evidence": "docs/06-limits.md L-BCP-5"
    },
    {
      "id": "PS1-step1-compile",
      "status": "fixed",
      "how": "Repair 2's tests compile; clippy's three errors in repair 2's tests fixed with no behaviour change (PS1, 91de72cb).",
      "evidence": "PS1 report; docs/05 D-7335 item 1"
    },
    {
      "id": "PS1-step1-failures",
      "status": "fixed",
      "how": "The ten root-only reds of PS1's first run: repair 2's own one runs where the bits bind (PS1); the nine old ones were moved there by PS2 (d16d4ca0) and PS3 (d418a3ec). The assembled suite has 0 failures as root.",
      "evidence": "stable run on cef062ebf194: 34 binaries, 3607 passed, 0 failed, 54 ignored"
    },
    {
      "id": "PS1-green-tip",
      "status": "fixed",
      "how": "GREEN-TIP 2e7a0b6d3b568190024e2e9b405e803805f3fdca (PS1 report line 2), the FROM of PS2 and PS3.",
      "evidence": "PS1 report"
    },
    {
      "id": "PS1-step2-G1-coverage",
      "status": "partial",
      "how": "G1 (91 files) at run 2: lines 57,738/59,292, branches 4,193/4,954; (F) 854, (U) 138, IGNORED 1,306, GAP 17 (the Mac 1, the operator 3, MS1 13); its kill-test pair closed by this assembly.",
      "evidence": "reports/base-cloud-20261103/gaps-PSA.txt.md"
    },
    {
      "id": "PS1-race-collision-arms",
      "status": "fixed",
      "how": "The import race's judgement is a helper driven by three collisions made on purpose (BCP-15).",
      "evidence": "docs/04 BCP-15"
    },
    {
      "id": "PS1-chance-arms",
      "status": "fixed",
      "how": "PS1 made three chance-only pieces of test code deterministic; the fourth, the kill test's unborn-month pair, is closed here by Stop::CutInCreation (a kill inside a month's creation written on purpose and resumed; no production hook).",
      "evidence": "gdfl_import_tests.rs cut_in_creation; docs/05 D-7338 item 3(c)"
    },
    {
      "id": "PS1-defect-fno-membership-overflow",
      "status": "fixed",
      "how": "The month-row decoder asks each u32 count alone, never sums them: a refusal, never a panic (BCP-13).",
      "evidence": "docs/04 BCP-13; docs/05 D-7335 item 4"
    },
    {
      "id": "G2-bar",
      "status": "partial",
      "how": "G2 (93 files) at run 2: lines 58,766/60,545 (1,779 uncovered), branches 4,140/4,724 (584 sides); (F) 623, (U) 146 (21 on a private-field type invariant, for the operator), IGNORED 1,594, GAP 0.",
      "evidence": "reports/base-cloud-20261103/gaps-PSA.txt.md"
    },
    {
      "id": "G2 FAULT and RACE rows",
      "status": "fixed",
      "how": "Driven through the #[cfg(test)] seams of D-7336 (steps between two host calls, planted host faults).",
      "evidence": "docs/05 D-7336"
    },
    {
      "id": "PS2 red checks of every new test",
      "status": "fixed",
      "how": "234 of 234 shown red at their own assertion (PS2's red-PS2.txt on its branch).",
      "evidence": "PS2 report"
    },
    {
      "id": "PS2 host reds",
      "status": "fixed",
      "how": "The six G3 root-only reds PS2 met run where the bits bind since PS3's d418a3ec; none fails as root at the assembly.",
      "evidence": "stable run on cef062ebf194"
    },
    {
      "id": "D-7336 (decision draft)",
      "status": "fixed",
      "how": "Written into docs/05-decisions.md from PS2's draft.",
      "evidence": "docs/05 D-7336"
    },
    {
      "id": "PS3-G3-bar",
      "status": "partial",
      "how": "G3 (71 files) at run 2: lines 58,795/60,041 (1,246 uncovered), branches 4,289/4,786 (497 sides); (F) 754, (U) 80, IGNORED 898, UNMEASURED 11 (L-BCP-5), GAP 0.",
      "evidence": "reports/base-cloud-20261103/gaps-PSA.txt.md"
    },
    {
      "id": "PS3-tenor-outside",
      "status": "fixed",
      "how": "tenor.rs 203-204 covered by vendor::tests::a_tenor_refused_for_unverified_hours_quotes_the_refusal (BCP-20), red when tenor.rs quoted the venue instead of the refusal.",
      "evidence": "crates/pull/src/vendor.rs; docs/04 BCP-20"
    },
    {
      "id": "PS3-host-reds",
      "status": "fixed",
      "how": "G3's six root-only reds run through where_permission_binds (D-0995).",
      "evidence": "PS3 d418a3ec"
    },
    {
      "id": "PS3-defect-staging-lock",
      "status": "fixed",
      "how": "gdfl_dropbox StagingLock unlocks on drop (BCP-18).",
      "evidence": "docs/04 BCP-18; docs/05 D-7337 item 4"
    },
    {
      "id": "PS3-defect-day-lock",
      "status": "fixed",
      "how": "tickstore_write DayLock unlocks on drop (BCP-18).",
      "evidence": "docs/04 BCP-18"
    },
    {
      "id": "PS3-defect-rolling-words",
      "status": "fixed",
      "how": "A roll past the calendar's last day says so (BCP-19).",
      "evidence": "docs/04 BCP-19"
    },
    {
      "id": "PS3-test-archive-root",
      "status": "fixed",
      "how": "The http test keeps no capture under $HOME/.brutex: an invented base URL.",
      "evidence": "docs/05 D-7337 item 4"
    },
    {
      "id": "PSA-assembly",
      "status": "fixed",
      "how": "PS1, PS2, PS3 stacked by replay-minus-reports.sh, P1-P4 holding at each call: ASSEMBLED dac5db855377bd0880854aa7826b4de35e179be3, 174 files over START in 94 commits, no merge; every shard file equals its tip's.",
      "evidence": "reports/base-cloud-20261103/report-BASE-COV-PULL-PSA.md, step (1)"
    },
    {
      "id": "PSA-gate-1d-words",
      "status": "fixed",
      "how": "The 648 segment-shaped literals the package added under crates/pull (156 by attempt 1 and repairs 1-2, 492 by the shards), each read at its producing literal, declared as BASE_COV_PULL_LITERAL; gate 1d names exactly feat/db-fix's own 481 (BCP-21; test gate_1d_declares_the_base_cov_pull_words).",
      "evidence": ".github/gates_tree.rs; docs/04 BCP-21"
    },
    {
      "id": "PSA-kill-point",
      "status": "fixed",
      "how": "PS1's request for a store pause point is not needed: Stop::CutInCreation writes what a kill inside a month's creation leaves and judges and resumes it; red with the store's repair of an existing interrupted genesis switched off by hand; green five times in a row.",
      "evidence": "crates/pull/src/gdfl_import_tests.rs; docs/05 D-7338 item 3(c)"
    },
    {
      "id": "PSA-docs",
      "status": "fixed",
      "how": "D-7335 to D-7339 written; BCP-08..12 flipped to ✓ (their fifteen tests pass), BCP-13..21 added, BCP-02 names its twelve tests (gate 10 refused it); L-BCP-1/-3/-4 restated, L-BCP-5 added. Gate 10 refuses only feat/db-fix's R427-11, HA-02, AG05-13; gate 10b and gate 15 pass; gate 27b names only feat/db-fix's D-6247.",
      "evidence": "docs/04, docs/05, docs/06"
    },
    {
      "id": "PSA-route-b-root-host",
      "status": "fixed",
      "how": "D-7339: a cloud Route B figure counts uid-65534 children only with TMPDIR 1777, a 0777 profile folder, umask 0000 for the script and 0022 for cargo, and kernel.pid_max raised; both runs held all four (62 of 132 profiles by uid 65534).",
      "evidence": "docs/05 D-7339"
    },
    {
      "id": "PSA-ssm-send",
      "status": "needs_operator",
      "how": "ssm.rs 966, 970, 971: an endpoint argument for the signed Parameter Store read, so a test can answer it from a loopback listener; production would pass the URL it builds from the region.",
      "evidence": "reports/base-cloud-20261103/base-cov-pull-fu.txt.md GAP rows"
    },
    {
      "id": "PSA-upkeep-ms1",
      "status": "needs_operator",
      "how": "upkeep.rs (1200:16 False, 1580:20 True, 1581, 1676:20 False) and upkeep_tests.rs (460-462, 1001:9 False, 1001:47 True, 1657:41 False, 1741-1743): files BASE-MSUM-BACKFILL owns and is changing; closed by whichever of MS1 and this part lands second.",
      "evidence": "reports/base-cloud-20261103/base-cov-pull-fu.txt.md GAP rows"
    },
    {
      "id": "PSA-macos-row",
      "status": "needs_operator",
      "how": "listing_tests.rs:490, the Darwin errno arm: the Mac's Route B, never claimed here.",
      "evidence": "reports/base-cloud-20261103/base-cov-pull-fu.txt.md GAP rows"
    },
    {
      "id": "PSA-feat-db-fix",
      "status": "fixed",
      "how": "origin/feat/db-fix fetched at the end: f01d7a0e430e8f9b01e424a299b0bed20738a0ef, an ancestor of the branch; it has not moved past the base, so nothing was merged.",
      "evidence": "git fetch origin feat/db-fix"
    }
  ],
  "files_changed": [
    "crates/pull/src/vendor.rs",
    "crates/pull/src/gdfl_import_tests.rs",
    ".github/gates_tree.rs",
    "docs/04-invariants.md",
    "docs/05-decisions.md",
    "docs/06-limits.md",
    "reports/base-cloud-20261103/OWNER-BASE-COV-PULL-PSA.md",
    "reports/base-cloud-20261103/report-BASE-COV-PULL-PSA.md",
    "reports/base-cloud-20261103/fix-BASE-COV-PULL-PSA.json.md",
    "reports/base-cloud-20261103/gaps-PSA.txt.md",
    "reports/base-cloud-20261103/base-cov-pull-fu.txt.md",
    "reports/base-cloud-20261103/deep-BASE-COV-PULL-PSA-a52270b1f00a-set.txt.md",
    "reports/base-cloud-20261103/deep-BASE-COV-PULL-PSA-cef062ebf194-set.txt.md"
  ],
  "files_outside_ownership": [
    "crates/pull/src/vendor.rs (G1, PS1's file: one test, settling PS3's need)",
    "crates/pull/src/gdfl_import_tests.rs (G1, PS1's file: test code only, closing PS1's kill-test pair)"
  ],
  "gates": {
    "fmt": "cargo fmt --all --check rc 0 on cef062ebf194 (the code tip's .rs)",
    "clippy": "cargo clippy -p pull --all-targets --locked -- -D warnings rc 0 on cef062ebf194 (stable 1.97.1)",
    "tests": "cargo test -p pull --locked --no-fail-fast rc 0 on cef062ebf194 (stable 1.97.1, root, 4 cores, 17:16:16-17:23:06 IST): 34 binaries, 3607 passed, 0 failed, 54 ignored; the same on the tree of a52270b1f00a",
    "route_b_tests": "instrumented (opt-level 0) suite in both Route B runs: 3607 passed, 0 failed, 54 ignored",
    "gates_tree": "rustc --edition=2024 --test -D warnings .github/gates_tree.rs: 51 passed; rustfmt --edition 2024 --check clean",
    "gate_1d": "481 undeclared, exactly feat/db-fix's own set (1129 before the declarations; 637 at START)",
    "gate_10": "refuses only what feat/db-fix refuses (R427-11; HA-02, AG05-13)",
    "gate_10b": "OK",
    "gate_27b": "only feat/db-fix's D-6247 twice",
    "gate_15": "OK",
    "web_build": "not applicable (web/ unchanged)",
    "web_test": "not applicable (web/ unchanged)"
  },
  "p50_p99": [
    "D3 coverage pull route B run 1 (a52270b1f00a): 39 min (build 1, tests 4, read 33); run 2 (cef062ebf194): 40 min (build 1, tests 4, read 35); this host's figures, 4 cores",
    "no bench run: no timed step added or changed"
  ],
  "coverage": {
    "route": "B (the kit's d3-route-b-cloud.sh unchanged; rustc 1.99.0-nightly (1a98b1e13 2026-08-07); llvm-cov and llvm-profdata LLVM 23.1.0-rust-1.99.0-nightly); a root cloud host under D-7339",
    "reading": "whole:pull",
    "set": "reports/base-cloud-20261103/deep-BASE-COV-PULL-PSA-cef062ebf194-set.txt.md (255 coverage lines, 1952 mutants lines)",
    "before": "c9d653d5f (second check): lines 151638/158587, branches 11522/14216",
    "run1": "a52270b1f00a: lines 175277/179855 (4578 uncovered), branches 12614/14454 (1840 sides)",
    "tip": "cef062ebf194 (run 2): lines 175299/179878 (4579 uncovered), branches 12622/14464 (1842 sides)",
    "groups_tip": {
      "G1": "lines 57738/59292, branches 4193/4954 (91 files)",
      "G2": "lines 58766/60545, branches 4140/4724 (93 files)",
      "G3": "lines 58795/60041, branches 4289/4786 (71 files)"
    },
    "classes_tip": {
      "F": 2231,
      "U": 364,
      "IGNORED_L_BCP_3": 3798,
      "UNMEASURED_L_BCP_5": 11,
      "GAP": 17
    },
    "lists": [
      "reports/base-cloud-20261103/gaps-PSA.txt.md",
      "reports/base-cloud-20261103/base-cov-pull-fu.txt.md"
    ],
    "coverage_proved": true,
    "not_proved": 0,
    "gaps": 17
  },
  "mutants": "deferred (operator 30 Oct)",
  "store_writes_needed": [],
  "notes": "STATUS: BLOCKED OUTSIDE. ASSEMBLED dac5db855377bd0880854aa7826b4de35e179be3 (PS1, PS2, PS3, each by replay-minus-reports.sh, P1-P4 held). The bar is NOT held: L-BCP-3 (3,798 rows), L-BCP-5 (11 rows), 17 GAP rows: the Mac's macOS row, the operator's signed-SSM-send rows, MS1's upkeep rows. NEEDS THE MAC: Route B whole:pull on its pin, the shadow GET re-check, macOS rows, non-root 14-core runs, L-BCP-3, the D1 landing. head is the docs commit 3f77a7b0, whose .rs are those of cef062ebf194, the tip the final gates and Route B run 2 measured. The local refs stack/psa-1 and stack/psa-2 were never pushed.",
  "held": false
}
```

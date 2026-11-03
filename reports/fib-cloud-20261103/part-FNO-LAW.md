# Part FNO-LAW

## Cloud build, Tue 3 Nov 2026 (this section supersedes the round-1 hold below)

- **held:** false. **Status:** built and ready for its independent check; nothing
  counts until the Mac's check passes it.
- **Where:** a cloud session (`CLOUD.md`), branch `cloud/fib-fno-law-20261103`,
  pushed; cut from `origin/cloud/fib-kit-20261103b` (`fa9022ba` = FNO-SEAM's
  passed tip `1ad6e24ef8e0` plus the kit folder). Never merged, no pull
  request, no other branch touched.
- **Base:** `1ad6e24ef8e0`, FNO-SEAM's passed tip. FNO-LAW depends on FNO-SEAM
  alone, so this is the final base, **not speculative**, and no catch-up is
  owed.
- **The part's commits** (`git rev-list --no-merges 1ad6e24ef8e0..HEAD`, less
  the kit commit `fa9022ba`, which holds only `reports/fib-cloud-20261103/`):
  `106be414`, `698f48dc`, `3c6b244e`, `387f5707`, `1039b1ff`, and the commit
  that adds this file and `report-FNO-LAW.md`. Each message ends with the line
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- **Drain test and disk floor:** not applicable: they name the Mac's drive, and
  the cloud container has none (`CLOUD.md` adaptation 2). No store, no
  `runtime/`, no app and no `:8080` were read, written, started or needed.

### Files (the design's list, each at its own slot or as the sole owner)

| File | Change |
|---|---|
| `CLAUDE.md` | §1 item 3 (the third surface, the label on one line), the equity-authority sentence naming FNO-COSTS's dated clock, one walk-forward sentence, §3 rule 3's identity-domain paragraph (FNO-SEC-WIRE) |
| `AGENTS.md` | the mirror: the same sentences in §1 and §3 rule 3 |
| `docs/00-charter.md` | the scope lock's fourth row and paragraph (beyond the design's list, so the charter does not contradict the new §1); the new section after CHG-1 with the 25 cost rows and the 8 execution facts |
| `docs/05-decisions.md` | D-8100 to D-8108 under `<!-- slot FNO-LAW -->`; D-8109 unused |
| `docs/04-invariants.md` | FNO-LAW-I01 to FNO-LAW-I04 under its slot |
| `docs/07-plan.md` | the plan row under its slot |
| `crates/core/tests/fno_law.rs` | new, the D2 set |

Every ledger edit is one pure insertion directly under FNO-LAW's slot line
(`git diff -U0 1ad6e24ef8e0`: one hunk, no line removed, in each of the three
ledgers). A scratch commit inserting rows under FNO-COSTS's slot in the same
three files merges with this branch with no conflict (`git merge-tree
--write-tree`, exit 0).

### Functions and tests (`crates/core/tests/fno_law.rs`)

Readers: `fold`, `section`, `slot_heading`, `law_problems`, `fence`, `heading`,
`entries`, `slots`, `slot_problems`, `op_numbers`, `block_problem`,
`outside_problem`, `ops_problem`, `ledger_problems`, `row`, `charter_problems`,
`law_slot`, `rows_problems`. Fixture builders: `law_text`, `without`,
`ledger_text`, `slot_text`, `charter_text`, `rows_texts`, `without_op`,
`swap_op`.

Tests (10 of 10 green, stable and Route B):
`the_law_holds_the_third_surface_the_label_and_the_universe_file_rule`,
`the_ledger_heads_d_8100_to_d_8108_under_the_law_slot_and_no_other_number_of_its_block`,
`the_charter_names_every_cost_row_and_every_execution_source_once_in_order`,
`the_law_rows_sit_under_their_slots_and_cite_only_tests_this_file_declares`,
`a_law_text_missing_any_one_phrase_or_misplacing_the_label_is_refused_by_name`,
`a_ledger_missing_moving_doubling_or_fencing_an_entry_is_refused`,
`a_heading_inside_a_fence_or_of_the_wrong_shape_heads_nothing`,
`a_slot_section_missing_its_heading_or_a_slot_is_refused`,
`a_charter_missing_doubling_or_reordering_a_row_is_refused_by_id`,
`a_row_citing_a_test_this_file_lacks_or_an_empty_slot_is_refused`.
What each proves, case by case, is `report-FNO-LAW.md` §2.

### coverage

**Measured here by Route B, against the final base `1ad6e24ef8e0`, not
speculative.** Nightly `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`
(`nightly-2026-08-08`), `RUSTFLAGS='-C instrument-coverage -Z
coverage-options=branch'`, `CARGO_PROFILE_DEV_OPT_LEVEL=0`, `cargo test -p core
--locked`, `llvm-profdata merge` and `llvm-cov report` over the D2 set:

`crates/core/tests/fno_law.rs`: **lines 586 of 586 (100.00%), branches 88 of 88
(100.00%)**, regions 1,160 of 1,160, functions 62 of 62. No zero-count marker
in `llvm-cov show`. The documents hold no instrumented region. The D3 recipe
file itself is on the Mac and was not read: this followed `CLOUD.md`'s
statement of Route B, and the Mac re-measures.

### gates

- `cargo fmt --all -- --check`: clean.
- `cargo clippy -p core --all-targets --locked -- -D warnings`: clean.
- `cargo test -p core --locked`: green, every binary; also green under Route B.
- Tests outside `core` that read an edited file: `vocab` all green; `store`'s
  readers green (`docs`, `libm_key`, `bisect_cost`, `sealed_codec_alloc`,
  `tail_proof`); `runner`'s `d_0694_record` green; `pull`'s charter checksum
  test green; `cli`'s `crate_graph_claims` (2 of 2), `pool_and_escape_docs`
  (11 of 11) and `c4_cli_02_limits` (3 of 3) green.
- Gates 27 and 14: OK. Gates 27b, 10 and 12: red on the base and here with the
  same refusals, none of them this part's (D-6247 twice; R427-11 and
  STACK1028-03; 87 unproven claims elsewhere). Gate 10 checks 10 more named
  tests here than at the base, all resolving. As the main session's §7.5 (c)
  says, gate 10's crate-first half is re-run after BASE-REDS-1023 lands.
- Gate 30's tool self-tests (they read the ledger): 38 of 38.
- Benches: none, by design. Mutation tests: deferred by the operator.
  `cargo deny check`: not installed here; no dependency or manifest changed.

### decisions drafted (own block only)

D-8100, D-8101, D-8102, D-8103, D-8104, D-8105, D-8106, D-8107 (it records the
docs end-of-file rule of check-FNO-SEAM-r3 §7.5 (a), as R2 asks), D-8108.
D-8109 unused.

### problems

- Not this part's, reproduced on the base: gate 27b (D-6247 twice), gate 10
  (R427-11, STACK1028-03), gate 12 (87 blocks), and `store`'s
  `cited_commits` test, which refuses `1a98b1e13` in `docs/06-limits.md`'s
  L-STACK1028 section (a file this part does not touch).
- Host, not code: five `store` unit tests need a non-root user (this container
  runs as root; `settings_tests.rs:619` says so itself).
- A lapse in this session: one shell command ended with a version probe of the
  Py-language's interpreter, its output discarded. Nothing used it and no
  tracked file names it. Reported, not hidden.

### findings

- PARTS's "25 dated cost rows (ids EX-2009 .. GST-SEBI)": in table order that
  range is 19 rows; the table's 25 run BRK-1 .. OT-INT. All 25 are written.
- T06-03's flex reads 5% a step in SPEC §10.2 and 5%, then 3%, then 2% in
  PARTS's FNO-BAND; the charter quotes both, for FNO-BAND to settle.
- SPEC §16 still lists 0.312 and 0.65 as missing rungs; the operator's
  31 Oct decision says they are not rungs (D-8103).
- The sha256 values and the URLs of T02-02, T03-04, T09-07, T10-05, T06-03 and
  T11-09 are in Mac-only files; the charter cites the files and copies no
  value it did not see.

### refused_files

None.

---

## Round 1 record (30 Oct 2026 23:37 IST): held by the 2 TB disk floor

The text below is the round-1 note as it stood, kept whole.

Status: held by the 2 TB disk floor. Nothing was built.

Checked: 2026-10-30 23:37 IST, before any write, as the task's DISK FLOOR rule requires.

Command and output (`df -k /Volumes/WD_BLACK`):

```
Filesystem   1024-blocks       Used  Available Capacity   iused       ifree %iused  Mounted on
/dev/disk7s1  7813821744 6564863368 1248122536    85% 156630972 12481225360    1%   /Volumes/WD_BLACK
```

Arithmetic (shell `$(( ))`):

- Available: 1,248,122,536 KiB
- Less 50 GiB headroom for this build: 1,248,122,536 - 52,428,800 = 1,195,693,736 KiB
- Floor (2 TB): 1,953,125,000 KiB
- 1,195,693,736 < 1,953,125,000, so the build must not start. Shortfall: 757,431,264 KiB (about 722 GiB).

Not done (all of it):

- No worktree created (`wt/FIB-FNO-LAW`, branch `fix/fib-fno-law` do not exist from this run).
- No edits to CLAUDE.md, AGENTS.md, docs/00-charter.md, docs/05-decisions.md, docs/04-invariants.md, docs/07-plan.md.
- No `crates/core/tests/fno_law.rs`.
- No cargo command run, so no target dir grown (`tgt/fib-shared`, `tgt/fib-shared-deep` untouched by this run).
- No coverage, fmt, clippy or test evidence; this part has no benches by design.
- Nothing deleted to make room (the task forbids it).

To resume: free space must reach at least 1,953,125,000 + 52,428,800 = 2,005,553,800 KiB available on WD_BLACK (by the operator's choice of what to move; nothing may be deleted by this build), then rerun the part from the df check.

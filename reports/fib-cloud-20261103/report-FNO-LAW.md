# FNO-LAW: cloud build report (Tue 3 Nov 2026)

Every result and page of the F&O strategy path carries the label
**"current F&O members only (survivorship bias not removed)"**. This report is
not a backtest result.

## 1. Result

- **Status: built.** Every file of the design is written, every gate named in
  `CLOUD.md` adaptation 4 was run, and the touched set is at **100% of lines
  and 100% of branches by Route B, measured here** with the Mac's own nightly.
  The Mac's independent check re-measures before anything counts.
- **Branch:** `cloud/fib-fno-law-20261103`, cut from
  `origin/cloud/fib-kit-20261103b` (`fa9022ba`, which is FNO-SEAM's passed tip
  `1ad6e24ef8e0` plus this folder). Pushed after every step.
- **Base for the D2 set and for coverage:** `1ad6e24ef8e0` (FNO-SEAM, passed).
  FNO-LAW depends on FNO-SEAM alone, so this base is final, not speculative.
- **The part's own commits** (`git log --oneline 1ad6e24ef8e0..HEAD`, less the
  kit commit `fa9022ba`): `106be414` (CLAUDE.md, AGENTS.md), `698f48dc` (the
  charter), `3c6b244e` (the ledger rows), `387f5707` (the test), `1039b1ff`
  (the test's last regions), and the commit that adds this report. Each ends
  with the `Co-Authored-By` line.
- **D2 set** (`git diff --name-only 1ad6e24ef8e0..HEAD -- crates/`):
  `crates/core/tests/fno_law.rs` alone, new. The other six files are documents
  and hold no instrumented region.

## 2. What was built, file by file

`git diff --stat 1ad6e24ef8e0..HEAD -- . ':(exclude)reports'`: 7 files, 1,503
insertions, 7 deletions.

### `CLAUDE.md`

- §1: "two shapes" becomes "three shapes", and **item 3** is the cash shares
  of the operator's current F&O list on the F&O strategy path, widened by
  D-8100: the list held as a data file, never as a list in code (`fno218.txt`,
  218 names when fixed), the blake3 of its bytes in every run identity of the
  path; not `FNO_UNDERLYINGS` (13 and 3 names apart, measured below); leavers
  not added to the traded set or the rank pool; point-in-time membership
  within, a day the membership cannot answer not traded; ICICIPRULI joined to
  ICICILIFE at read time by a join map that is data; signals on Zerodha
  candles; fills on GDFL 1-second bars at the worst price printed in the second
  (D-8101); two cost clocks, the worse per trade the headline (D-8102); and the
  label, word for word, on one line.
- The equity-authority sentence ("until a charter-sourced equity charge stack
  exists") now names the stack: FNO-COSTS's dated cash-equity intraday clock
  (D-8102), every rate a row of the charter, and a result it has not priced
  stays gross and labelled. One sentence adds that on the F&O path the
  out-of-sample validation is walk-forward, always (D-8105). The three
  sentences `core::universe`'s test pins in §1 (the 208, the 210, the
  `FNO_UNDERLYINGS` rule) are unchanged, word for word.
- §3 rule 3 gains one paragraph: the F&O path's own identity domain, built by
  FNO-SEC-WIRE as the sole owner of `runner::identity` (`fno_run_digest` over
  `FnoRunV1`, `fno_identity` over `FnoCandidateV1`, each under its own
  append-only tag), the names of the READINESS ADDENDUM's item 2. The first
  `blake3(...)` span of the file, which `vocab`'s own test compares with its
  doc, is unchanged.

### `AGENTS.md` (the mirror)

One paragraph in §1, after the futures paragraph, in the same sentences as
`CLAUDE.md` item 3 and the authority sentence, plus "on this path membership is
point-in-time, never the snapshot above" (its §1 says the tables are a
snapshot); the same §3 rule 3 paragraph. No line removed.

### `docs/00-charter.md`

- The scope lock: "three shapes"; a **fourth row** (the F&O path's shares, a
  data file, `D-8100`); a paragraph with the measured difference from
  `FNO_UNDERLYINGS`; and "Nothing outside the two symbols above is ever swept"
  corrected to "the rows above" with a note of what it said. The design names
  only source rows for the charter; the scope lock was edited too because the
  charter has authority over scope (`CLAUDE.md` §10) and would otherwise
  contradict the new §1.
- **A new section after the CHG-1 rate sources** (an interior anchor, so it
  cannot collide with other work's end-of-file appends): "F&O strategy path:
  dated cash-equity charges and execution facts — read 30 Oct 2026". Its intro
  says how the rows were obtained and that **no sha256 is copied** (they are in
  the Mac-only `costs-evidence/sha256-official.txt`, cited by path). Then:
  - **25 cost rows**, every row of `costs-table/2026-10-30/r1` in the table's
    order: BRK-1, BRK-SQ, BRK-DP, BRK-CT, EX-2009, EX-2020C, EX-2021, EX-2023,
    EX-2024A, EX-2024B, EX-2026, IP-OLD, IP-2023, IP-2026, SF-2017, SF-2019,
    SF-2020, SF-2021, STT-ND, SD-STATE, SD-ND, GST-1, GST-SEBI, OT-CLR,
    OT-INT; each with component, rate, base and side, from, to, clocks,
    document, URL, quote, read date (2026-10-30) and status, copied from the
    kit's `costs-table.json.md`.
  - **8 execution facts**, each with its fact, its source as recorded, status
    and the part that applies it: T02-02 (NSE's `C_VAR1_DDMMYYYY_1.DAT`
    margin files and the leverage rule), T03-01 (Dhan's 15:10 square-off),
    T03-05 (Rs 20 + GST a square-off order), T03-04 (the 15:08 rejection
    window), T09-07 (the order-rate ceilings), T10-05 (the 3% market
    protection), T06-03 (the band rule), T11-09 (the closing-auction session
    ends).

### `docs/05-decisions.md`, under `<!-- slot FNO-LAW -->`

D-8100 (the path and its universe), D-8101 (one fill model and rule 6, with
N4's quote range), D-8102 (two cost clocks and rule 7; names FNO-COSTS as the
stack), D-8103 (rules 2 and 3 as law; bits 121 to 131 untouched; `TOL_FIB_MILLI`
one point), D-8104 (no vocabulary widening), D-8105 (walk-forward always and
the one goal; the 38 checks; the closest recipes), D-8106 (1 day decided on D,
traded on D+1; the old refusal unchanged), D-8107 (the slot and family process,
with the end-of-file rule of check-FNO-SEAM-r3 §7.5 (a) and gate 14's cover
table, §7.5 (b)), D-8108 (OP-01 to OP-46, one line each). **D-8109 is unused**,
and D-8107 says so.

### `docs/04-invariants.md`, under `<!-- slot FNO-LAW -->`

FNO-LAW-I01 (the law's §1 and rule 3), FNO-LAW-I02 (the ledger and the slot
lines), FNO-LAW-I03 (the charter), FNO-LAW-I04 (FNO-LAW's own rows). Each row
states only what `fno_law.rs` proves and cites its tests as `core::fno_law::…`,
the form gate 10 checks.

### `docs/07-plan.md`, under `<!-- slot FNO-LAW -->`

One row: what FNO-LAW holds, built on 3 Nov 2026, its independent check owed,
landing with F1. It holds no commit hash (`store`'s `cited_commits.rs` reads
this file).

### `crates/core/tests/fno_law.rs` (new, 931 lines, std only)

Readers (each pure, each run on the real documents and on invented text):
`fold`, `section`, `slot_heading`, `law_problems`, `fence`, `heading`,
`entries`, `slots`, `slot_problems`, `op_numbers`, `block_problem`,
`outside_problem`, `ops_problem`, `ledger_problems`, `row`, `charter_problems`,
`law_slot`, `rows_problems`. Fixture builders: `law_text`, `without`,
`ledger_text`, `slot_text`, `charter_text`, `rows_texts`, `without_op`,
`swap_op`.

The ten tests:

| Test | What it proves |
|---|---|
| `the_law_holds_the_third_surface_the_label_and_the_universe_file_rule` | `CLAUDE.md` §1 and `AGENTS.md` §1 hold all 11 phrases and the label byte for byte; rule 3 of both holds the 2 identity phrases; §1 is read whole |
| `the_ledger_heads_d_8100_to_d_8108_under_the_law_slot_and_no_other_number_of_its_block` | D-8100..D-8108 once each, in order, under the slot; no other number of the block anywhere; D-8108's 46 OP lines; the 24 slot lines in docs 04, 05, 06, 07; the reader reads over a thousand entries from D-0001 |
| `the_charter_names_every_cost_row_and_every_execution_source_once_in_order` | 25 cost rows and 8 facts, once each, in order; each cost row with an `https://` source and the read date; T02-02 with the margin file; the table version and the digest file named; the scope lock's D-8100 row |
| `the_law_rows_sit_under_their_slots_and_cite_only_tests_this_file_declares` | FNO-LAW-I01..I04 under the slot, each citing tests this file declares; the plan row under its slot |
| `a_law_text_missing_any_one_phrase_or_misplacing_the_label_is_refused_by_name` | each of the 13 phrases removed alone is named alone; every phrase rewrapped still passes; the label capitalised, upper-cased, broken over two lines, or only after §1 is refused; no §1, a §1 never closed, no rule 3 |
| `a_ledger_missing_moving_doubling_or_fencing_an_entry_is_refused` | each of the 9 entries removed alone (D-8108 also loses its OP lines), each of the 8 neighbouring pairs swapped, one doubled, the spare headed, an entry before the section, an entry under FNO-COSTS's slot, FNO-LAW's slot line lost (every entry outside), an OP line missing, doubled, swapped or malformed three ways |
| `a_heading_inside_a_fence_or_of_the_wrong_shape_heads_nothing` | a shorter run inside a longer fence, a backtick line inside a tilde fence, a tilde run with text after it, a backtick fence; five hashes, no space, one hash, five digits, a letter or `_` after, too short, not digits, not `D-`; an unfenced heading after them all is read |
| `a_slot_section_missing_its_heading_or_a_slot_is_refused` | the section heading missing; a slot missing, swapped, doubled |
| `a_charter_missing_doubling_or_reordering_a_row_is_refused_by_id` | each of the 25 cost rows and the 8 facts removed alone (T02-02's removal also loses the margin file), a swapped, a doubled and an unknown added row, a row with no link, a row with another read date, T02-02 without the margin file, the version and the digest file each missing, no scope-lock row, no section; a line with no closing bar is not a row |
| `a_row_citing_a_test_this_file_lacks_or_an_empty_slot_is_refused` | a row citing an undeclared test, a row citing another module's test, a row missing, an empty invariants slot, an empty plan slot, no slots at all |

**Why the block's numbers are written as numbers in the file.**
`decision_citations.rs` requires every `D-` and four digits under `crates/` to
head an entry, and the block's spare heads none: a literal of it in the test
would turn that test red. The file says so in its header.

## 3. Facts measured in this session (commands run here)

- `comm` of the kit's `fno218.txt.md` (218 non-empty lines, `sort | uniq -d`
  empty) against `FNO_UNDERLYINGS` less `FNO_INDEX_UNDERLYINGS`, from
  `crates/core/src/universe.rs:914` and `:1159` on this branch: 13 in the file
  and not the code (ANANDRATHI, ATHERENERG, COROMANDEL, CUB, DATAPATTNS, ENRIN,
  ICICILIFE, KARURVYSYA, MAHABANK, NAVINFLUOR, SAGILITY, SYRMA, UJJIVANSFB), 3
  in the code and not the file (BAJAJHLDNG, DALBHARAT, ICICIPRULI). Same as
  SPEC 3.1.
- `vocab::table::NEXT_FREE` = 370 (`crates/vocab/src/table.rs:1436`),
  `vocab::mask::WORDS` = 6 (`mask.rs:18`), `TOL_FIB_MILLI` = 10
  (`tolerance.rs:121`), bits 121-131 "Fibonacci over the current session,
  running" (`table.rs:21`, rows 441-451), `CurDayFib` (`indicators/src/lib.rs:406`),
  `Leg::Undetermined` (`:245`), `cli::EVERY_RUNG` with eight rungs and
  `swept_rung` refusing `1day` (`crates/cli/src/lib.rs:10226`), the ICICI
  rename held back in `core::alias` (`alias.rs:549-553`).

## 4. Gates

Toolchains: the pinned `1.97.1` (`rust-toolchain.toml`) for fmt, clippy and the
tests; `nightly-2026-08-08`, `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`, for
Route B (the build `docs/06-limits.md` names for the Mac's Route B).

| Gate | Command | Result |
|---|---|---|
| fmt | `cargo fmt --all -- --check` | clean (exit 0) |
| clippy | `cargo clippy -p core --all-targets --locked -- -D warnings` | clean |
| touched crate's tests | `cargo test -p core --locked` | green: 205 unit tests, every integration binary, `fno_law` 10 of 10, `decision_citations` 4, `graph` 15, `banned_lists` 2, `cost_invariants` 5, `findings` 14 |
| the same under Route B | the Route B command below, all of `core` | green, every binary |
| `vocab` (reads `CLAUDE.md`, the charter, `AGENTS.md`, the ledger, the invariants) | `cargo test -p vocab --locked --no-fail-fast` | green, every binary |
| `store` (reads the ledger, the invariants, the plan, `CLAUDE.md`) | `cargo test -p store --locked --no-fail-fast` | 39 of 40 integration binaries green, among them every one that reads an edited file (`docs`, `libm_key`, `bisect_cost`, `sealed_codec_alloc`, `tail_proof`), and the doc-tests; **two reds, the base's and the host's, not this part's** (§7) |
| `runner` `d_0694_record` (reads the ledger and the invariants) | `cargo test -p runner --locked --test d_0694_record` | green, 3 of 3 |
| `pull` charter checksum test (reads the charter) | `cargo test -p pull --locked --lib the_sourced_expiry_rows_carry_the_charters_checksum` | green, 1 of 1 (`reference_data::tests::…`) |
| `cli` doc readers | `cargo test -p cli --locked --no-fail-fast --test crate_graph_claims --test pool_and_escape_docs --test c4_cli_02_limits` | green: `crate_graph_claims` 2 of 2, `pool_and_escape_docs` 11 of 11 (reads the invariants and the ledger), `c4_cli_02_limits` 3 of 3 (reads the ledger) |
| gate 27 | `.github/gates_ledger.rs invariant-ids` | OK: 3,483 ids unique (base 3,479) |
| gate 27b | `.github/gates_ledger.rs decision-numbers` | rc 1 on the base and here, the same one refusal: D-6247 heads two entries (lines 81917 and 82161, not this part's); this part adds 9 headings, none doubled |
| gate 10 | the workflow's commands, `invariant_paths.rs` then `invariant-tests` | rc 1 on the base and here, the same two rows: R427-11 (the base row the main session's §7.5 (c) names) and STACK1028-03; 4,054 named tests checked here against 4,044 at the base, so FNO-LAW's 10 citations all resolve |
| gate 12 | `.github/gates_bounds.rs gate12` with the workflow's allowlist | rc 1 on the base and here, the same refusal list (87 blocks that name nothing, none in this part's file); only the counts move: one file, 35 (crate, fn) pairs and 58 doc blocks more, all this file's |
| gate 14 | `.github/gates_bounds.rs gate14` with the workflow's cover table | OK, identical to the base |
| gate 30's tool self-tests (read the ledger) | `rustc --edition=2024 --test -D warnings .github/gates_store_steps.rs` | 38 of 38 |
| merge with a sibling slot | a scratch commit inserting rows under FNO-COSTS's slot in docs 04, 05, 07, then `git merge-tree --write-tree` with this branch | exit 0, no conflict |

The gate commands are in `gates.sh` and the Route B ones in `routeb.sh`, both
in this session's scratchpad (not tracked); each gate was run at the base
(a worktree of `1ad6e24ef8e0`) and at the tip, and the logs were compared.

### 4.1 The two late builds

The `pull` and `cli` targets were still building when this report was first
written; both finished green and their rows above are the results.

## 5. Coverage (Route B, measured here)

```
RUSTFLAGS='-C instrument-coverage -Z coverage-options=branch' \
CARGO_PROFILE_DEV_OPT_LEVEL=0 CARGO_TARGET_DIR=<scratch> \
LLVM_PROFILE_FILE=<scratch>/prof/core-%p-%m.profraw \
cargo +nightly-2026-08-08 test -p core --locked
llvm-profdata merge -sparse <scratch>/prof/*.profraw -o core.profdata
llvm-cov report <fno_law test binary> --instr-profile=core.profdata \
  --show-branch-summary crates/core/tests/fno_law.rs
```

| File | Regions | Functions | Lines | Branches |
|---|---|---|---|---|
| `crates/core/tests/fno_law.rs` | 1,160 of 1,160 (100.00%) | 62 of 62 | **586 of 586 (100.00%)** | **88 of 88 (100.00%)** |

`llvm-cov show --show-regions --show-branches=count` prints no zero-count
marker for the file. The first measurement found 6 unrun regions and no
unrun line or branch; all six were closed (two `?` exits that cannot fire
became `split_at`, two reachable cases gained fixtures, and one assert lost its
message arguments). Not measured by the D3 recipe file itself, which is on the
Mac (`decisions-20261027/d3-recipe.txt`): this follows `CLOUD.md`'s statement
of it. The Mac's Route B re-measures.

## 6. Benches

None, by design (the part is documents and one test file).

## 7. Problems and findings

1. **Base reds, not this part's, each reproduced on the base worktree with the
   same output:** gate 27b (D-6247 twice), gate 10 (R427-11, STACK1028-03),
   gate 12 (87 unproven claims elsewhere), and `store`'s
   `cited_commits::every_commit_the_store_records_cite_is_on_main_or_says_why_not`,
   which refuses `1a98b1e13`, the nightly's commit hash written in
   `docs/06-limits.md`'s L-STACK1028 section (this part does not touch that
   file, `git diff 1ad6e24ef8e0 -- docs/06-limits.md` is empty, and the
   refusal names no other citation).
2. **Host, not code:** five `store` unit tests fail here because this cloud
   container runs as root, which reads a mode-000 directory and writes through
   a mode-555 one (`settings_tests.rs:619` says so in its own words: "this host
   reads a 000 directory (root?): the property cannot be shown here"; the
   quarantine tests set `0o555`). They read no document.
3. **The design's id range.** PARTS names the charter's cost rows "the 25
   dated cost rows (ids EX-2009 .. GST-SEBI ...)". In table order EX-2009 ..
   GST-SEBI is 19 rows; the table's 25 run BRK-1 .. OT-INT, and its own meta
   says "25 rows". The charter and the test take all 25.
4. **T06-03 has two readings in the kit.** SPEC §10.2 reads the band's flex as
   5% a step; PARTS's FNO-BAND names 5%, then 3%, then 2%. The charter row
   quotes both as they stand and points at the research file's lines; FNO-BAND
   settles it.
5. **SPEC §16's ladder row** still lists "0.312 and 0.65" as missing; the
   operator's decision of 31 Oct says they are not rungs. D-8103 records the
   decision.
6. **A lapse in this session, recorded plainly:** one shell command, typed
   while checking the ledger, ended with a version probe of the Py-language's
   interpreter, its output discarded. Nothing used it, nothing depends on it,
   and no tracked file names it (`vocab`'s `no_py_language` test is green).
   It is against the operator's rule and is reported, not hidden.

## 8. Not done here, and why

- **`cargo test --workspace --locked` in full** was not run: 4 cores and the
  size of `cli` and `api`. Run instead: the touched crate (`core`) whole, and
  every test outside it that reads a file this part edits (found by a grep of
  every `include_str!` and `read_to_string` of the seven paths). Not built:
  `api`'s `server.rs` test, which reads only rows P-01 to P-04 of the
  invariants (untouched).
- **`cargo deny check`:** not installed here; the part adds no dependency and
  no manifest line, so its answer is the base's.
- **Mutation tests:** deferred by the operator.
- **The drain test and the disk floor:** they name the Mac's drive, which this
  container does not have (`CLOUD.md` adaptation 2); no store, no app, no
  `:8080` were touched or needed.
- **Evidence on the Mac only, not read here:** the sha256 file
  `state/fib-strategy-20261030/costs-evidence/sha256-official.txt`,
  `state/fno-winners-20261013/sources-leverage-execution.md` (the URLs of
  T02-02, T03-04, T09-07, T10-05, T06-03 and T11-09), the operator's memory
  files, and the D3 recipe. The charter cites each by path and copies no value
  it did not see.

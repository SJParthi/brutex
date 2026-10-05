# GDFL round 3: report

**Round 3 did NOT find zero.** Three new defects failed tests on 64fcdbb. Two of them are in code that rounds 1 and 2 wrote: the journal's torn-line and foreign-line rules (D-3173, D-3169) and the undecodable-name claim (D-3175, D-3167).

All three are fixed in commit **631022a** on branch `claude/attack-gdfl` in `/home/claude/gdfl-wt`. It is local and not pushed.

## Attacks

All tests are in `crates/pull/src/gdfl_r3_attack_tests.rs`, wired into `gdfl_import.rs` the way round 2's file is. The file has no `eprintln!`, so Gate 23 needed no change.

"Failures on 64fcdbb" was checked by running this file in a throwaway worktree of 64fcdbb: 5 tests failed and 2 passed.

| attack | cases | failures on 64fcdbb | fixed? | evidence |
|---|---|---|---|---|
| Journal: a torn line whose ` (torn)` close was itself torn (`d (t`, `b (to`, `inc (torn`, and so on), plus 8 foreign tails made of mark pieces | 9 torn tails and 8 foreign tails, each over 2–3 runs | **failed**: `d (t` was refused as "not a journal line" on every run | yes, D-3196 | `r3_01_…` (file line 417). Fix: `torn_fragment`, `gdfl_import.rs:554` |
| Journal: random crash histories. Real `Journal::append` and `Journal::load` calls, with any write (a line, or the load's own close) truncated to any strict prefix. Each load is checked against a model of the lines written whole. | 1,500 histories, 18,000 loads, more than 2,000 line tears and more than 500 close tears | **failed** at case 3 with tail `begi (tor` | yes, D-3196 | `r3_02_…` (line 489) |
| Undecodable names of retired underlyings (`LTI…`, `NIFTYIT…`) under `LT`, `NIFTY` and other filters | 6 filters × 5 names | **failed**: an `LT` run refused `LTI06APR24100CE` | yes, D-3197 | `r3_03_…` (line 598). Fix: `gdfl_nfo::shaped_underlying` (`gdfl_nfo.rs:437`), used by `claims_undecodable` (`gdfl_import.rs:1095`) |
| Columnar tick-store block (kind 1) compared with the entry's stated size: a numeric bomb, a rows × columns text bomb, and an honest block at size, size−1 and size+1 | 2 bombs, 3 honest sizes | **failed**: a 173-byte block stating 64 bytes rebuilt 18,000,002 bytes | yes, D-3198 | `r3_04_…` (line 725). Fix: `columnar` and `Column::numeric` (`gdfl_tickstore.rs:298` and `:415`) |
| Random **index** worlds through the tick store and the zips, half of them filtered. The worlds include twins, bent rows, no in-session row, ending inside the session, INDIA VIX, NIFTY IT and stray members. Each is compared with a slow reference, and the rerun is checked. | 400 worlds; more than 1,000 bars, 300 refusals and 150 filtered | 0 | n/a | `r3_05_…` (line 1145) |
| Random **filtered stock** worlds through both sources. Round 2 never ran a filter on stocks. Each is compared with the reference, and the rerun is checked. | 400 worlds; more than 1,000 bars, 300 refusals and 150 filtered | 0 | n/a | `r3_06_…` (line 1154) |
| Options: one filtered run per underlying (`NIFTY`/`NIFTYNXT50`, `LT`/`LTI`, `M&M`, with broken names of each), then an unfiltered run, all into one store. Checked against an unfiltered store, against an entry balance per run, and against the claims summed across filters. The rerun is checked too. | 120 worlds, 6 runs each | **failed**: the filters claimed 15 entries and the unfiltered run 14. This is the same defect as D-3197. | yes, D-3197 | `r3_07_…` (line 1308) |

### What was tried and found sound

- **The receipt balance holds by structure.** Entries equal files + skips + refusals in every branch of `nfo_day`, `cm_day` and `DayWork::file`. Three cases were checked against the code: a listing `Err`, a failed journal `begin` (counted in `files`, with a failure), and `from_rows` failures. The random worlds confirm it for every filter × refusal × source × kind combination tried.
- **Two names of one contract.** Duplicate decoded contracts, same-name duplicates and `.CSV`/`.csv` twins are refused or counted consistently through both sources.
- **Journal states.** These were tried: absent, empty, torn, foreign, partly written, mixed keys from earlier runs, `begin` after `incomplete`, and a load after a load. Apart from D-3196, all are consistent.
- **Two observations that are not counted as findings:**
  - Round 1 already made an `incomplete` line deliberate. Because of it, a day that a run began and recorded `incomplete` is not listed in `resumed` on rerun. The `Report::resumed` doc says "crashed or incomplete earlier run", which is ambiguous on this point.
  - The journal does not record which version of the bar definition built a day. Days marked `done` before D-3170 (no look-ahead) are therefore never rebuilt. This is a migration question, not a test failure.

## Fixes and decision entries

All four entries are appended to `docs/05-decisions.md`.

- **D-3196 — A torn close of a torn journal line is still a torn line.** Before the D-3169 verb rule is applied, `torn_fragment` strips every trailing piece of ` (torn)`, whole or cut. The mark's 7 bytes are distinct, so stripping is unambiguous. What remains must not be empty, so pure mark pieces with nothing before them (` `, ` (t`, ` (torn)`) stay foreign. DPT-18.
- **D-3197 — An undecodable options name is the underlying its shape spells.** A name with a dated `DD MON YY STRIKE` shape or a monthly `YY MON STRIKE` shape is claimed only by a filter that names exactly the text before the shape. Both shapes split at the same place. A shapeless name keeps the prefix rule from D-3175 and D-3167. Whether NSE ever listed LTI or NIFTYIT options on a day this import reaches is UNVERIFIED; the rule does not depend on it. DPT-19.
- **D-3198 — A columnar block builds nothing past its stated size.** `columnar(body, rows, size)` refuses with a new `PAST_SIZE` refusal when:
  - the row count is above the size;
  - the header is above the size;
  - a column's stated `text_len` is above the size;
  - a text column's payload is longer than its `text_len`;
  - a numeric column's payload is longer than `9*rows+2`;
  - the output passes the size, checked field by field.

  `Column::numeric` also refuses once its rendered text passes `text_len`.

  One existing unit test changed: `a_column_that_is_not_its_rows_refuses` stated a 1-byte size for 3-row blocks and now states 64 bytes, so each refusal it checks is still the column's own. DPT-20.
- **D-3199 — Round 3: what was tried and found sound.** Covers r3_05–r3_07 and the timing below. DPT-21, DPT-22.

### Invariant rows

These rows are appended to the GDFL table in `docs/04-invariants.md`.

| ID | Statement | Test |
|---|---|---|
| DPT-18 | A journal torn any number of times still loads to the lines written whole | `r3_01_…`, `r3_02_…` |
| DPT-19 | An undecodable options name belongs to the underlying its shape spells | `r3_03_…`, `r3_07_…` |
| DPT-20 | A columnar block builds nothing past its entry's size | `r3_04_…`, `a_column_that_is_not_its_rows_refuses` |
| DPT-21 | Index worlds and filtered stock worlds match the reference through both sources | `r3_05_…`, `r3_06_…` |
| DPT-22 | A run of filters converges on the unfiltered store | `r3_07_…` |

`docs/11-findings.md` has 4 new lines in the GDFL dispositions section.

## Commands and results (at 631022a)

**Static gates.** The `jobs.language-purity` steps were extracted from `ci.yml` into `/tmp/claude-0/r3gates/*.sh` and run with `/tmp/claude-0/r3gates-run.sh` from the worktree root, using `RUNNER_TEMP=/tmp/claude-0/rt`, `SOURCE_SCAN=/tmp/claude-0/source-scan` and `GITHUB_ENV=/tmp/claude-0/ghenv`.
- 28 steps passed and Gate 1e and Gate 7 were skipped, with 0 failures. The passing steps include Gate 1d, 10, 10b, 12, 27 and 27b.
- Gate 1d at first refused the new scratch tags. They are now declared in `GDFL_LITERAL` in `.github/gates_tree.rs`: `r3-*`, `22` and `333`.

**Formatting and lint.**
- `cargo fmt --all --check`: clean.
- `CARGO_TARGET_DIR=/home/claude/gdfl-target cargo clippy -p pull --all-targets --locked -- -D warnings`: clean.

**Tests.**
- `cargo test -p pull --locked --lib gdfl` as root: 240 passed, 2 failed. The two failures are the known root-only tests, `a_journal_that_cannot_be_written_fails_the_day_by_name` and `a_temporary_or_absent_day_is_none_and_an_unreadable_one_refuses`.
- The same binary copied to `/tmp/gdfl-nobody` and run under `setpriv --reuid=65534 --regid=65534 --clear-groups`: **242 passed, 0 failed**.
- On 64fcdbb, the r3 file alone: 5 failed (r3_01, r3_02, r3_03, r3_04, r3_07) and 2 passed (r3_05, r3_06).

**Caution about the shared target directory.** A build from another worktree path can leave stale `pull` binaries that cargo treats as up to date. After the throwaway base worktree, the `pull` sources had to be touched to force a rebuild.

## Timing (O(1))

`NfoDay::locate` after D-3166's single map. These are round 1's `dpn_12` and `dpn_15` tests, test profile (optimised with debug info), run concurrently with other tests.

| entries | p50 | p99 | max |
|---|---|---|---|
| 10^3 | 85 ns | 489 ns | 39 µs |
| 10^4 | 195 ns | 613 ns | 4.27 ms |
| 10^5 | 379 ns | 841 ns | 47 µs |
| 10^6 | 702 ns | 1,244 ns | 33 µs |

- **Duplicates:** the median is 88 ns with 1 duplicate and 58 ns with 100,000, so cost does not grow with duplicates.
- **Listing size:** cost is **not flat in absolute time** as the listing grows: p50 is about 8× higher from 10^3 to 10^6 entries. Each lookup is still one hash probe by its code. The growth is consistent with cache misses on a larger table, but that cause is UNVERIFIED and this is not a bench-gate measurement.
- `torn_fragment`, `shaped_underlying` and the columnar checks are on refusal or once-per-file paths, not per-row lookups. They were not timed.

## Needs vendor data

- Whether NSE or GDFL ever listed `LTI` or `NIFTYIT` options on a day from 2019-12-02 on. D-3197 holds either way.
- The real tick-store `FORMAT.md` §4. The columnar size bounds follow from the reader's own shape (every row has a terminator, each plane is at most 8 bytes wide), not from the specification text.
- The round-2 items are still open:
  - the long-dated months 2022-06, 2023-06 and 2023-12;
  - whether monthly names continue after the cutover;
  - the missing D-2802..D-2807 entries.

## Files modified

- `crates/pull/src/gdfl_import.rs`
- `crates/pull/src/gdfl_nfo.rs`
- `crates/pull/src/gdfl_tickstore.rs`
- `crates/pull/src/gdfl_r3_attack_tests.rs` (new)
- `.github/gates_tree.rs`
- `docs/04-invariants.md`
- `docs/05-decisions.md`
- `docs/11-findings.md`

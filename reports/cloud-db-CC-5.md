STATUS: IN PROGRESS

# CC-5 — Copies register (cloud build report)

Branch `cloud/db-CC-5-20261024`, cut from `origin/feat/db-fix` at `e851da1ca` (Merge DATA-2).
Spec: `origin/cloud/db-base-20261024:reports/db-specs-20261024/CC-5.md`. Decision block D-7040..D-7049.
Prerequisites: none (spec section 1). FILES_ONLY links: none.

## Done so far

- `crates/store/src/settings.rs`: key 12 `disk.extra_copy_roots` (`Key::ExtraCopyRoots`, slot 11,
  `KEYS` 11 -> 12), `COPY_ROOT_SEPARATOR` `|`, `MAX_COPY_ROOTS_BYTES` (= the settings route's own
  4,096-byte `secret_shape` ceiling, an existing project number; no count of roots is typed).
  `copy_roots()` is the one reader: absolute, no `..`, no empty piece, none twice, none nested,
  checked with an ordered set (O(r log r)). `check` adds that each folder exists; `check_stored`
  holds a stored value to its shape only. Test
  `the_extra_copy_roots_key_is_checked_and_stored_canonically`: passes.
- `crates/api/src/settings_route.rs`: the two required arms (`default_of`: `(None, UNSET_WAIT)`;
  `reader_of`: `Ok(...)` naming `pull::copies_register`, `GET /disk/copies.json`, `cli disk-copies`).
- `crates/pull/src/copies_register.rs` (new) and `copies_register_tests.rs` (new, 24 tests, all
  pass): the read-only `Fs` seam (`lstat`, `list`, `lines` through `store::safe_open`; no writing
  method exists), `config()`, `reader_of()` (D-7044), a walk that enters every directory once by
  `(st_dev, st_ino)` on `available_parallelism` workers (no typed entry, depth, list or thread
  count: the operator's 24 Oct rule), `(st_dev, st_ino)` hazards within one walk, streamed receipts
  (`ALL_VERIFIED.txt`, `VERIFIED.tsv` matched by inode, size, mtime and ctime, never `st_dev`;
  `*.prove.out` with `ALL-PROVED`), twins, the retirement list and its text, the events, the cache
  (`answer`: re-walk only when the roots or a root's own stat change; no request waits on
  another's walk), `total()` for R4-29.

- `crates/api/src/copies_route.rs` (new) and `copies_route_tests.rs` (new, 5 tests, all pass):
  `GET /disk/copies.json` in the ops-read pool; unset key = `200 waiting` naming the key; the first
  walk since start = `503 walking` to any other request, never a wait; the register body rendered
  once per walk and joined to each request's own head (`age_ms`, `fresh`, `why`, `rewalking`).
  Wired in `server.rs` (one `mod` line, one `.route`), `operation_audit_tests.rs` (EXEMPT, DATA)
  and `pool_table_tests.rs` (OpsRead row).

- `crates/cli/src/disk_copies_command.rs` (new, 3 tests, all pass): `cli disk-copies`, no arguments,
  always a fresh walk; exit 1 on a hazard, a root not walked whole, the key unset or a refused
  store root; exit 2 on any argument. Wired in `crates/cli/src/lib.rs` (`mod`, one dispatch arm,
  `COMMANDS` 62 -> 63, one usage entry, the zero-argument exemption), `store_write.rs` (reader
  list) and `sweep_wiring_tests.rs` (non-sweep list).
- `crates/pull/tests/no_scheduler_delete.rs` (R4-06): `DECLARED` 12 -> 15 and `LISTED` 4 -> 7 with
  `copies_register.rs`, `copies_route.rs` and `disk_copies_command.rs`; its 5 tests pass.

- The /ingest panel (written by a parallel subagent, reviewed): `web/src/routes/ingest/panels/Copies.svelte`
  (order 95, through R4-01's registry; hazards red; the retirement list downloadable as text),
  `web/src/lib/copies.js`, `web/tests/copies.test.js` (10 tests), and one dev-proxy line in
  `web/vite.config.js` (`web/tests/proxy.test.js` requires every fetched route to have one).
- The bench `crates/api/benches/ratio/copies.rs` (`/disk/copies.json` at 1x/10x/100x files, n = 200,
  after the settings rows) and its three lines in `crates/api/benches/ratio.rs`.
- `docs/02-store-format.md` §42 row, `docs/04-invariants.md` CC5-01..CC5-16,
  `docs/05-decisions.md` D-7040..D-7048.

## Left

- `docs/06-limits.md` L-CC5 rows with the measured figures; the final merge of `origin/feat/db-fix`;
  every gate (fmt, clippy, the workspace tests, deny, coverage, mutants, the CI shell gates).

## Log

### 2026-10-24 — start

- GO present, HOLD absent, spec present. Branch cut from `e851da1ca`.
- Gate tools: `cargo-deny` 0.20.2, `cargo-llvm-cov` 0.9.1, `cargo-mutants` 27.1.0 installed from
  crates.io (GitHub release downloads answer 403 through the proxy).

### 2026-10-24 — save point 1 (HELD)

- Files: `crates/store/src/settings.rs`, `crates/store/src/settings_tests.rs`,
  `crates/api/src/settings_route.rs`, `crates/pull/src/lib.rs`, `crates/pull/src/copies_register.rs`,
  `crates/pull/src/copies_register_tests.rs` (header only).
- `cargo clippy -p pull -p store --locked`: no warning, no error.
- Commit `cca93bc4`, pushed.
- `HOLD` found on `origin/cloud/db-base-20261024` at this save point: stopped here, waiting for
  `GO` with no `HOLD`.
- NEXT: write `crates/pull/src/copies_register_tests.rs` (the eight named tests first, failing
  before the module, then the extreme cases), run `cargo test -p pull -p store --locked`.

### 2026-10-24 — resumed, save points 2 and 3, then HELD again

- 12:37 IST the Mac lifted HOLD ("activate all the sessions"); work resumed from NEXT.
- The README's new section (the operator's words of 24 Oct, "no typed number outside the Winner
  Gate") was applied: removed `MAX_COPY_ROOTS`, the walk's entry and depth caps, the list caps, the
  receipt read cap and the statement cap. Commit `ad16891c`.
- Wrote `crates/pull/src/copies_register_tests.rs`: the eight named tests and 16 extreme cases.
- `cargo test -p pull --lib --locked copies_register`: `test result: ok. 24 passed; 0 failed`.
- `cargo test -p store --lib --locked settings`: `22 passed; 3 failed`. The three
  (`a_directory_that_stops_being_readable_is_a_fault_not_nothing_new`,
  `a_disk_that_refuses_the_write_is_refused_with_nothing_visible`,
  `a_landed_save_whose_read_back_cannot_look_is_saved_with_the_reason`) are permission-bit tests;
  this container runs as root, which ignores permission bits. To be shown red on the unmodified
  base in the final gate run (not yet shown).
- `cargo clippy -p pull -p store --all-targets --locked`: no warning.
- `HOLD` back on the base at 12:55 IST ("HOLD for parts without GO-<NAME>"); no `GO-CC-5`. Stopped.
- NEXT: the api route `crates/api/src/copies_route.rs` + `copies_route_tests.rs`, its two
  `server.rs` lines, its rows in `operation_audit_tests.rs` and `pool_table_tests.rs`.

### 2026-10-24 — resumed after the 17:30 reset, save point 4, then HELD again

- 17:30 IST HOLD lifted ("every cloud part builds"). Merged the newest `origin/feat/db-fix`
  (`7b397bbe`, Merge CLOUD-MUTCLI) into the branch: clean, merge commit `c03ceb09`.
- Wrote `crates/api/src/copies_route.rs` and `copies_route_tests.rs`; wired `server.rs`,
  `operation_audit_tests.rs`, `pool_table_tests.rs`.
- `cargo test -p api --lib --locked copies_route`: `test result: ok. 5 passed; 0 failed`.
- `cargo test -p api --lib --locked -- pool_table operation_audit settings_route`:
  `test result: ok. 55 passed; 0 failed`.
- `cargo clippy -p api --all-targets --locked`: one test warning (`manual_assert_eq`), fixed.
- `HOLD` back at 18:15 IST ("HOLD for parts without GO-<NAME>"); no `GO-CC-5`. Stopped.
- NEXT: the cli verb `crates/cli/src/disk_copies_command.rs` (zero arguments; a fresh walk; exit 1
  on a hazard, a wait or a root not walked whole), its dispatch arm, `COMMANDS` 60 -> 61, its usage
  line, `store_write.rs` reader list, `sweep_wiring_tests.rs` non-sweep list, the zero-argument
  exemption in `a_known_command_with_the_wrong_arity_is_not_called_unknown`; then R4-06's
  `no_scheduler_delete.rs` rows.

### 2026-10-24 — GO-CC-5 (19:47 IST), save point 5

- `GO-CC-5` appeared on the base: CC-5 builds while HOLD stands for parts without their own GO.
- Wrote `crates/cli/src/disk_copies_command.rs`; wired `cli/src/lib.rs`, `store_write.rs`,
  `sweep_wiring_tests.rs`; added the three files to R4-06's `no_scheduler_delete.rs`.
- `cargo test -p cli --lib --locked -- disk_copies a_known_command every_command_is_listed the_words`:
  `test result: ok. 7 passed; 0 failed`.
- `cargo test -p cli --lib --locked -- disk_copies every_command a_known_command store_write sweep`:
  `test result: ok. 85 passed; 0 failed`.
- `cargo test -p pull --test no_scheduler_delete --locked`: `test result: ok. 5 passed; 0 failed`.
- `cargo clippy -p cli -p pull --all-targets --locked`: no warning.
- NEXT: the /ingest panel (`web/src/routes/ingest/panels/Copies.svelte`, `web/src/lib/copies.js`,
  `web/tests/copies.test.js`, the rebuilt `web/build`), then the bench `crates/api/benches/ratio/copies.rs`.

### 2026-10-24 — save point 6, then HELD (every GO-<NAME> removed, 20:20 IST)

- The /ingest panel by a parallel subagent: `npm ci --prefix web` ok; `npm --prefix web run build`
  exit 0 (the panel compiles); `node --test web/tests/*.test.js` on Node 24 (CI's):
  `tests 1578, pass 1578, fail 0`; on this container's Node 22: `pass 1573, cancelled 5`
  (`ask.test.js` 3, `census-worker.test.js` 2; files the panel does not touch). W4 `0 (ceiling 0)`,
  W5 `No comment swallows a rule`. W3: `svelte-check errors: 32 (ceiling 0)`, all 32 in files this
  part does not touch (0 in the panel's): red before this part.
- **BASE RED, W1: the committed `web/build` is stale on the unmodified base.** Committed
  `version.json` `80659b2a3b706360`; the base's own source builds `fe2953bf00677395` (eight `/db`
  sources changed since R3-03's rebuild of 19 Oct with no rebuild: R3-04 `f68b710f`, R3-30
  `a258ca1c`, among them). The build is reproducible (two builds byte-identical). This part does
  NOT commit `web/build` (rebuilding it would rewrite other parts' output); with the panel the
  bundle builds as `858702fad398a0d9`. NEEDS THE MAC: rebuild `web/build` at merge-back.
- Shell gates (the extracted `run:` blocks of ci.yml's first job, run on this branch and on an
  unmodified worktree of `origin/feat/db-fix` `7b397bbe`): red on BOTH, base reds owned by
  BASE-REDS-1023: 1c, 1d, 10, 11, 12, 19, 23, 27b, 30. On this branch, gates 10, 11, 12, 19, 27b and
  30 add no finding of this part. Gate 1d adds 55 literals of this part's files to the base's 466
  undeclared (declaring them is an edit of `.github/gates_tree.rs`, left with the gate's owner).
  Gate 23: the `println!` of the timing test was removed (no new finding). Gate 28 was red on this
  branch (`"tmp"` in `is_partial`): fixed by reading partial names from the data. Gate 1 was red only
  while the subagent's rebuild had removed tracked bundle files; `git checkout -- web/build`
  restored them. Not yet re-run after these fixes.
- Bench `cargo bench -p api --bench ratio --locked --no-run -j 1`: `Finished bench profile ... in
  19m 58s` (with 4 jobs the release link of `cli` was killed for memory, SIGKILL).
- Commit `da57ae23`, pushed.
- The served bench run started before the hold (`cargo bench -p api --bench ratio --locked --
  --served-only`) recompiled after the last edit and was killed (SIGKILL: two release links of
  `cli` at once ran out of memory): exit 101, NO figure taken.
- NEXT: build the bench with `-j 1`, then read the served bench's copies rows (`cargo bench -p api --bench ratio --locked -j 1 --
  --served-only`, started before the hold) and write `docs/06-limits.md` L-CC5 rows; re-run the
  shell gates; merge the newest `origin/feat/db-fix`; then fmt, clippy, the workspace tests, deny,
  coverage (nightly installed for `--branch`) and mutants on the touched modules.

### 2026-10-25 — resumed (GO, HOLD lifted 13:1x IST)

- New container; `HOLD` found at start, polled; `GO` with no `HOLD` (base `1dcc33e9`, "Lift HOLD: all
  nine cloud parts build") at the first poll.
- Merged the newest `origin/feat/db-fix` (`666ddf7f`, Merge DATA-1w): two both-sides-added conflicts
  (`docs/04-invariants.md`: CC-5's section beside R3-13's; `docs/05-decisions.md`: D-7040..D-7048
  beside D-7300), resolved by keeping both whole.
- After the merge: `cargo build --workspace --locked --all-targets`: no warning, no error.
  `cargo fmt --all --check`: exit 0. `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings`:
  `Finished`, no warning.
- Shell gates re-run (the `run:` blocks of ci.yml's `language-purity` job, `GITHUB_ENV` emulated so
  gate 0's `SOURCE_SCAN` reaches the rest; gate 1e not run here, it builds a second worktree), on
  this branch and on an unmodified worktree of `origin/feat/db-fix` `666ddf7f`: red on BOTH, the same
  nine, base reds: 1c, 1d, 10, 11, 12, 19, 23, 27b, 30. Findings diffed line by line (paths and
  counts normalised): 1c, 10, 30 identical; 11, 12, 19, 23, 27b differ only in their scanned-line and
  file counts (12's REFUSED list identical modulo line numbers); 1d: 521 undeclared on this branch
  against 466 on the base, the 55 literals of this part's files (left with the gate's owner, as
  recorded at save point 6). Every other gate (0, 1, 1b, 1g, 1f, 2, 9, 9b, 10b, 7, 13, 15, 16, 17, 21,
  22, 24, 28, 25, 27, 26, 14, 29, 29b): exit 0 on this branch.

STATUS: IN PROGRESS

# BASE-COV-API-AS3 (shard G3, repair r3): working report

This file is rewritten as the session goes; line 1 is the state of the branch now.

- Branch `cloud/cov-api-as3-20261105`, cut from the kit commit `f4111a0132473d8771d8624a9538b1362926f70b`;
  START `6ea46e59c07488eeccff388407976041852aae2c`.
- Sessions: token `-1` pushed to `7700e2f0`; the main session paused at `edd54703`; token `-3` (`a5f009a8`) worked to
  `44a0a656` and was cut by the account's weekly limit (Thu 5 Nov, about 21:28 IST). This continuation holds
  `BASE-COV-API-AS3-20261105-4` (`7b6102cb`). The `-3` session's helpers had unpushed work on `mastersrun.rs`,
  `optiongreeks.rs`, `autopilot.rs`, `selectionv6json.rs` and a saved Boolean evidence probe: it was lost with
  its container, so those files are worked again here.
- Host (fresh container, 6 Nov): 4 cores, 15 GB, root, Linux x86_64. Pinned nightly installed as
  `nightly-2026-08-08`: `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`, llvm-cov and llvm-profdata
  `LLVM version 23.1.0-rust-1.99.0-nightly`. Every kit file's sha256 matches `MANIFEST.md`.

## Measurements so far (this host's figures; the Mac re-measures)

- Route B (the kit's `d3-route-b-cloud.sh`, unchanged, package api) on `7b6102cb` (the `-3` session's code):
  build 3 min, tests 11 min (`tests passed 3109 failed 14 ignored 16`), readout to 08:50 IST; NOT PROVED: the 22
  benches and `lib.rs` only (L-BCA-2). The set read was the kit's set file plus one `coverage <file> whole` line
  for each of the 7 crates/api files the branch added (whole:api names every file; the kit's set predates them).
  TMPDIR was a 1777 tmpfs folder and the profile folder 1777 so the children `isolated::where_permission_binds`
  re-runs as uid 65534 write their profiles (CLOUD.md item 10).
- One integration test hangs on this 4-core host and was ended with `exit(0)` from gdb so its binary wrote its
  profile: `cloud_mutants_bars.rs` `spot_json_is_refused_by_name_when_every_store_read_slot_is_held` (G2's file,
  unchanged from START) fills `detail::MAX_STORE_READ_CONCURRENT` holders behind a barrier of that size plus
  one, while this host's store-read pool has fewer slots, so the barrier never fills. The measurement of record
  will pass `--skip` for it and list it for the Mac.

## Done in this continuation (pushed `a0275622`)

- `mastersrun.rs`: new tests `a_refetch_that_changed_nothing_and_a_reprimed_step_are_each_named`,
  `a_reload_that_would_lose_a_readable_feed_is_refused_and_logged`,
  `a_refresh_task_that_does_not_return_is_a_500_that_says_where_to_look`; the refresh child reads its summary
  record against its page and drives `refresh` itself; test-code rows rewritten branch-free. Left: the
  credentialed success arm and the whole-refresh 200 (a Parameter Store read, L-BCA-6), `refresh_work` past
  `masters_dir` (real public hosts, L-BCA-6), and (U) rows with proofs (to be listed in `fu-AS3.txt.md`).
- `autopilot.rs`: new test `a_probe_name_the_host_refuses_keeps_the_hosts_own_words`; the switch child reads a
  case table; test-code rows rewritten. Left: (U) rows (Day bounds, `succ`, store timeframes, survey slots), the
  clock-unusable paths (the system clock), and the zero-wait pass of `fly` (3113/3115).
- `optiongreeks.rs`: new file `optiongreeks_cover_tests.rs` (9 tests). Left: (U) rows and the
  `option_order_broken` arm (a zero stamp the store's writer never files).

## Not yet

- The (U) proofs and the GAP classification of every G3 row; `gaps-AS3.txt.md`, `fu-AS3.txt.md`.
- Red under a hand change for every new test of this round (the `-3` session showed its own 13; its helpers'
  tests and this continuation's are still to show).
- The 8-extremes check per tested file; root host reds proved as uid 65534; the gates on the code tip; the
  measurement of record; `fix-BASE-COV-API-AS3.json.md`.

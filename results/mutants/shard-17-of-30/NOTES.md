# Mutation testing: shard 17 of 30 (job MUT-17)

Labels follow README-CLOUD.md. MEASURED means a number from a named run on this box. EXACT means a count. No figure here is a
trading result.

## Run

- **Source:** branch `cloud/spikefade-shard-20261116` at `15fd4f4737ea11bcef398efa75db240d38128d10`. Code only: no market
  data, and none of the 3 parameter tables.
- **Box:** a Linux x86_64 cloud container with 4 vCPU and 15 GB RAM. `rustc 1.97.1 (8bab26f4f 2026-07-14)`, cargo-mutants
  27.1.0 (`cargo install cargo-mutants --locked`). One tool added to the box: zstd 1.5.5 via apt (README-CLOUD.md section
  3). Nothing was added to the checkout.
- **Command,** run from the checkout root (`baseline.log` spells out all 14 `--skip` names, in shard 3's order):

  ```sh
  cargo mutants --workspace --shard 17/30 --jobs 2 --cargo-arg=--locked \
    -- -- --exact --skip <each of the 14 tests listed under "Exclusions">
  ```

- **Wall (MEASURED, `outcomes.json`):** 2026-11-16 11:40:31Z to 12:31:05Z, so 50 min 34 s.
- **cargo-mutants' own unmutated baseline (MEASURED):** 14 s build + 5 s test. The auto-set test timeout was 29 s.

## Result (EXACT)

Shard 17 is global positions 8875 to 9396 of the workspace's 15,639 mutants under the default `slice` sharding. All 522
are in `crates/spikefade/src/report3.rs`, lines 2013 to 3228.

| function | mutant lines | caught | missed | timeout | unviable | total |
|---|---|---:|---:|---:|---:|---:|
| compare_skip | 2013-2024 | 1 | 14 | 0 | 0 | 15 |
| stage_b_day_major | 2041-2316 | 21 | 131 | 0 | 6 | 158 |
| batch_next | 2333-2335 | 9 | 0 | 0 | 0 | 9 |
| lpb33 | 2343-2348 | 3 | 13 | 0 | 0 | 16 |
| kit_setup | 2357-2411 | 4 | 14 | 0 | 0 | 18 |
| batch_order33 | 2426-2427 | 5 | 0 | 0 | 0 | 5 |
| stage_b_day_major_v33 | 2439-3066 | 18 | 201 | 0 | 9 | 228 |
| cand_flags | 3077-3085 | 1 | 3 | 0 | 0 | 4 |
| cand_whole | 3098-3120 | 0 | 8 | 0 | 0 | 8 |
| offer_row | 3128 | 0 | 1 | 0 | 0 | 1 |
| row_candidate | 3136-3137 | 5 | 24 | 0 | 0 | 29 |
| row_better | 3148 | 3 | 8 | 0 | 0 | 11 |
| row_take | 3153 | 0 | 1 | 0 | 0 | 1 |
| d3_rewalk | 3163-3228 | 2 | 17 | 0 | 0 | 19 |
| **all 14 functions** | | **72** | **435** | **0** | **15** | **522** |

All 15 unviable mutants replace `&&` with `||`, and none of them compiles.

## Read this before using the MISSED count

- **Scope.** cargo-mutants 27.1.0 tests each mutant only against its own package's tests. Here that is `spikefade`'s one
  test target: 158 tests run, 1 is ignored and 9 are skipped (see "Exclusions").
- **Eight functions are indistinguishable from a stub.** Every whole-body replacement in them survives, so no test that
  runs here observes their effect:
  - `stage_b_day_major` → `Ok(())` and `stage_b_day_major_v33` → `Ok(())`;
  - `kit_setup` → all 3 values;
  - `cand_flags`, `cand_whole`, `offer_row` and `row_take` → `()`;
  - `row_better` → `true` and `false`.
- **Two more are partly checked.**
  - `lpb33` → `0` survives, but `lpb33` → `1` is caught.
  - `row_candidate` → `None` and 22 of its 27 `Some(..)` values survive. The other 5 are caught.
- **Where the misses are.** The two stage-B day-major functions alone hold 332 of the 435 misses.
- **Whole-body replacements that were caught:** `batch_next` (4), `batch_order33` (3), `lpb33` → `1`, `d3_rewalk` →
  `Ok(())`, and 5 of `row_candidate`'s `Some(..)` values.
- **Effect of the exclusions on this shard.** None of the 9 skipped spikefade tests names any of the 14 functions in its
  body. This is a static check of direct references only. Indirect calls were not traced.

## Four catches were out-of-memory kills (MEASURED)

- **Which mutants.** The 4 mutants that replace `batch_next` with a constant `Ok((0, 0))`, `Ok((0, 1))`, `Ok((1, 0))` or
  `Ok((1, 1))`. Each one makes a `tests34::batch_*` test loop and allocate without bound.
- **What happened.** Each time, the kernel's cgroup OOM killer ended that mutant's own test binary, at 9.1 to 13.7 GB
  anon RSS. Each of the 4 logs shows `signal: 9, SIGKILL`.
- **How they were recorded.** cargo-mutants recorded all 4 as caught. On a box with more memory they would have hit the
  29 s timeout instead. Either way the tests notice the mutant.
- **Nothing else was affected.** These were the only 4 OOM kills of the run (`dmesg`). No other mutant log shows a kill or
  an abort, and no other process was killed.

## Baseline (`baseline.log`)

- **The exact step-2 command,** `cargo test --locked --workspace`, exits 101 after 3 s (MEASURED), and 0 tests run. Cargo
  stops when `core`'s test targets fail to compile.
- **Every test target, built with `--keep-going`:** exactly 6 fail to compile, all in `core`. They are the lib unit tests,
  `banned_lists`, `claim_reader`, `decision_citations`, `findings` and `graph`. Each one fails on `include_str!` of a file
  of the app repository that this branch does not hold.
- **Every target that compiles, with no exclusions** (MEASURED): 658 passed, 14 failed, 3 ignored.
- **The same, with the 14 exclusions** (MEASURED): 658 passed, 0 failed, 3 ignored, 14 skipped.
- **This shard's scope,** `cargo test --locked -p spikefade -- --exact <14 skips>` (MEASURED): 158 passed, 0 failed,
  1 ignored, 9 filtered out, in 5.8 s.

## Exclusions

Each reason below was read from the test's own panic message on this box.

| test | package | why it cannot pass here |
|---|---|---|
| `menu::tests::sf3_menu_and_sets_load_with_the_signed_counts` | spikefade | needs `crates/spikefade/data/EXITS-DYN-*.tsv`; the file read falls back to empty, so `menu_head: missing` |
| `menu::tests::sf3_menu_fail_closed` | spikefade | the same |
| `menu::tests::sf3_tp_floor_not_armed_at_or_above_entry` | spikefade | the same |
| `menu::tests::sf3_distance_floor_counted` | spikefade | the same |
| `menu::tests::sf3_tp_theta_level_and_absence` | spikefade | the same |
| `tests33s::v33_search1d_sets_load_and_count` | spikefade | `EXITS-DYN-MENU-v1.tsv`: No such file or directory |
| `tests3::sf3_ttp_replaces_tp_close` | spikefade | the same file, `NotFound` |
| `tests34::root_digest_same_for_every_form` | spikefade | runs zstd at the Mac-only path `/opt/homebrew/bin/zstd` (`kit.rs:470`; PORTABLE is off in the test thread, so neither PATH nor `SF_ZSTD` is used) |
| `tests34::sfz_roundtrip_all_rows` | spikefade | the same |
| `every_declared_cost_invariant_reaches_a_bench` | core (`cost_invariants`) | needs `docs/04-invariants.md` |
| `the_document_declares_the_five_operations_rule_4_names` | core (`cost_invariants`) | the same |
| `every_measured_cost_invariant_is_declared` | core (`cost_invariants`) | the same |
| `every_crate_carries_a_ratio_bench` | core (`cost_invariants`) | needs `crates/spikefade/benches/ratio.rs` |
| `the_reconciliation_is_reading_something` | core (`cost_invariants`) | needs `docs/04-invariants.md` |

- **Which skips matter here.** The 5 `core` skips match no test that runs in this shard. They are passed so that every
  shard uses one list, the same one shard 3 used.
- **Judgement call.** The 2 `tests34` tests need a file outside the repository: a Mac tool path. The rest need files that
  the repository does not hold.

## MISSED (435, EXACT)

- **Format.** Each entry is `line:col original→replacement[,replacement...]`, and every line is in
  `crates/spikefade/src/report3.rs`.
- **`body→`** means the whole function body is replaced by the listed values, separated by ` | `.
- **`S(a, b, c)`** stands for `Some(((a, b, c), Default::default(), Default::default()))`.
- **Raw form.** `missed.txt` holds the same 435 mutants in cargo-mutants' own form.

```text
compare_skip (14 missed): 2013:27 +=→-=,*=  2014:28 +=→-=  2015:135 !=→==  2016:33 +=→-=,*=  2021:25 +=→-=,*=  2022:25 +=→-=,*=  2023:25 +=→-=,*=  2024:25 +=→-=,*=
stage_b_day_major (131 missed): 2041:5 body→Ok(())  2042:46 *→+,/  2042:51 *→+,/  2043:54 /→%,*  2046:70 +→-,*  2046:90 *→+,/  2048:102 +→-,*  2049:26 *→+,/  2049:33 /→*  2049:41 +→-,*  2064:84 delete field row of WinRes  2064:106 delete field run of WinRes  2078:12 delete !  2114:53 *→/  2115:25 +→*  2120:57 <→==,>,<=  2120:90 <→==,>,<=  2123:19 >→==,>=  2138:12 +=→-=,*=  2141:14 <→==,>,<=  2142:22 +→-,*  2143:17 +=→-=,*=  2145:25 -→+,/  2145:31 *→+,/  2145:45 *→+  2145:50 *→+  2150:53 *→+,/  2151:25 +→-,*  2156:57 <→==,>,<=  2156:90 <→==,>,<=  2159:24 ==→!=  2178:16 +=→*=  2179:56 +→-,*  2179:64 *→/  2182:104 *→+,/  2182:109 *→+,/  2196:40 +→-,*  2197:34 >=→<  2201:69 <→==,<=  2201:102 <→==,>,<=  2202:34 >=→<  2207:73 ==→!=  2212:55 *→+  2212:60 +→-,*  2212:65 *→+,/  2212:69 +→-  2219:48 +=→-=,*=  2226:32 +=→-=,*=  2228:28 delete !  2236:19 +=→-=,*=  2238:16 delete !  2251:37 *→+,/  2251:51 *→+  2251:56 *→+,/  2251:60 +→-,*  2251:66 *→+  2251:71 +→-  2251:76 *→+,/  2251:80 +→-,*  2254:31 +=→-=  2256:33 +=→-=,*=  2257:33 +=→-=,*=  2257:56 >→==,<,>=  2261:49 *→+  2261:54 +→*  2267:36 &&→||  2267:46 &&→||  2267:59 >→==,<,>=  2269:36 delete !  2269:43 ||→&&  2273:47 +=→-=,*=  2277:47 +=→-=,*=  2303:21 >→<,>=  2303:25 &&→||  2303:36 >=→<  2303:44 &&→||  2303:50 <→==,<=  2316:47 -→/
lpb33 (13 missed): 2343:5 body→0  2344:42 /→%,*  2346:70 +→-,*  2346:90 *→+,/  2347:102 +→-,*  2348:14 *→+  2348:21 /→%,*  2348:29 +→-
kit_setup (14 missed): 2357:5 body→Ok(None) | Ok(Some(String::new())) | Ok(Some("xyzzy".into()))  2357:49 ||→&&  2365:58 !=→==  2378:30 >→==,<  2389:26 >=→<  2408:14 <→==,>,<=  2409:22 +→-,*  2411:105 *→+
stage_b_day_major_v33 (201 missed): 2439:5 body→Ok(())  2456:54 /→%,*  2459:70 +→-,*  2459:90 *→+,/  2460:102 +→-,*  2461:26 *→+,/  2461:33 /→%,*  2461:41 +→-,*  2467:43 delete !  2467:140 -→+,/  2469:27 ||→&&  2473:58 !=→==  2481:108 ==→!=  2484:84 delete field row of WinRes  2511:56 &&→||  2524:12 delete !  2548:50 delete !  2574:19 >→==,<  2589:12 +=→-=,*=  2595:14 <→==,<=  2602:16 delete !  2602:60 &&→||  2607:17 +=→*=  2610:24 *→+,/  2613:92 ==→!=  2615:40 &&→||  2615:43 delete !  2647:22 +=→-=,*=  2649:20 delete !  2658:54 *→+,/  2658:69 *→+,/  2666:16 +=→-=,*=  2667:56 +→*  2667:64 *→+,/  2673:24 +=→-=,*=  2673:29 -→+,/  2675:25 +=→-=,*=  2676:23 +=→-=,*=  2677:24 +=→-=,*=  2677:99 *→+,/  2680:25 &&→||  2680:32 ==→!=  2689:52 *→+,/  2691:27 +=→-=,*=  2699:147 *→+,/  2699:152 +→-,*  2733:69 *→+,/  2733:75 +→-,*  2739:49 !=→==  2742:44 *→+,/  2743:46 +→-,*  2748:93 *→+,/  2749:58 ==→!=  2758:56 +=→-=,*=  2767:48 +=→-=,*=  2768:52 +=→-=,*=  2770:95 <=→>  2771:52 +=→-=,*=  2775:56 ==→!=  2775:61 &&→||  2776:60 +=→-=,*=  2778:60 +=→-=,*=  2784:56 +=→-=,*=  2792:37 +=→*=  2794:40 +=→-=,*=  2797:32 delete !  2809:23 +=→-=,*=  2811:20 delete !  2816:24 +=→-=,*=  2818:24 +=→-=,*=  2819:24 +=→-=,*=  2819:75 >=→<  2820:24 +=→-=  2827:28 +=→-=,*=  2827:33 -→+,/  2829:29 +=→-=,*=  2874:48 +=→-=,*=  2881:32 +=→-=,*=  2883:28 delete !  2894:19 +=→-=,*=  2896:16 delete !  2906:24 +=→-=,*=  2906:29 -→+,/  2908:25 +=→-=,*=  2920:37 *→+,/  2920:50 +→-,*  2923:31 +=→-=,*=  2925:33 +=→-=,*=  2926:33 +=→-=,*=  2926:56 >→==,>=  2930:49 *→+,/  2930:54 +→-,*  2936:36 &&→||  2936:46 &&→||  2936:59 >→==,<,>=  2938:43 ||→&&  2942:47 +=→-=,*=  2945:46 +=→-=,*=  2954:47 +=→-=,*=  2956:46 +=→-=,*=  2970:37 &&→||  2982:36 +=→-=,*=  2994:76 -→/  2994:99 -→+,/  2995:65 -→+,/  2996:75 -→+,/  3006:30 -→/  3007:30 -→/  3028:21 >→==,<,>=  3028:36 >=→<  3032:54 delete !  3037:21 >→==,<,>=  3037:25 &&→||  3037:36 >=→<  3037:44 &&→||  3037:50 <→>  3043:28 -→+  3060:47 -→+,/  3064:47 *→+  3064:104 +→-,*  3064:220 +→-,*  3066:17 &&→||  3066:24 ==→!=
cand_flags (3 missed): 3077:5 body→()  3085:12 delete !  3085:19 ||→&&
cand_whole (8 missed): 3098:5 body→()  3098:8 delete !  3109:61 ==→!=  3111:26 ||→&&  3111:29 delete !  3115:172 >=→<  3120:28 +=→-=,*=
offer_row (1 missed): 3128:5 body→()
row_candidate (24 missed): 3136:5 body→None | S(0, 0, 0) | S(0, 0, -1) | S(0, 1, 0) | S(0, 1, 1) | S(0, -1, 0) | S(0, -1, 1) | S(1, 0, 0) | S(0, -1, -1) | S(1, 0, 1) | S(1, 1, 1) | S(1, 1, -1) | S(1, -1, 1) | S(1, -1, -1) | S(-1, 0, 0) | S(-1, 0, 1) | S(-1, 0, -1) | S(-1, 1, 0) | S(-1, 1, 1) | S(-1, 1, -1) | S(-1, -1, 0) | S(-1, -1, 1) | S(-1, -1, -1)  3137:18 ==→!=
row_better (8 missed): 3148:5 body→true | false  3148:9 >→<,>=  3148:15 ||→&&  3148:42 <→==,>,<=
row_take (1 missed): 3153:5 body→()
d3_rewalk (17 missed): 3164:30 ||→&&  3199:117 >=→<  3201:53 *→+,/  3202:25 +→-,*  3206:57 <→>,<=  3206:90 <→==,>,<=  3228:65 <→==,>,<=  3228:98 <→==,>,<=
```

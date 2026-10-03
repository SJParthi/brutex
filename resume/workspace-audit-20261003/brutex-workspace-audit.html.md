<title>Brutex Workspace Audit</title>
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Public+Sans:wght@400;600;800&family=IBM+Plex+Mono:wght@400;600&display=swap">
<style>
/* Layout: one reading column; verdict strip, then five tabbed comparison tables (findings, O(1), Rust-only, gates, held). */
:root{
  --bg:#f6f7f8; --panel:#ffffff; --ink:#17202a; --muted:#5b6672; --line:#dde2e7;
  --accent:#1d5f8a; --accent-soft:#e3eef6;
  --high:#a3122a; --high-bg:#fbe5e8; --med:#a65a00; --med-bg:#fdf0dc; --low:#4b5d6e; --low-bg:#e9eef2; --info:#6b6f75; --info-bg:#f0f1f2;
  --ok:#1f7a45; --ok-bg:#e1f3e8; --warn:#a65a00; --bad:#a3122a;
  --sans:"Public Sans",system-ui,-apple-system,"Segoe UI",sans-serif;
  --mono:"IBM Plex Mono",ui-monospace,SFMono-Regular,Menlo,monospace;
}
@media (prefers-color-scheme: dark){:root:not([data-theme="light"]){
  --bg:#11161b; --panel:#182028; --ink:#e4e9ee; --muted:#9aa6b2; --line:#2a3540;
  --accent:#6fb3e0; --accent-soft:#1c3446;
  --high:#ff8095; --high-bg:#3d1820; --med:#f0b25c; --med-bg:#3a2a12; --low:#b3c1ce; --low-bg:#25303a; --info:#a4abb2; --info-bg:#22282e;
  --ok:#6fd49a; --ok-bg:#163526; --warn:#f0b25c; --bad:#ff8095; color-scheme:dark}}
:root[data-theme="dark"]{
  --bg:#11161b; --panel:#182028; --ink:#e4e9ee; --muted:#9aa6b2; --line:#2a3540;
  --accent:#6fb3e0; --accent-soft:#1c3446;
  --high:#ff8095; --high-bg:#3d1820; --med:#f0b25c; --med-bg:#3a2a12; --low:#b3c1ce; --low-bg:#25303a; --info:#a4abb2; --info-bg:#22282e;
  --ok:#6fd49a; --ok-bg:#163526; --warn:#f0b25c; --bad:#ff8095; color-scheme:dark}
body{background:var(--bg);color:var(--ink);font-family:var(--sans);font-size:15px;line-height:1.5}
.wrap{max-width:1180px;margin:0 auto;padding-inline:16px;padding-block:28px 60px}
h1{font-size:clamp(26px,4vw,38px);font-weight:800;letter-spacing:-.02em;margin:0 0 6px;text-wrap:balance}
.sub{color:var(--muted);margin:0 0 22px;max-width:72ch}
.sub code,td code{font-family:var(--mono);font-size:.86em}
.verdict{display:grid;grid-template-columns:repeat(auto-fit,minmax(170px,1fr));gap:12px;margin-bottom:18px}
.tile{background:var(--panel);border:1px solid var(--line);border-radius:8px;padding:14px 16px;min-width:0}
.tile .n{font-size:30px;font-weight:800;font-variant-numeric:tabular-nums;line-height:1.1}
.tile .l{color:var(--muted);font-size:13px}
.tile.high .n{color:var(--high)} .tile.med .n{color:var(--med)} .tile.ok .n{color:var(--ok)}
.answer{background:var(--accent-soft);border-radius:8px;padding:14px 18px;margin-bottom:22px}
.answer h2{font-size:16px;margin:0 0 6px}
.answer ul{margin:0;padding-left:20px}
.tabs{display:flex;flex-wrap:wrap;gap:6px;border-bottom:1px solid var(--line);margin-bottom:14px}
.tabs button{font:600 14px var(--sans);background:none;border:0;border-bottom:3px solid transparent;color:var(--muted);padding:8px 12px;cursor:pointer}
.tabs button[aria-selected="true"]{color:var(--accent);border-bottom-color:var(--accent)}
.tabs button:focus-visible,.filters select:focus-visible,.filters input:focus-visible{outline:2px solid var(--accent);outline-offset:2px}
.filters{display:flex;flex-wrap:wrap;gap:8px;margin-bottom:10px;align-items:center}
.filters select,.filters input{font:14px var(--sans);padding:6px 8px;border:1px solid var(--line);border-radius:6px;background:var(--panel);color:var(--ink)}
.filters input{flex:1 1 200px;min-width:0}
.count{color:var(--muted);font-size:13px}
.tbl{overflow-x:auto;background:var(--panel);border:1px solid var(--line);border-radius:8px}
table{border-collapse:collapse;width:100%;font-size:14px}
th{text-align:left;font-size:12px;text-transform:uppercase;letter-spacing:.05em;color:var(--muted);padding:10px 12px;border-bottom:1px solid var(--line);white-space:nowrap;position:sticky;top:0;background:var(--panel)}
td{padding:10px 12px;border-bottom:1px solid var(--line);vertical-align:top}
tr:last-child td{border-bottom:0}
td.where{font-family:var(--mono);font-size:12px;color:var(--muted);max-width:260px;word-break:break-word}
td.what b{display:block;font-weight:600;margin-bottom:2px}
td.what span{color:var(--muted)}
.pill{display:inline-block;font:600 12px var(--sans);padding:2px 8px;border-radius:999px;white-space:nowrap}
.s-high{color:var(--high);background:var(--high-bg)} .s-medium{color:var(--med);background:var(--med-bg)} .s-low{color:var(--low);background:var(--low-bg)} .s-info{color:var(--info);background:var(--info-bg)}
.v-ok{color:var(--ok);background:var(--ok-bg)} .v-bounded{color:var(--accent);background:var(--accent-soft)} .v-not{color:var(--med);background:var(--med-bg)} .v-bad{color:var(--high);background:var(--high-bg)}
.st-fixed{color:var(--ok);background:var(--ok-bg)} .st-partial{color:var(--accent);background:var(--accent-soft)} .st-documented{color:var(--info);background:var(--info-bg)} .st-open{color:var(--med);background:var(--med-bg)}
td.note{color:var(--muted);font-size:13px;min-width:200px}
.proof{font-size:12px;color:var(--muted);white-space:nowrap}
.num{font-family:var(--mono);font-variant-numeric:tabular-nums;white-space:nowrap}
.foot{color:var(--muted);font-size:13px;margin-top:22px;max-width:80ch}
@media (max-width:640px){td.where{max-width:140px}}
#panel h3{font-size:15px;margin:20px 0 8px}
</style>

<div class="wrap">
  <h1>Brutex workspace audit</h1>
  <p class="sub">PR #74 head <code>1087e54</code> (branch <code>final/all-fixes</code>), audited 2026-10-03 by 20 parallel workers. Every row below is backed by a Rust probe test that was run, a command's output, or a quoted <code>file:line</code>. Findings already queued in lane 1, 2, 3, the 53 from the last audit and batch-1 are left to the other threads and not counted here.</p>

  <div class="verdict" id="tiles"></div>

  <div class="answer">
    <h2>The short answer</h2>
    <ul>
      <li><b>Rust only:</b> yes. All 659 tracked files outside <code>web/</code> have allowed types, no dependency compiles C or embeds another language, and the binaries link only libc. The CI guards have 7 small bypass holes, none used today.</li>
      <li><b>O(1):</b> the five operations CLAUDE.md names hold. Mask evaluation measured flat (0.998× at 300 bits vs 1). Some operations cannot be O(1) by nature; each is named in the O(1) tab with why.</li>
      <li><b>Correctness:</b> the core sweep gave no wrong answer under attack (brute-force oracle 0 missing / 0 extra, byte-identical at 1 to 64 lanes). The real defects are at the edges: storage, pulls, the API server, indicators warm-up, and CI policy.</li>
      <li><b>Not a guarantee:</b> coverage, mutation testing and <code>cargo deny</code> were not run here; those are CI's job on #74.</li>
    </ul>
  </div>

  <div class="tabs" role="tablist" id="tabs"></div>
  <section id="panel"></section>

  <p class="foot">Full worker reports with verbatim probe output: <code>/mnt/project-files/audit-20261003-workspace/</code>. Probe test files were deleted after each run; the working tree is clean. "Proof" says how each row was established: <b>probe</b> = a Rust test written and run at 1087e54; <b>run</b> = a command run; <b>code</b> = read from source with file:line only.</p>
</div>

<script>
const F=[
// [id, area, severity, proof, title, plain explanation, where]
["gaps-6","runner / cli","high","code","Stock splits look like massive winners","Splits, bonuses and demergers are not detected or adjusted. A 1:5 split reads as an 80% crash or jump, so it can top the equity rankings that hunt rare big winners. Documented as a limit, but it hits the stated objective directly.","runner/src/audit.rs:158-170"],
["hunt-runner-1","runner","medium","probe","Pessimistic P&L biased upward","A target first touched on the time-exit bar is credited as a target hit, even in the pessimistic reading. Probe: time exit worst −75 paisa, target worst +440 on the same bar.","runner/src/grid.rs:3796-3810, 5137-5143"],
["hunt-indicators-1","indicators","medium","probe","EMA and ATR report warm too early","Both are seeded from one bar and declared warm after exactly `period` bars, when that first bar still carries 13.8% of EMA200. Engine EMA200 = 21,366 vs standard 20,050, so close_below_ema200 fired on a close above the true EMA.","indicators/src/trend.rs:198-247, 303-335"],
["hunt-pull-1","pull","medium","probe","F&O gap audit misses lost minutes after 3 Aug 2026","The gap audit still uses the 375-minute session; F&O runs to 15:40 since 2026-08-03. 10 missing minutes reported as lost=0.","pull/src/gaps.rs classify_with_subject"],
["hunt-pull-2","pull","medium","probe","Tiny or negative prices stored as zero","The expired-options decoder snaps 0.004 and −0.004 to price 0 and keeps them. The intraday decoder refuses the same input.","pull/src/rolling.rs fn paisa"],
["attackdata-1","store","medium","probe","Truncated bar file silently rebuilt empty","A month file truncated to zero or back to its header is recreated as an empty month on the write path; the .crc sidecar proving 3 bars existed is ignored and nothing is logged.","store/src/file.rs:1288-1294"],
["hunt-store-1","store","medium","probe","Bar reader accepts overlay and Greeks files","An overlay file copied to a month's .bin name opens and serves bars that pass checksums but have low > high.","store table_of(Bars) → Layout::KNOWN"],
["hunt-cli-a-1","cli","medium","probe","Run ledger can accept a duplicate run","When another writer rewrites a row and appends, a held writer accepts a duplicate and refuses the original as already recorded. Same class D-0936 fixed for Selection V1-V3 only.","cli/src/results.rs:1385-1425"],
["hunt-cli-a-2","cli","medium","probe","One duplicate receipt makes every result set unreadable","Same gap in detail-sets.bin: a duplicate receipt is written, then every later open refuses the whole manifest.","cli/src/result_set.rs:494-540"],
["hunt-api-1","api","medium","probe","Closing the browser tab cancels a hand pull half-way","The pull runs inside the request; on disconnect the handler is dropped. Landed bars stay, but the audit record is never written and the telemetry run id is never released.","api/src/server.rs /pull/spot, /pull/fno"],
["hunt-api-2","api","medium","probe","Ctrl-C cannot stop the server during a sweep","The sweep is a spawn_blocking task and runtime drop waits on it with no limit (probe: blocked 2.9 s, unbounded in real runs).","api/src/sweeprun.rs"],
["attacksweep-1","api","medium","probe","Slow-client guard bypassed with two bytes","A leading \\n\\n stops the request-head deadline. 4 idle connections at a cap of 4 blocked a real GET for over 4 s.","api/src/server.rs:16869-16895"],
["hunt-api-3","api","medium","probe","Any open web page can wipe the server log","Cross-site requests that fail are logged; 100,000 of them in 29 s evicted the whole 64 MiB history.","api/src/server.rs, telemetry"],
["hunt-api-4","api","medium","code","A retired F&O symbol can block every recovery","Recovery checks every stored attempt against the current symbol table before filtering by plan, so one old row naming a retired symbol blocks all plans.","api/src/recovery.rs:1061"],
["webcontract-1","web / api","medium","probe","Backtest page never shows condition names","The page reads commit_digest from /vocab.json, which the API never sends, so every entry rule shows 'name not loaded'. Web tests pass only because they inject it.","web/src/routes/backtest/+page.svelte:2104; api server.rs:32648"],
["errpaths-1","pull","medium","probe","AWS_PROFILE ignored for credentials","Identity discovery never reads AWS_PROFILE and silently falls back to [default] when the env identity is half set. Probe got the default key with AWS_PROFILE=prod.","pull/src/ssm.rs:307-312"],
["hunt-costs-1","costs","medium","probe","STT priced at the entry day's rate","STT is charged on the sell leg but priced at entry date: a 31 Mar → 1 Apr 2026 trade paid ₹120 instead of ₹180. Latent: no production caller yet.","costs/src/trip.rs:1222"],
["hunt-costs-3","telemetry / cli","medium","probe","Two cli processes corrupt the shared log","Both got run id 1; 60 records had 30 distinct sequence numbers; a rotated file grew past its size cap; health reported nothing.","telemetry/src/sink.rs"],
["hunt-ci-1","CI","medium","run","A PR can weaken its own gates and merge itself","The only required check, ci-ok, comes from the PR's own ci.yml; there is no CODEOWNERS and auto-merge arms every same-repo PR. PRs #17-#20 merged with 0 reviews.","auto-merge.yml:66-70; ci.yml:7885-7913"],
["hunt-ci-2","CI","medium","run","main is never re-tested after a merge","Merges by the bot token don't trigger push CI; the last push and scheduled runs on main both failed.","ci.yml:4-6"],
["testgaps-1","CI / docs","medium","code","Four ✓ invariants cite tests that don't exist","Gate 10 checks only 3-segment test paths, so ER-01, MR-23, AD-02 and ED-01 cite renamed or never-written tests.","invariant_paths.rs:128; 04-invariants.md:2741, 3045, 3552, 3577"],
["testgaps-6","CI","medium","code","11 invariants proven only by tests CI never runs","Their proofs are Rust tests under web/ compiled by hand; no CI step runs them.","04-invariants.md:4386-4932"],
["gaps-1","cli","medium","run","About 16 cli modules unreachable from any command","The old Step-3 V1-V4 chain compiles and is tested but no command or route reaches it. Making them private gave 1,334 dead-code items.","cli/src/lib.rs:160-276"],
["gaps-3","runner","medium","code","Multiple-testing and walk-forward code is test-only","benjamini_hochberg, the walk-forward bottom-half rate and the V2/V3 projections have no production caller.","runner/src/significance.rs:360; pbo.rs:160"],
["gaps-5","cli","medium","code","Cross-sectional winner table is in-sample only","The pool table answers 'which stock before it moves' with no out-of-sample split and no multiple-testing correction across 210 instruments. Documented.","cli/src/pool.rs:55-72, 817-881"],
["gaps-7","core / cli","medium","code","Survivorship bias in the equity universe","F&O membership is today's list, not point-in-time. Documented.","core/src/universe.rs"],
["gaps-8","costs / runner","medium","code","No equity charges and zero slippage","costs::trip is never called; equity results are gross. Documented; blocks Selection V6 by law.","runner/src/trade.rs:64-80"],
["o1surface2-1","cli","medium","code","cli descend rebuilds a whole rung per step","Each support step reloads the span twice and rebuilds the column. The fix in D-0997 covered `cli elite` only.","cli/src/lib.rs:14934-14949"],
["hunt-conc-1","cli","medium","code","Ledger row order follows thread timing (known)","sweep-all, range-all and pool write rows inside rayon workers. Still open as GAP13-13.","cli/src/batch.rs:631, 724"],
// low
["hunt-cli-a-3","cli","low","probe","Two sealed rows with one identity accepted on open","Cold open of runs.bin returns Ok(2) where the receipt manifest refuses.","cli/src/results.rs:1000-1013"],
["hunt-cli-b-1","cli","low","code","Re-running a finished search keeps adding checkpoints","Expression search always publishes a final checkpoint, so reruns change the disk (breaks §3 rule 5).","cli/src/expression_search.rs:531-564"],
["hunt-pull-3","pull","low","probe","Exceptional-session warning repeated per bucket","The 2024-03-02 DR Saturday at 2-minute buckets printed 54 identical lines.","pull/src/fold.rs complete_minutes_with_calendar"],
["hunt-indicators-2","indicators","low","probe","Opposite patterns fire on the same bar","Engulfing and harami both fire on equal-body reversals (43 of 26,471 rows).","indicators/src/pattern.rs:526-535"],
["hunt-indicators-3","indicators","low","probe","Homing pigeon and identical three crows are the wrong shape","Classical shapes never set bits 228 and 230. Tasuki gap and unique three river also differ (code only).","indicators/src/pattern.rs"],
["hunt-runner-2","runner","low","probe","White/SPA and Romano-Wolf disagree at p = 0.05","One uses < and the other ≤, so one report can contradict itself.","runner/src/significance.rs"],
["hunt-runner-5","runner","low","code","SPA uses an i.i.d. standard error","Hansen's test uses a long-run variance.","runner/src/significance.rs"],
["hunt-costs-2","telemetry","low","probe","Run id can repeat after restart","Reserved ids beyond the last written event are reused.","telemetry/src/sink.rs:620-628"],
["hunt-costs-4","telemetry","low","probe","One clock jump stamps every later event in the future","The timestamp clamp persists across restarts (event stamped 2100-01-01).","telemetry/src/sink.rs"],
["hunt-costs-5","costs","low","code","Cost rates have no charter source","CLAUDE.md §3 rule 1 requires docs/00-charter.md sources; rates cite the predecessor instead.","docs/00-charter.md"],
["attacksweep-2","telemetry","low","probe","Disk-full glues the next event to a fragment","An event reported Written cannot be read back after a short write.","telemetry/src/sink.rs:1173-1193"],
["attacksweep-3","api","low","probe","Repeated POST form keys use the first value","Fixed for query strings only (D-1202).","api/src/server.rs:781"],
["hunt-store-2","store","low","probe","Torn first header makes an empty month unopenable","UnknownVersion(0) forever.","store/src/file.rs"],
["hunt-store-3","store","low","code","Writer recreates a missing store root","An unmounted volume gets bars written to the wrong disk.","store writer"],
["hunt-store-4","store","low","code","New parent folders not synced","A crash can lose a new month's directory.","store/src/file.rs"],
["hunt-store-5","lake","low","probe","Lake ignores timestamp unit and time zone","ns, ms or IST files decode silently as UTC microseconds. Nothing uses lake yet.","lake"],
["hunt-store-7","store","low","run","4 tests fail when run as root","D-0995 moved such tests to the unprivileged helper but missed these four. All pass as uid 65534.","store/tests/catalog.rs; src/catalog_tests.rs"],
["webcontract-2","web","low","code","Masters page puts vendor text into innerHTML","Up to 500 characters of a vendor's response body reach innerHTML with no script-src policy.","web/masters.js; pull/http.rs:3141"],
["webcontract-3","web / api","low","code","Masters page hides a failed reload","A 502 reloaded:false still prints 'All four are on disk'.","web/masters.js; mastersrun.rs:381"],
["errpaths-2","cli","low","probe","A symlink loop reads as 'no receipt'","Path::exists() returns false on error; the reader refuses the same file. Same shape at 6 sites in step3_comparison.rs.","cli/src/result_set.rs:857"],
["errpaths-3","runner","low","probe","An invalid stop ladder silently becomes empty","unwrap_or_default turns [80,40,20] into zero stop cells with no refusal.","runner/src/grid.rs:2131"],
["errpaths-4","indicators","low","probe","Swapped tolerance widths accepted","Known condition bits fell 308 → 234 with zero refusals. Production uses pinned widths.","indicators/src/evaluator.rs:594"],
["hunt-conc-2","cli","low","code","Catalog and later-period commands write from workers","Same class as GAP13-13 in two more places.","boolean_catalog_prepared.rs:225; boolean_oos_command.rs:100"],
["hunt-conc-3","cli","low","code","Refusal text depends on HashMap order","With several bad populations the named one can differ per process.","admission_store.rs:1297"],
["attackdata-2","store","low","probe","−0.0 Greek reported AlreadyPresent","Bytes differ but the store says identical; spot −5 and vol 1e300 are also stored.","store greeks"],
["attackdata-3","pull","low","probe","Repeated JSON keys keep the last value silently","","pull/src/http.rs:971"],
["attackdata-4","pull","low","probe","Over-precise price text can land one paisa off","Only above 17 significant digits; 0 mismatches in 2M realistic prices.","pull/src/http.rs:1414"],
["attackdata-5","pull","low","probe","Fold nets negative volumes","10 + −7 = 3 looks plausible. Latent: no decoder produces negatives.","pull/src/fold.rs"],
["attackdata-7","pull","low","probe","Restated bar refused with the wrong reason","Error says TimestampsOutOfOrder.","pull ingest"],
["attackdata-8","store","low","probe","Checksum flag can be cleared to skip verification","Needs deliberate editing; random corruption cannot do it.","store header"],
["o1store2-1","store","low","probe","Timestamp lookup can take more reads than documented","37,960 bars fit in one month (16 reads, doc says 14); one-second months up to 22.","docs/06-limits.md D-1434"],
["o1surface2-2","api","low","code","Autopilot tick reads every vendor manifest twice","Doc says once per pass.","api/src/autopilot.rs:3443, 3487"],
["o1surface2-3","api","low","code","Pull landing does disk I/O on HTTP worker threads","Not timed.","api/src/server.rs:7534"],
["o1eng2-1","runner","low","probe","Sliding window rebuilds when exits move backward","Since D-1410 exits can move back (111 of 227 bars), turning amortised O(1) into Θ(window).","runner/src/outcome.rs:599-621"],
["hunt-ci-3","CI","low","probe","'cargo deny || true' passes the step-runs check","Also a line inside 'if false'.","source_scan.rs:1739-1790"],
["hunt-ci-4","CI","low","code","Nothing guards ci-ok itself","Its needs list and success check are unguarded.","ci.yml:7885-7913"],
["hunt-ci-5","CI","low","run","3,630 lines of gate tools skip lint, coverage and mutation","source_scan.rs fails default clippy with 10 errors.",".github/*.rs"],
["hunt-ci-6","CI","low","code","Gate 27 skips 179 invariant rows","Ids with a digit before the dash (FV4-01).","ci.yml:4469-4471"],
["hunt-ci-7","CI","low","code","Gate W4 runs without pipefail","Accepts a silent zero.","ci.yml:7865-7880"],
["hunt-ci-8","CI","low","code","Auto-merge says 'not enabled' while still armed","Also picks the wrong PR for stacked PRs (hunt-ci-9).","auto-merge.yml:148, 221-267"],
["rustonly2-1","CI","low","probe","Gate 1 counts an orphan lib.rs as compiled","A .github/evil.rs or crates/zz/src/lib.rs holding Python passed.","source_scan.rs:1897-1898"],
["rustonly2-2","CI","low","probe","Gate 0 misses 8 inline-interpreter forms","Heredocs, piping into node, perl -ne, python3 -Bc, --eval=.","source_scan.rs:1855-1866"],
["rustonly2-3","CI","low","probe","Gate 2 misses 3 build-script tricks","Writing .cargo/config.toml, -fuse-ld=, a spawning build-dependency.","source_scan.rs:1066-1103"],
["rustonly2-4","all","low","code","Nothing refuses 'sh -c' from Rust code","Gate 1e shadows 19 names but not sh or bash.","ci.yml:276-282"],
["rustonly2-6","vocab","low","probe","Lock fingerprint misses version-only bumps","ring 0.17.14 → 0.17.99 left it unchanged.","vocab/tests/workspace_is_rust.rs:320-338"],
["rustonly2-7","CI","low","code","Gate 1g misses RUSTFLAGS linker settings","Also CARGO_HOME and nextest --config-file.","ci.yml:461"],
["testgaps-7","tests","low","run","12 ignored tests never run in CI","No workflow passes --ignored.","lake/tests/real_lake.rs and others"],
["testgaps-11","docs","low","code","Limits doc names deleted code","§93 cli::derived_ceiling; §82 a web.yml that never existed.","docs/06-limits.md:5038, 5979"],
["gaps-11","cli","low","code","No handoff from discovery to qualification","The later-period catalog file is written by hand.","cli/src/lib.rs:353-384"],
["gaps-10","api","low","code","V6 results have no API route or page","","cli/src/ledger_v6.rs:75"],
// info (selected)
["hunt-ci-10","CI","info","code","'Coverage 100%' check enforces 90/89","Documented in D-0677; the check name is stale.","ci.yml:7181"],
["hunt-ci-12","CI","info","code","No permissions: block; actions pinned by tag","","ci.yml"],
["rustonly2-10","law","info","run","core-foundation-sys compiles on macOS","Only on the operator's Mac target, via chrono → parquet → lake.","Cargo.lock"],
["webcontract-5","api","info","code","Static assets sent without cache headers","Every page load re-downloads every chunk.","api/src/assets.rs"],
["hunt-api-6","api","info","code","Browser-launch child never reaped","Stays a zombie; the URL passed is safe.","api/src/server.rs"],
["hunt-cli-a-5","cli","info","code","One interrupted V6 write blocks that rung","Refuses loudly until the same source is rerun; cost not stated in limits.","cli/src/selection_v6.rs"],
["hunt-conc-4","cli","info","probe","Budget doc says bigger machines earn more","Core count cancels out: 9362 on 1 to 128 cores. Good for reproducibility; doc wrong.","cli rungs_within_cell_budget"],
["o1eng2-3","docs","info","code","Limits says 25 intents per minute; code allows 200","","runner/src/portfolio.rs:42; 06-limits.md:7122"],
["o1store2-2","docs","info","code","Limits doc still lists heap buffers on the cold path","The code has none and a test enforces it.","06-limits.md:9119, 12052"],
["hunt-runner-4","docs","info","code","Doc says 325 cells; test asserts 625","","runner/src/significance.rs"],
["hunt-costs-6","docs","info","code","costs cites D-0039; the costs decision is D-0041","","costs/src/lib.rs:65"],
["hunt-store-6","docs","info","code","Store doc says ragged tails are truncated","The code deliberately does not (D-0189).","docs/02-store-format.md §7"],
];

const P=[["vocab::ConditionMask::hits [batch=256]", "1 bit set", "100000", 2.1, 2.4, 20.4, 298.9], ["vocab::ConditionMask::hits [batch=256]", "300 bits set", "100000", 2.1, 4.3, 67.5, 1851.0], ["vocab::table::definition [batch=256]", "bit 0", "100000", 0.7, 1.4, 1.9, 397.5], ["vocab::table::definition [batch=256]", "bit 369 (last)", "100000", 1.0, 1.8, 2.6, 592.0], ["vocab::table::index_of [batch=32]", "first name", "100000", 14.5, 24.6, 163.1, 3179.5], ["vocab::table::index_of [batch=32]", "last name", "100000", 11.7, 23.0, 188.2, 4991.5], ["vocab::table::index_of [batch=32]", "miss 'zzzz'", "100000", 15.1, 23.8, 575.7, 6237.1], ["engine::Column::support per BAR (call/bars)", "1k bars", "10000", 1.4, 5.1, 52.6, 479.6], ["engine::Column::support whole call", "1k bars", "10000", 1380.0, 5068.0, 52566.0, 479551.0], ["engine::Column::support per BAR (call/bars)", "1M bars", "2000", 7.2, 9.1, 12.5, 14.8], ["engine::Column::support whole call", "1M bars", "2000", 7202343.0, 9139812.0, 12521422.0, 14780559.0], ["engine::primitives::offer k=1 dedup [batch=8]", "100 offered", "10008", 24.0, 37.1, 67.1, 4738.4], ["engine::primitives::offer duplicate reject [batch=8]", "100 offered", "10000", 23.1, 55.2, 136.2, 4885.6], ["engine::primitives::offer k=1 dedup [batch=8]", "100,000 offered", "12500", 36.4, 104.0, 1316.5, 14072.6], ["engine::primitives::offer duplicate reject [batch=8]", "100,000 offered", "10000", 25.6, 32.1, 47.5, 3183.9], ["store::Layout::offset_of [batch=256]", "100 bars", "100000", 1.0, 1.8, 4.9, 331.4], ["store::BarFile::read_record warm same index", "100 bars", "10000", 43.0, 62.0, 206.0, 4763.0], ["store::BarFile::read_record scattered index", "100 bars", "10000", 57.0, 3878.0, 26136.0, 255984.0], ["store::BarFile::first_at_or_after (O(log n) by design)", "100 bars", "10000", 2713.0, 11392.0, 38499.0, 265343.0], ["store::Layout::offset_of [batch=256]", "2,592,000 bars (1s x 30d, max month)", "100000", 1.0, 1.4, 2.7, 420.5], ["store::BarFile::read_record warm same index", "2,592,000 bars (1s x 30d, max month)", "10000", 44.0, 164.0, 367.0, 30079.0], ["store::BarFile::read_record scattered index", "2,592,000 bars (1s x 30d, max month)", "10000", 4471.0, 11929.0, 54282.0, 138063.0], ["store::BarFile::first_at_or_after (O(log n) by design)", "2,592,000 bars (1s x 30d, max month)", "10000", 54820.0, 104171.0, 219895.0, 1142330.0], ["pull::calendar::kind_of [batch=256]", "early open day 18232", "100000", 2.7, 5.8, 92.6, 15303.6], ["pull::calendar::kind_of [batch=256]", "late open day 20700", "100000", 3.0, 6.6, 116.0, 3771.0], ["pull::calendar::kind_of [batch=256]", "late closed day 20695", "100000", 1.9, 4.3, 113.3, 1796.8], ["pull::calendar::kind_of [batch=256]", "outside table", "100000", 1.5, 3.3, 72.5, 862.9], ["core::InstrumentKey::is_sweepable [batch=256]", "first share 360ONE", "100000", 13.6, 34.2, 143.8, 5009.6], ["core::InstrumentKey::is_sweepable [batch=256]", "last share ZYDUSLIFE", "100000", 17.0, 67.0, 210.8, 10928.7], ["core::InstrumentKey::is_sweepable [batch=256]", "index NIFTY", "100000", 8.0, 11.5, 81.6, 6701.4], ["core::InstrumentKey::is_sweepable [batch=256]", "miss cash ZZZZZZ", "100000", 11.3, 26.5, 184.3, 3838.8], ["telemetry::Sink::emit written (write(2), no fsync)", "empty log (8 MiB bound)", "10000", 749.0, 3337.0, 21466.0, 61466.0], ["telemetry::Sink::emit filtered [batch=64]", "empty log (8 MiB bound)", "10000", 2.8, 5.4, 20.8, 1124.2], ["telemetry::Sink::emit written (write(2), no fsync)", "near rotation (6608646 of 8388608 bytes)", "10000", 753.0, 3381.0, 18929.0, 130388.0], ["telemetry::Sink::emit filtered [batch=64]", "near rotation (6608646 of 8388608 bytes)", "10000", 2.8, 5.1, 7.3, 936.2], ["telemetry::Sink::emit written", "64 KiB bound, 24 rolls in window", "10000", 750.0, 2880.0, 61509.0, 97974.0], ["api::server::param", "14 B, key last", "10000", 78.0, 135.0, 260.0, 37394.0], ["api::server::param", "8155 B, key first", "10000", 129.0, 205.0, 407.0, 31129.0], ["api::server::param", "8156 B, key last", "10000", 5864.0, 30208.0, 92193.0, 775097.0], ["api::census::held_page (200 rows)", "offset 0 of 248,000", "10000", 11142.0, 29040.0, 54751.0, 151803.0], ["api::census::held_page (200 rows)", "offset 247,800 of 248,000", "10000", 11168.0, 29398.0, 42498.0, 870801.0], ["cli::results::Results::open (full scan, n=200 calls)", "1,000 existing runs", "200", 2154725.0, 3473807.0, 3803936.0, 3803936.0], ["cli::results::Results::append (sync_all per call)", "1,000 existing runs", "10000", 205190.0, 474839.0, 1544019.0, 3625542.0], ["cli::results::Results::open (full scan, n=200 calls)", "50,000 existing runs", "200", 116965002.0, 153400064.0, 164208478.0, 164208478.0], ["cli::results::Results::append (sync_all per call)", "50,000 existing runs", "10000", 222828.0, 779748.0, 4007401.0, 22214081.0], ["vocab::Expression::evaluate per bar (runner path) [batch=64]", "short program, 4 instructions", "10000", 12.8, 25.4, 292.4, 1599.5], ["vocab::Expression::evaluate per bar (runner path) [batch=1]", "max program, 1151 instructions", "10000", 3015.0, 13668.0, 58711.0, 804366.0]];
const PV=[["ConditionMask::hits", true, "flat (p50 2.1/2.1; p99 2.4/4.3)", "fixed 6-word branchless op; the p99.9 jump is noise (single-ns ops, batched)"], ["vocab table definition / index_of", true, "flat", "array index / bounded probe"], ["Column::support per bar", false, "NOT flat: 1.4 \u2192 7.2 ns/bar p50 (5x)", "same op count per bar, but 1k rows (48 KB) sit in cache and 1M rows (48 MB) stream from DRAM. It's O(1) in work and memory-bandwidth-bound in time. The 1k-bar tail (p99.9 52 ns/bar) is a scheduler preemption landing inside a 1.4 \u00b5s call"], ["store Layout::offset_of", true, "flat (1.0/1.0)", "multiply-add"], ["store read_record, warm same index", true, "p50 flat (43/44), p99 62\u2192164", "one pread syscall; tail = syscall/page-cache variance"], ["store read_record, scattered index", false, "NOT flat: p50 57 ns \u2192 4.5 \u00b5s", "a different checksum block each call means a block pread (\u22644 KB) + sidecar read + CRC every time. At 100 bars everything is one cached block. The O(1) holds in operation count; the wall cost is the cold-block verify (documented as UNVERIFIED in the code, now measured)"], ["store first_at_or_after", false, "NOT flat: 2.7 \u2192 55 \u00b5s p50 (documented O(log n))", "~7 vs ~22 probes, each one potentially a cold block verify, so it grows by more than the log ratio"], ["pull calendar::kind_of", true, "flat (2.7/3.0)", "bitmap + \u22649 compares"], ["core is_sweepable", true, "flat-ish (13.6 vs 17.0 p50; p99 34 vs 67)", "open-addressed probe length differs by symbol (worst case documented \u22648). Bounded, not identical"], ["engine k=1 dedup offer (new insert)", true, "p50 near-flat (24\u219236), p99 37\u2192104, p99.9 67\u21921,316", "pre-sized, so no rehash; a 100k-entry table falls out of L1/L2, so cache misses. Expected-O(1) hash with a memory-hierarchy tail"], ["engine dedup duplicate reject", true, "flat", ""], ["telemetry emit written", true, "flat (p50 749/753; p99 ~3.3 \u00b5s)", "one write(2), no fsync. The 64 KiB-bound case shows the rotation tail: p99.9 61 \u00b5s (rename chain + reopen, 24 rolls)"], ["telemetry emit filtered", true, "flat (2.8 ns)", "one relaxed atomic"], ["api held_page 200 rows", true, "flat (11.1/11.2 \u00b5s p50)", "slice skip is O(1); cost is the 200 Coverage rows + a Vec per row (allocation). Censuses were Absent, so no manifest probe was timed"], ["api param()", false, "NOT flat: 78 ns \u2192 5.9 \u00b5s when the key is last in an 8 KiB query", "linear scan of the query, bounded by MAX_REQUEST_TARGET_BYTES. That's O(bound), as documented, not O(1)"], ["cli Results::append", true, "p50 flat (205/223 \u00b5s, fsync), p99.9 1.5\u21924.0 ms, max 3.6\u219222 ms", "sync_all dominates. The large-ledger tail is fsync jitter plus `seen` HashMap growth (sized to the open-time count, so it rehashes while 10k appends land). These samples can't separate the two"], ["cli Results::open", false, "NOT flat: 2.2 ms \u2192 117 ms (53x for 50x)", "full scan, known O(n)"], ["Expression::evaluate per bar", false, "NOT flat: 12.8 ns (4 instr) \u2192 3.0 \u00b5s (1151 instr)", "\u0398(program length), as its doc states; bounded by MAX_INSTRUCTIONS"]];
const O=[
// [operation, where, unit, verdict class, verdict, measured, why / fix]
["Mask evaluation (hits)","vocab/src/mask.rs; engine/src/column.rs","bar × candidate","ok","O(1)","300 bits vs 1 bit: 0.998×","Six fixed 64-bit words, branchless."],
["Condition lookup","vocab/src/table.rs","call","ok","O(1)","code","Array index."],
["Bar lookup by index","store Layout::offset_of","bar","ok","O(1)","37× file: warm 0.974×, cold 1.100×","Computed offset; fixed per-handle buffer (D-1433)."],
["k=1 duplicate rejection","engine/src/lib.rs","position","bounded","Expected O(1)","code","Pre-sized HashSet insert; amortised, not worst case (stated in CLAUDE.md)."],
["k≥2 duplicate rejection","engine/src/lib.rs","pair","ok","None needed","code","Prefix join is injective, so no set exists."],
["Result append","engine drain; cli results","row","bounded","Amortised O(1)","results append flat at 1k vs 10k (fsync-bound)","Vec growth is amortised."],
["Calendar day kind","pull::calendar::kind_of","call","ok","O(1)","0.976×","At most 9 comparisons."],
["Universe membership","core::universe is_sweepable","call","ok","O(1)","code","Hash lookup plus 5-entry check."],
["Telemetry emit","telemetry sink","event","ok","O(1)","1.036× written, 0.996× filtered","Flat with log size."],
["/store paging by offset","api held_page","request","ok","O(1)","offset 0 vs 199,800: 0.99×","Does not scan to the offset."],
["Expression evaluation","runner expression","bar","bounded","O(1) per bar, O(length) per program","n vs 16n: 1.01×; ≈3 µs/bar at 1,151 instructions","Program length is capped by MAX_INSTRUCTIONS."],
["Scoring one candidate","engine Column::support","candidate","not","Θ(bars)","16× bars → 13.89× time","Inherent: support means counting every bar once. Per-bar step is O(1)."],
["Subset prune (Apriori)","engine every_subset_is_frequent","candidate","not","Θ(k), k ≤ 384","≈18 ns per probe (69 / 735 / 3,572 ns at k = 4 / 42 / 202)","Inherent to checking all k subsets. Documented."],
["Per-level canonical sort","engine sort_canonically","level","not","O(F log F)","code","Needed for byte-identical output. Documented; a merge of sorted batches could make it O(F)."],
["Bar lookup by timestamp","store first_at_or_after","lookup","not","O(log n)","2.268× on a 37× file","Bisection. Documented (D-1434). Achievable as O(1) for fixed-grid minute bars by computing the minute offset."],
["Ingest re-offer check","store already_stored","bar","not","O(log n)","2.195×","Same bisection. Documented."],
["Opening the results ledger","cli Results::open, latest_for","rung","bad","O(runs)","1k vs 10k runs: 14.13× / 10.9×","Known W2-cli8-4; doc claims O(1). Fixable with an identity index file."],
["Filtered /store census","api census::filtered","request","not","O(entries)","20k vs 200k: 9.73×","A filter must look at every entry. Documented."],
["Query parsing","api param","request","bounded","O(query length)","800 B vs 8 KB: 9.61× (0.1 ms at cap)","Bounded by the 8 KiB request cap."],
["cli descend step","cli lib.rs:14934","support step","bad","Θ(rung)","code","Rebuilds the column every step. Undocumented. Fixable as D-0997 did for elite."],
["Exit sliding window","runner outcome.rs:599","query","bad","Θ(window) when exits move back","111 of 227 bars moved back","Amortised O(1) assumption broke with D-1410."],
["Calendar from observed days","pull Calendar::from_observed","build","not","O(span)","1000× span: 3735×","One-time build. Documented."],
["Static file serve","api assets","request","not","O(file size)","code","Whole-file read per request. Documented (o1api-7)."],
];

const R=[
["Tracked files outside web/","659 of 969","pass","All have allowed extensions; .yml only in .github/workflows, .json only .claude/launch.json."],
["Symlinks, submodules, LFS, shebangs","none","pass","All files mode 100644; no .gitattributes; no .rs starts with #!."],
["Dependencies that compile C/C++ or bind another language","0 of 203 host packages","pass","cargo tree --target all; ring and cc are present in the lock but not compiled for Linux."],
["What the binaries link","libc, libm, libgcc_s, ld-linux","pass","ldd on target/debug/api and cli."],
["build.rs that starts a process","0 (cli/build.rs only, no spawn)","pass","Read in full."],
["Crates that need web/ to build","0","pass","No include_str!/#[path] into web/; api reads web/build at request time only."],
["Inline JavaScript in CI","0 (was an 86-line node -e)","pass","Fixed since the last audit."],
["Shell inside CI YAML","59 run steps; awk 73 lines, sed 62","note","Allowed by §2's .yml rule; no gate reads YAML content."],
["Process started by production Rust","xdg-open / open / explorer.exe","note","Opens the browser; documented in D-1202."],
["Guard bypasses (none used today)","7 low findings","warn","rustonly2-1, -2, -3, -4, -6, -7 and hunt-ci-5 in the Findings tab."],
];

const G=[
["cargo build --workspace --all-targets --locked","pass","Finished in 11 min 27 s, exit 0."],
["cargo fmt --all --check","pass","Exit 0 on a clean worktree at 1087e54."],
["cargo clippy --workspace --all-targets --locked -- -D warnings","pass","Exit 0."],
["cargo test --workspace --locked --no-fail-fast","pass*","6,565 passed, 4 failed, 12 ignored. The 4 are store permission tests that fail only as root; all 4 pass as uid 65534."],
["Previously failing cli test (generated_search_recovers_same_ordinal…)","pass","Passed in the full run and twice alone (116 s, 150 s). The audit-run failure was load or environment, not logic."],
["cargo test --workspace (fixes merged with PR #74, fd6df2c)","pass*","6,680 passed, 2 failed, 12 ignored. Both failures were clashes between the audit fixes and newer PR #74 code (an api body limit, a pull error kind); both fixed in a21d031 and passing alone. The full suite was not re-run after that fix; PR #74 CI re-runs it."],
["Pushed to PR #74 branch final/all-fixes","pass","a21d031, a fast-forward from 331b05c. No new PR."],
["cargo deny check","not run","cargo-deny not installed here; CI Gate 3 runs it."],
["100% line and branch coverage","not run","CI enforces 90 / 89, not 100 (hunt-ci-10, documented D-0677)."],
["No surviving mutant","not run","CI Gate 18 runs it."],
];

const H=[
["Brute-force oracle vs the sweep on 12 random columns","0 missing, 0 extra frequent sets"],
["Same sweep at 1, 2, 3, 4, 7 and 64 lanes, and halted sweeps","Byte-identical"],
["Full audit run under 1, 2, 4 CPUs and different thread counts","7 outputs byte-identical (791,066 bytes)"],
["Resume from every checkpoint boundary; truncated checkpoints","Same result; every truncation refused"],
["Empty bars, all bits set, bits 383/384/u32::MAX, retired bits","No panic; explosive case halted loudly at the ceiling"],
["Thresholds 0 and u64::MAX","Handled"],
["Store header attacks: future version, count overflow, wrong stride, flipped byte","All refused"],
["Extreme timestamps, prices, volume, OI, duplicates, empty batch","All refused by name"],
["Real disk full on a 40 KiB tmpfs; read-only mount","Clean failure, bars intact; reads fine, writes refused"],
["Second writer / reader on one file","Both get Locked"],
["Malformed CSV and JSON vendor payloads","Refused by name"],
["300k random cost calculations","0 panics, no float"],
["Greeks at T=0, σ=0, S=0, K=0, NaN, inf","Each refused with a named error; put-call parity within 1.9e-16"],
["isqrt over ≈4M inputs","0 wrong"],
["VWAP sigma vs exact arithmetic on 9,444 bars","0 mismatches"],
["Nine path-traversal spellings at the static server","400 or 404, nothing leaked"],
["mask_words above 2^53 in the browser","Sent as strings, decoded with BigInt: no precision loss"],
["CSRF from another tab to start a pull or sweep","Blocked: local Host, exact Origin and fetch metadata required"],
["Equity results reaching Selection V6 or execution","Impossible: closed NIFTY/BANKNIFTY enums"],
["VIX entering vocabulary, ranking or identity","Never"],
["Input-reachable panics in production code","None found; lints deny unwrap/expect/indexing, overflow-checks on"],
];

const ST={
"gaps-6":["partial","Proved a split moves no intraday trade's P&L but changes ~79 prior-day conditions. Every stored stock report now names its largest overnight move (D-1540). Refusing needs a charter-sourced threshold: UNVERIFIED."],
"hunt-runner-1":["fixed","Time exit outranks a target on the time-exit bar in the pessimistic reading (D-1541)."],
"hunt-indicators-1":["fixed","EMA and ATR seeded from the mean of their first period (D-1542)."],
"hunt-pull-1":["fixed","Gap audit uses the venue's dated hours, 15:40 close included (D-1529)."],
"hunt-pull-2":["fixed","Sub-paisa and negative prices refused (D-1530)."],
"attackdata-1":["fixed","Sealed month refused when the file existed with a non-empty sidecar (D-1520). A month deleted whole is still recreated; stated limit."],
"hunt-store-1":["fixed","Bar door accepts only the bar layout (D-1523)."],
"hunt-cli-a-1":["fixed","Indexed prefix re-hashed on growth; rewrite refused (D-1560)."],
"hunt-cli-a-2":["fixed","Same prefix check on detail-sets.bin (D-1560)."],
"hunt-api-1":["fixed","Pulls run detached from the connection (D-1581)."],
"hunt-api-2":["partial","Shutdown waits at most 10 s, then names what it abandons (D-1582). The sweep itself has no cancel flag yet."],
"attacksweep-1":["fixed","Blank leading lines no longer stop the head deadline (D-1580)."],
"hunt-api-3":["partial","Cross-site failure logging capped at 50 lines a minute (D-1583); same-origin clients still logged in full."],
"hunt-api-4":["fixed","Recovery filters by plan before checking symbols (D-1584)."],
"webcontract-1":["fixed","/vocab.json now sends commit_digest (D-1586)."],
"errpaths-1":["fixed","AWS_PROFILE honoured; half-set env identity refused (D-1534)."],
"hunt-costs-1":["fixed","Each leg priced at its own date's regime (D-1535)."],
"hunt-costs-3":["fixed","Cross-process lock; a second sink on one directory is refused (D-1537)."],
"hunt-ci-1":["partial","CODEOWNERS plus an owner-approval check in auto-merge (D-1604). Fully closing it needs branch protection's code-owner review, a setting only the owner can turn on."],
"hunt-ci-2":["fixed","Hourly check dispatches CI on main when its head has no run (D-1605)."],
"testgaps-1":["fixed","Gate 10 checks two-segment and bare names; four rows cite real tests (D-1606)."],
"testgaps-6":["fixed","Gate 6d builds and runs the web/ Rust tests: 68 pass; 2 need operator data and are named."],
"gaps-1":["documented","No retirement decision exists, so nothing deleted; modules recorded as unwired (D-1568)."],
"gaps-3":["documented","Recorded as unwired with a test that pins the record (D-1547)."],
"gaps-5":["open","Needs an out-of-sample split and correction across 210 instruments: a new feature, not a bug fix."],
"gaps-7":["open","Needs point-in-time F&O membership data, which the charter does not source."],
"gaps-8":["open","Needs a charter-sourced equity charge stack."],
"o1surface2-1":["documented","Reusing the column is unsafe here (identity-bound); per-step bound now in limits (D-1567)."],
"hunt-conc-1":["partial","sweep-all now files rows in walk order (D-1564); range-all and pool documented."],
"hunt-cli-a-3":["fixed","Cold open refuses duplicate sealed identities (D-1561)."],
"hunt-cli-b-1":["fixed","Exhausted rerun publishes nothing (D-1562)."],
"hunt-pull-3":["fixed","Named once per day (D-1533)."],
"hunt-indicators-2":["fixed","Equal-body reversal lights neither pattern (D-1543)."],
"hunt-indicators-3":["fixed","Four patterns take classical shapes (D-1543)."],
"hunt-runner-2":["fixed","p = 0.05 clears inclusively everywhere (D-1548)."],
"hunt-runner-5":["documented","No sourced HAC bandwidth; stated (D-1549)."],
"hunt-costs-2":["fixed","Restart seeds above the last block's run ids (D-1536)."],
"hunt-costs-4":["fixed","Future time floor named and counted (D-1538)."],
"hunt-costs-5":["open","UNVERIFIED: the charter needs primary sources for cost rates. Not invented."],
"attacksweep-2":["fixed","Fragment closed before the next event (D-1539)."],
"attacksweep-3":["fixed","Repeated form keys refused (D-1587)."],
"hunt-store-2":["fixed","Torn genesis repaired when nothing is committed (D-1521)."],
"hunt-store-3":["fixed","Missing store root refused (D-1522)."],
"hunt-store-4":["fixed","Created directories' parents synced (D-1522)."],
"hunt-store-5":["fixed","Lake refuses wrong unit or non-UTC timestamps (D-1528)."],
"hunt-store-7":["fixed","Tests run where permission bits bind (D-1526)."],
"webcontract-2":["fixed","Vendor text escaped (D-1585)."],
"webcontract-3":["fixed","Failed reload is said; footer corrected (D-1585)."],
"errpaths-2":["fixed","symlink_metadata; only NotFound is absent (D-1561)."],
"errpaths-3":["fixed","Invalid stop ladders refused (D-1545)."],
"errpaths-4":["partial","Widths::new refuses swapped widths (D-1546); public fields kept for degraded-path tests."],
"hunt-conc-2":["documented","Ordered phases need transaction re-plumbing; bound stated (D-1564)."],
"hunt-conc-3":["fixed","Reconcile walks identities in sorted order (D-1565)."],
"attackdata-2":["fixed","Byte equality; greek domain enforced (D-1524)."],
"attackdata-3":["fixed","Repeated JSON keys refused (D-1531)."],
"attackdata-4":["documented","Needs serde_json arbitrary_precision; limit stated."],
"attackdata-5":["fixed","Negative volume and over-wide buckets refused (D-1532)."],
"attackdata-7":["fixed","OverlapDisagrees names the conflict (D-1525)."],
"attackdata-8":["documented","Needs a new on-disk format version (§3 rule 8); limit stated (D-1528)."],
"o1store2-1":["fixed","Doc bound corrected (D-1527)."],
"o1surface2-2":["fixed","Tick reads only its own vendor once (D-1588)."],
"o1surface2-3":["fixed","Landing runs off the HTTP workers (D-1589)."],
"o1eng2-1":["documented","Θ(window) bound stated in limits; Newey-West effect UNVERIFIED."],
"hunt-ci-3":["fixed","step-runs refuses swallowed or skipped commands (D-1600)."],
"hunt-ci-4":["fixed","Gate checks ci-ok's needs, always() and success-only (D-1601)."],
"hunt-ci-5":["fixed","Gate tools clippy-clean; Gate 6c lints them."],
"hunt-ci-6":["fixed","Gate 27 reads 1,488 ids, up from 1,309."],
"hunt-ci-7":["fixed","pipefail and silent-zero refusal (D-1609)."],
"hunt-ci-8":["fixed","A stop disarms; PR picked by head sha (D-1604)."],
"rustonly2-1":["fixed","Only members and built tools count as compiled."],
"rustonly2-2":["fixed","Every inline-interpreter form refused."],
"rustonly2-3":["fixed","Indirect spawns from build scripts refused."],
"rustonly2-4":["fixed","Shell or interpreter spawned from crate code refused."],
"rustonly2-6":["fixed","Fingerprint hashes versions."],
"rustonly2-7":["fixed","Gate 1g reads more config doors (D-1610)."],
"testgaps-7":["documented","Each ignored test documented; they need operator data."],
"testgaps-11":["fixed","Stale limits entries corrected."],
"gaps-11":["open","Automated handoff from discovery to qualification is a new feature."],
"gaps-10":["open","A V6 results route and page is a new feature."],
"hunt-ci-10":["fixed","Check renamed to its real 90% / 89% floors."],
"hunt-ci-12":["partial","CI declares contents: read; actions not yet pinned to commit SHAs."],
"rustonly2-10":["documented","macOS-only native dependency; no change needed on Linux."],
"webcontract-5":["fixed","Content-hashed assets served immutable (D-1591)."],
"hunt-api-6":["fixed","Browser-launch child reaped (D-1590)."],
"hunt-cli-a-5":["open","Not addressed in this pass."],
"hunt-conc-4":["fixed","Doc corrected."],
"o1eng2-3":["fixed","Doc corrected."],
"o1store2-2":["fixed","Doc corrected (D-1527)."],
"hunt-runner-4":["fixed","Doc corrected (D-1547)."],
"hunt-costs-6":["fixed","Doc and D-number corrected (D-1535)."],
"hunt-store-6":["fixed","Doc corrected (D-1527)."],
};

const esc=s=>String(s).replace(/[&<>"]/g,c=>({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;"}[c]));
const code=s=>esc(s).replace(/`([^`]+)`/g,"<code>$1</code>");
const STL={fixed:"Fixed",partial:"Partly fixed",documented:"Documented limit",open:"Still open"};const stN=k=>Object.values(ST).filter(v=>v[0]===k).length;
const sevN=s=>F.filter(f=>f[2]===s).length;
document.getElementById("tiles").innerHTML=[
 ["",F.length,"new findings (1 high, "+sevN("medium")+" medium)"],
 ["ok",stN("fixed"),"fixed (code fixes carry a test)"],
 ["",stN("partial"),"partly fixed"],
 ["",stN("documented"),"documented limits"],
 ["med",stN("open"),"still open (need data, a feature or you)"],
 ["ok",H.length,"attacks that held"],
].map(([c,n,l])=>`<div class="tile ${c}"><div class="n">${n}</div><div class="l">${l}</div></div>`).join("");

const tabs=[["findings","Findings ("+F.length+")"],["o1","O(1) check ("+O.length+")"],["rust","Rust only"],["gates","Build and tests"],["p99","Tail latency ("+PV.length+")"],["held","Attacks that held ("+H.length+")"]];
let cur="findings"; try{cur=localStorage.getItem("bx-tab")||cur}catch(e){}
if(location.hash&&tabs.some(t=>t[0]===location.hash.slice(1)))cur=location.hash.slice(1);
const tabsEl=document.getElementById("tabs"),panel=document.getElementById("panel");
function drawTabs(){tabsEl.innerHTML=tabs.map(([k,l])=>`<button role="tab" id="t-${k}" aria-selected="${k===cur}" data-k="${k}">${l}</button>`).join("")}
tabsEl.addEventListener("click",e=>{const b=e.target.closest("button");if(!b)return;cur=b.dataset.k;try{localStorage.setItem("bx-tab",cur)}catch(e){}drawTabs();draw()});
const areas=[...new Set(F.map(f=>f[1].split(" / ")[0]))].sort();
function draw(){
 if(cur==="findings"){
  panel.innerHTML=`<div class="filters"><select id="fs" aria-label="Severity"><option value="">All severities</option><option>high</option><option>medium</option><option>low</option><option>info</option></select><select id="fa" aria-label="Area"><option value="">All areas</option>${areas.map(a=>`<option>${esc(a)}</option>`).join("")}</select><select id="fst" aria-label="Fix status"><option value="">Any fix status</option><option value="fixed">Fixed</option><option value="partial">Partly fixed</option><option value="documented">Documented limit</option><option value="open">Still open</option></select><select id="fp" aria-label="Proof"><option value="">Any proof</option><option value="probe">Probe ran</option><option value="run">Command run</option><option value="code">Code read</option></select><input id="fq" type="search" placeholder="Search findings" aria-label="Search findings"><span class="count" id="fc"></span></div><div class="tbl"><table><thead><tr><th>Severity</th><th>Finding</th><th>Fix</th><th>Area</th><th>Proof</th><th>Where</th><th>ID</th></tr></thead><tbody id="fb"></tbody></table></div>`;
  const $=id=>document.getElementById(id),fs=$("fs"),fa=$("fa"),fp=$("fp"),fq=$("fq"),fc=$("fc"),fb=$("fb"),fst=$("fst");
  const rend=()=>{const s=fs.value,a=fa.value,p=fp.value,q=fq.value.toLowerCase();
   const t=fst.value;const rows=F.filter(f=>(!t||(ST[f[0]]||["open"])[0]===t)&&(!s||f[2]===s)&&(!a||f[1].startsWith(a))&&(!p||f[3]===p)&&(!q||f.join(" ").toLowerCase().includes(q)));
   fc.textContent=rows.length+" of "+F.length;
   fb.innerHTML=rows.map(f=>`<tr><td><span class="pill s-${f[2]}">${f[2]}</span></td><td class="what"><b>${code(f[4])}</b><span>${code(f[5])}</span></td><td class="note"><span class="pill st-${(ST[f[0]]||["open"])[0]}">${STL[(ST[f[0]]||["open"])[0]]}</span><br>${esc((ST[f[0]]||["",""])[1])}</td><td>${esc(f[1])}</td><td class="proof">${f[3]==="probe"?"Probe ran":f[3]==="run"?"Command run":"Code read"}</td><td class="where">${esc(f[6])}</td><td class="num">${esc(f[0])}</td></tr>`).join("")};
  ["fs","fa","fp","fst"].forEach(id=>document.getElementById(id).addEventListener("change",rend));fq.addEventListener("input",rend);rend();
 } else if(cur==="o1"){
  const cls={ok:"v-ok",bounded:"v-bounded",not:"v-not",bad:"v-bad"};
  const lab={ok:"O(1)",bounded:"Bounded / amortised",not:"Not O(1), inherent or documented",bad:"Not O(1), fixable"};
  panel.innerHTML=`<p class="sub">Green is constant time. Blue is constant within a fixed cap or on average. Amber cannot be O(1) by its nature, or is documented in docs/06-limits.md. Red is not O(1) and could be fixed. Ratios compare a run at a small size with a run at a larger size: a flat ratio near 1 means O(1).</p><div class="tbl"><table><thead><tr><th>Operation</th><th>Verdict</th><th>Measured</th><th>Why, and can it be O(1)?</th><th>Unit</th><th>Where</th></tr></thead><tbody>${O.map(o=>`<tr><td class="what"><b>${esc(o[0])}</b></td><td><span class="pill ${cls[o[3]]}" title="${lab[o[3]]}">${esc(o[4])}</span></td><td class="num">${esc(o[5])}</td><td>${esc(o[6])}</td><td>${esc(o[2])}</td><td class="where">${esc(o[1])}</td></tr>`).join("")}</tbody></table></div>`;
 } else if(cur==="rust"){
  const c={pass:"v-ok",note:"v-bounded",warn:"v-not"};
  panel.innerHTML=`<div class="tbl"><table><thead><tr><th>Check</th><th>Result</th><th>Verdict</th><th>Evidence</th></tr></thead><tbody>${R.map(r=>`<tr><td class="what"><b>${esc(r[0])}</b></td><td class="num">${esc(r[1])}</td><td><span class="pill ${c[r[2]]}">${r[2]==="pass"?"Rust only":r[2]==="note"?"Allowed":"Guard gap"}</span></td><td>${esc(r[3])}</td></tr>`).join("")}</tbody></table></div>`;
 } else if(cur==="p99"){
  const f=ns=>ns>=1e6?(ns/1e6).toFixed(ns>=1e7?0:1)+" ms":ns>=1e3?(ns/1e3).toFixed(ns>=1e4?0:1)+" µs":(ns>=100?ns.toFixed(0):ns.toFixed(1))+" ns";
  panel.innerHTML=`<p class="sub">Each operation the code calls constant time, timed at a small and a very large input on the 4-core build box (load about 2, so a single max is often a scheduler preemption). At least 10,000 timed calls after warm-up, except Column::support at 1M bars (2,000) and Results::open (200). Flat means p50 and p99 do not grow with the input. Measured 2026-10-03 on integrate @ fad2895; source: scratchpad out/p99.md.</p>
  <h3>Verdict per operation</h3><div class="tbl"><table><thead><tr><th>Operation</th><th>Small → large</th><th>Measured</th><th>Why</th></tr></thead><tbody>${PV.map(v=>`<tr><td class="what"><b>${code(v[0])}</b></td><td><span class="pill ${v[1]?"v-ok":"v-not"}">${v[1]?"Flat":"Grows"}</span></td><td>${esc(v[2])}</td><td>${code(v[3])}</td></tr>`).join("")}</tbody></table></div>
  <h3>Every measurement</h3><div class="tbl"><table><thead><tr><th>Operation</th><th>Input size</th><th class="num">Calls</th><th class="num">p50</th><th class="num">p99</th><th class="num">p99.9</th><th class="num">max</th></tr></thead><tbody>${P.map(r=>`<tr><td class="what"><b>${code(r[0])}</b></td><td>${esc(r[1])}</td><td class="num">${esc(r[2])}</td><td class="num">${f(r[3])}</td><td class="num">${f(r[4])}</td><td class="num">${f(r[5])}</td><td class="num">${f(r[6])}</td></tr>`).join("")}</tbody></table></div>`;
 } else if(cur==="gates"){
  const c=v=>v.startsWith("pass")?"v-ok":"v-bounded";
  panel.innerHTML=`<div class="tbl"><table><thead><tr><th>Command</th><th>Result</th><th>Detail</th></tr></thead><tbody>${G.map(g=>`<tr><td class="where" style="max-width:none">${esc(g[0])}</td><td><span class="pill ${c(g[1])}">${esc(g[1])}</span></td><td>${esc(g[2])}</td></tr>`).join("")}</tbody></table></div>`;
 } else {
  panel.innerHTML=`<div class="tbl"><table><thead><tr><th>Attack</th><th>Result</th></tr></thead><tbody>${H.map(h=>`<tr><td class="what"><b>${esc(h[0])}</b></td><td><span class="pill v-ok">Held</span> ${esc(h[1])}</td></tr>`).join("")}</tbody></table></div>`;
 }
}
drawTabs();draw();
</script>

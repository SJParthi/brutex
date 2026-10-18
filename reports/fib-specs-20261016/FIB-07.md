# FIB-7: The Winner Gate (absorbs FIB-GATE from gate-package.json): net-only streaming gate with every check and its number, ranking key, walk-forward stepper with gate A and gate B, survivor marks and the luck check

Depends on: FIB-6

Specification as of 18 Oct 2026 (amended 18 Oct: five downside checks, every market situation, walk-forward family, daily profit focus, mover persistence, FIB-15 live proof and its requirements on other parts; every FIB decision id moved +1000 to D-6300..D-6579 because the old ids collided with feat/db-fix and the PREC blocks). Operator blocks at the end supersede earlier text.

DECISION BLOCK D-6420..D-6439. COMMON RULES (binding): read CLAUDE.md in your worktree first - it is law; build in your own git worktree cut from feat/db-fix (wt/DBFIX is read-only for you) and, after a pause, reuse your worktree's commits and edits; Rust only outside web/; never use the Py-language (no such files, no interpreter commands, never its name in any file); export PATH=/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/bin:$PATH before any cargo command; never write the real store /Volumes/WD_BLACK/brutex/fresh-20260919/runtime/store or the tick store; read-only real-data proofs use at most 4 workers and write only into a NEW scratch directory of yours; never start, stop or restart :8080 (a shadow server uses your own port); never rm -rf, find -delete, git clean or git worktree remove; fixtures are synthetic with prices that cannot be mistaken for real ones, never a copied vendor row; use only decision numbers from your block (re-grep docs/05-decisions.md before committing), put every new invariant in docs/04-invariants.md beside its test and every measured number in docs/06-limits.md (append-only rows; on a merge conflict keep both sides); a step counts only after the FIB-1 bench harness reports its p50/p99/max; gate 22 (vocab, indicators and engine depend on vocab only; vocab, indicators, engine and runner never use std::fs, File, OpenOptions, read_dir, mmap, process or include_bytes!) and gate 17 (those four never call telemetry or print - counters only); no depth, k or max-level parameter anywhere (CLAUDE.md section 6); no Default on any profile, gate or authority type - every value is named and recorded in the run identity; NO TYPED NUMBERS (operator 11:24 IST): your strategy source files contain no numeric literal except 0 and 1 and the named constants of vocab::fib_cube::FIXED_NUMBERS (each carries its category - winner-gate threshold, operator session time, trade size, operator pattern size, Fibonacci ratio, unit, representation - and its decision id), and your package ships a source-scan test that proves it; every other value is generated from data; prices are paisa i64 with i128 intermediates, statistics are f64 at full precision and never rounded for storage (CLAUDE.md section 7); say 'order flow delta' and 'order flow cumulative delta', never the bare word; equities stay labelled research (gross of every charge until FIB-11, never Selection V6 or execution authority, CLAUDE.md section 1); DONE = CLAUDE.md section 9 in your worktree (cargo fmt --check, clippy --workspace --all-targets -D warnings, test --workspace --locked with the documented cli --skip, cargo deny check), 100% line and branch coverage of your new files and no surviving mutant in them; commit WIP every 15 minutes and squash before your report; anything you could not finish is listed as NOT DONE, never claimed. OPERATOR RULES (16 Oct 2026; the newest rule wins over anything older): (a) MINUTES ALWAYS COME FROM ZERODHA (1-minute bars; 2..60-minute candles and the day bar are folded from them); GDFL is used only at the seconds level (fills at the worst price of each second; order flow per GDFL second summed into each Zerodha minute window); (b) the main idea is the PRECISE ENTRY: the smallest stop with the largest room; (c) PATTERNS ANYWHERE: a rest or a 2-3 candle micro-pattern (partial U, V, upside-down partial U) can form on a level, between levels or at the day extreme; the distance to the nearest Fib level is a measured feature, never a precondition; (d) the WINNING SHAPE: tight base -> a big trigger candle closing in the trade's direction near its own extreme with order flow the same way -> stop just beyond the base -> room to the next level of about 2.5x the stop or more; a trigger against the move cancels the long and makes the mirror short the candidate; base tightness, trigger body, trigger close location, trigger order-flow sign, stop size and room-to-stop are separate measured rows; (e) the WINNER GATE (confirmed 16 Oct 11:12 IST, state/pages-20261016/winner-gate.html): after real charges (precise NSE/SEBI method, Dhan's calculator as a comparison column) at trade sizes Rs 1,00,000 and Rs 5,00,000: win rate >= 51%, average win >= 2x average loss in R, profit factor >= 2, largest win >= 2x largest loss, >= 100 trades a year, stop >= 0.2% of the entry price, and walk-forward on unseen months (together they pass the gate, at least 70% of unseen months positive, no unseen month worse than a loss floor learned from the learning months); a mix failing gate A on its learning window is never deep-simulated, reported or walk-forwarded, while the cheap growth of child mixes follows trade count only (win rate is not monotone); after the gate, rank by the smallest stop; (f) order flow delta and order flow cumulative delta are SIGNED (negative when selling dominates); (g) every piece of work has exactly one owner; (h) the forced exit is priced only from GDFL seconds in [15:09:00, 15:10:00); a tradable day is a full calendar session with no special session and no halt; the rank pool is the point-in-time F&O membership (PIT-1); no indicators (VWAP, pivots, CPR, opening range and previous-day levels are reference stamps only); one timeframe per trade (setup and entry decision on the same N-minute candles, anchored 09:15); (i) operator 11:13 IST: every decision and every reported number is NET of the entire precise Dhan charges of each trade (brokerage on both legs, NSE transaction charge + IPFT, SEBI fee, STT, stamp duty, GST, DP where it applies, Dhan's auto square-off fee when the exit was a broker square-off); a trade is a WIN only if its net P&L > 0; gross is only a labelled extra column and never decides anything; (j) operator 11:24 IST: the winner-gate thresholds are the ONLY fixed numbers in the application; every other threshold, bucket, window, level, width, percentile and parameter is generated from each stock's own past data, refined where winners crowd and computed incrementally (a new day computes only itself); any other literal number in a strategy path is a defect. THE FUNNEL (this plan's shape, D-6300): STAGE A (cheap) finds every candidate (every base, trigger and stop) on Zerodha minutes and walks it on Zerodha 1-minute bars; each candidate lands in exactly one full cell of the 24-row FIB cube schema v1 (vocab::fib_cube); a BUC iceberg cube grows mixes by trade count only and gate A (the winner gate on NET trades, CHG-1's precise charges, at Rs 1,00,000 and Rs 5,00,000) judges every visited mix on every walk-forward learning window [0, m) - without a charge model there is no gate, only a labelled plumbing run; STAGE DEEP re-walks only the rows of gate-A survivors on GDFL 1-second bars at the worst price (Zerodha-basis levels converted exactly with DATA-1's per-stock-day factor pair), re-judges them and walk-forwards them (gate B); the newest months are a sealed final exam opened once, only after the operator's yes. Packages: FIB-1 schema/scaffold/identity, FIB-2 own-past histograms and the mover rank, FIB-3 bases, FIB-4 triggers, FIB-5 walker, FIB-6 lattice and scoring, FIB-7 the Winner Gate (it absorbs the FIB-GATE part of state/fib-run-20261016/gate-package.json, so the gate has one owner) and walk-forward, FIB-8 cli day input and file kinds, FIB-9 cli orchestration, FIB-10 new-work authorities, FIB-11 charges, FIB-12 first real run, FIB-13 unseen-month check, FIB-14 report and pages. GOAL: the WINNER GATE as its own part - it ABSORBS the FIB-GATE part the main session wrote at 11:22 IST (state/fib-run-20261016/gate-package.json; read it: every requirement and test there is in this spec, its D-5600..D-5619 block stays unused, and its CHG-1 dependency moves to FIB-11 because the gate is pure over a charge model and its tests use synthetic charges) - plus the walk-forward stepper and the luck check. Owned files: crates/runner/src/winner_gate.rs (APPLICATION-WIDE, outside runner::fib, because 'it applies to every strategy, combination and stock in the whole application'; adopting it in the older report paths is a later package, not this one), crates/runner/src/fib/wf.rs, crates/runner/src/fib/luck.rs, crates/runner/tests/winner_gate.rs, crates/runner/tests/fib_wf.rs, crates/runner/benches/fib_rows/gate.rs. Pure; gate 17. Operator rules implemented: rule (e) as confirmed 16 Oct 11:12 IST (state/pages-20261016/winner-gate.html, read it); 11:13 IST (every decision is NET of the entire precise charges; a trade is a WIN only if its net P&L > 0; gross never decides anything); 11:24 IST (the gate thresholds are the only fixed numbers - read them from vocab::fib_cube::FIXED_NUMBERS, the single source, never retype them); 'walk-forward: learn on past months, test the next unseen month, roll monthly; the newest months stay untouched as the final exam; every result shows a confidence range and a multiple-testing luck check (count every mix tried)'. (1) winner_gate.rs - WinnerGateV1 with NO Default, built only from caller data: a STREAMING ACCUMULATOR that any engine part feeds one closed NET trade at a time (O(1) per trade: counts, wins, losses, sums of net R and net money, the largest win and loss, the minimum stop, per-month buckets and per-year counts) and that answers pass or fail per check WITH THE NUMBER; and the same verdict for any month range [a, b) by prefix sums (O(months) to build, O(1) per range). Checks, all on NET trades at a NAMED size (Rs 1,00,000 or Rs 5,00,000 - a mix is a winner at a size only if it passes at that size; both are computed, and the report states the size): 1 win rate >= 51% (a win is net money > 0; zero is a loss); 2 average win R >= 2 x |average loss R| (after charges); 3 profit factor = sum of net winning money / |sum of net losing money| >= 2 (computed from money, never from averages; no losses passes only if there are wins); 5 largest win R >= 2 x |largest loss R|; 6 >= 100 trades a year (the real count of trades taken - after one position per stock, excluding unpriceable and unfilled - times 12 / the window's months; the fewest in any full calendar year shown beside it); 7 stop >= 0.2% of the entry price (true by construction under the floor reading; the minimum stop bp under the refuse reading; both shown); check 4 is gate B (wf.rs). A gate call without a charge model REFUSES by name ('the winner gate needs net trades: no charge model wired'); gross figures travel only as a labelled extra column. A GateRefusal record (named, counted) is what keeps a failing mix out of deep simulation (seconds fills, finer values, all exits), out of reports as a winner and out of walk-forward. PassBarV1 (15 Oct: win >= 50%, average win >= 2x, largest win >= 2x, >= 100 a year, stop >= 0.2%, every calendar year with trades positive, both trade directions positive) is computed and SHOWN for comparison only, never deciding. Every verdict names EVERY failed clause and by how much, in the clause-naming no-Default style of runner::bound (crates/runner/src/bound.rs:49, :195). RANKING KEY (rule e: 'rank on the pass bar first, then the smallest stop in R'): gate verdict, then mean stop bp ascending (the precise entry), then |average loss R| ascending, then average win R descending, then trades descending, then the MixKey; the reading of 'smallest stop in R' is UNVERIFIED and all three columns are shown. (2) wf.rs - WfVisitor implementing FIB-6's Visitor. Only learning months [0, n_learn) are present: a row in an exam month is refused by name. The first window W0 = one gate year (12 months, D-6309: the gate counts trades a YEAR, so a shorter window cannot judge check 6) and the min_support handed to FIB-6 = 100, the gate's own yearly count over that first year (a mix with fewer raw trades over the whole span can never pass check 6 in any window, so pruning it loses nothing) - both derived, neither a flag. STAGE A: for every step m in [W0, n_learn): gate A(m) = WinnerGateV1 (checks 1-3 and 5-7) on [0, m) with the stage-A NET outcomes at each size; a mix passing gate A at any step is a SURVIVOR: write (MixKey, step bitmap, verdict numbers) and mark its rows in a bitset over row indices (O(support), the cost of its visit) so stage DEEP re-walks only those rows; a mix that fails every step is never deep-simulated, reported or walk-forwarded - it is counted, with per-clause failure counts. STAGE DEEP (rows carry deep outcomes; only survivor MixKeys are scored, through a pre-sized hash set - an expected O(1) probe, CLAUDE.md section 3 rule 4's qualification): selected(m) = gate A on [0, m) with the DEEP net outcomes; month m's trades of every selected mix accumulate its OOS record; GATE B per mix at the end (check 4 as confirmed): (i) its OOS months TOGETHER pass checks 1-3 and 5-7 after charges, (ii) at least 70% of its OOS months that had a trade have net R > 0, (iii) no OOS month m has net R below its LOSS FLOOR = the worst monthly net R inside that mix's learning window [0, m) at step m (learned from learning months only; recorded as a decision, as the gate page asks); per-step top-K keepers by the ranking key (K = runner::topn::MAX_TOP = 25, crates/runner/src/topn.rs:39 - an existing repo bound, not a strategy threshold); closest misses per failed clause in bounded keepers written to a DIAGNOSTICS file only (a failing mix is never reported as a result); a PBO-like placement per step from a fixed-bin histogram of OOS scores across evaluated mixes (runner::pbo::place_v1 semantics, crates/runner/src/pbo.rs:194, labelled 'not CSCV, not authoritative PBO'). Selection at m depends only on months < m (property test). (3) luck.rs - trials = the sum over partitions of visited non-equivalent mixes x profiles x sizes, plus every earlier run in the results ledger (all generations and profiles); per survivor the t of mean net R against runner::significance::bonferroni_t_student (crates/runner/src/significance.rs:374) and expected_max_bailey (:260); benjamini_hochberg (:558) over the survivors' OOS p-values; Wilson 95% for the win rate and the t interval for mean R of every reported mix (f64 at full precision); and the MEASURED luck baseline FIB-GATE asks for: a label-permutation mode (outcomes permuted across the rows of each partition by a seeded Fisher-Yates over outcome indices before the lattice, the coordinates untouched) whose gate-A and gate-B passers are 'how many would pass by luck' - FIB-9 runs it as its own labelled run (--permute-seed) and the report shows it beside the real count. TESTS THAT MUST FAIL BEFORE (crates/runner/tests/winner_gate.rs and fib_wf.rs): every check at its boundary - win 50.99% fails and 51.00% passes; average-win ratio 1.99x fails and 2.00x passes; profit factor 1.99 fails and 2.00 passes; largest-win ratio 1.99x / 2.00x; 99 / 100 trades a year (with a partial first year); stop 0.199% / 0.200% under both readings; charges turning a gross pass into a net fail at a 0.2% stop (synthetic charges at both sizes; the gross column still shows the pass, labelled); a small gross win that becomes a net loss counts as a LOSS; a gate call without a charge model refused; a month split where 69% vs 70% of OOS months are positive; a loss-floor breach; the floor uses learning months only; a child passing under a failing parent is still grown (with FIB-6's lattice) and can be a survivor; a refused deep simulation counted by name; prefix ranges equal brute force; the streaming accumulator equals the prefix verdict; selection at m unchanged when any month >= m changes; an exam row refused; keepers stay bounded; the ranking order; Wilson on known values; BH on a known vector; the permutation mode keeps every coordinate and only moves outcomes; trials reconcile with FIB-6's Tally; FIXED_NUMBERS is the only source of the thresholds (source scan); determinism. BENCH (crates/runner/benches/fib_rows/gate.rs): per trade fed to the accumulator, per (mix, step) evaluation at 12, 48 and 96 months, the full stage-A stepper per mix, per row deep update and per report p50/p99/max. REAL DATA: none (pure). LEDGER: D-6420 WinnerGateV1 exactly, quoting the page's confirmed text and the 11:13 and 11:24 IST rules; D-6421 the gate refuses without charges (gross never decides); D-6422 the ranking key; D-6423 W0 = one gate year and min_support = 100, both derived; D-6424 gate B's three clauses and the loss-floor reading; D-6425 the survivor marks and the GateRefusal record; D-6426 PassBarV1 kept for comparison only; D-6427 FIB-GATE absorbed here and the permutation luck baseline. UNVERIFIED: 'trades a year (real count)' read as the window rate of trades actually taken (the fewest full year shown); the loss-floor reading; the 70% denominator (months with a trade); the reading of 'smallest stop in R'. OPERATOR 16 Oct 13:00 IST - NOT ONE LUCKY PERIOD: every mix reports its net results per DAY, WEEK, FORTNIGHT (15 days), MONTH, QUARTER, HALF-YEAR and YEAR: trades, wins, net R, share of positive periods, worst period, longest run of losing periods, and the best period's share of the total profit. A parameter-free LEAVE-ONE-OUT check is part of the gate: drop the single best day, the best week, the best fortnight, the best month and the best quarter (one at a time) and the mix must STILL pass every gate check; a mix whose pass depends on one lucky period is refused by name. Daily trade counts are tracked, and the worst day (many stocks on one market day) is shown. OPERATOR 16 Oct 13:07 IST - STOP SIZE FULLY DYNAMIC: the 0.2% minimum stop is REMOVED from the Winner Gate and from FIXED_NUMBERS (it supersedes every 'stop >= 0.2%' clause above); stop size is generated from each stock's own past like every other value and ANY stop is tested; tiny stops are judged only by the gate (net of the entire charges, which already makes a tiny stop pay a large share of R). The fixed gate numbers are now ONLY: win rate >= 51%, average win >= 2x average loss, profit factor >= 2, largest win >= 2x largest loss, >= 100 trades a year across the stocks traded, longs and shorts each positive and every year positive, walk-forward (unseen months together pass, >= 70% of them positive, none below the learned loss floor) and the leave-one-out check (drop the best day/week/fortnight/month/quarter and still pass). OPERATOR 16 Oct 13:57 IST - TRADES PER YEAR: the minimum is now >= 500 trades a year counted across all stocks the mix trades (about 2 per trading day); it supersedes every '>= 100 trades a year' clause above (a partial year is pro-rated by its tradable days and labelled). Gate numbers now: win >= 51%, avg win >= 2x avg loss, PF >= 2, largest win >= 2x largest loss, >= 500 trades a year, longs and shorts each positive and every year positive, walk-forward clauses, leave-one-out; no minimum stop; all net of the entire charges.

OPERATOR 16 Oct 16:04 IST - LONG AND SHORT ARE SEPARATE SETUPS (supersedes every 'Undirected', 'both sides searched together' and 'short mirrored' clause above wherever it would share a condition, value, threshold, stop, exit or learned number across the two directions). The operator: long and short trades in our F&O intraday stocks have different patterns and different conditions. So: (1) EVERY MIX IS ONE DIRECTION. The BUC lattice runs as two separate searches, LONG (side values long_with_gainer, long_mirror_on_loser) and SHORT (side values short_with_loser, short_mirror_on_gainer); no mix ever pools long and short trades, and for this run the run identity's direction is Long or Short, never Undirected. (2) EVERY GENERATED OR LEARNED NUMBER IS PER DIRECTION, learned from that direction's own trades and own past only: own-past histograms, tolerances, durations, stop and room seeds, the exit variant set (SL / TP / TTP / TSL, each with or without), the learned loss floor, the walk-forward selection and the leave-one-out checks. Nothing is copied from the long side to the short side or back. (3) ONLY THE GEOMETRY IS MIRRORED, because that is how a short is measured (level 0 at the day low, close location measured from the high, the stop above the base). A mirrored measurement is not a shared condition. (4) EVERY REPORT SHOWS LONG AND SHORT SEPARATELY, side by side, each with its own winners, closest misses and period tables (day, week, fortnight, month, quarter, half-year, year). (5) GATE COUNTING, until the operator confirms one: compute and show BOTH, and label neither as the winner rule yet. (i) DIRECTION ALONE: a one-direction setup passes only if it passes the whole gate by itself (>= 500 of its own trades a year across the stocks, every year positive, walk-forward, leave-one-out, all net of the entire charges). (ii) LONG + SHORT PAIR: one long setup and one short setup, each found separately, run together must pass the whole gate with >= 500 trades a year counted across both, and the long side and the short side must each be positive on its own. The gate numbers themselves do not change.


OPERATOR 16 Oct 16:45 IST - GATE COUNTING CONFIRMED: THE LONG + SHORT PAIR (supersedes item (5) of the 'LONG AND SHORT ARE SEPARATE SETUPS' block above, and nothing else in it). The winner rule is the PAIR: one long setup and one short setup, each found by its own separate search with its own conditions and its own learned numbers, are run together. The pair passes only if it passes the whole Winner Gate together: win rate >= 51%, average win >= 2x average loss, profit factor >= 2, largest win >= 2x largest loss, >= 500 trades a year counted across both sides, every year positive, walk-forward (unseen months together pass, >= 70% of them positive, none below the learned loss floor), and leave-one-out (drop the best day / week / fortnight / month / quarter and it still passes). All of it is net of the entire precise charges. On top of that, the long side and the short side must EACH be positive on their own, net. Each direction alone is still computed and shown, side by side, but only under the label 'information - each side alone, not the gate'. A pair is chosen in the learning months only; its two setups are fixed before the unseen months are read (no picking the pair after seeing them). The pair search is bounded: only setups that are positive on their own learning months enter pairing, and every pair tried is counted and reported, so the luck check (bootstrap, procedure portfolio) sees the true number of pairs tried.


OPERATOR 16 Oct 17:11 IST - FINAL LONG/SHORT GATE RULE: QUALITY PER SIDE, QUANTITY PER PAIR (supersedes the 16:45 'GATE COUNTING CONFIRMED: THE LONG + SHORT PAIR' block above; the 16:04 'LONG AND SHORT ARE SEPARATE SETUPS' items (1)-(4) stand unchanged). The operator's aim: 'find the reliable setup to make the massive amount of money'. A WINNER is a PAIR of one long setup and one short setup, each found by its own separate search with its own conditions and its own learned numbers, that passes ALL of the following, every figure net of the entire precise charges. EACH SIDE ON ITS OWN (so no weak side rides on a strong one): win rate >= 51%, average win >= 2x average loss, profit factor >= 2, largest win >= 2x largest loss, and positive in total on the unseen months. THE PAIR TOGETHER (so longs cover falling years and shorts cover rising years): >= 500 trades a year counted across both sides; every year positive; the full walk-forward (the unseen months together pass, >= 70% of the unseen months positive, no unseen month below the learned loss floor); leave-one-out (drop the best day / week / fortnight / month / quarter, one at a time, and it still passes); and the pair's own win rate, average win, profit factor and largest-win checks also pass. REPORTS compute all three verdicts for every candidate and show them side by side: 'winner rule: quality per side, quantity per pair' (the only one labelled WINNER), 'information: pair only', and 'information: each side alone'. A pair is chosen on the learning months only; both setups are fixed before any unseen month is read; only setups that pass the per-side quality checks on their own learning months enter pairing; every pair tried is counted and the count goes to the luck check (bootstrap and procedure portfolio). The gate numbers themselves do not change.


OPERATOR 16 Oct 2026 17:15 IST - NOTHING TYPED (sweep; supersedes every typed value named below): Uses FIB-1's R1-R10.

(1) min_support. Was: 100 (D-6423). Now FIB-1 R5. Update D-6423.

(2) W0. Was: one gate year. Now FIB-1 R6: two quarters, with the one-gate-year reading also run and counted. n_learn and the exam come from the exam lock, never 'all months minus 12'.

(3) Check 6, check 7 and their tests.
- Check 6 reads: '>= 500 trades a year counted across the long + short pair, a partial year pro-rated by its tradable days'. Was: 100, annualised by 12 / window months.
- Check 7 is deleted, together with every 'checks 1-3 and 5-7'. Gate A becomes: checks 1, 2, 3 and 5 on each side; then on the pair, check 6, every year positive, the pair's own 1, 2, 3 and 5, walk-forward and leave-one-out.
- Delete the tests at 0.199% / 0.200% and at 99 / 100 trades, and 'net fail at a 0.2% stop'.
- Add the pro-rated 500-a-year pair boundary, one below and exactly at, including a partial year.

(4) PassBarV1 is dropped from the FIB gate code. The older paths keep it, labelled 'superseded 15 Oct pass bar'. Update D-6426.

(5) Ranking. Was: a typed tie-break chain.
- Order by gate verdict, then the operator's smallest stop, then MixKey.
- This orders reports only and never selects. One sort per report, BOUNDED.

(6) Keepers. Was: top-K with K = MAX_TOP = 25. Now FIB-1 R9.

(7) Closest misses. Was: bounded keepers.
- Per (direction, size, failed clause), bin each mix's integer margin into a FIB-2 log histogram. Write the full histogram to DIAGNOSTICS.
- Write the nearest non-empty bin's mixes as an append-only segment: a nearer bin starts a new segment, a tie appends.
- Memory is O(1) per key.
- The test 'keepers stay bounded' becomes 'every survivor is appended, and the segment holds exactly the tied mixes'.

(8) PBO-like placement. Was: fixed bins of unstated size.
- Record the OOS milli-R per (mix, step), split by sign into two FIB-2 log histograms.
- The in-sample best is the first gate-A passer in the operator's order. Report its placement as a [lower, upper] half-rank range, labelled 'not CSCV'.

(9) Luck. Was: FWER 0.05 in Bonferroni and BH, Wilson 95%, and one permutation run with a typed seed.
- Now FIB-1 R8.
- Add adjusted-p functions to runner::significance.
- A source-scan test refuses bonferroni_t_student, clears_bonferroni and benjamini_hochberg's rejection count on the FIB path.

(10) PAIRING: implement the 17:11 pair step in wf.rs and luck.rs.
- Only per-side quality passers pair.
- A long pairs only with shorts whose support is at least Q minus its own (a two-pointer walk).
- Check order: the O(1) additive clauses, then every year positive (O(years)), then walk-forward (O(months), by adding the two sides' per-month prefix vectors), then leave-one-out.
- Every pair evaluated is a trial. fib plan projects the pair count and refuses past the budget.

(11) The loss floor is learned per direction for each side, and on the pair for the pair walk-forward, from learning months only.

(12) Trials: the full list in FIB-1 R8.

OPERATOR 16 Oct 2026 17:20 IST - O(1) AT P99 AND EXTREME CASES (attack wf o1-extreme; binding, on top of the text above): (1) FM-F057 [O1-04, XL-01, OB-02, O1-58] CASE: no part owns pairing. There are L x S pairs with O(days) clauses per pair, and no file holds both directions. DO: FIB-7 owns crates/runner/src/fib/pair.rs (pure). FIB-8 adds two kinds: 'side passers' (per direction, partition, MixKey, profile and size: per-day net money and R, n, wins, win/loss money, max win R, max loss R, stage-A step bitmap) and 'pairs'. FIB-9 adds P5, run after both direction searches and again after deep. At step m and size s, L and S are the SideQuality passers on [0,m). An empty side is a counted refusal 'no_pair: no <side> setup passed per-side quality on [0,m)', never 0/0. Pairs are enumerated canonically (long partition, MixKey, profile; short partition, MixKey, profile); any long partition may pair with any short one (FIB-7 decision). Both sides use one size, and each keeps its own exit and profile. Tier 1: the count clause, O(1) from n_L + n_S (an exact upper bound). Tier 2, for count passers only: the merged-stream clauses (checks 1, 2, 3 and 5, every year, months, floor, leave-one-out) under the deciding FM-F061 reading. Under the additive reading, a per-pair state rolls forward one month per step at O(sessions of one month). Per-pair OOS records are fixed-stride per (pair ordinal, OOS month). Every enumerated pair is a trial (pairs x steps x sizes), reconciled in the luck receipt, and the permutation baseline runs the same pairing. A verdict at m reads months < m only. The total, O(|L| x |S| x steps x sizes), is stated as not O(1) per run. FIB-12 projects |L|, |S| and the pair work; a run over --memory-budget-gib is refused by name. The information verdicts ('pair only', 'each side alone') cover the enumerated domain only, and the size of the non-enumerated domain is stated. TEST fib_pair.rs: 0 short passers give one refusal and unchanged trials; 3 x 2 passers give 6 verdicts per step and size; a verdict at m is unchanged when months >= m change; pair stats equal a brute-force merge; the rolled state equals a rebuild; separate-process directions equal one process; info_verdicts_cover_exactly_the_enumerated_domain; bench tier 1 at pair counts spanning >= 100x, p99 ratio <= 1.25. (2) FM-F058 [O1-03, XL-03, OB-01] CASE: a per-side min_support of 100 (or 500) is false under a pair count, and pruning drops pairable sides; the naive bound 500 - R_other is 1. DO: req = ceil(500 x D_W0 / Y) in i128 (D and Y per FM-F076). Pass 1, per direction, at s1 = req - (req >> 1). N_big(d) = the largest whole-span raw count among d's pass-1 SideQuality passers. Pass 2 visits only raw counts in [s2(d), s1), where s2(d) = max(1, req - N_big(opposite)). The proof (the larger side of any passing pair has raw >= s1 and passes SideQuality) goes in the entry that replaces D-6423. req, s1, s2 and N_big are recorded per direction in the identity. Remove the literal 100 from FIB-7, FIB-9 and FIB-12. FIB-12 projects visited mixes at s2. If s2 == 1, or the projection exceeds the budget, FIB-9 refuses that direction by name with the numbers, never silently applying a stronger prune. A per-side minimum count would be a new operator rule (UNVERIFIED). TEST: an 80-raw long side and a 450-raw short side are visited and pass pair check 6 (today pruned); two-pass pairs equal brute force at min_support 1 on seeded inputs; degenerate_side_refused_by_name; a source scan finds no literal 100 in wf.rs. (3) FM-F059 [XL-02] CASE: gate A applies check 6 and 'longs and shorts each positive' to a single direction. DO: WinnerGateV1 returns two kinds. SideQuality = checks 1, 2, 3 and 5 on one direction's net trades, with no count, no every-year and no opposite-direction clause. PairGate = check 6 across both sides, every year, the walk-forward clauses, leave-one-out, the pair's own checks 1, 2, 3 and 5, and each side positive unseen. Gate A = SideQuality (plus FM-F060). GateRefusal names the kind. Clauses that cannot apply are N/A, never FAIL. O(1) per trade. TEST: a long-only side at 300 a year passing 1, 2, 3 and 5 survives; no single-direction text says 'shorts not positive'; two 300-a-year sides pass PairGate check 6. (4) FM-F060 [XL-45] CASE: every SideQuality passer is deep-simulated, even when no partner can form a passing pair. DO: gate A for a side at m = SideQuality on [0,m) AND membership in at least one stage-A pair that passes PairGate's learning-window clauses at m. Marks = the rows of marked sides, with their step bits. Other sides are counted 'no_passing_pair' and not deep-simulated. Sides that pass the 16:04 'each side alone' gate are marked too, for that information verdict. FIB-12 samples 'no_passing_pair' sides to measure the false-reject rate. O(1) per side per step. TEST: a side whose every partner fails check 6 is not marked; a partner passing at m only sets bit m; deep walks exactly the marked rows. (5) FM-F061 [XD-13, XL-13, OB-06] CASE: the long side holds a stock while the short side triggers on it, and one account nets the two. DO: reading B, 'one position per stock across the pair': merged canonical order, the later entry is 'blocked_opposite_open', counted. Reading A, 'independent', labelled 'may not be executable in one account (netting UNVERIFIED)'. The overlap share is shown, and the deciding reading is the operator's. Overlapping (stock, day) cells are found by intersecting per-side bitsets in O(words); only those cells are re-scored, in O(trades on overlapping cells), BOUNDED. Elsewhere the pair is additive. In deep, same-second legs share that second's volume in canonical row order. Rounding is per FM-F124. The FIB-13 union uses the same rule. TEST: a long from 10:00 to 11:30 and a short at 10:45 on one stock: B blocks, A keeps both, overlap 2/2; a pair with no overlap equals the sum of its sides; same-second legs never exceed that second's volume. (6) FM-F062 [XL-44, OB-07] CASE: a mix that passes stage-A gate A only at a later step (because months >= m were good) is deep-selected at m. DO: selected_deep(m) requires a stage-A pass at some m' <= m (one bit test on the step bitmap) AND deep gate A on [0,m). The same rule applies to sides entering pairing and to pairs. O(1). TEST: a mix that fails stage A at every step <= m and passes at m+5 is not selected at m; perturbing months >= m leaves the selection at m byte-identical. (7) FM-F063 [O1-05] CASE: prefix sums cannot subtract the extremes (largest win and loss, worst month) for a range [a,b). DO: prefix windows [0,m) keep running extremes (O(1) per step). Arbitrary ranges use per-mix sparse tables over per-month largest win R, largest loss R and net R, built on demand at O(M log M), with O(1) exact queries. The caller passes a per-month tradable-day prefix. The minimum stop is shown, never a check. TEST: range_verdict_equals_brute_force_on_all_ranges, including a largest win in month a-1; prefix_path_allocates_nothing. (8) FM-F064 [O1-06, XL-12, OB-09, XD-48] CASE: leave-one-out has no algorithm, unit, tie rule, anchor or denominator rule. DO: best = the largest net money at the named size, ties to the earliest period. Periods are on the IST calendar: ISO week; fortnight both as month halves (1-15, 16-end) and as a rolling 15-calendar-day window (labelled readings of '15 days'); month; quarter. Edge periods are clipped to the window. After a drop: sums by subtraction; largest win and loss from the top-2 per-period extremes with distinct period ids; every year re-tested only for the period's year; the touched months re-signed; the floor from the remaining months. Main reading: the dropped period's tradable days leave D, with the unchanged-days reading shown beside it. An emptied window FAILS 'loo_empty_window'. Empty periods neither win nor break a losing run. Runs in gates A and B, per pair on the merged series, on [0,m) and on the OOS concatenation. Amortised O(1) per (mix or pair, step, granularity) from running state; the per-day series is O(days) once per visit. TEST: leave_one_out_equals_recompute, including a best day that holds the largest win, a year-straddling fortnight and a clipped week; ties golden; a one-quarter window gives loo_empty_window; bench at three window sizes, p99 ratio <= 1.25. (9) FM-F065 [O1-51] CASE: naming every failed clause allocates per (mix, step). DO: the hot-path verdict is a fixed-size Copy array indexed by clause enum (passed, value, threshold), and GateRefusal keeps one counter per clause. Text is rendered at report time. TEST: no_allocation_per_verdict. (10) FM-F066 [O1-22] CASE: BH sorts S p-values. DO: either name O(S log S) once per run in docs/06-limits.md with a bench at three S declared BOUNDED, or use an exact LSD radix sort on f64::total_cmp bit keys for O(S). TEST: bh_radix_equals_sort. (11) FM-F067 [O1-23, OB-13] CASE: the luck count scans every earlier run, has no rule per run kind, and a crashed run goes uncounted. DO: a FIB-owned append-only trials ledger, one fixed-stride record per run (run id, own trials, cumulative trials, digest of the previous record), appended under the results ledger lock. It is keyed by identity, so a rerun adds 0. Search and calibration runs count. Net re-score, verify, permutation and plumbing runs add 0 and are listed. Real-data scratch runs with --gate winner append. The Tally is sealed per partition, so a crashed run counts what it visited. Steps are a separate multiplier. Looks made outside the ledger are listed by name. Results::open's O(runs) once per process is stated. Reading the last record is O(1). TEST: cumulative_trials_equals_sum_of_receipts; concurrent_appends_keep_one_chain; a rerun adds 0; a crashed run counts its sealed partitions. (12) FM-F068 [O1-25] CASE: a fixed-bin OOS histogram merges distinct scores into ties, unlike place_v1. DO: scores as integer milli-R, counted per step in a pre-sized hash map (expected O(1) per insert), plus one O(distinct) pass for 'better' and 'tied'; this equals place_v1. A quantised alternative must state its resolution and is never called place_v1. TEST: histogram_placement_equals_place_v1 with ties. (13) FM-F069 [O1-26] CASE: FIB-13 needs each mix's OOS concatenation, and nothing persists it. DO: DEEP writes one fixed-stride record per ever-selected mix (MixKey plus per-OOS-month MonthStats), indexed by ordinal; reading is an O(1) seek plus O(months). TEST: oos_from_records_equals_rescan. (14) FM-F070 [O1-24] CASE: bench rows are missing for leave-one-out, ranges and pair tiers. DO: declare the stepper BOUNDED O(steps) and the report BOUNDED O(rows shown) in docs/06-limits.md. Add axis-named rows for leave-one-out per (mix, step, granularity), range queries and tier-1 pairs, each with p99 and max ratio <= 1.25. TEST: the gate.rs rows pass at three sizes, and rows without a class or axis are refused (FM-F001/F002). (15) FM-F071 [XL-06, XD-45, OB-37] CASE: empty or zero denominators produce NaN, which silently fails or passes a check. DO: every check returns Ratio(num, den) or NoDenominator(reason) and compares by cross-multiplication. n = 0 -> FAIL 'no_trades'. No wins -> FAIL 'no_wins' on checks 1, 2, 3 and 5. Zero losing money -> checks 2, 3 and 5 PASS iff wins > 0, flagged 'vacuous_no_losses'; zero-money losses still count as losses, and their count is shown. t, Wilson and Bonferroni at n < 2 or zero variance return a named 'undefined'. The ranking is a total order: NoDenominator after finite values, then MixKey. A total <= 0 gives 'no profit to share'. Any non-finite input refuses the verdict 'statistic_not_finite'. JSON writes {num, den} or a reason, never inf or NaN. O(1). TEST: an all-wins mix passes as vacuous; zero-money losses cause no division; a zero-trade window gives no_trades; every f64 is finite; the ranking order is golden; --json has no inf or NaN. (16) FM-F072 [XL-07, OB-72] CASE: a 2-3 trade all-win side passes per-side quality vacuously and rides on a strong partner, and a side with 0 unseen trades is undefined. DO: each side is judged on [0,m) to enter pairing, and on the OOS concatenation of its pair's selected months for the WINNER rule; both are shown. 0 OOS trades -> FAIL 'side_no_unseen_trades'. Every per-side verdict shows n, wins, losses, its share of the pair's trades, Wilson and the daily t. Two WINNER readings: literal (vacuous passes count) and strict (they do not), recorded UNVERIFIED in D-6424, with pairing run under both. O(1). TEST: a 2-win side shows 'WINNER (literal; long side vacuous)' and a strict FAIL; 0 OOS trades fails by name. (17) FM-F073 [XL-08, OB-08] CASE: a mix selected at 1-2 steps passes on 1-2 OOS months. DO: two readings. (a) Pro-rated per 13:57, labelled 'OOS span k months, pro-rated'. (b) Per D-6309, the OOS must span one gate year, otherwise 'oos_shorter_than_gate_year (k of 12)'. The 70% clause and check 6 use the same months in both. 'Selected in s of S steps' is shown beside every verdict. A pass only under (a) shows 'winner (OOS shorter than one gate year)', never a plain WINNER. Recorded UNVERIFIED. O(1). TEST: one positive 60-trade OOS month gives (a) pass, labelled, and (b) a refusal by name. (18) FM-F074 [XL-09, OB-10, XD-46] CASE: when every learning month is positive, the floor is positive and a profitable unseen month breaches it; zero-trade months are unstated. DO: compute the floor over traded learning months only, and show both 'as computed' and 'min(0, worst)'; the deciding reading is PROPOSED in D-6424. OOS months with no trade are not compared. No traded learning month -> FAIL 'no_loss_floor'. A pair's deciding floor comes from its combined monthly series, and each side's own floor (16:04) is checked on its OOS months and shown; the 16:04/17:11 conflict is recorded. O(1) running minimum. TEST: worst learning month +3R and an OOS month at +1R: min(0,.) gives no breach, as-computed breaches, both printed; an empty month is ignored; the pair floor = the minimum of the summed series. (19) FM-F075 [XL-10, OB-36, XD-47] CASE: 'every year positive' is undefined for partial years, no-trade years, calendar versus gate years, and the window it applies to. DO: IST calendar years of the judged window, evaluated on [0,m) and on the OOS concatenation. A partial year is judged on its own months, labelled 'partial year (k months)', with its count scaled by the calendar's tradable days for that year over the window's tradable days in it. A year with tradable days but no trade FAILS 'year_without_trades'; the PassBarV1 'years with trades' reading and the gate-year reading are shown, labelled. Unit per FM-F077. O(1) via a running count of non-positive closed years. TEST: a negative 3-month 2020 is named; a no-trade year fails in the deciding column and passes in the PassBarV1 column; the running count equals brute force. (20) FM-F076 [XL-11, XD-16] CASE: '12 / months' and the 13:57 pro-rating give no formula, and pool-wide holes (Zerodha's 35 missing sessions in 2020-12, 2021-12, 2022-12 and 2024-11; GDFL ending 2026-09-24) count as zero-trade days. DO: pass iff trades x Y >= 500 x D (i128). D = the observed tradable days of the window for that stage = calendar tradable days minus pool_day_missing:<feed>. Y = the tradable days of the 12 calendar months ending at the window end, from the calendar. D = 0 -> FAIL 'no_tradable_days'. FIB-9 P0 writes a per-day coverage census (pool size, members with Zerodha minutes, members with GDFL seconds). Months and years with holes are labelled 'n of m tradable days observed'. Holes are excluded from period tables and leave-one-out. The calendar-day rate is shown beside. fib plan reads the gap ledger (D-3335) and GDFL completeness and names every hole before the run. Gate number unchanged. O(1) via prefix counts. TEST: uniform trades with and without a 10-day hole give an equal observed-day rate (today's month formula differs); D = 0 is refused; the census names the hole days. (21) FM-F077 [XL-34] CASE: sign tests name no unit, so a month can be +0.3R and -Rs 40. DO: the 70% clause and the floor stay in net R. Every-year, each-side, the unseen total and the best period use net money at the stated size. The other unit is shown beside each, and the choice is recorded UNVERIFIED. O(1). TEST: that month is positive for the 70% clause and negative for the year clause, with the unit named in each label. (22) FM-F078 [XL-05, XD-41] CASE: unpriceable slots (empty or thin forced window, feed hole, corrupt bar, locked book) are excluded, which flatters the gate. DO: two readings. EXCLUDED (as today) is shown as information. AT_WORST_SEEN prices each entered-but-unpriced slot at its MAE to 15:09:59 (deep: the worst real second; stage A: the worst minute up to 15:09), net of all charges including the square-off fee. It is never a win, even if net > 0, and is labelled 'not a fill: real loss may be larger'. The worst price after 15:10 is shown as information only, never as a fill. The gate decides on AT_WORST_SEEN, recorded in D-6424 as conservative and UNVERIFIED. The unpriceable count and share are columns per mix, side and pair. Gate B shows the share of stage-A trades with no deep outcome. O(1). TEST: 60 wins and 40 losses, 10 of them unpriceable at MAE -6R: EXCLUDED passes, AT_WORST_SEEN fails and names the cause; a positive worst-seen result is not a win; a source scan finds no price at or after 15:10 in the reading. (23) FM-F079 [XD-26, XL-48] CASE: a pass rests on one flagged trade, or on deep refusing exactly the worst stock-days. DO: keep a second set of per-side and per-pair sums that excludes trades with any data flag (entry_beyond_stop, fill_on_suspect_print, flagged_print_fill, silence_in_trade, gap_through_stop, touch_unwitnessed, cross_feed_touch). Each verdict states how many flagged trades it includes and its result without them, labelled 'pass depends on N flagged trades'. Each deep verdict carries its deep coverage (by refusal reason) and a mixed reading that fills missing deep outcomes with stage-A outcomes. Whether flagged-only or deep-only passes may decide is PROPOSED; the gate and the WINNER label are unchanged until the operator rules. O(1). TEST: a side that passes only through one suspect-print trade shows the label and FAILS with flagged trades excluded; with the 3 worst trades on no_execution_seconds days, deep-only passes, the mixed reading fails, and the coverage names the reason. (24) FM-F080 [OB-11] CASE: per-trade t and Wilson treat correlated same-day trades as independent. DO: compute t, its interval and Wilson on the daily net R series, per mix and per pair. Show n_trades, n_days and the effective n; per-trade figures are labelled information. O(1) per trade. TEST: cloning every trade onto ten identical stocks leaves t unchanged (today about sqrt(10) higher). (25) FM-F081 [XL-35, OB-12, OB-38, OB-73] CASE: permuting outcome indices moves exit seconds, money scale and day; one seed gives one draw; deep never re-applies the permutation. DO: a seeded Fisher-Yates per (partition, direction) within IST-day strata, moving each row's whole outcome bundle (R, win flag, charges in R, every size and profile). Rows keep their own seconds and their qty x risk. B maps are scored in one lattice walk (O(B) per row-touch), with B taken from the bench budget. Report B, the null distribution and p >= 1/(B+1), and 'one draw' when B = 1. The map is part of the stage-A identity. fib deep applies the same map, labelled luck-baseline, and FIB-12 lists that command. The null hypothesis goes in D-6427. O(rows) per map. TEST: exit seconds are unchanged and >= entry; blocked counts equal the real run's; both sizes move together; B maps in one walk equal B separate runs; with a planted edge, permuted gate-B passers fall to the noise level. (26) FM-F082 [OB-39] CASE: identical trade sets in several exit partitions are listed as separate winners. DO: a blake3 trade-set digest per visited mix (O(support), part of the visit's own cost). Keepers and winners are grouped by digest as 'also in partitions ...'. Trials unchanged. TEST: identical_trade_sets_in_two_exit_partitions_are_one_keeper_row. (27) FM-F083 [XD-29, XL-43, OB-40] CASE: a pass driven by one share, or by many concurrent positions, is invisible. DO: information columns per mix and per pair: shares traded, distinct stock-days, the best share's share of net profit, the verdict with that share dropped, max and p99 concurrent positions (fixed log-bin histogram), and peak notional per day. They never decide (PROPOSED). O(1) per trade via a pool-width array with a touched-list reset and a running open count. TEST: 90% of profit from one share shows 90% and a failing drop-best info verdict; 50 entries in one minute give a peak of 50. (28) FM-F084 [XL-38] CASE: dust partial fills count as full trades. DO: a second reading weights each trade by filled / intended quantity in check 6 and in the win rate, shown beside, UNVERIFIED; the filled-share distribution is reported. O(1). TEST: 500 trades with 200 filled at 1% count 302 in the weighted column. (29) FM-F085 [XL-39] CASE: float division can flip an exact boundary. DO: wins x 1000 >= 510 x n (i64); win_money >= 2 x |loss_money| (i128); trades x Y >= 500 x D; sum_win_R x n_loss >= 2 x |sum_loss_R| x n_win on canonical-order sums. TEST: 51/100 and 5,100/10,000 pass, 5,099/10,000 fails, PF exactly 2 passes, and a source scan finds no '/' in any threshold comparison.

DOWNSIDE AMENDMENT TOUCH-POINTS FOR THIS PART (18 Oct 2026; the full block is in FIB-07, which owns every definition):
FIB-7 (owner): adds crates/runner/src/winner_gate/downside.rs, crates/runner/src/fib/exit_order.rs and crates/runner/tests/winner_gate_downside.rs, and takes ownership of crates/runner/src/fib/pair.rs and crates/runner/tests/fib_pair.rs (all five to be added to plan.json owned_files). Changes winner_gate.rs (pub mod downside), fib/wf.rs, fib/luck.rs, tests/fib_wf.rs and benches/fib_rows/gate.rs. Ledger entries at block offsets +8..+12 (D-6428..D-6432 before FM-F008's renumbering)

REQUIREMENTS FROM FIB-15 LIVE PROOF (18 Oct 2026; FIB-15.md section (12); the freeze of a live setup is refused until each lands):
- RQ-7 FIB-7: the worst week-to-date accumulator, the closed-week minimum, the append-a-day leave-one-out API (amortised O(1) per day), per-scheme references, the 13:31 regime clause, and the mixed reading covering every data-refused stock-day with Zerodha minutes. Until: this part's build is refused by the plan check.

OPERATOR 18 Oct 2026 ~13:07 IST - DOWNSIDE CHECKS (worst day, worst week, max drawdown, losing streaks, recovery time; per direction and for the pair; net of the entire precise charges; binding, on top of the text above). This block adds clauses to gate B and to FIB-13's exam. It supersedes nothing else, except where a line below says so. Where the 18 Oct 13:26 (daily focus), 13:29 (walk-forward family) and 13:31 (every market situation) blocks say 'the five downside checks', they mean the clauses defined here.

The operator, verbatim: "Yes, add all five downside checks (worst day, worst week, max drawdown, losing streaks, recovery time) to the Winner Gate, per direction and for the pair, net of all charges."

Why (operator): every day and every week must be checked so that no lucky setup survives, and the losses must be checked fully, because in real live trading we would face a blood bath.

The accepted proposal: each check compares the UNSEEN months with what the same setup did in its own LEARNING months, so no fixed number is added.
- (1) Worst day: no unseen day loses more than the worst learning day.
- (2) Worst week: no unseen week loses more than the worst learning week.
- (3) Max drawdown: the biggest fall from a high in the unseen months is no deeper than in the learning months, at Rs 1,00,000 and at Rs 5,00,000.
- (4) Losing streaks: the longest run of losing trades, losing days and losing weeks in the unseen months is no longer than while learning.
- (5) Recovery time: the time to climb back from a drawdown is no longer than while learning.

Declined by the operator: a fixed share of positive weeks. It is not added.

No gate number changes and no number is added. FIXED_NUMBERS itself is not changed (see (9)), and no new literal enters any strategy file.

(1) UNITS, SIZES AND STREAMS.
- NET. Every figure is net of the entire precise charges (11:13 IST). CHG-1, through FIB-11 in precise mode, decides. FIB-11's Dhan-calculator re-score ('fib net') calls the same downside code under its own identity, is labelled, and decides nothing. Gross is a labelled column only.
- DECIDING UNIT. Net MONEY in CHG-1's exact integer unit: crates/costs' scaled unit (paisa x RATE_SCALE), the unit FM-F123 stores unrounded lines in. Everything is held in i128 in that unit: trade nets, day and week nets, curve samples, references and comparisons. Paisa is used for display only.
- Why money:
 - The blood bath is rupees lost in one account.
 - At one trade size, a 1R loss costs very different rupees on a tight stop and on a wide stop.
 - The day rounding adjustment is money (FM-F126).
 - It keeps FM-F077's grouping: every year, each side, the unseen total and the best period are judged in money. Only the 70% clause and the monthly floor stay in net R.
- What each check counts:
 - D1 and D2: a period's net money.
 - D3: how far the realised money curve falls below its running high.
 - D4: counts of trades, traded days and traded weeks.
 - D5: observed tradable days.
 - Net R is shown beside every money figure.
- BOTH SIZES, ALWAYS, NEVER SCALED. Every clause runs per (unit, profile), where FIB-6's profile carries the size (FM-F117). At each size the unit has its own:
 - trade set ('size_below_one_share' refuses a slot at one size only);
 - exit seconds (FM-F035's spill depends on qty);
 - blocking (FIB-6's one position per stock, and FM-F061 reading B);
 - day ledger and curve.
- The flat parts of the charges weigh more at the smaller size: the per-order brokerage cap, Dhan's square-off fee, and the rounding per client, security and day. So signs, runs, worst periods and the whole curve can differ between the two sizes.
- A unit passes at a size only if it passes at that size (D-6502). Both sizes are computed, and every figure names its size.

(2) CALENDAR, PERIODS AND STATUS.
- OBSERVED TRADABLE DAY: CAL-1's tradable_day minus the pool-wide holes pool_day_missing:<feed> of every feed the stage reads (deep reads Zerodha minutes and GDFL seconds; FM-F076).
 - Rule (h) defines tradable_day: a full calendar session with no special session and no halt. A weekend session is refused (FM-F115). See (8) for halt days.
 - A HOLE is not a day. It is never winning, losing, flat or empty. It is never part of a run, and it is never elapsed recovery time.
 - A span with holes is labelled 'spans k hole days' or 'n of m tradable days observed', with the calendar reading beside.
 - A stock refused for one day (FIB-8's per-instrument check, FM-F094) leaves the day a day. Refused stock-days are counted per period.
- CALENDAR TABLE (FIB-10; built once per run beside FM-F076's census). For each observed day it holds:
 - its calendar index cidx(d): the number of observed days before d, from the CAL-1 prefix minus holes;
 - its civil day number;
 - the ISO 8601 week key (ISO week-year, week number) of its IST date;
 - its month-half key (days 1-15, 16-end), month id and quarter id;
 - its halt flag.
- The ISO key comes from the calendar authority (pull::calendar, beside weekday_of), never from a strategy file. downside.rs compares only keys and indices.
- The rolling fortnight uses FIB-1's days_per_fortnight row. FM-F064 and FIB-6 name 15 calendar days, but origin/cloud/fib1-20261016 holds 14. FIB-1 fixes the row: the spec is the authority and the branch is the stale copy.
- RECORD INDEX: for an unseen record, ridx(d) = cidx(d) minus the observed days of the non-selected months before d. The offset is a running count updated at month boundaries. Both indices cost O(1).
- TRADING WEEK: the ISO week of the IST date. Inside a span, a week is its observed days in that span.
 - A week cut by the span's edges is CLIPPED. The edges are: the start or end of a learning window, the record's first and last weeks, and a non-selected month.
 - A clipped week is a full participant: it is compared, and it counts in runs. It is labelled 'clipped week (k of n tradable days)'.
 - Two consecutive selected months JOIN a week that straddles them.
 - A week with no observed day in the span does not exist. A week with no priced trade is EMPTY.
- TRADE STATUS: a trade WINS if and only if its unrounded net money at the size is > 0. Otherwise it is NON-WINNING (11:13: zero is a loss). Its status comes from the slot's total unrounded net over all of its exit orders.
- DAY NET = the sum of the day's trades' unrounded nets, minus max(a, 0).
 - a = the day's rounded charges minus the sum of its unrounded trade charges, in FM-F126's sign: a positive a is an extra charge.
 - As in FM-F126, a negative a never lifts a deciding figure. The exact a is shown beside.
 - Ledger: a side uses its own DayLedger, keyed (side unit, stock, day). A pair uses its own DayLedger (FM-F124).
 - WEEK NET = the sum of its day nets inside the span.
- PERIOD STATUS: a traded day or week is WINNING if its net > 0, LOSING if its net < 0, and FLAT if its net = 0. LOSING and FLAT are NON-WINNING.
 - A FLAT period starts or extends a losing run. As a run's first element, it sets the run's m0. It adds no loss to D1 or D2. Flat periods are counted and shown.
 - An EMPTY period (observed, with no priced trade) neither wins nor loses, and it neither starts, breaks nor extends a run. It adds no loss. It IS elapsed time for D5.
- From here on, 'losing run' means a run of NON-WINNING trades, traded days or traded weeks.

(3) TRADE ORDER AND THE REALISED CURVE.
- EXIT ORDERS. Under FM-F125, a slot has one exit order per distinct exit: its own exit or stop (a spill inside it stays one order), plus any forced remainder.
 - FIB-5's slot output gains, for each exit order, its last fill second, its filled qty and its exit value, all in exact integers. This is OutcomeRecord v2; v1 stays decodable.
 - Each exit order books its realised money: its exit value, minus its share of the entry value, minus its own charge lines.
 - The entry share is qty-weighted. The integer remainder goes to the slot's last order, rounded against the trade, so the orders sum exactly to the slot's net.
 - The entry order's charge lines are booked with the slot's first exit order.
 - A v1 record books every order at the slot's exit second, labelled 'exit orders merged (OutcomeRecord v1)'.
- AN ENTERED BUT UNPRICED SLOT, priced under AT_WORST_SEEN (FM-F078), is NON-WINNING. It books min(0, its worst-seen net money) at SESSION_FORCED_WINDOW_END_SECS - 1 (15:09:59), labelled 'not a fill'.
 - A positive worst-seen figure never lifts the curve, never ends a drawdown and never makes a day win.
 - The unclamped figure is shown beside.
- TRADE ORDER, for D4's trade run: (observed day, last exit second, first entry second, stock slot, decision id), taken from FIB-5's slot output.
 - After these keys, a tie can only be a long and a short of one stock in a pair under FM-F061 reading A (one position per stock per mix, D-6402).
 - A tie group is evaluated in BOTH orders. There are two run states, which converge at the next win, and the run check fails if either fails. Both are shown. No direction is typed first.
 - This supersedes D-6403's entry-ordered streak wherever a streak decides or leads a report, because a loss is felt when it closes. FIB-6's scorecard keeps its own figure, labelled.
- EXIT ORDERING is done once per (unit, profile), when the unit's lane is built ((10)), by runner::fib::exit_order (FIB-7, pure).
 - It is a stable LSD radix sort over the packed key, with 2^u8::BITS buckets.
 - The number of passes = the packed key's byte width (from size_of), minus the byte positions that are constant across the unit (min == max).
 - Input already in order is detected by one O(n) check and not sorted.
 - Cost: O(passes x (n + 2^u8::BITS)). Pair walks only merge sorted lanes.
- EQUITY CURVE E, per (unit, profile, scheme, reading, span): the cumulative booked net money. E = 0 before the span's first sample.
 - SAMPLES: one per distinct exit instant, with every exit order of that second applied together, so a tie order can never draw a dip the account never saw. Also one at each traded day's close, applying -max(a, 0).
 - RUNNING HIGH H = max(0, every sample so far in the span). DRAWDOWN = H - E.
- THE CURVE IS REALISED, AND THE DAY CLOSE IS NOT ALWAYS EXACT.
 - Positions count when their exit orders are booked.
 - The unrealised trough of concurrent open positions is not measured. Measuring it needs a per-minute mark of every open position, which is not O(1) per trade.
 - A slot that is unpriceable because no print existed in the forced window (a circuit-locked stock) may in reality have been carried overnight, through MIS conversion or an auction close-out. The rule and penalty are UNVERIFIED and have no charter source. That real loss is unknown: it is flagged, never modelled (see (8)).
- DRAWDOWN EPISODE.
 - The PEAK is re-bound at EVERY sample with E >= H. So an episode's peak is the last such sample before E first goes below H. It falls on day p.
 - Its RECOVERY is the first later sample with E >= that H, on day r. A return exactly to the high ends the episode, and a later dip starts a new episode with its own peak and m0.
 - Its trough is its lowest sample, on day t (the first, if tied). Its depth = H - min E.
 - idx is the clock in use (cidx or ridx). UNDER-WATER time = idx(r) - idx(p). CLIMB time = idx(r) - idx(t).
 - While the episode is open at the close of observed day d: under water so far = idx(d) + 1 - idx(p), and climb so far = idx(d) + 1 - idx(t). These are the least each can still measure.

(4) THE UNITS JUDGED.
- A SIDE is judged on its STANDALONE stream at each size. That stream is its own direction's trades as its own mix (FIB-6's one position per stock), with its own DayLedger and no blocking by the other direction.
 - Its learning references and its unseen checks both use this stream.
 - Under WINNER, its unseen record is this stream over the pair's selected months (FM-F072). For 'information: each side alone', it is this stream over the side's own selected months.
 - Why: 16:04 item 2 requires every learned number per direction, from that direction's own trades. Under FM-F061 reading B, the in-pair side stream loses entries blocked by the OTHER direction and shares the pair's rounding. Building a side's references from it would make one long setup's references depend on its short partner.
 - The side's in-pair stream (blocked entries removed and counted) is shown as information.
 - Side references depend only on the side, so they are computed once per (side, profile, scheme) and stored ((10)).
- THE PAIR is judged on the merged stream of its two standalone side lanes, under FM-F061's reading, with the pair's own DayLedger (FM-F124).
 - Reading B: one position per stock across the pair; the later entry is 'blocked_opposite_open' and counted. Reading A: the two sides are independent.
 - The pair's day nets = the sides' day nets, corrected on the overlap cells that FM-F061 re-scores, plus the pair ledger's adjustment.
 - The pair's references come from the merged stream only, never from the sides' references: a pair's worst day is not the sum of its sides' worst days.
- FM-F061 IS NOT YET RULED. Until the operator names a reading, both are computed and BOTH decide.
 - A pair is a plain WINNER only if it passes under both.
 - A pair that passes under one reading only is shown as 'WINNER under reading <A|B> only (FM-F061 not yet ruled)'.
 - This interim rule is a term of the gate-B judgement identity ((9)), so a ruling re-keys only the judgement.
- ROWS WITH NO DEEP OUTCOME are NEUTRAL. These are FM-F052's no_execution_seconds and factor_refused, and an unpriceable forced exit with no worst-seen second.
 - A neutral row is not a win and not a loss, carries no money, and never starts, breaks or extends a run. Neutral rows are counted per period, by reason.
 - A day whose only rows are neutral is EMPTY, labelled 'k neutral rows'.
 - FM-F079's mixed reading is computed beside: each missing deep outcome is filled by its stage-A outcome, and stage-A unpriceable slots are priced AT_WORST_SEEN. A pass that fails it is labelled 'pass depends on N deep-refused trades'.
 - Whether the mixed reading decides stays PROPOSED, as in FM-F079.

(5) LEARNED REFERENCES (learning months only, per scheme, per version).
- Ref(u, profile, scheme, m) comes only from that scheme's learning window W(scheme, m), per the 13:29 family:
 - ANCHORED: [first month, m);
 - ROLLING: [m - L, m).
 - It never reads a month >= m, or a month outside W.
- VERSION. Ref(.., m) is built from the unit's trade set under the Generation version in force at step m (R1: born at or before m). These are the same trades gate A saw at m.
 - When a version born at step b changes a unit's trade set, the unit's lane, its day data and its learning state are rebuilt from the first month under that version. This costs O(trades before b) per changed unit and version, BOUNDED ((10)).
 - References for m < b stay bound to the older version.
 - Each month of the unseen record reads the trades of the version in force in that month.
- Ref holds eight values. Each also carries its evidence count, its provenance (the date, ISO week or episode that set it), and its second-worst value from a distinct period, run or episode.
 - L_day = max(0, the largest -net over the traded days in W).
 - L_week = the same over the weeks in W with a priced trade, clipped at W's edges.
 - DD = the largest drawdown of W's own curve, which starts at 0 at W's start.
 - RT, RD and RW = the longest losing runs of trades, traded days and traded weeks inside W. A run cut by W's start counts only its part inside W.
 - UW and CL = the longest under-water time and the longest climb time inside W, counted in W's observed days.
- OPEN ELEMENTS: the week, runs and episode still open at W's end enter ONLY the snapshot Ref(m), at their so-far values.
 - The open week enters at its clipped net. A run enters at its length so far. The episode enters at its depth so far, and at its under-water and climb times so far.
 - A censored episode is labelled 'censored at step m (k days, not recovered)'. That is a lower bound: never infinity and never dropped.
 - The running maxima take CLOSED elements only, so Ref(m) = max(the closed maxima, the open element's so-far value).
 - Why: a clipped week can recover after the boundary, and a censored climb can shrink when a deeper trough comes. Folding a so-far value into the running maxima would loosen every later reference.
- FM-F074's positive floor is not used. The clauses compare losses, and a day that made money lost nothing.
- ZERO REFERENCE: a window with no losing day (or week, trade, drawdown, or day under water) has reference 0 for that clause. Any unseen loss of that kind FAILS, labelled 'learned 0: no losing learning <period>'.
 - There is no infinite floor (it would let every loss pass), no skipped clause (a free pass), and no typed allowance (NOTHING TYPED).
 - The evidence is shown beside every reference: traded, losing and flat learning days and weeks; trades; episodes.
- NO REFERENCE: if W has no traded learning day, every clause FAILS 'downside_no_learned_reference:<clause>'. It is never N/A.
- START-MONTH RULE: every unseen element is judged against Ref(m0), where m0 is the month of its first unseen sample:
 - a day: its own month;
 - a week: the month of its first unseen day;
 - a run: the month of its first non-winning element;
 - a drawdown episode: the month of its peak sample. A peak at the record's opening 0 uses the month of the record's first sample.
 - Why: the walk-forward steps one month at a time, so month m is unseen at step m and becomes learning at step m+1. A reference re-read later would contain the start of the very episode it judges, and a drawdown spanning several steps would raise its own bar month by month. Under this rule, no part of an element is inside its own reference.
 - The bound reference can be larger or smaller than a re-read one, because references are not monotone: clipped weeks recover, censored climbs shrink, and rolling windows drop months. The re-read ('rolling') reference is shown beside. It is never claimed that this rule is stricter.
- EXAM: Ref is frozen at EXAM_FROM (D-6520), per scheme, as W(scheme, EXAM_FROM). Exam months never enter learning.

(6) THE CHECK ON THE UNSEEN MONTHS.
- UNSEEN RECORD, per (unit, profile, scheme, reading): the OOS concatenation of that scheme's selected months, as the walk-forward reads them.
 - Under WINNER: the pair's selected months, for the pair and for each side (FM-F072).
 - For 'each side alone': the side's own selected months.
 - Under FM-F073's readings (a) and (b). A pass only under (a) is never a plain pass.
- JOINING:
 - The curve starts at 0 at the record's first sample and stays flat across a non-selected month.
 - A run continues across a non-selected month, since nothing of the record lies between.
 - A week is clipped at a non-selected month and joined across consecutive selected months.
- TWO CLOCKS FOR D5; BOTH DECIDE (PROPOSED (d)).
 - The RECORD clock ridx: the setup trades only in its selected months.
 - The CALENDAR clock cidx: the account still holds the loss while the setup is not selected, and the usual reason a setup is deselected is a recent loss.
 - Learning windows are contiguous, so the two clocks agree on them.
- The clauses use integer comparisons only: i128 in CHG-1's exact unit, counts and indices, with no division and no f64. Equal passes. One exact unit, one period or one day more fails.
 - D1 WORST DAY: for every unseen traded day, -net <= L_day(m0). Fails 'worst_day_exceeds_learned'.
 - D2 WORST WEEK: for every unseen week with a priced trade (clipped or joined), -net <= L_week(m0). Fails 'worst_week_exceeds_learned'.
 - D3 MAX DRAWDOWN: at every unseen sample, H - E <= DD(m0 of its episode). Fails 'drawdown_exceeds_learned'.
 - D4 LOSING STREAKS: at every extension, the trade run <= RT(m0 of that run), the traded-day run <= RD(m0), and the traded-week run <= RW(m0). A week extends its run when it closes. Fails 'losing_trade_run_exceeds_learned', 'losing_day_run_exceeds_learned' or 'losing_week_run_exceeds_learned'.
 - D5 RECOVERY TIME: at every observed day close while under water, on each clock, both of these must hold: under water so far <= UW(m0 of the episode), AND climb so far <= CL(m0). Both readings decide (PROPOSED (b)). Fails 'recovery_underwater_exceeds_learned:<record|calendar>' or 'recovery_climb_exceeds_learned:<record|calendar>'.
- About D5:
 - An episode fails on its first day over, whether or not it ever recovers.
 - An episode still open at the record's end, and within its references, passes labelled 'open at end (j of k days)'. Its verdict carries 'unseen drawdown still open'. It is never a plain pass.
- RECORDS, per sub-clause (each D5 clock separately), each kept in O(1):
 - the FIRST BREACH: unit, profile, scheme, reading, the day, week or episode id, the value, the reference, m0, ridx and cidx;
 - the WORST unseen value;
 - the CLOSEST CALL: the minimum over elements of (Ref(m0) - value), with its element.
- The downside sub-clauses have their own clause enum and their own fixed-size Copy verdict array, on FM-F065's pattern. FM-F065's existing array is unchanged. GateRefusal gains one counter per sub-clause, in a new record version. Text is rendered at report time.
- STEP-m PREFIX VERDICT, for the 13:26 daily-focus ordering:
 - verdict(u, scheme, m) = no sub-clause has its first breach in the record's months < m. D5 is evaluated through the last observed day before m.
 - It costs O(1) per (unit, m), from the stored first-breach months, and never reads months >= m.
 - Leave-one-out is not part of it; it is labelled 'prefix verdict, no leave-one-out'.
 - A unit with no unseen month before m is admitted, labelled 'no unseen record before m', and counted (PROPOSED (q)).
 - The 13:26 ordering keys 'smaller worst learning day / week' are that scheme's L_day(m) and L_week(m).

(7) WHERE THE CLAUSES SIT.
- They run only in gate B (the deep walk-forward, under EVERY scheme of the 13:29 family; a failure names the schemes it failed) and in FIB-13's exam.
- Gate A has no unseen month. In SideQuality and in PairGate's learning-window clauses, D1-D5 are N/A, never FAIL (FM-F059). No gate-A verdict changes, and neither does any FM-F060 mark, survivor, R2 split, R5 bound or FM-F058 proof.
- WINNER (17:11). Both are required, at the size, under every scheme, and under both FM-F061 readings until one is ruled:
 - EACH SIDE ON ITS OWN passes D1-D5 on its standalone stream, against its own references;
 - THE PAIR TOGETHER passes D1-D5 on the merged stream, against the pair's references.
- 'information: pair only' carries the pair's D1-D5. 'information: each side alone' carries each side's D1-D5 on its own selected months, plus its own leave-one-out.
- The clauses are ANDed with: the unseen months together pass; the 70% clause (net R); the monthly floor (net R); every year positive (money); each side positive unseen; and the 13:31 regime clause. None relaxes another. A positive year can hold a failing day, and a failing day does not change its year's sign.
- LEAVE-ONE-OUT (FM-F064), on the pair's unseen record: drop the record's best day, ISO week, fortnight (the month-half and rolling readings), month and quarter, one at a time.
 - 'Best' means the largest net money at the size.
 - Ties: every tied candidate is evaluated as its own drop, and the reading fails if any of them fails. The table shows 'k tied'; FM-F064's earliest-period rule picks the row shown first.
 - After each drop, D1-D5 must still pass on what remains, against the UNCHANGED references. The learned values are what was learned; the drop only asks whether the unseen pass rests on one lucky period (PROPOSED (h)).
 - Main reading: the dropped period's observed days leave both D5 clocks. The reading that keeps them as empty days is shown beside.
 - Fails 'loo:<period>:<sub-clause>'. An emptied record FAILS 'loo_empty_window'.
 - In gate A, leave-one-out has no downside clause.
 - The same forks run on each side's D1-D5 inside the WINNER pass, and are SHOWN. 17:11 placed leave-one-out on the pair, so whether the per-side forks decide is the operator's call (PROPOSED (h)).
- TIERS (FM-F057, extended). A unit enters a tier only after passing the one before. Later clauses then show 'not evaluated (failed <clause>)'. A reported winner has every clause evaluated.
 - (i) the count clause, O(1);
 - (ii) the O(1) additive clauses on the unseen sums: the pair's checks 1, 2, 3 and 5, each side's, and each side positive unseen;
 - (iii) every year positive, O(years);
 - (iv) the month clauses (70%, floor) and the 13:31 regime clause, O(months);
 - (v) FM-F064's leave-one-out of the clauses above, O(days). This tier also fixes each reading's best period or periods;
 - (vi) DOWNSIDE BY DAY: D1, D2, D4's day and week runs, and their leave-one-out forks, from per-day data; O(traded days) per (unit, profile, scheme, reading);
 - (vii) DOWNSIDE BY TRADE: D3, D5, D4's trade run, and their forks, from the exit-ordered lane; O(trades).
- The INFORMATION READINGS of (8) run in a second pass, only for units that pass every deciding clause. So non-deciding work never multiplies the pair total.

(8) DISCLOSURES AND INFORMATION READINGS. Each is labelled. None decides unless the operator rules, and each is listed in (16).
- LENGTH BIAS. The literal clause compares the worst unseen value with the worst learned value. The worst value, the longest run and the deepest drawdown of a sample all grow with its length, and the unseen record is often longer than the window it is judged against. So a long record fails more often and a short one passes more often, for reasons that are not quality.
 - Every downside row shows U (unseen elements: traded days, weeks with a trade, runs, episodes), L (the learning evidence of its binding reference), and E (the unseen elements beyond their binding reference).
 - The LENGTH-MATCHED reading passes if and only if E < the sum, over binding months g, of U_g / L_g. With one binding month this says: the ceil(U/L)-th worst unseen value is no worse than the learned worst. With U_g <= L_g in every month it equals the literal clause. It is compared exactly at report time, from counters persisted per month. An overflow refuses the reading by name.
 - The EXCHANGEABLE-NULL PASS RATE: shuffle the unit's whole day bundles (each observed day's exit-ordered trades and adjustment, empty days included) across its span's observed days, and report the share of draws in which each clause passes. Seeds are blake3, as in R8. The draw count doubles while the measured cost stays within the run's measured deep plus P5 cost. It is computed for reported winners and for every row FIB-14 shows: 'this check passes X% of the time by record length alone'.
 - Every downside pass carries 'unseen U vs learning L'.
 - The literal clause decides, because it is the operator's rule. The length-matched reading is the operator's choice (open question).
- REFERENCE PROVENANCE. Each reference names the date, week or episode that set it, and its second-worst learning value.
 - The 'reference without its single worst learning period' reading takes each reference from its second-worst value, and is computed beside.
 - A pass that fails this reading is labelled 'passes only against <period>'.
- HALT-DAY EXPOSURE. Rule (h) removes CAL-1's halt days from every stream: 2021-02-24 (from SEBI's record), and 2020-03-13 and 2020-03-23 (measured and suspected, UNVERIFIED until an NSE circular is recorded). A live account would still have held positions opened before the halt.
 - On those days only, a labelled reading admits the decisions made before the halt's first halted second, which is information a live trader had.
 - It walks them through FIB-5's HALTS rule: no fill inside the halt, gapped stops at the first real print, and the forced exit in the normal window.
 - It feeds its own learning and unseen halves. It never reaches own-past statistics, a Generation version, gate A, selection, or any deciding half.
 - Every verdict whose span contains a halt day carries 'halt day <date> excluded by rule (h)', with that reading's D1-D5 beside.
- THE ACCOUNT (FIB-13). D1-D5 run on FIB-13's procedure union, at both sizes. The union follows FM-F133: deduplicated by (decision id, exit variant id), one position per stock across members, and the account's DayLedger.
 - Members are fixed at their selection step. References come from the same member set replayed over W(m0), under the start-month rule.
 - The exam is frozen at EXAM_FROM.
 - It is shown with FIB-13's per-day peak margin column.
 - Not modelled: a broker square-off triggered by intraday mark-to-market loss (Dhan's policy is UNVERIFIED).
- DATA. Each worst day, week and episode row shows its neutral rows by reason, its refused stock-days (FM-F094) and the mixed reading of (4). Also shown: the reading without flagged trades (FM-F079's list), the EXCLUDED unpriceable reading (FM-F078), and the unclamped worst-seen money.
- POSSIBLE OVERNIGHT CARRY. Per day, the count of slots unpriceable because no print existed in the forced window.
 - Any verdict whose record, worst day, worst week or worst episode contains one carries 'position may have been carried overnight: real loss UNVERIFIED'.
 - No auction value is modelled until docs/00-charter.md cites the rule (CLAUDE.md section 3 rule 1).
- INTRADAY. Two figures, with their drawdowns:
 - the bound 'sum of the day's MAEs, overlap not checked: not a measurement'. Per day, it is the previous close minus, for every slot of that day, its MAE money to its exit at the size plus its charges;
 - the day-close-only drawdown (D-6403's).
- R AND GROSS. Net R beside every money figure: the R curve is summed in exit order, labelled, with FM-F126's R adjustment from FIB-11. Also the count of periods whose sign in R differs (FM-F077), and gross, labelled.
- DRIFT. The pool size, the stocks traded and the trades, both in the breach period and in the period that set its reference. D5 is also shown in trades elapsed.
- DEPTH. Depth beside every D5 row. Also the under-water area (the sum, over day closes under water, of H - E), beside the largest learning episode area.
- THE SIDE IN THE PAIR. Each side's in-pair stream under reading B, as defined in (4).
- JOINED EXAM. The exam's first episode, continued from gate B's open state (the same H, peak and m0), beside the exam verdict.
- ACHIEVED CONFIDENCE (R8; report only; reported winners only).
 - D1 and D2: a day-block bootstrap of the learning days and of the unseen days, each inside its own span. Report the share of replicates in which the resampled unseen worst is no worse than the resampled learning worst. (Resampling only the unseen record would give 1 for every passer.)
 - D3 to D5: resample the unseen record's days and recompute against the unchanged references. A resampled day keeps its original month for binding.
 - Report the share, its replicate count and its Monte Carlo standard error. Replicates double while the measured cost stays within budget; the count is a resource, never an identity term.

(9) TRIALS AND IDENTITY.
- TRIALS. The clauses add no hypothesis: the gate is the old gate ANDed with more clauses, so over the same hypotheses winners(new) is a subset of winners(old). This is tested.
 - Re-judging an earlier deep run under this block adds 0 trials, and it is listed in FM-F067's ledger as a look.
 - Schemes (13:29) and regime splits (13:31) count as their own blocks say.
 - The permutation draws (R8, FM-F081) run tiers (vi) and (vii), so 'how many would pass by luck' is measured under the new gate.
 - Each PROPOSED item in (16) has a deciding choice now: the conservative one.
 - A ruling that changes a deciding choice AFTER any real gate-B result was opened counts every rule set shown as trials (2^k, for the k readings shown both ways), listed by name. A ruling made before any real result is opened adds 0.
- RULES LEDGER. FIXED_NUMBERS is not changed. Its row type (name, u64, Category, decision) has no slot for a rule without a value, and a placeholder value would be whitelisted by is_fixed_number.
 - FIB-1 adds a sibling ledger, vocab::fib_cube::WINNER_GATE_RULES. Each row holds: a versioned name; the stage, GateA or GateB; canonical descriptor bytes naming the rule's deciding choices; and a decision label.
 - It gets five GateB rows: worst_day_rule_v1; worst_week_rule_v1; max_drawdown_rule_v1; losing_streak_rule_v1 (trades, days, weeks); and recovery_time_rule_v1 (under water and climb, on the record and calendar clocks).
 - A ruling APPENDS a v2 row and never edits v1.
 - The ledger's digest hashes (name, stage, descriptor) only. Decision labels are excluded, so FM-F008's renumbering re-keys nothing.
- IDENTITY.
 - UNCHANGED: the stage-A identity, its completion seal (FM-F112), the deep identity, and FM-F007's pair identity. FIXED_NUMBERS is unchanged, no gate-A clause changes, and deep outcomes do not depend on the gate. So a stage A sealed before this block is accepted by deep, and nothing is rerun.
 - NEW: a GATE-B JUDGEMENT identity, under its own domain tag and inside params, so CLAUDE.md section 3 rule 3 keeps its nine terms. It hashes:
  - the pair identity or the side run id;
  - the deep run id;
  - the stage-A seal digest;
  - the WINNER_GATE_RULES GateB digest;
  - the deciding choice of every PROPOSED item, and the FM-F061 interim rule;
  - the walk-forward scheme-family digest;
  - the OutcomeRecord version.
 - P5, wf and exam record it. Re-judging gives a new judgement id and rewrites nothing.
 - A run judged before this block renders its own clause set under FM-F145's 'judged under an older rule set'.

(10) COST AND MEMORY. O(1) per trade and per event; nothing is rescanned; every total that is not O(1) is stated.
- LANE, per (unit, profile). It is built in P5, only for units entering tier (vi), and it is never persisted.
 - The unit's rows come from FM-F052's guided descent.
 - Each row's outcome is an O(1) seek in its persisted OutcomeRecord, which is already priced as FM-F123 lines.
 - The lane is exit-ordered once ((3)). Per-day data and the side's day adjustments come from its DayLedger.
 - Cost: O(support + passes x (n + 2^u8::BITS)) per (unit, profile).
 - Worker scratch is pre-sized once, from the largest raw count among the units the worker will walk (known before the walk from 'side passers' n). So the walk never allocates.
- ANCHORED LEARNING. A running learning half per (unit, profile): O(1) per trade, per day close, per week close and per month boundary. RefNow for a month boundary with no event is computed lazily in O(1) when an unseen element binds to it, because censored lengths grow with the boundary's index.
- ROLLING LEARNING. Per (unit, profile, scheme, step), the same streaming code is re-fed W's slice of the lane: O(trades + traded days in W), BOUNDED.
 - Under-water and climb times have no constant-size summary that combines across months, so no O(1) range structure is claimed.
 - Total per unit and scheme: O(the sum over steps of |W|), stated.
- VERSION REBUILD: O(trades before b) per (changed unit, version). BOUNDED and stated.
- UNSEEN HALF, EVENT-DRIVEN. Work happens only at exit instants, traded-day closes, week closes and month boundaries.
 - An empty gap costs O(1). The first D5 breach inside it is at index idx(p) + UW(m0), or idx(t) + CL(m0), on each clock. The index is stored, and its date is resolved at report time from the calendar table, at O(log months) per breach shown.
 - Each trade is O(1), each event is O(1), and reading a verdict is O(1).
 - A bound reference is copied into the unseen state at the element's first unseen sample: at every sample with E >= H (for D3 and D5), at each run's first non-winning element (D4), and at each week's first unseen day (D2). D1 reads RefNow.
 - The walk holds no reference array.
- SIDE REFERENCES are computed once per (side, profile, scheme) and written per month to 'downside refs v1'. A pair walk reads them by an O(1) seek per month.
- PAIR WALK. A two-way merge of the two sides' exit-ordered lanes: O(1) per trade, with no sort.
 - FM-F061 reading B re-scores the overlap cells only.
 - The pair DayLedger closes in O(touched groups) per day. That is AMORTISED O(1) per trade, since every touched group holds a trade.
 - One pass feeds four halves, per deciding FM-F061 reading and per scheme: the pair's learning half (all months), the pair's unseen half, and both sides' unseen halves on the pair's selected months.
 - A pair is walked only if it reaches tier (vi). Each (pair, profile, scheme, reading, trade) is fed once per half.
- LEAVE-ONE-OUT FORKS. One per period reading and per tied candidate: an O(1) copy of the unseen half, created at the best period's first sample (known from tier (v)). A fork skips the period's trades and days, and is then fed the same events as the original.
 - Forks are KEPT ALIVE TO THE RECORD'S END; there is no convergence discard. A fork whose peak lies before the dropped period stays deeper by the dropped profit, so states that look equal are not equal. Keeping every fork gives exact verdicts and exact margins.
 - Cost: O(1) per trade per fork. The forks per unit = the period readings plus the tied candidates; the tie count is shown and projected.
- MEMORY: one DownsideState per (unit, profile, scheme, reading).
 - It is a fixed-size Copy struct with no heap and no Default, built only from caller data.
 - It holds the learning and unseen halves, the top-two values, the first breaches and the closest calls.
 - Its byte size is printed by a test and recorded in docs/06-limits.md.
 - States per pair walk = (pair + 2 sides) x schemes x deciding readings, plus the forks. The scheme count comes from the Generation's scheme family and is never typed.
- PERSISTED, as new sibling kinds. They are append-only, and FIB-8's existing kinds and strides are unchanged.
 - 'downside v1', per (unit, profile, scheme, reading): the verdict array, first breaches, closest calls, worst values and counts.
 - 'downside refs v1', per (unit, profile, scheme, month): the eight references, their second-worst values, evidence counts and provenance, plus the unseen elements and exceedances bound to that month.
- TOTAL AND REFUSAL. P5 counts the work exactly, after tiers (i) to (v) and before any walk, in O(pairs).
 - The count covers the units entering tier (vi), and the sum over them of (n_L + n_S) x schemes x deciding readings x (1 + forks) x (1 + B draws), plus the rolling recomputes and the version rebuilds.
 - Past the measured budget, P5 refuses by name with those numbers. It never truncates.
 - FIB-12 projects the same formula from its sample.
 - R8's draw doubling, already bounded by the stage-A cost, is also bounded by the measured deep plus P5 cost.
 - This is not O(1) per run. docs/06-limits.md states it beside FM-F057's total.

(11) BENCH: crates/runner/benches/fib_rows/gate.rs, through FIB-1's harness.
- Every row has a class, a structural work counter and an axis.
- Every row runs at three scales spanning >= 100x, the largest at FIB-12's production size; otherwise it prints EXTRAPOLATION.
- Every row reports p50, p99 and max. A WORST-CASE row passes only if its p99 and max ratios are <= 1.25. AMORTISED rows follow FM-F001.
- The rows:
 - downside_feed_per_trade: WORST-CASE; axis: trades in the stream.
 - downside_event: WORST-CASE; axis: traded days at a fixed calendar span (empty days cost nothing).
 - downside_week_close and downside_month_bind: WORST-CASE; axis: weeks, and months.
 - rolling_reference_recompute: BOUNDED, O(|W|); at three window lengths.
 - version_rebuild: BOUNDED; axis: trades before the version.
 - exit_order_per_trade: WORST-CASE per trade, from n = 2^u8::BITS upward, up to the whole pool exiting in one minute; the per-unit term O(passes x 2^u8::BITS) is printed.
 - pair_merge_walk_per_trade: WORST-CASE; axis: pair stream size.
 - pair_day_close: AMORTISED; axis: trades in one day, up to the whole pool.
 - loo_fork_per_trade: WORST-CASE; axis: record size, at the maximum live forks.
 - downside_verdict_read and prefix_verdict_read: WORST-CASE; axis: months in the record.
 - downside_no_allocation: zero allocations per trade, per event and per verdict.

(12) REPORTS: per mix and per pair, at each size and scheme, with long, short and pair side by side (16:04 item 4), leading with the day (13:26).
- A DOWNSIDE TABLE has one row per sub-clause: worst day, worst week, max drawdown, losing-trade run, losing-day run, losing-week run, and under-water time and climb time on each clock. Its columns:
 - LEARNED: the reference on the final learning window;
 - BINDING: the reference the closest-call element was judged against, with its m0 and provenance;
 - UNSEEN: the worst unseen value, and the day, week or episode it came from;
 - MARGIN: the closest call, an integer;
 - VERDICT: PASS, FAIL by name, N/A, 'not evaluated', or a labelled pass;
 - U, L and E, with the length-matched reading;
 - the null pass rate;
 - net R beside, and the other size beside.
- Beside the table:
 - every reading of (8), labelled;
 - the counts: losing, flat and empty periods in learning and unseen; clipped weeks; hole days; neutral rows; refused stock-days; censored learning episodes; open unseen episodes; tie groups;
 - the leave-one-out table (reading by sub-clause, with 'k tied').
- The per-step references and binding months are a paged per-month vector (FM-F141).
- Surfaces:
 - 'fib report' (FIB-14), after gate B;
 - /fib/mix.json and the pair rows (FM-F057), with every field name carrying its basis, size and scheme (FM-F147);
 - the /fib page: plain tables, phone width, light and dark;
 - FIB-13's gate-B detail and its exam report (learned vs exam).
- The 13:26 lead figures (worst day, worst week, longest run of losing days, the day-by-day line) come from the D1-D5 state: money, in exit order. FIB-6's scorecard streaks, drawdown, worst periods and recovery are labelled 'scorecard (R, day close, entry order) - information'.
- CLOSEST MISSES: per (direction, size, scheme, failed sub-clause), the margin histogram (NOTHING TYPED item 7), in exact money units, periods or days. It is shown only under 'diagnostics: failed the winner gate - not a result'.

(13) FILES, OWNERS AND LEDGER.
- FIB-7's owned_files in plan.json gain:
 - crates/runner/src/winner_gate/downside.rs (pure; gates 17 and 22);
 - crates/runner/src/fib/exit_order.rs (pure);
 - crates/runner/tests/winner_gate_downside.rs;
 - crates/runner/src/fib/pair.rs and crates/runner/tests/fib_pair.rs. FM-F057 names both, but no package owned them until now.
- Existing FIB-7 files that change: winner_gate.rs (one line, 'pub mod downside;'), fib/wf.rs, fib/luck.rs, tests/fib_wf.rs and benches/fib_rows/gate.rs.
- Outside FIB-7, one line each, in the owner's package:
 - FIB-1: WINNER_GATE_RULES with its five v1 rows; 'pub mod pair;' and 'pub mod exit_order;' in fib/mod.rs; days_per_fortnight fixed to FM-F064's 15 days; and, before any real stage A is sealed, a FIXED_NUMBERS digest that excludes decision labels.
 - FIB-5: for each exit order (FM-F125), its last fill second, filled qty and exit value. This is OutcomeRecord v2; v1 stays decodable.
 - FIB-6: the scorecard relabel.
 - FIB-8: the kinds 'downside v1' and 'downside refs v1' (an appended section in docs/02-store-format.md); the GateRefusal census v2; and the halt-day exposure input (decisions before the first halted second only, never committed to own past).
 - FIB-9: P5 runs tiers (v) to (vii) after deep, for every scheme; it does the lane gather, the exact count and the refusal before any walk; it runs the halt-day reading through deep; and the permutation draws run through the new tiers.
 - FIB-10: the calendar table. OUTSIDE OWNERSHIP: one iso_week_key function beside pull::calendar::weekday_of.
 - FIB-11: per-order charge lines, and each ledger's day adjustment in FM-F126's sign. 'fib net' calls runner::winner_gate::downside on the Dhan re-scored lanes, labelled, deciding nothing.
 - FIB-12: the projection and the NEEDS THE MAC measurements.
 - FIB-13: gate-B detail per scheme; the exam frozen at EXAM_FROM; the step-m prefix verdict in the 13:26 ordering; the account reading; and the joined exam reading.
 - FIB-14: the table, the readings, the routes and the page.
- LEDGER: FIB-7's block offsets +8 to +12 (D-6428 to D-6432 in today's numbering).
 - FM-F008's single recorded renumbering runs before any FIB commit. It must also cover feat/db-fix's D-6301 and D-6320 to D-6322 (as of 84329a96b), origin/cloud/mut-api-a-20261017's D-6420 to D-6423, and the PREC-1..PREC-9 ranges.
 - every_decision_id_is_issued_once runs over feat/db-fix plus every unmerged branch.
 - Offsets +13 to +19 stay free for the 13:31 regime clause and the operator's rulings. A continuation block is reserved if they overflow.
 - Decision ids are labels, never identity bytes.
 - +8: the five clauses, quoting the 18 Oct decision verbatim; 'no new number'; the declined fixed share of positive weeks; and the literal worst-vs-worst rule with its length bias disclosed (UNVERIFIED which reading the operator keeps).
 - +9: units and sizes. Money in CHG-1's exact unit decides, and R is shown beside. Both sizes run, and neither is scaled from the other. max(a, 0) and its sign (UNVERIFIED, as FM-F077 and FM-F126).
 - +10: periods; status (FLAT is non-winning); the trade order (D-6403's entry order becomes information) and tie groups; exit orders; the realised curve with its bound; unpriced slots clamped.
 - +11: references. Per direction on standalone sides, and on the merged pair. From the learning window only, per scheme, and bound to the version at m. Open elements enter only their snapshot. Zero, missing and censored references. The start-month rule. The exam.
 - +12: placement. Gate B and the exam only, under every scheme. Sides and pair under WINNER, with the FM-F061 interim rule. AND with the other clauses. Leave-one-out and its forks. The tiers. The prefix verdict. The trial rule for later rulings. The judgement identity.
- docs/04-invariants.md gains rows, each beside its test:
 - references read only W(scheme, m), under the version in force at m;
 - every unseen element is judged against its start month;
 - open elements never enter the running maxima;
 - streaming equals brute force;
 - the event-driven walk equals the every-day walk;
 - forks equal recompute;
 - a zero reference fails any unseen loss;
 - a side's references do not depend on its partner;
 - the stage-A identity is unchanged by this block;
 - no allocation per trade, event or verdict.
- docs/06-limits.md gains rows:
 - the realised-only curve, and the possible overnight carry;
 - the per-unit cost, O(support + trades + traded days + months) per (profile, scheme, reading);
 - rolling references, O(|W|) per step;
 - version rebuilds;
 - the pair total and its multipliers (schemes, readings, forks, draws);
 - the fork lifetime;
 - the radix cost, O(passes x (n + 2^u8::BITS));
 - DownsideState bytes, and states per pair walk;
 - the amortised pair day close;
 - the bootstrap and null-draw costs;
 - persisted bytes per unit;
 - the literal clause's length bias;
 - that the start-month reference can be larger or smaller than a re-read one;
 - that D5 counts an overnight-only dip as one day.

(14) TESTS THAT MUST FAIL BEFORE. They are in crates/runner/tests/winner_gate_downside.rs unless another file is named.
- crash_day_fails_worst_day_while_month_year_and_floor_pass: an equal loss passes; one exact unit more fails, and so does one paisa more.
- worst_week_boundary_clipped_and_joined: ISO weeks across a month end, a year end (2020-W53, 2026-W01) and a holiday Monday; a clipped edge week; a week joined across two selected months.
- drawdown_boundary_and_sizes: a synthetic flat per-order charge passes at Rs 5,00,000 and fails at Rs 1,00,000, and neither size is scaled.
- size_dependent_spill_changes_exit_order, and trade_absent_at_small_size_is_not_in_its_stream.
- losing_trade_run_uses_exit_order: a loss from 10:00 to 14:00 and a win from 11:00 to 11:30. Exit order gives W, L; entry order gives L, W. Also a golden total order.
- tie_group_runs_evaluated_in_both_orders.
- flat_and_empty_status: a zero-net trade is non-winning. A FLAT day starts and extends a run and sets its m0. An EMPTY day is skipped. A hole is not a day. With learned-0 references, a flat unseen day passes D1, D2 and D3 and fails D4's day run, labelled 'learned 0'.
- losing_day_and_week_runs_boundary.
- recovery_underwater_and_climb_each_fail_by_name_on_each_clock.
- calendar_clock_counts_deselected_months.
- open_unseen_episode_fails_on_first_day_over_or_passes_labelled.
- episode_rebinds_at_every_return_to_high.
- censored_learning_episode_enters_at_observed_length: a run of k passes and k + 1 fails.
- clipped_learning_week_enters_only_its_snapshot: a learning week at -30,000 before the boundary and +25,000 after it. Ref(m) takes 30,000; Ref(m+1) takes 5,000.
- zero_reference_fails_any_unseen_loss, including a small-window case.
- no_traded_learning_day_fails_by_name.
- reference_reads_window_before_m0_only: a seeded property test. Perturbing any month >= m0, or any month outside W, leaves every binding reference byte-identical.
- reference_bound_to_version_at_m.
- start_month_rule_blocks_the_ratchet.
- same_instant_exits_make_one_sample.
- exit_orders_are_separate_samples: their sum equals the slot's net, and a v1 record is labelled.
- day_rounding_adjustment_sign: a positive a deepens a drawdown and turns a flat day into a losing one. A negative a changes no deciding figure. On an overlapping stock-day, the pair's ledger differs from the sum of its sides.
- exact_unit_boundaries: a +0.4-paisa trade wins.
- realised_curve_with_intraday_bound_shown.
- unpriced_slot_clamped_at_15_09_59.
- circuit_locked_slot_flags_possible_overnight_carry.
- no_deep_outcome_rows_are_neutral_and_mixed_reading_shown.
- downside_streaming_equals_brute_force: seeded, with exit orders, ties, holes, non-selected months, adjustments, versions and rolling windows.
- event_driven_equals_every_day_walk.
- rolling_reference_equals_brute_force_window, and rolling_reference_reads_only_its_window.
- step_m_prefix_verdict_unchanged_when_months_ge_m_change.
- length_matched_reading_equals_literal_when_unseen_not_longer.
- exchangeable_null_pass_rate_reported.
- reference_without_single_worst_period_shown.
- confidence_d1_d2_resamples_both_spans.
- exit_order_equals_a_comparison_sort: with constant bytes skipped, and with already-sorted input.
- winners_with_downside_subset_of_without.
- downside_no_allocation_per_trade_event_or_verdict.
- downside_source_scan: it tokenises the source, excluding comments and strings. It refuses any numeric literal except 0 and 1 in downside.rs and exit_order.rs, and it refuses the '/' operator and any f64 inside the named deciding functions.
- In fib_wf.rs:
 - downside_is_na_in_gate_a;
 - side_references_are_per_direction;
 - unseen_record_joins_selected_months;
 - exam_reference_frozen_at_exam_from;
 - every_scheme_must_pass_and_failures_name_schemes;
 - stage_a_identity_unchanged_by_downside_rules;
 - deep_accepts_stage_a_sealed_before_block;
 - rejudge_adds_zero_trials_and_is_listed_as_a_look;
 - ruling_after_opening_counts_rule_sets.
- In fib_pair.rs:
 - winner_needs_each_side_and_pair_downside;
 - pair_reference_is_the_merged_stream;
 - long_side_reference_identical_across_two_short_partners_under_reading_b;
 - both_fm_f061_readings_decide_until_ruled;
 - loo_forks_equal_recompute: every reading; ties; a best day that rescues a drawdown; a peak before the dropped best day with the drawdown continuing; joined runs; a fortnight across a year end; a clipped week; both D5 clocks; and the per-side forks;
 - loo_empty_window;
 - tiers_short_circuit_and_winners_evaluate_every_clause: a pair that fails FM-F064's leave-one-out never enters tier (vi), checked with the work counter;
 - permutation_draws_apply_downside;
 - p5_refuses_before_walking_past_budget.
- Owners' tests:
 - FIB-1: winner_gate_rules_have_no_value_and_are_not_whitelisted, and rules_digest_excludes_decision_labels.
 - FIB-5: exit_orders_round_trip_in_outcome_record_v2.
 - FIB-8: downside_records_fixed_stride, v1_kinds_decode_after_block, and halt_day_input_admits_only_pre_halt_decisions.
 - FIB-10: period_keys_golden.
 - FIB-11: precise_and_dhan_rescores_share_one_downside_implementation.
 - FIB-13: procedure_union_downside_reading, exam_joined_reading_continues_open_episode, and prefix_verdict_orders_13_26_selection.
 - FIB-14: downside_table_per_side_and_pair_at_both_sizes_and_schemes, json_downside_fields_carry_basis_size_and_scheme, older_run_renders_older_rule_set_banner, and closest_misses_only_under_diagnostics.

(15) EXTREME CASES (failure matrix). Each row: CASE, then what is DONE, then the TEST.
- FM-F201: a crash day stops every open position at once, while the months, the years and the floor all pass. DO: D1 on that day's net money at each size. TEST: crash_day_fails_worst_day_while_month_year_and_floor_pass.
- FM-F202: no losing learning element of some kind. DO: a zero reference, labelled 'learned 0'. TEST: zero_reference_fails_any_unseen_loss.
- FM-F203: no traded learning day in W. DO: FAIL by name, never N/A. TEST: no_traded_learning_day_fails_by_name.
- FM-F204: a learning drawdown is still open at m. DO: it enters at its censored length, in the snapshot only. TEST: censored_learning_episode_enters_at_observed_length.
- FM-F205: an unseen drawdown is still open at the record's end. DO: it is judged every day, and otherwise gets a labelled pass. TEST: open_unseen_episode_fails_on_first_day_over_or_passes_labelled.
- FM-F206: a drawdown spanning several steps would raise its own reference. DO: the start-month rule, with the re-read reference shown and no strictness claimed. TEST: start_month_rule_blocks_the_ratchet.
- FM-F207: a week is cut by an edge, a non-selected month, an ISO year end or a holiday. DO: clip it, label it, and join it across selected months. TEST: worst_week_boundary_clipped_and_joined.
- FM-F208: holes fall inside an episode, run or week. DO: a hole is not a day; label it; show the calendar reading. TEST: flat_and_empty_status, and a recovery with and without a hole.
- FM-F209: overlapping positions hide an intraday trough. DO: the realised curve decides, and the MAE-sum bound is shown labelled 'overlap not checked'. TEST: realised_curve_with_intraday_bound_shown.
- FM-F210: a stop cascade closes many trades in one second. DO: one sample per instant, and both orders for a tie group. TEST: same_instant_exits_make_one_sample, and tie_group_runs_evaluated_in_both_orders.
- FM-F211: the rounding adjustment changes a day, or differs between the pair and its sides. DO: subtract max(a, 0) at the day-close sample, using each unit's own ledger. TEST: day_rounding_adjustment_sign.
- FM-F212: flat charges flip signs between the two sizes. DO: one stream per profile, never scaled. TEST: drawdown_boundary_and_sizes.
- FM-F213: something nets exactly zero. DO: a zero trade is non-winning; a zero day or week is FLAT. TEST: flat_and_empty_status.
- FM-F214: the pass rests on one lucky period. DO: leave-one-out forks kept alive to the end. TEST: loo_forks_equal_recompute.
- FM-F215: non-selected months lie between selected ones. DO: the record joins them, and both D5 clocks decide. TEST: unseen_record_joins_selected_months, and calendar_clock_counts_deselected_months.
- FM-F216: a slot is entered but unpriced. DO: it books min(0, worst-seen) at 15:09:59 and is never a win. TEST: unpriced_slot_clamped_at_15_09_59.
- FM-F217: a strong side hides the other side's blood bath. DO: each side runs its own D1-D5 on its standalone stream. TEST: winner_needs_each_side_and_pair_downside.
- FM-F218: references are copied across directions or built from the sides. DO: standalone sides, and a merged pair. TEST: side_references_are_per_direction, and pair_reference_is_the_merged_stream.
- FM-F219: a float or a division flips a boundary. DO: i128 comparisons in CHG-1's exact unit. TEST: exact_unit_boundaries, and downside_source_scan.
- FM-F220: the whole pool exits in one minute. DO: the radix with derived passes. TEST: exit_order_equals_a_comparison_sort, and the bench row.
- FM-F221: leave-one-out empties the record. DO: 'loo_empty_window'; forks live to the end, as stated. TEST: loo_empty_window.
- FM-F222: the clauses leak into gate A. DO: N/A in gate A. TEST: downside_is_na_in_gate_a.
- FM-F223: the exam re-learns. DO: the reference is frozen at EXAM_FROM. TEST: exam_reference_frozen_at_exam_from.
- FM-F224: a tiny learning window. DO: the literal reference, with its evidence shown. TEST: zero_reference_fails_any_unseen_loss, small-window case.
- FM-F225: the record is much longer than its learning window. DO: the literal clause decides; U, L, E, the length-matched reading and the null pass rate are shown. TEST: length_matched_reading_equals_literal_when_unseen_not_longer, and exchangeable_null_pass_rate_reported.
- FM-F226: a halt day holds positions opened before the halt. DO: rule (h) stands, and the halt-day exposure reading is shown. TEST: halt_day_input_admits_only_pre_halt_decisions.
- FM-F227: one extreme learning period loosens every later reference. DO: provenance, and the 'without its single worst' reading. TEST: reference_without_single_worst_period_shown.
- FM-F228: a later version reshapes a unit's learning trades. DO: the reference is bound to the version at m, with a rebuild. TEST: reference_bound_to_version_at_m.
- FM-F229: many winners lose together on one day. DO: FIB-13's account reading. TEST: procedure_union_downside_reading.
- FM-F230: deep refusals cluster on stress days. DO: the rows are neutral and counted, with the mixed reading beside. TEST: no_deep_outcome_rows_are_neutral_and_mixed_reading_shown.
- FM-F231: a clipped learning week later recovers. DO: it enters the snapshot only. TEST: clipped_learning_week_enters_only_its_snapshot.
- FM-F232: a circuit-locked position is carried overnight. DO: flagged, never modelled. TEST: circuit_locked_slot_flags_possible_overnight_carry.
- FM-F233: a side's pass inside a pair rests on its own lucky period. DO: per-side forks are shown. TEST: loo_forks_equal_recompute.
- FM-F234: a reading is chosen after the results are seen. DO: the trial rule of (9). TEST: ruling_after_opening_counts_rule_sets.
- FM-F235: the block would re-key stage A. DO: identity unchanged, plus a judgement identity. TEST: stage_a_identity_unchanged_by_downside_rules.
- FM-F236: rolling schemes need max-type references over a window. DO: a per-step recompute. TEST: rolling_reference_equals_brute_force_window.
- FM-F237: the daily-focus ordering needs a verdict at step m. DO: the prefix verdict. TEST: step_m_prefix_verdict_unchanged_when_months_ge_m_change.
- FM-F238: sparse pairs would pay for every calendar day. DO: event-driven processing. TEST: event_driven_equals_every_day_walk.
- FM-F239: a slot exits in several orders. DO: one sample per order. TEST: exit_orders_are_separate_samples.
- FM-F240: the store changes stride. DO: new sibling kinds. TEST: v1_kinds_decode_after_block.
- FM-F241: the work becomes known only in P5. DO: an exact count and a refusal before any walk. TEST: p5_refuses_before_walking_past_budget.

(16) PROPOSED / UNVERIFIED. Each has its deciding choice now, and the operator may rule on each. (9) gives the trial rule for a ruling made after results are opened.
- (a) Money in CHG-1's exact unit decides; R is shown beside.
- (b) Both D5 readings decide: under-water time and climb time.
- (c) A zero-net trade, day or week is non-winning: it starts or extends a run and sets m0, and adds no loss.
- (d) The record joins selected months, and both D5 clocks decide: record and calendar.
- (e) The start-month rule.
- (f) Clipped edge weeks are full participants.
- (g) The realised curve decides; the intraday bound is information.
- (h) Leave-one-out keeps the references unchanged; the per-side forks are information.
- (i) Both FM-F061 readings decide until one is named.
- (j) Rows with no deep outcome are neutral; the mixed reading is information.
- (k) The deciding day adjustment is max(a, 0).
- (l) An unpriced slot books min(0, worst-seen).
- (m) The literal worst-vs-worst comparison decides; the length-matched reading is information.
- (n) Halt days stay excluded by rule (h); the halt-day reading is information.
- (o) A reference keeps its single worst learning period; the 'without it' reading is information.
- (p) The account (procedure union) reading is information.
- (q) The step-m prefix verdict admits a unit with no unseen record before m, labelled.
- (r) The exam starts a fresh curve; the joined reading is information.
- (s) Sides are judged on their standalone streams; the blocked in-pair stream is information.


OPERATOR 18 Oct 2026 ~13:31 IST - EVERY MARKET SITUATION (FIB-7 gate + FIB-14 reports; operator: "we need to make money irrespective of any situation"). The Winner Gate numbers are unchanged; this adds WHERE "positive" must hold.

(1) Market regimes, generated (no typed threshold): each calendar month is labelled by NIFTY's own month move (close of the month's last tradable day vs the previous month's) into three groups by the median-split R2 rule seeded on LEARNING months only: falling, flat, rising (the cut points are per walk-forward step and per scheme, from [learning window] only). NIFTY is context only (CLAUDE.md section 1: indices are context in step 1); no index enters the condition vocabulary through this.

(2) Gate clause (pair; net of the entire precise charges; per walk-forward scheme of the 18 Oct family): the unseen months of EACH regime together are net positive, for the pair, and the per-side quality checks are reported per regime. A regime with no unseen month in a scheme is reported 'no unseen month in this regime' and counts as not proven, never as a pass. A setup that only makes money in rising markets is refused by name ("fails in falling markets").

(3) Information only, never a gate or selection input: the same tables by INDIAVIX month level (terciles of its learning months), because CLAUDE.md section 1 keeps INDIAVIX reference-only (stamped onto trades, never in conditions, ranking or run identity); and by named periods (2020-03 crash month, 2022 drawdown months, budget days, monthly-expiry days, result seasons) shown as rows when the calendar of record names them.

(4) Cost: one regime label per month per scheme; per mix one counter per (regime, month) with prefix sums, O(1) per trade. Every regime split is a trial for the luck check.


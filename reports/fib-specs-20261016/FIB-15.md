# FIB-15: Live proof: a daily forward paper-trading record of each frozen long + short setup on newly delivered data, with the same code and full charges, judged by the Winner Gate and the downside checks, with an automatic brake

Depends on: FIB-13, FIB-14, FIB-7, FIB-9, FIB-10, FIB-11, FIB-12; outside parts: CHG-1, CHG-2, CAL-1, PIT-1, DATA-1, DATA-2, OF-1, R4-06, R4-07, R4-11, R4-12, R4-26

Specification as of 18 Oct 2026 (amended 18 Oct: five downside checks, every market situation, walk-forward family, daily profit focus, mover persistence, FIB-15 live proof and its requirements on other parts; every FIB decision id moved +1000 to D-6300..D-6579 because the old ids collided with feat/db-fix and the PREC blocks). Operator blocks at the end supersede earlier text.

# FIB-15: Live proof: a daily forward paper-trading record of each frozen long + short setup on newly delivered data, with the same code and full charges, judged by the Winner Gate and the downside checks, with an automatic brake

Depends on: FIB-13, FIB-14 (and through them FIB-1..FIB-12); new-work CHG-1, CHG-2, CAL-1, PIT-1, DATA-1, DATA-2, OF-1; db parts R4-07, R4-11, R4-12, R4-26, R4-06 (R4-16, R4-28, R3-21, R3-31 should merge first); the ingest-chain per-day authority stage (owner needed); and the requirements on other owners in section (12). See DEPS.

Specification as of 18 Oct 2026, final after two critic passes. It was written against the FIB specs of 16 Oct 18:50 IST and the operator blocks of 18 Oct: 13:07 downside checks, 13:09 this part, 13:25 mover persistence, 13:26 daily focus, 13:29 walk-forward family, 13:31 every market situation. FIB-07's 13:07 downside amendment (workflow wf_8ee6b7f3-229) was not yet in plan.json at writing time. FIB-07 owns every gate and downside definition; where this text and FIB-07 differ on how a reference or clause is computed, FIB-07 wins and this part follows it. What this part needs from FIB-07 is RQ-7, and a plan check refuses this part's build until FIB-07.md and plan.json carry it.

```text
DECISION BLOCK D-6200..D-6219. Reserved 18 Oct in state/fix/governor.json (reserved_decision_blocks). The highest decision id seen elsewhere is D-6017, on cloud/mut-store-*. The next block, D-6220..D-6229, is held by SURV-1. Re-grep docs/05-decisions.md and the origin branches before committing; FM-F008's every_decision_id_is_issued_once stays green. Use no other number.

OPERATOR DECISION, 18 Oct 2026 ~13:12 IST, verbatim:
  "Yes, add FIB-15 Live proof: a daily paper-trading record on live data with the same code and full charges, judged by the Winner Gate and downside checks, with an automatic brake outside the learned worst day, week or drawdown."
His aim, in his words: "our idea is to find the maximised winners and always consistent long term reliable provable real time proof of the live trading ... in real time live trading we should make the consistent long term profit".
13:26 IST, also his words: "I just need to make money profits on a day to day basis".
The framing he accepted:
- A backtest proves only the past. Proof is what the frozen setup does from now on.
- brutex proves, records and brakes.
- Placing real trades stays the operator's decision and is NOT part of FIB-15.

COMMON RULES (binding)
- Read CLAUDE.md in your worktree first; it is law. The wt/DBFIX copy: the swept shares are FNO_UNDERLYINGS less FNO_INDEX_UNDERLYINGS; cli is not loop-free and emits at structural boundaries.
- Build in your own git worktree cut from feat/db-fix. wt/DBFIX is read-only for you. After a pause, reuse your worktree's commits and edits.
- Rust only outside web/.
- Never use the Py-language: no such files, no interpreter commands, never its name in any file.
- Run export PATH=/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/bin:$PATH before any cargo command.
- Never write the real store /Volumes/WD_BLACK/brutex/fresh-20260919/runtime/store or the tick store.
- Read-only real-data proofs use at most 4 workers and write only into a NEW scratch directory of yours.
- Never start, stop or restart :8080. A shadow server uses your own port.
- Never rm -rf, find -delete, git clean or git worktree remove.
- Fixtures are synthetic, with prices that cannot be mistaken for real ones. Never copy a vendor row.
- Use only decision numbers from this block.
- Put every new invariant in docs/04-invariants.md beside its test, and every measured number in docs/06-limits.md. Rows are append-only; on a merge conflict keep both sides.
- A step counts only after the FIB-1 harness reports p50/p99/max with its class and axis (FM-F001, FM-F002).
- Gate 22 and gate 17: the runner files of this part are pure. No std::fs, File, OpenOptions, read_dir, mmap, process, include_bytes!, telemetry or print; counters only. cli passes runner every calendar-derived input (tradable days per month, period and ISO-week ids from FIB-7's period functions), as FM-F063 does; runner never names pull.
- No depth, k or max-level parameter anywhere.
- No Default on any frozen-setup, reference, brake, verdict, reading or authority type.
- NO TYPED NUMBERS ON THE STRATEGY PATH (FIB-1's scope): crates/runner/src/fib/live.rs, crates/runner/src/fib/brake.rs and the judge logic in crates/cli/src/fib/live.rs contain no numeric literal except 0, 1 and the named constants of vocab::fib_cube::FIXED_NUMBERS. Ship the source-scan test. Outside that path: HTTP statuses use axum's StatusCode names; file-layout offsets and strides are named consts declared once in live_files.rs; period and ISO-week ids come from FIB-7's period functions; bench constants come from FIB-1's harness. Every other value is generated, learned at the freeze, or read from the sourced calendar.
- 13:25 scan: live.rs, brake.rs and cli live.rs are added to FIB-14's source scan that refuses any read of the closing-rank table from decision, gate or brake code.
- Money is paisa i64 with i128 intermediates. Statistics are f64 at full precision and never rounded for storage (CLAUDE.md section 7).
- Say 'order flow delta' and 'order flow cumulative delta', never the bare word.
- Equities stay labelled research (CLAUDE.md section 1: never Selection V6 or execution authority). Until CHG-2 or the operator's section 1 edit merges (RQ-9), every equity figure shows the gross column beside net with the section 1 sentence quoted. A forward record changes nothing about that law.
- docs/10-shared-core.md: FIB-15 lives entirely in brutex (runner, cli, api, web). It designs nothing in tickvault or any other repository, adds no live-trading consumer, and changes no API of the shareable crates (core, vocab, indicators, engine, greeks, costs).
- DONE = the list at the end. Commit WIP every 15 minutes and squash before your report. Anything not finished is listed as NOT DONE, never claimed.

OPERATOR RULES IN FORCE
Each rule applies exactly as FIB-13 and FIB-14 carry it; the newest wins.
- (a) Feeds by level. Minutes always come from Zerodha. GDFL is used only at the seconds level: fills at the worst price of each second, and order flow. Data never comes from Dhan or Groww. Dhan is only the broker whose charges apply (6 Oct, 13 Oct).
- (h) Session rules. The forced exit is priced only from GDFL seconds in [15:09:00, 15:10:00). A tradable day is a full CAL-1 session. The pool is PIT-1. One timeframe per trade, anchored at 09:15.
- (i) Every decision and reported number is NET of the entire precise charges. Gross is a labelled column that decides nothing.
- (j) Nothing typed, plus the 17:15 rules R1-R10. Only the Winner Gate numbers are fixed.
- 16:04 items (1)-(4): long and short are separate setups; only geometry is mirrored.
- 17:11 Winner Gate: QUALITY PER SIDE, QUANTITY PER PAIR. Each side: win rate >= 51%, average win >= 2x average loss, profit factor >= 2, largest win >= 2x largest loss, positive on unseen data. The pair: >= 500 trades a year across both sides, pro-rated by tradable days; every year positive; walk-forward (>= 70% of unseen months positive, none below the learned loss floor); leave-one-out; the pair's own four quality checks.
- 18 Oct 13:07: five downside checks, per direction and for the pair, at Rs 1L and Rs 5L, net of all charges, unseen compared with learned: worst day; worst week; max drawdown; losing streaks of trades, days and weeks; recovery time. No fixed share of positive weeks.
- 18 Oct 13:25: mover persistence is information only, never a gate, brake or selection input.
- 18 Oct 13:26: reports lead with the day.
- 18 Oct 13:29: walk-forward family. Every learned number comes from each scheme's own learning window; the walk-forward clauses and the downside checks must hold under EVERY scheme.
- 18 Oct 13:31: every market situation. For the pair, the unseen months of EACH NIFTY regime (falling, flat, rising; cut points by the R2 median-split rule on learning months only, per scheme) together are net positive; a regime with no unseen month is 'not proven', never a pass; refused by name ('fails in falling markets'). INDIA VIX regimes and named periods are information only.

GOAL
1. Freeze each exam-winning pair into an immutable setup, once.
2. Each session day after the freeze, once that session's inputs have landed and passed their checks, run the SAME decision, fill, charge and gate code over only that session, with the build that was frozen.
3. Append the paper trades and net results at both sizes to an append-only, hash-chained record.
4. Judge the record continuously by the Winner Gate and the five downside checks against the frozen learned references, under every walk-forward scheme and every market regime.
5. Stop the setup automatically, with the exact reason, the moment a forward day, week or drawdown falls outside the frozen worst day, week or drawdown, and show beside it how often an unchanged setup would trip by chance.
6. Show it all on /fib/live and its JSON routes, with an event for every judged day, every pending or skipped day and every brake.
Nothing is ever placed, sent or prepared.

(1) WHAT FIB-15 IS, AND WHAT 'LIVE' MEANS (D-6200)

1.1 It is computed after the close. A day is judged once its inputs have landed and passed their checks. This is honest, not a shortcut:
- No real order is placed, so nothing the record does can move a price.
- The decision fold reads bars 0..N at bar N by its SHAPE (CLAUDE.md section 3 rule 7): pull::fold hands candles one at a time to the FIB-3/FIB-4 detectors, which keep running state and never index ahead. So a decision computed after the close over the session's minute file equals the decision a process running at bar N's close would have taken, PROVIDED the bars it reads equal the bars a live process would have seen at N.
That proviso is not proven:
- Zerodha historical minutes pulled after the close may differ from what a stream would have delivered at N (late prints, vendor corrections). UNVERIFIED, unmeasured.
- Zerodha back-adjusts history as of the pull day (charter, D-3323). A session pulled on its own evening is on its own basis; every landed day records its pull date, and a day pulled on a later evening is labelled (4.9).
- Stock-day ELIGIBILITY is decided from whole-day data after the close (4.7); that is stated, labelled and checked, not hidden.
Fills come from GDFL seconds after the decision candle closes, at the worst price of each second, exactly as in the backtest (FIB-5). Like the backtest, the record assumes the paper order fills within each second's traded volume without moving the price, and that a side present in the L1 row is reachable (RQ-6). Both assumptions are labelled.

1.2 'Live' in v1 means FORWARD and UNSEEN, not real-time:
- every judged session opened after the setup was frozen AND after the newest session stored in any decision or fill feed at the freeze (3.1);
- its data reached the store after the freeze;
- nothing about it could have informed the choice.
FM-F131 (FIB-12) offers exactly this as the clean test, because the sealed exam months were already looked at.

1.3 Real-time intraday signals need a sourced real-time feed, and docs/00-charter.md records none. Its Kite Connect rows are: the historical candles page (kite.trade/docs/connect/v3/historical/); the market-quotes page, read for the instruments specification and dump; the exceptions page, read for the order cap, which is recorded as not applicable; the orders page, read only for the list of exchange codes; forum threads 7756, 14149, 13953 and 14874 (and 14817 and 13841 on global indices); and the vendor's own SDK repository, read for exchange constants. None of them sources a streaming or WebSocket feed. The GDFL rows describe delivered tick files only; no real-time product is recorded, and even the daily delivery time is UNVERIFIED (R4-12's gdfl_time). Dhan and Groww are excluded as data sources by the operator.
Real-time intraday is therefore UNVERIFIED and OUT of v1. A later part may add it only after all three of: (i) a charter row sources a real-time feed from Zerodha or GDFL (URL, date read, what it delivers, its timestamps); (ii) a measurement shows its bars equal the historical bars on the same minutes, or names how they differ; (iii) the operator rules on it. Its decisions would sit BESIDE the after-close record, never instead of it.

(2) THE FROZEN SETUP (D-6201)
All of the following are canonical bytes in the freeze record. No field has a default.

a. THE PAIR
- pair_identity (FM-F007): long run id, short run id, pairing-rule version, FIXED_NUMBERS digest.
- The stage-A completion-seal digests and deep run ids (FM-F112).
- The exam record id and its verdict columns under every scheme of the walk-forward family (13:29) and the regime clause (13:31).
- The deciding FM-F061 reading (B or A, as FIB-7 recorded it); the other is computed beside.

b. EACH SIDE, separately; nothing is shared but geometry
- Its MixKey: 24 coordinate bytes, OPEN where free.
- Its partition values: the exit variant id with the ExitSet version; the entry-style variant; the timeframe width in whole minutes.
- Its own-exit profile and its side values.

c. THE GENERATION VERSION CHAIN per direction (R1)
- The exact version recorded before the exam opened (FM-F135), with its cut lists, rank edges, time-of-day edges, ExitSet, entry-style variants and histogram split depths (R3).
- The run-time schema descriptor (FM-F006), with FIB_CUBE_VERSION and its schema-ledger digest.
- On a frozen setup there is never an R2 refinement, a re-selection, a re-pairing or a new version.

d. THE GATE
- The FIXED_NUMBERS digest and the clause set that selected and examined the setup (FM-F145).
- A later rule set is shown beside, under 'judged under a newer rule set', never instead.

e. AUTHORITIES AND REQUIRED RULE VERSIONS, by rule, not by table extent
- The CAL-1 tradable_day rule commit; the PIT-1 file version and traded-surface rule (D-6481 status); the DATA-1 factor rule version, DATA-2 rule T, the OF-1 file version, and the authority words the run used.
- The session-structure rule version (pull::vendor's CAS extension and fetch::cash_close_verdict).
- The build's versions of the halt-day rule (RQ-5), the one-sided-book rule (RQ-6), the basis-verdict authority (RQ-4), the FM-F107 recovery rule (RQ-2, when present) and the DayUniverse union (RQ-13).
- Interim authorities are refused at the freeze: 'interim_authority:<word>'. A missing required rule is refused 'rule_missing:<name>'.
- The tables keep growing append-only. Each judged day binds the exact versions it read.

f. CHARGES
- The CHG-1 rate-set version at the freeze and each source's checked-through receipt (4.11).
- Precise mode decides; the Dhan calculator is a labelled comparison.
- The FIB-11 net-model version.

g. THE CODE AND THE PINNED BUILD
- The stamped commit that wrote the freeze through cli::store_write::root_for (called from inside cli; root_for and writes_the_store are pub(crate)). An unstamped build is refused 'unstamped_build'.
- The release cli binary that wrote the freeze, stored content-addressed by blake3 (4.14). A binary that cannot be stored is refused 'build_not_storable'.

h. THE TRADED SURFACE LIST
- The bytes of FNO_UNDERLYINGS less FNO_INDEX_UNDERLYINGS at the freeze commit, not only their digest. The setup is judged with this list whatever the judging commit's list says.

i. LEARNED REFERENCES (section 6), per scheme, per reference version.

j. THE STARTING STATE
- The per-stock own-past checkpoint digests (FIB-2 sparse snapshots) at the close of the exam's last month, sealed by fib exam through FIB-9's P2 (RQ-1).
- OWN PAST IS NOT FROZEN. It rolls forward one tradable day at a time by FIB-2's commit rule and the per-status rule of 4.4, exactly as in the backtest. The setup is a RULE over each stock's own past; freezing that past would change the rule.

k. FREEZE-TIME FACTS, kept in setup.frz and NOT in the setup id
- The results-ledger sequence of the freeze (order is by FM-F136 sequence, never by host clock).
- Its host-clock IST time, labelled as host clock.
- The newest session holding any stored bar in any decision or fill feed at the freeze, from the landing receipts.
- F0 (3.1).
- For a successor (ENTRY iii): the predecessor id, its state and the operator record.

FREEZE KEY AND IDENTITY
- FREEZE KEY: (exam record id, pair_identity) for an exam freeze; (deep run id, pair_identity) for an operator-chosen freeze. A second freeze with the same key prints the existing id whatever the commit, the clock or the rate set, and writes nothing.
- SETUP ID = blake3 under the 8-byte domain tag b'BTXFIBL1', with FIB-1's tag + u32 length + bytes framing of the nine terms (CLAUDE.md section 3 rule 3, including feed):
  - mask = to_mask(long MixKey), then to_mask(short MixKey), in the FIB namespace;
  - direction = Long, then Short;
  - instrument = the pool descriptor (the PIT rule version, the traded-surface rule, the digest of the surface list bytes of item h); never a day's member list;
  - timeframe = 'zerodha:<long N>min|<short N>min';
  - params = items a-i plus the freeze key; never item k;
  - data_digest = the deep run's MMR peaks (FM-F003) over every month the setup was chosen and examined on;
  - vocab_version = FIB_CUBE_VERSION plus the run-time descriptor digest;
  - commit;
  - feed = 'zerodha+gdfl', plus '+of1' when the order-flow authority is in the identity.
- INDIA VIX and the reference indices never enter the setup id, a day id or a day data digest (CLAUDE.md section 1; FIB-1 D-6305).
- The freeze record digest (setup.frz, including item k) is the first link of the day chain, so F0 and the freeze facts are bound by the chain without entering the id.
- A frozen setup NEVER changes. A different setup for the same pair exists only through the successor rule, with its own id, F0 and record. The old record stays.

ENTRY (D-6202)
(i) Automatic: 'fib live freeze --from-exam <exam-record-id> --out-store'.
- It freezes every pair of the exam set whose exam verdict is WINNER under the deciding readings, every scheme and the regime clause, each as its own setup, in canonical pair order, with no choice anywhere, skipping every pair whose freeze key already has a setup.
- The schedule kind runs it once per finished exam record and never again for that record.
(ii) Operator-chosen: 'fib live freeze --pair <pair-id> --from-run <deep-run-id> --by <name> --why <text> --out-store'. It freezes one other pair from a sealed, verified deep run, labelled 'operator-chosen, not an exam winner' on every line where it appears.
(iii) Successor: 'fib live freeze --successor-of <id> --by <name> --why <text> --out-store', cli only, through cli::operation_audit (Origin::Cli).
- Allowed only when <id> is STOPPED at some size or RETIRED.
- Every line of the successor carries 'successor of <id>, <STOPPED at <sizes> on <day> by <clause> | RETIRED: <reason>>', and /fib/live shows the predecessor's record.
- A successor of a STOPPED setup inherits STOPPED at every size the predecessor is stopped at, until an UNSTOP record of the successor answers it (8.7). A successor of a RETIRED setup starts PROVING from its own F0.
- Without (iii), a freeze of a pair_identity that already has a setup in any state is refused 'pair_already_frozen:<id>'.
Every form refuses by name:
- exam_not_finished, exam_started_not_finished (the start needs the exam's end checkpoint; FM-F136)
- no_exam_end_checkpoint (RQ-1)
- unsealed_run, unverified_run
- interim_authority:<word>, rule_missing:<name>
- fixed_numbers_mismatch, schema_mismatch (FM-F112)
- exam_month_in_learning
- basis_source_missing (4.8)
- charges_unchecked (4.11)
- unstamped_build, build_not_storable
- host_clock_behind_store (3.1)
- zerodha_window_closed (3.3)
- unmeasured_at_freeze:<n stocks>:<missing GDFL days> (3.4, while RQ-2 is absent)
- learning_window_not_deep:<months> (6.1)
- exam_breaches_brake_reference:<clause> (6.6)
- pair_already_frozen:<id>
- disk_projection_exceeds_floor (10.1)
Zero exam winners is a valid outcome. The page then reads 'no setup in live proof: no pair passed the exam', with the counts.

(3) F0, BRIDGE DAYS AND THE STARTING STATE (D-6203)

3.1 F0 is the first TRADABLE session in CAL-1's sourced calendar whose 09:15:00 IST open (OperatorSessionTime) comes after BOTH:
- (a) the freeze's host-clock time, and
- (b) the close of the newest session holding any stored bar in any decision or fill feed at the freeze, computed from the store's landing receipts, never from the clock.
Both are recorded. IST is computed from the UTC epoch through pull::session::IstMoment (fixed +05:30), never from the local time zone. If (a) is earlier than (b)'s close, the freeze is refused 'host_clock_behind_store'. So a session already stored, and possibly seen, at the freeze can never be booked as forward.

3.2 The starting state is the exam-end checkpoint (item j). Own past then rolls forward through the bridge days.

3.3 BRIDGE DAYS are the sessions after the exam's last month and before F0, including any whose data lands after the freeze.
- They run in warm-up mode (FM-F111): detector and walker run; own past is committed per 4.4; no row and no setup trade is recorded, shown or judged.
- The input, pending and overtaken rules of section 4 apply to them unchanged; each is counted 'bridge: committed' or 'bridge: not judged <reason>'.
- F0 is not judged until every bridge day is closed (PENDING 'bridge_open').
- The freeze is refused 'zerodha_window_closed' while R4-12's window end is earlier than the last bridge day.
So the state at F0 is exactly the state the backtest code would carry.

3.4 UNMEASURED STATE. The GDFL bulk copy ends 2026-09-24 (FM-F076). Under FM-F107 a day with no seconds summary makes a stock's raw tick 'unmeasured' from then on, so unless GDFL delivers the later sessions, every stock's exam-end state is unmeasured.
- While RQ-2 is absent, the freeze is refused 'unmeasured_at_freeze:<n stocks>:<missing GDFL days>' if any traded-surface stock's starting state is unmeasured.
- With RQ-2 present, the freeze lists the unmeasured stocks and the missing days; each is refused by name on forward days ('unmeasured', in the census) until re-measured, labelled 're-measured after <gap>'.

(4) THE DAILY RECORD (D-6204, D-6205)

4.1 ONE RECORD PER SESSION DAY
- From F0 on, every day CAL-1's sourced calendar (sourced_kind_of) lists as a session gets exactly one primary record, in session order.
- A record's ORDINAL is its append index in the days directory, never derived from the calendar. Lookup by date is a path (days/<YYYY-MM-DD>.fld). Each record holds its ordinal and the dates of the earlier records of its month (bounded by the month's sessions); a fixed-stride ordinal-to-date index is sealed per month at month close (10.1).
- Statuses: JUDGED; NOT TRADABLE: <reason>; NOT JUDGED: <reason>. A day that is waiting has no record yet and is PENDING: <reason>; brake currency (8.8) shows it.
- Non-session days get no record; CAL-1's running sums count them.
- 'Next session' everywhere in this part means the next TRADABLE session (CAL-1 tradable_day Ok). A NOT TRADABLE day waits only for the inputs the backtest's own-past rule reads on such a day (4.4) and is recorded as soon as those are closed; a Muhurat with no notified timings (2026-11-08, NSE/CMTR/71775) never becomes a deadline.

4.2 INPUTS AND COMPLETENESS
The inputs of D are the decision, fill and charge inputs only:
a. Zerodha 1-minute bars for D for every member of the pull universe (4.9), read through cli::stored::load, the one door (a day-range door once RQ-10 lands).
b. GDFL cash seconds for D, and the NFO options days PIT-1 needs.
c. The authority extensions for D, sealed: CAL-1's measured-day version containing D; PIT-1 resumed to D; DATA-1's day segment; DATA-2's day check; OF-1's day build, when 'of1' is in the identity; the per-stock-day seconds summary (FM-F107); the basis verdict (4.8) and the NSE corporate-action list read for D.
d. The rate set (4.11).
COMPLETENESS is decided from the store's own per-day landing and verification receipts (IA-9's verification, IA-6p's peer check, R4-11's chain verdict and CRC), WHICHEVER path landed the day: R4-12's schedule, the /pull form, R4-11's chain, or a manual import. The receipt names the path. A day landed by hand is judged exactly like one landed by the schedule.
A Zerodha day is complete only when every pull receipt for D has a vendor response time after D's session close (IstMoment). Otherwise D is PENDING 'zerodha_day_incomplete': a 'Run now' during the session lands a partial day, and the later complete landing is a new generation and is the one bound. A shortfall of minutes in one stock is DATA-1's stock-level flag, exactly as in the backtest.
NOT INPUTS: INDIA VIX and the reference indices. Their receipts go in a separate information-only list that is never folded into the day data digest, the MMR, the day id or the setup id. A missing one is recorded 'stamp absent' and never makes a day pending (CLAUDE.md section 1). If a frozen detector reads an index, the freeze names that index and its minutes join (a); otherwise none does. The NIFTY month close used by the regime clause (7.1) is a month-level gate input, bound in D-6208 and kept in the month verdict record, never in a day id; a month whose NIFTY close is missing reads 'regime label pending'.
MEMBERSHIP: until PIT-1 is resumed to D, D is PENDING 'membership_pending'. When PIT-1 finds a member that was not pulled for D, the ingest stage pulls its minutes for D before D is judged (RQ-16).

4.3 REASONS, ORDER AND WHAT 'LATE' MEANS
Days are judged strictly in session order, because D+1's own past needs D's own-past effect (4.4) settled.
PENDING reasons. D waits; these never overtake D:
- zerodha_day_pending, zerodha_day_incomplete, gdfl_day_pending, authority_pending:<word>, membership_pending, calendar_unmeasured, corporate_actions_unread;
- waiting_operator_window: R4-12 stopped at the operator's window end; moving it is an expected operator step with a docs/05 entry, so this is never final;
- store_unavailable: drive not mounted, access lost, or below R4-07's floor;
- input_changed_during_run (FM-F109);
- frozen_build_unavailable (4.14);
- bridge_open (3.3).
OVERTAKEN (final). D becomes NOT JUDGED 'late:<input>' iff the store ledger holds a sequence s at which BOTH:
- (1) the next tradable session is complete, and
- (2) the newest vendor-answer record for D's missing input at s says the source is ABSENT: no file for D in R4-11's drop-folder or Dropbox listing, or R4-12's run record says the vendor answered empty for D.
Both sequences are recorded in D's record. 'Late' is therefore computed from the append-only landing and journal sequences, never from when the judge happened to run: two judges over the same ledger give the same primary record. A day that is merely still queued, or one blocked by this Mac (store_unavailable, disk floor, access lost, input_changed_during_run), is never overtaken.
OTHER FINAL reasons:
- check_failed:<check>: a failed pool-wide check (IA-9 verification, chain CRC, DATA-2 fold), final at once; a later repair is a restated chain (5.2).
- pool_unpriceable: every traded-surface stock is refused on D by a data reason (4.6).
- pool_day_missing:<feed>: FM-F076's census rule.
- operator_closed:<reason> (below).
OPERATOR CLOSE: 'fib live close-day <id> --day <D> --by <name> --why <text> --out-store', cli only, through cli::operation_audit. It closes a PENDING day as NOT JUDGED 'operator_closed', so a day whose vendor never answers cannot stall the record forever. It is refused on a day that is not PENDING or with an empty name or reason. It never judges a day.
CALENDAR REVISION: when a circular recorded after a day's record changes that day (a short-notice holiday, a special session, an unscheduled closure), a CALENDAR_REVISION record is appended and the day is relabelled through a restated chain (5.2). Records never move and ordinals never change. A weekday the sourced calendar calls a session that has no bars stays PENDING, then 'late:zerodha', until a recorded circular says it was closed. A measurement alone never makes a day NOT TRADABLE.
Reasons are an append-only enum held in a versioned reason table (10.1), counted per reason, month and year.

4.4 OWN-PAST EFFECT OF EACH STATUS
The backtest's own rule is CALLED for each case, never re-implemented:
- JUDGED: committed by FIB-2's commit rule.
- NOT JUDGED with D's Zerodha minutes landed and verified (late:gdfl, a GDFL check_failed, pool_unpriceable, operator_closed after the minutes landed): FIB-9's P2 commits D in warm-up mode (FM-F111: observations, no row), because P2 reads only Zerodha minutes (FM-F107).
- Membership unknown: feeds own past only (FM-F111).
- Zerodha minutes missing (late:zerodha, a Zerodha check_failed): a hole, as the backtest excludes one.
- NOT TRADABLE: the backtest's own rule for that day kind.
- Stock-level refusals: the backtest's per-stock rule. A basis latch (4.8) withholds that stock's commits until its rebuild.
docs/04-invariants.md carries this table with one test per row, and daily_judge_equals_batch_replay spans every row.

4.5 NEVER FILLED LATER WITH OTHER DATA
A day is computed only from the frozen feeds for that day: Zerodha minutes for decisions and the rank, GDFL seconds for fills and order flow. Never another vendor; never GDFL-folded minutes in place of Zerodha minutes; never Dhan or Groww data; never interpolation; never a carry from another day. Late data from the same feeds becomes a late reading or a restated chain (section 5), never part of the primary record.

4.6 SAME CODE (D-6205)
The judge calls these functions and never re-implements them:
- FIB-8 DayInput: Zerodha minutes, the in-run 09:15 fold, the previous close (FM-F092 as amended by RQ-4).
- FIB-9 P0-P2 for one day: closes, the mover rank over that day's PIT members, the FIB-3/FIB-4 detectors at the frozen widths with each stock's own past, stamps.
- The frozen Generation's cut lookup (R1).
- FIB-1 contains() against each side's MixKey and partitions.
- FIB-5's walker at Resolution::Second on GDFL seconds, with DATA-1's exact factor pair, the one-sided-book rule (RQ-6), the halt-day rule (RQ-5, 4.10) and the forced exit only from [15:09:00, 15:10:00). The AT_WORST_SEEN reading decides (FM-F078).
- FIB-6's time-ordered one-position-per-stock scoring per side.
- FM-F061's pair readings.
- FIB-11's charges at both sizes in both modes, with an explicit CHG-1 rate selector (4.11), and the per client-security-day rounding applied per setup, labelled 'as if this setup were the account's only activity in that security that day'.
- FIB-7's streaming gate and downside accumulators, through RQ-7's append-a-day API.
- FIB-2's commit rule and the per-status rule of 4.4.
The minute-resolution outcomes of the same decisions are recorded beside as information: the forward gap of the cheap stage, and FM-F079's mixed reading.
DATA REFUSALS ARE NEVER ZERO-TRADE DAYS. A stock-day refused for any data reason goes to the refusal census (a primary reason plus FM-F116's multi-label census) and never counts toward a judged zero-trade day. If every traded-surface stock is refused on D, D is NOT JUDGED 'pool_unpriceable'.
Shared work: P0-P2 run once per (pinned build, frozen width set) per day for every setup that shares them. A new day computes only itself.
PROOF: daily_judge_equals_batch_replay. Judging days one at a time equals one batch replay of the span through the backtest path GIVEN THE SAME EXCLUSION SET AND REBUILD EVENTS (RQ-3), byte for byte in every result field.

4.7 ELIGIBILITY USES WHOLE-DAY DATA
Decisions read bars 0..N (FFC-L06). Stock-day eligibility (DATA-1 factor_not_constant, DATA-2 differs, no_execution_seconds, vendor_known_defect, the basis verdict, clock_misaligned) is computed over the whole of D after the close, which a live process could not know at the decision bar. This look-ahead in ELIGIBILITY is stated and checked:
- For every refused stock-day that has Zerodha minutes, the stage-A 'would-have-traded' outcome enters FM-F079's mixed reading (RQ-7), which the brake checks (8.2).
- The refused share is shown per day.

4.8 BASIS CHANGES: CORPORATE ACTIONS (D-6219)
The problem:
- A session pulled on its own evening is raw. On an ex-date the stored previous session is on the old basis and D is on the new one. The backtest's history, pulled later with both days adjusted, never sees that step (FIB-8 item 2).
- FM-F092(c) accepts a close across two landings when DATA-1's pair for D-1 equals the pair at D's first matched minute. Going forward both feeds are raw on both days, so both pairs are 1, and a raw-to-raw close across an ex-date would be ACCEPTED.
- Zerodha's adjustments are not always proportional (ASHOKLEY GDFL/2 - Rs 2.48), drift run to run (CANBK, 359 runs), are quantised to 50 paisa, and leave volume unscaled in 45 of 101 adjusted shares (charter, D-3323).
- An undetected ex-date distorts the move, the rank and own past while looking exactly like a market result: the failure wearing a success's clothes that CLAUDE.md section 4 bans.
The rules:
- The PREVIOUS CLOSE for D comes from the ingest stage's re-read of D-1 pulled in the SAME landing as D (RQ-16), so FM-F092(c) 'the same landing' holds as it does in the backtest.
- The BASIS VERDICT per stock-day is an ingest-stage authority output (RQ-16), consumed by FIB-8 through an authority trait in both paths (RQ-4). For a historical day it is FM-F092's existing proof. For a forward day it needs, for that stock: (1) the NSE corporate-action list (R4-26 COR-06, tag nse) read for D, with a dated read receipt; and (2) the per-bar class of the re-read D-1 against the stored D-1: same | ratio_step | affine_step | revised | differs, with digests. FIB-15 adds no basis logic of its own.
- Under R4-26's law a measured class may only REFUSE. A stock is accepted on D only when the NSE list for D was read and either (a) it has no row for that stock with ex-date D and the re-read class is 'same', or (b) it has a row and the stock has been rebuilt (below). Every other combination refuses the stock by name: basis_unproven (no NSE read for that stock and day), basis_unsourced (a measured step with no nse row), basis_differs (revised or differs).
- LATCH. Any refused outcome sets 'basis_unresolved since <D>' for that stock. It is refused on EVERY later day until a sealed rebuild clears it; a 'same' re-read on D+1 never clears it. The latch and its clearing are events with digests.
- REBUILD, after an nse row: the ingest stage re-pulls the stock's full history as Zerodha serves it on or after D (RQ-16; a new store generation, append-only). FIB-9's P2 rebuilds own past from the stock's warm-up start through the newest committed day. The rebuild is recorded as an event with its digest and is an input to the batch replay (RQ-3). Cost BOUNDED, O(the stock's history) per action.
- NSE's adjusted base price rule is not sourced in docs/00-charter.md: UNVERIFIED. FIB-15 never computes a previous close from a ratio.
- With no NSE corporate-action source at all, the freeze is refused 'basis_source_missing'. With the source present but D's list not yet read, D is PENDING 'corporate_actions_unread' (pool-wide), closed only under 4.3.
- Letting a measured ratio step decide without an nse row is a law conflict with R4-26; it needs the operator's ruling and a docs/05 entry (open question).
- A rename (ZOMATO to ETERNAL style) is handled in 4.9.

4.9 UNIVERSE, RENAMES, CATCH-UP AND THE CAS ERA
- PULL UNIVERSE = PIT members on D, UNION the frozen surface list, UNION every series with own past in the starting state (RQ-13), so a leaver's own past keeps accruing as it does in the backtest (FM-F111). RANK POOL = PIT members on D under the frozen PIT rule. TRADED SURFACE = the frozen list (item h). An entrant is ranked per the frozen pool rule and never traded by this setup; appending an entrant to FNO_UNDERLYINGS leaves every frozen record byte-identical.
- RENAMES are detected daily by ISIN equality in the instruments dump (R4-28) and NSE's series files, an ingest stage (RQ-16) that appends a dated rename row labelled 'measured by ISIN'. A measured row may only refuse: the stock is refused 'rename_unsourced' until an nse-sourced row exists, then its own past continues under the new key. Receipts are content-addressed (4.12), so a rekey (gdfl-rekey --apply) never breaks replay (RQ-11).
- CATCH-UP. Every landed day records its pull date. When the Mac was off or the daily Kite sign-in was missed, R4-12 catches up oldest first and Zerodha serves those days back-adjusted as of the catch-up day. A stock-day pulled after a later session's ex-date for that stock (an nse row with ex-date in (D, pull date], or DATA-1's factor not 1 for D) is refused 'pulled_late_adjusted' and latched as in 4.8. The census counts 'catch_up'. Whether GDFL's Dropbox keeps a week of day files is UNVERIFIED.
- CAS ERA. From 2026-08-03 a CAS-eligible share's Zerodha minutes end at 15:14 (charter, D-3320, D-3324), so FIB-8's previous close is the 15:14 minute close on every forward day, while the anchored learning window is entirely on the 15:29 close. The freeze records the session-structure rule version. Every forward comparison is labelled 'references learned before the 2026-08-03 closing-auction change'. The exam's CAS-era months' worst day, worst week and drawdown are shown beside the references as information (FM-F139 splits them).

4.10 HALTS, OUTAGES AND CIRCUIT BREAKERS
A halt happens INSIDE a session, so excluding a halt day after the fact is look-ahead and drops exactly the days a real account loses most.
- HALT-DAY RULE (RQ-5, the same code in the backtest and here, frozen as a rule version): entries before a recorded halt start stay in the record, priced AT_WORST_SEEN through the halt (FM-F078) and labelled 'halt: real loss may be larger; the position may carry overnight (UNVERIFIED)'. Only entries after the halt start are NOT TRADABLE. The references are computed under the same rule (6.1).
- POSSIBLE HALT: when D's GDFL seconds show a pool-wide silence in the continuous phase longer than the longest pool-wide silence on any learning session not recorded as a halt (generated, frozen with the references; no length is typed), D is judged under both readings: 'no halt' (the primary, until CAL-1 records a halt row) and 'halt from the silence start' (a reading the brake checks, 8.2). When CAL-1 later records the halt row, a restated chain (5.2) follows.

4.11 CHARGES ON FORWARD DAYS (D-6207)
Two explicit CHG-1 selectors, passed by the daily judge and the batch replay alike; neither ever defaults (RQ-8):
- DECIDING: the operator's standing rule, CHG-1's 'current rates on every date' (today's Dhan + statutory rates on every trade date). Every trade, learning and forward, is priced at the newest recorded rate set. When a new rate set is recorded, a new reference version is appended (6.5; recomputed from the sealed learning re-score, BOUNDED once per rate change) and the forward figures move with it. Nothing earlier is rewritten; a brake that tripped stays tripped.
- BESIDE, checked by the brake: 'rates in force on each date' (CHG-1's dated regime lookup), applied to learning and forward trades alike. A date that no regime row covers is 'not computed: charges_unrecorded' for that reading only.
- INFORMATION: the frozen-rate figures, like for like with the freeze-time references.
- CHECKED THROUGH: each CHG-1 source (the NSE, SEBI and NCL circular lists; Dhan's pricing page) carries a 'checked through <date>' receipt in docs/00-charter.md (RQ-8). A judged day after the oldest checked-through date carries 'charges not re-checked after <date>'; the verdict over such days reads 'PASSING, charges unverified after <date>', never plain PASSING; the brake still acts. No receipt at all refuses the freeze 'charges_unchecked'. Dhan's pricing page states no effective dates: UNVERIFIED.
- A rate change recorded after days were judged makes a restated chain for the in-force reading (5.2).

4.12 A JUDGED RECORD
Fixed stride; variable-length parts live in side files.
- The session day, its ordinal, its status, and the landing sequences that decided it.
- Per side and for the pair, at each size and each rate reading: trades, wins, losses, net money (precise; Dhan comparison labelled; gross labelled), net R sums, largest win and loss, unpriceable and flagged counts, profitable-day flag.
- The brake values, the brake warnings and the verdict summary.
- The running counters after the day (8.5), so the newest record alone serves the status in O(1).
- The offset and count of the day's trades in trades/<date>.flt.
- The offset, count and digest of the day's receipts and refusal census in days/<date>.rcp, coded against the versioned reason table. Adding a reason is a new table version, never a new stride.
- RECEIPTS ARE FORM-INDEPENDENT: for each input, the blake3 of the committed records in D's index range plus the store generation number, never the file bytes. Appending D+1 to a month file, sealing a month (SC-6) or a rekey changes no receipt. Folded into the day data digest (MMR leaves).
- The state-snapshot digest after the commit.
- The judging build: commit and binary digest; the reference version used.
- The previous record's digest (hash chain).
- The day id = blake3 under b'BTXFIBD1' over: setup id, session day, day data digest, previous record digest, judging commit, feed.

4.13 A TRADE RECORD holds:
- the stock (series id and symbol on D), side, partition, decision second;
- entry and exit seconds and prices (GDFL raw paisa), the quantity at each size, the exit reason, flags (incl. one_sided_book, halt);
- gross, every charge line, and net at each size in both modes and both rate readings; net R;
- the minute-resolution outcome (information);
- reference stamps (index direction, INDIA VIX, order flow delta and order flow cumulative delta at the decision, labelled APPROXIMATE) and the 13:25 mover-persistence flag. These are information only; an absent stamp reads 'stamp absent' and never blocks a day.

4.14 PINNED BUILD AND CODE CONTINUITY (D-6205, D-6206)
- The freeze stores the release cli binary that wrote it, content-addressed by blake3 under <store>/results/fib-v1/live/builds/. Each setup is judged by its pinned build: the schedule kind launches it as a child process after checking its digest; a mismatch reads 'frozen_build_digest_mismatch'. A missing or unrunnable binary makes days PENDING 'frozen_build_unavailable'. Newer builds never judge a setup they have not proven, so an unrelated fix never ends a record.
- CONTINUITY. A newer commit may take over a setup only through 'fib live replay <id> --commit-check': it replays every judged day from the starting state with the bound inputs under the new commit and compares every RESULT field, excluding the judging commit and the ids derived from it.
  - Equal: a CONTINUITY record (old commit, new commit, days replayed, digest) is appended, and the new commit may judge this setup.
  - Different: the new commit does not judge this setup. A 'restated under commit <c>' chain (5.2) is appended beside as information, showing what the change would alter. The primary stays on the pinned build.
  - Cost BOUNDED, O(judged days x day work), once per commit per setup. The schedule runs it after a redeploy, before a setup is ever handed to the new build.
- RETIRED (terminal) only when the pinned build can no longer judge (binary missing or failing its digest, or a bound input in a store format it cannot read) and no continuous commit exists. It is recorded with the reason. The record stays; a new start needs the successor rule.
- Stated limit: continuity is proven from the frozen starting state. It does not re-derive the exam-end checkpoint under the new commit, because FIB-15 never opens an exam month (FM-F136). If FIB-13 provides a state-only replay through the exam months (RQ-17: own past only, no rows, no outcomes, recorded 'exam months read for state, no outcome'), the commit-check also compares that checkpoint; otherwise D-6206 states the hybrid.
- Whether a copied release binary runs from the store volume on this Mac is UNVERIFIED until the rehearsal measures it.

4.15 REHEARSAL, NEVER PROOF
'fib live rehearse --from <day> --out <scratch>' runs the whole daily path over already-stored LEARNING days.
- Every line is labelled 'REHEARSAL: days already stored and seen - not forward proof'.
- It is refused with --out-store, and refused on any exam month.

(5) LATE READINGS, RESTATED CHAINS AND RESTATED LEARNING DATA (D-6216)

5.1 LATE READING: own past unaffected. When the same frozen feeds deliver an input of a day already closed as NOT JUDGED, and own past for that day was already committed from its Zerodha minutes (4.4), the day's trades are computed from the snapshot before it and inserted into the CORRECTED reading.
- The primary record is untouched.
- The corrected reading's counters are a day-indexed segment tree whose node holds (sum, max prefix, min prefix, max drawdown, longest losing-day run with its prefix and suffix runs), plus one tree over closed weeks; a week's week-to-date trough is recomputed inside that week only.
- Cost O(log n) per inserted day, declared in docs/06-limits.md.

5.2 RESTATED CHAIN: own past affected. Triggers: a late Zerodha day; a vendor re-delivery or repair (FM-290); an authority revision (CAL-1 including CALENDAR_REVISION, PIT-1, DATA-1, DATA-2, OF-1); a rate change under the in-force reading; a possible halt confirmed by a halt row; a 'restated under commit <c>' check.
- From the snapshot before the first affected day, re-judge forward to the newest day under the revised inputs, as a parallel labelled chain with its own counters, own past, snapshots and hash chain.
- A newer restatement supersedes the previous corrected chain: one current corrected chain at a time; the older ones stay, listed in the events.
- The primary is flagged 'own past built on superseded <authority> since <day>'.
- Cost BOUNDED, O(days since the first affected day) per restatement, declared in docs/06-limits.md. FFC-L14's O(1)-per-day claim is for the primary reading only, and says so.

5.3 Readings never enter the primary record or its own past, and never rewrite anything.

5.4 VENDOR RE-DELIVERIES. No importer re-imports a changed day today: R4-11 takes done days from its import journal, so a second scan imports nothing. Until RQ-12 lands, the page states 'vendor re-deliveries are not detected'.

5.5 LEARNING DATA RESTATED SINCE THE FREEZE. When a month inside the setup's data_digest is restated (GDFL re-processing FM-290, a Zerodha re-adjustment, a DATA-1, DATA-2 or OF-1 repair):
- an event 'learning data restated since freeze: <months, authority>' is appended;
- the unchanged frozen setup is re-scored on the corrected months, with no re-selection and 0 trials added (FM-F067);
- the corrected references, the corrected exam verdict and, with RQ-17 only, the corrected checkpoint digest are shown beside the frozen ones;
- the brake checks both reference sets, the tighter tripping, until the operator chooses.
FM-F140's 'no exam while a re-delivery is pending' is a prerequisite of the first freeze: no exam, no freeze.

5.6 PROVISIONAL MINUTE READING. When D's Zerodha minutes have landed and GDFL has not, the stage-A minute walker (the same code) computes a provisional reading of D from own past through D-1. If it breaches a clause against the stage-A references (6.1), one event 'PROVISIONAL BRAKE WARNING (minute resolution, not deciding)' is emitted. It never stops a setup and is superseded when D closes.

(6) LEARNED REFERENCES (D-6210)

6.1 DEEP RE-SCORE. The references are computed at the freeze by FIB-7's own gate and downside functions, never re-implemented, from a DEEP RE-SCORE of the frozen pair over every learning month:
- a net re-score at Resolution::Second under the pinned build's rules (halt-day, one-sided book), adding 0 trials (FM-F067), sealed and content-addressed, with its deep-coverage census;
- a learning month with no deep re-score refuses the freeze 'learning_window_not_deep:<months>'; stock-day deep refusals are carried in the census;
- the stage-A and the FM-F079 mixed-reading references are frozen beside, so the forward minute, provisional and mixed readings are compared like for like.
Cost BOUNDED, O(learning seconds touched by the pair's decisions), once per freeze.

6.2 PER SCHEME. The anchored scheme learns on [0, n_learn), the final step's window the exam judged against; each rolling scheme L on [n_learn - L, n_learn). All are subsets of the one re-score. For each scheme, per direction and for the pair, at Rs 1,00,000 and Rs 5,00,000, net, in money and in R, under each rate reading:
- the worst day net money;
- the worst closed ISO week (IST);
- the worst week-to-date at any day close (the like-for-like reference for a running week);
- the maximum drawdown, at FIB-7's granularity;
- the learned worst drawdown over windows of every length up to the window's own length (information, for 8.4);
- the longest losing runs of trades, of days and of weeks;
- the longest recovery time, in tradable days;
- the loss floor (both FM-F074 readings, deciding as D-6424);
- the NIFTY regime cut points (13:31);
- the longest pool-wide GDFL silence on non-halt sessions (4.10);
- the learning share of hole days (7.4);
- for information: the per-side quality figures, the share of profitable days, the average and median net per day (13:26).
Per-side references and accumulators use the frozen deciding FM-F061 reading; the other reading is beside.
A reference FIB-7 reports as undefined is frozen as undefined with its reason; the brake then uses FIB-7's recorded deciding reading for it (for example FM-F074 min(0, worst)), and the page names it.

6.3 INFORMATION BESIDE, never deciding: the same references over the whole past seen at the freeze; the exam window's unseen worst day, worst week and drawdown; the number of passers the 13:26 selection chose among.

6.4 SELECTION BIAS, stated in D-6210 and docs/06-limits.md: the 13:26 order prefers the smaller worst learning day and week, so the learning references are statistics the selection optimised, the smallest among many tries. The brake may therefore trip early on ordinary variance; 8.4 shows how often.

6.5 REFERENCE VERSIONS. A newly recorded rate set (4.11) or restated learning data (5.5) appends a reference version. The version that judged each day is in its record.

6.6 EXAM CHECK AT THE FREEZE. The five downside values on the exam months are compared with the frozen anchored references. If FIB-13's exam verdict already includes the five downside checks against these references (FIB-07's amendment), the freeze records that and cites it. Otherwise a breach refuses 'exam_breaches_brake_reference:<clause>'.

(7) JUDGING (D-6208, D-6209)

7.1 THE WINNER GATE ON THE FORWARD RECORD
After every judged day, the record is judged by the 17:11 gate through FIB-7's functions, net, at each size.
EACH SIDE: win rate >= 51%; average win >= 2x average loss; profit factor >= 2; largest win >= 2x largest loss; positive in total over the forward record.
THE PAIR:
- trades x Y >= 500 x D, compared in i128. D is FIB-7's FM-F076 D unchanged: the calendar tradable days of the forward window minus pool_day_missing:<feed>. Y = the tradable days of the 12 calendar months ending at the window end. A second reading keeps every not-judged session day in D, shown beside.
- every calendar year positive (a partial year is labelled, FM-F075);
- the pair's own four quality checks;
- >= 70% of closed forward months positive (months with a trade);
- no closed forward month below the frozen loss floor;
- leave-one-out: drop the best day, week, fortnight (both readings), month and quarter, one at a time, and it still passes (FM-F064), through RQ-7's append-a-day API.
UNDER EVERY SCHEME (13:29): the floor, the downside comparisons and the regime clause use each scheme's frozen references. PASSING needs every scheme; the page names each scheme that fails.
EVERY MARKET SITUATION (13:31): each closed forward month is labelled falling, flat or rising by NIFTY's month move (close of the month's last tradable day against the previous month's), with each scheme's frozen cut points; NIFTY is context only (CLAUDE.md section 1). For the pair, the forward months of each regime together are net positive. A regime with no forward month reads 'no forward month in this regime: not proven', and the verdict is not PASSING until every regime has one. Per-side quality per regime is shown. INDIA VIX regimes and named periods are information only.
Three columns, side by side: 'winner rule: quality per side, quantity per pair' (deciding); 'information: pair only'; 'information: each side alone'.
FIB-7's readings carry over unchanged: FM-F072, F073, F077, F078, F079, F080, F083, F084.

7.2 THE FIVE DOWNSIDE CHECKS
The forward trades are fed into FIB-7's own accumulators. Each comparison is per direction and for the pair, at both sizes, under every scheme:
1. forward worst day vs the frozen worst day;
2. forward worst week vs the frozen worst week;
3. forward max drawdown vs the frozen max drawdown;
4. forward longest losing runs of trades, days and weeks vs the frozen ones;
5. forward recovery time, including the current time under water, vs the frozen longest.
A breach FAILS that check, named with both numbers. Day, week and drawdown breaches also trip the brake (8).
Also shown (13:26): the forward share of profitable days, the average and the median net per day, next to the frozen learning figures. The median is kept in an order-statistic structure, O(log n) per insert or replacement, declared.

7.3 JUDGEABILITY (D-6209)
W0 comes from FIB-1 R6's function applied to the gate's leave-one-out clause: the record is judgeable from the first ordinal at which (the quarters leave-one-out drops) + 1, that is two, calendar quarters starting on or after F0 are COMPLETE, meaning every session day in them has a closed primary record. A partial quarter at F0 does not count.
- Before W0: 'not yet judgeable: quarters complete q of 2; k judged and j not-judged session days; n pair trades; the gate's pro-rated count at k days is Q(k) = ceil(500 x k / Y)'. Each clause's current value is shown, labelled 'provisional'.
- From W0 on, the verdict is PASSING or FAILING, naming every failed clause, scheme and regime.
- Until the record spans one gate year, it carries FM-F073's label 'forward shorter than one gate year (k of 12 months, pro-rated)', and the one-gate-year reading is shown beside it.

7.4 COVERAGE. 'judged k of m session days; n not judged, by reason.' When the not-judged share exceeds the learning share of hole days (6.2), a WARNING is shown.

7.5 LATE AND RESTATED. The verdict is computed on the primary record and on the corrected reading. PASSING needs both. Any difference is shown as 'depends on late or restated days'.

7.6 STABILITY AND SELECTED LOOKS. After W0 the verdict is re-read every day, so the first PASSING day is a selected look.
- Beside the verdict: the share of days since W0 on which it read PASSING, and the current run of PASSING days.
- The first pass is labelled 'a selected look'.
- An anytime-valid sequential reading on the daily net R series is shown as information. Its construction and published reference are recorded in D-6209 by the builder; if none can be cited, it is NOT DONE, never invented.

7.7 CONFIDENCE AND LUCK, COST SPLIT
- The daily path computes only O(1) figures from running moments on the daily net R series: the t statistic, a Wilson interval for the win rate, Bonferroni over the trials ledger.
- The bootstrap, permutation and SPA / RC / RW figures (R8) run in 'fib live luck <id>', declared BOUNDED O(replicates x days) in docs/06-limits.md, sealed per run and shown 'as of record <ordinal>'.
- A bench row proves that the daily judge does no bootstrap work.

(8) THE BRAKE (D-6211, D-6212, D-6214)

8.1 CLAUSES. Only these trip the brake, as the operator ruled. Comparisons are in paisa i64 and STRICT: equal is not outside.
- DAY: a forward day's net money < the frozen worst day.
- WEEK, either of: the week-to-date net at a day close < the frozen worst week-to-date (like for like); a closed forward week < the frozen worst closed week.
- DRAWDOWN: the forward drawdown at FIB-7's granularity is deeper than the frozen max drawdown.
Each clause is checked for the long side, the short side and the pair, at Rs 1L and at Rs 5L.
Losing runs and recovery time are judged (7.2) and shown as a WARNING; they never stop the setup.

8.2 READINGS CHECKED, a fixed enum. The brake trips if ANY of these breaches, and names it:
- the primary record under the deciding rates;
- the primary record under rates in force on each date (4.11);
- FM-F079's mixed reading, including the would-have-traded outcomes (4.7);
- the corrected reading (5.1, 5.2);
- the possible-halt reading, when one exists (4.10).

8.3 REFERENCES CHECKED
- DECIDING: the anchored scheme's frozen references (the window the exam judged against), at the reference version in force. This choice is PROPOSED in D-6211, with an open question, because a short rolling window's minimum trips an unchanged setup often (8.4).
- Every other scheme's references are checked on the same readings and shown as a WARNING 'outside rolling-L reference', with its false-alarm chance.
- An 'as braked: tightest scheme' line beside shows what tripping on the tightest scheme would have done.
- After restated learning data (5.5), both reference sets are checked and the tighter trips.

8.4 FALSE-ALARM CHANCE, shown beside every clause and stated in D-6211 and docs/06-limits.md.
- If days were exchangeable and the setup unchanged, the chance that at least one of m forward days falls below the minimum of N learned days is m/(N+m). For closed weeks, weeks take the place of days.
- For drawdown, the learned worst drawdown over windows the same length as the forward window (6.2) is shown beside.
- A family figure across the comparisons actually checked is shown as the upper bound min(1, sum), labelled 'upper bound, assumes nothing about dependence'. A bootstrap estimate is in 'fib live luck'.
- These are labelled approximations: exchangeability is assumed, not proven.

8.5 WHAT HAPPENS ON A BREACH
- At that moment the setup's status at that size becomes STOPPED, with: size; side or pair; clause; reading; scheme; session day; forward value and frozen reference, in money and R; the reference version; the record ordinal.
- Default scope: a side's breach stops the pair, naming the side (conservative). Beside it, an 'as braked: only the breaching side stopped' line shows the other choice, with the pair's quantity clause judged on what remains (open question).
- The headline status is STOPPED as soon as any size is stopped. Each size is listed.
- Cost: O(1) per day for the deciding brake, from fixed-size running counters (cumulative money, running peak, week sum and week id, week-to-date trough, closed-week minimum, drawdown) over 3 lines (long, short, pair) x 2 sizes x the fixed reading enum. The scheme warnings add one counter set per scheme: O(family size) = O(log months) per day, declared. Feeding a trade is O(1).

8.6 IT STAYS STOPPED
- There is no automatic restart and no timer.
- The record keeps being judged after a stop, so the proof has no holes. Every later day is flagged 'after brake'.
- The 'as braked' line counts no trades from the first session after the breach day; the breach day's own result is included.
- A breach found through a reading (a corrected chain) is recorded at the judge run that found it, naming the day it applies to; the as-braked line starts after that run.

8.7 UN-STOPPING IS AN OPERATOR ACTION, cli only in v1
Command: 'fib live unstop <id> --size <s>|all --by <name> --why <text> --out-store'.
- Refused unless STOPPED at that size; refused with an empty name or an empty reason.
- It appends an UNSTOP record holding: who, as typed; the OS user; the host-clock IST time, labelled as host clock; the ledger sequence; why; the brake record it answers; the drawdown since F0 and the frozen reference.
- It also goes through cli::operation_audit (Origin::Cli), so /audit shows it.
- There is NO api un-stop route: the api has no authentication, so anyone who can reach the port could press it.
- The OS user is stored and never served; the routes serve the self-declared name, labelled 'self-declared'.
After an un-stop:
- DAY and WEEK re-arm with the frozen references.
- DRAWDOWN is measured from the equity at the un-stop, labelled 'drawdown since un-stop'.
- The headline always shows the drawdown since F0 and the un-stop count; the drawdown since F0 stays judged in the downside checks.

8.8 BRAKE CURRENCY (D-6214)
The brake can act only on judged days. If the newest closed tradable session is not judged by the next tradable session's 09:15:00 IST open (OperatorSessionTime, the only deadline), then until it is judged or closed the setup shows 'BRAKE NOT CURRENT since <day>: <cause>'.
- Causes: every PENDING reason (4.3), waiting_operator_window, frozen_build_unavailable, and the CALENDAR HORIZON: when the host date passes SOURCED_LAST_DAY (2026-12-31 today), 'calendar unsourced after <day>' (R4-12's 'stopped: calendar_horizon'). No session is ever invented from a weekday.
- From the first day of the last sourced year (CAL-1 item 6), a warning names the horizon.
- With the schedule kind on, one fib.live.pending event is emitted per host day. With it off, the page computes currency at request time from CAL-1 and the newest record (O(1)) and states that no events fire.

(9) NO EXECUTION (D-6213)
FIB-15 never places, sends, modifies or prepares a real order, and holds no broker credential. Enforced as follows:
- The fib live modules read the store only. No HTTP client, socket or vendor client is constructed. A test runs every verb with a network seam that fails if it is touched.
- No credential reader is imported: not pull's Parameter Store path shape, not api::credential_law.
- The 'fib_live' schedule arm passes no credential_law type to the judge; it launches the pinned binary as a child. D-6213 records that the api process holds the Zerodha data credential (R4-12) and that FIB-15 never receives it. A source-scan test checks that the arm and every fib live module name no credential_law, pull::http or std::net item.
- No order words appear (place_order, modify_order, cancel_order, order_id); a source scan checks this.
- No output names a session that has not closed, and nothing writes an order-shaped list for a coming session; tests check both.
- Dhan appears only as the broker whose charges CHG-1 prices. Kite's order endpoints exist on the vendor's API (the charter read the orders page for exchange codes); whether this account's token could place orders is UNVERIFIED and irrelevant, because FIB-15 holds no token.

(10) SHOW (D-6215)

10.1 FILES. Every file is sealed once through store::staging::create (tmp, fsync, rename, directory fsync), using FIB-8's header, footer and reader functions. A file is never patched or appended in place: each daily append is one new small sealed file, O(1). A matching sealed file is skipped; a different identity at the same path is refused.
Layout, under <store>/results/fib-v1/live/:
- index/<setup-seq>.fli: one sealed row per setup ever frozen, with its freeze key.
- builds/<blake3>: the pinned release binaries.
- snapshots/<blake3>.snp: FIB-2 sparse snapshots in one content-addressed store per (pinned build, frozen width set), shared across setups. Kept: each month-end and the newest. An intermediate snapshot is rebuilt by replay from its month-end, BOUNDED O(days in the month), declared.
- reasons/<version>.frt: the versioned reason table.
- Per setup, <id>/:
  - setup.frz: the freeze record;
  - days/<YYYY-MM-DD>.fld: the day record (fixed stride); days/<YYYY-MM-DD>.rcp: its receipts and refusal census;
  - trades/<YYYY-MM-DD>.flt: fixed-stride trades;
  - index/<YYYY-MM>.fdi: the ordinal-to-date index, sealed at month close;
  - readings/<chain-id>/<YYYY-MM-DD>.flr: corrected, restated, possible-halt and provisional readings; the current corrected chain id is named by the newest event;
  - events/<YYYY-MM>/<seq>.fle: fixed-stride events, one sealed file each, plus a sealed per-month events file at month close;
  - luck/<ordinal>.flk: sealed luck runs.
Write order: trades, receipts, snapshot, then the day record last, as the commit point.
Disk: R4-07's floor_check runs before any byte, with the reservation taken under the same lock R4-11's chain uses (RQ-18). Nothing is ever truncated. Bytes per day and per snapshot are measured at rehearsal and projected to W0 and beyond, labelled EXTRAPOLATION; the freeze is refused 'disk_projection_exceeds_floor' when the projection to W0 exceeds the floor-checked free space.
Every store generation a judged day bound stays addressable (RQ-11); verify refuses 'bound_generation_missing:<file>' by name and never skips.
docs/02-store-format.md gets a section 'FIB live record v1'.

10.2 VERBS (cli fib live ...)
- freeze (forms i, ii, iii)
- judge [--setup <id>]: every pending day of every PROVING or STOPPED setup, in order; idempotent; exclusive create-new lock (FM-F114); checks the chain incrementally on each run, O(new records).
- close-day
- unstop
- status <id> [--json]
- replay <id> --commit-check
- verify <id>: full, on demand: chain, footers, counters recounted from the trade files, trials receipts, bound generations; BOUNDED O(record).
- luck <id>
- rehearse
The store is written only with --out-store. --out writes elsewhere.

10.3 ROUTES
crates/api/src/fib_live_routes.rs. Read-only GETs, reading only through cli::fib::live_files readers that return cli-owned view types: api's Cargo.toml is unchanged and api names no runner type. Cost O(limit) per request and O(1) per row. limit and offset are named and required; limit is bounded by a measured value set the way FIB-1 R10 sets it, never typed; above it the request is refused 400.
- /fib/live/setups.json?limit=&offset=
- /fib/live/setup.json?id= : the frozen setup, with MixKeys decoded through FIB-14's /fib/schema.json?run= digest (the page refuses on a mismatch); references per scheme and version; status per size; brake state, reasons, warnings and false-alarm chances; the verdict per clause, side, pair, size, scheme, regime and reading; downside comparisons; brake currency; predecessor link.
- /fib/live/days.json?id=&limit=&offset=
- /fib/live/day.json?id=&day=YYYY-MM-DD&limit=&offset= : the primary day, its trades page, and the current corrected chain's reading for that day (an O(1) path lookup).
- /fib/live/equity.json?id=&limit=&offset= : per-day equity from the day records' running counters, paged.
- /fib/live/events.json?id=&limit=&offset=
Errors by axum StatusCode names: NOT_FOUND for an unknown setup or a day outside [F0, newest]; BAD_REQUEST for a garbage, missing or over-bound parameter; CONFLICT for an unsealed or unverified freeze. Every numeric name carries its basis and size (FM-F147). No inf or NaN.

10.4 PAGE /fib/live
web/src/routes/fib/live. Plain tables. The page LEADS WITH THE DAY (13:26). Sections, in order:
- The setups: status per size, brake currency, verdict word, judged / not-judged counts, F0, successor links.
- Per setup, first the plain words: 'On forward days this setup made money on X% of days; its worst day lost Y; its average day made Z' (generated values). Then:
  - the brake: reason, day, numbers, scheme, reading, and the false-alarm chance;
  - the day-by-day equity line (paged from equity.json) and the share of profitable days, forward vs learned;
  - the Winner Gate on the forward record: three columns, every scheme, every regime, stability, and not-yet-judgeable progress;
  - the five downside checks, forward vs frozen;
  - long and short side by side;
  - the daily record, paged, with reasons, after-brake flags and the as-braked lines (pair, side only, tightest scheme);
  - the events.
Labels:
- 'paper record - no real order was placed, sent or prepared';
- 'forward: every judged session opened after the freeze; judged after the close on data delivered after the session - not real-time';
- 'decisions on Zerodha minutes; fills on GDFL seconds at the worst price';
- 'net after charges, precise NSE/SEBI method, today's rates on every date; rates in force on each date shown beside';
- 'charges not re-checked after <date>', where it applies;
- 'Dhan calculator (estimate)';
- the CLAUDE.md section 1 gross sentence and the gross column, until CHG-2 merges;
- 'research only, never Selection V6 or execution authority';
- 'engineering rules for testing, not trading advice';
- 'no setup is profitable every day' (13:26 item 4);
- 'references learned before the 2026-08-03 closing-auction change';
- 'vendor re-deliveries are not detected', until RQ-12 lands;
- 'GDFL clock basis unchecked', until RQ-15 lands;
- 'an unchanged setup would trip this clause by chance with about this probability';
- 'operator name is self-declared';
- 'operator-chosen, not an exam winner' and 'successor of <id>', where they apply.
Layout: phone width with a 16 px side gutter and no horizontal page scroll; light and dark. npm run build and npm test pass in your worktree, and no crate gains a Node dependency.

10.5 LOGS AND AUDIT
Events go through telemetry::emit_for_run from cli at structural boundaries only (gate 17):
- judge run start and finish;
- per (setup, session day): fib.live.day, with judged | not_judged:<reason> | not_tradable:<reason>, trades, and net at both sizes;
- fib.live.pending: once per host day past the deadline (schedule on);
- fib.live.brake: every field of 8.5; fib.live.brake_warning;
- fib.live.provisional_warning;
- fib.live.unstop, fib.live.close_day, fib.live.freeze, fib.live.reading, fib.live.restated_chain, fib.live.latch, fib.live.basis_rebuild, fib.live.calendar_revision, fib.live.reference_version, fib.live.learning_restated, fib.live.continuity, fib.live.retired.
Operator actions (freeze ii and iii, close-day, unstop) also go through cli::operation_audit. Brakes raise an alert through R3-31 when it has merged; otherwise they appear on /logs only, and that is stated.

(11) SCHEDULING (D-6217) AND TRIALS (D-6218)
There is no second loop. One schedule kind, 'fib_live', is added to R4-12's scheduler (crates/api/src/schedule.rs, one step).
- It ships OFF, and is enabled only on the operator's yes, after its config/store-steps.toml rehearsal entry passes R4-06's gate.
- After a new closed session lands by any path, it launches each setup's pinned build as a child process: 'cli fib live judge --setup <id> --out-store'.
- It runs freeze --from-exam once per finished exam record, and never again for that record.
- After a redeploy it runs the commit-check (4.14) before any setup could be handed to the new build.
The per-day authority extensions are ingest-chain stages owned on the ingest side (RQ-16); each new-work spec says 'the importer hook belongs to the ingest-automation package'. FIB-15 reads them and never builds them. If one is missing, the day is PENDING 'authority_pending:<word>'.
TRIALS AND LOOKS:
- Each frozen setup is one trial, keyed by setup id. A look is one newly appended primary judged day, keyed by day id (R8: 'every daily paper-trading look'). Retries, reruns and re-judges add 0.
- Both go into FIB-7's trials ledger (FM-F067). The page shows the setups ever frozen, the looks, and R8's adjusted p-values (as of the newest luck run).
- Every judge run appends 'forward record <setup>: outcomes seen for <days>' to FIB-12's prior-look list (D-6520, FM-F131), so a later search or exam refuses or labels those months (RQ-14). Until RQ-14 lands, FIB-15 keeps the entries in its own events and the page states that FIB-12 does not yet read them.
- Setups are judged each alone. Their competition for the same seconds' volume in one account is not modelled; this is stated in docs/06-limits.md.

(12) REQUIREMENTS ON OTHER OWNERS
Each is a requirement on its owner, never an edit by this part. Until it lands, FIB-15 does what the last column says.
- RQ-1 FIB-13: seal the exam-end checkpoint at the close of the exam's last month. Until: freeze refused 'no_exam_end_checkpoint'.
- RQ-2 FIB-9: the FM-F107 recovery rule (3.4). Until: freeze refused 'unmeasured_at_freeze' when any traded-surface stock's starting state is unmeasured.
- RQ-3 FIB-9: the batch replay takes the exclusion set and rebuild events as explicit inputs. Until: daily == batch is proven only on spans without holes or rebuilds, and the rest is NOT DONE.
- RQ-4 FIB-8 (amend FM-F092) with the ingest stage: the per-stock-day basis-verdict authority (4.8). Until: freeze refused 'rule_missing:basis_verdict'.
- RQ-5 FIB-10 and FIB-5: the halt-day rule (4.10). Until: freeze refused 'rule_missing:halt_day'.
- RQ-6 FIB-5: the one-sided-book rule (DEPS). Until: freeze refused 'rule_missing:one_sided_book'.
- RQ-7 FIB-7: the worst week-to-date accumulator, the closed-week minimum, the append-a-day leave-one-out API (amortised O(1) per day), per-scheme references, the 13:31 regime clause, and the mixed reading covering every data-refused stock-day with Zerodha minutes. Until: this part's build is refused by the plan check.
- RQ-8 CHG-1: both rate selectors as explicit inputs; per-source checked-through receipts. Until: freeze refused 'charges_unchecked'.
- RQ-9 CHG-2, or the operator's section 1 edit with its docs/05 entry. Until: the gross column and the section 1 sentence beside net.
- RQ-10 cli::stored owner: the day-range door (FM-F106). Until: judge declared BOUNDED O(month-to-date bars) per stock-day.
- RQ-11 store owners (D-3350, SC-6, gdfl-rekey): every bound generation stays addressable and immutable. Until: verify refuses 'bound_generation_missing:<file>'.
- RQ-12 R4-11 follow-up: re-delivery detection. Until: 'vendor re-deliveries are not detected'.
- RQ-13 R4-26: the DayUniverse union (4.9). Until: freeze refused 'rule_missing:day_universe'.
- RQ-14 FIB-12: forward entries in the prior-look list. Until: kept in FIB-15's events, stated on the page.
- RQ-15 DATA-1: the per-stock-day GDFL clock-basis check, refusing 'clock_misaligned' (all stocks affected: pool_unpriceable). Until: 'GDFL clock basis unchecked' on every day.
- RQ-16 ingest-chain per-day authority stage (owner needed): the stages in DEPS. Until: days PENDING 'authority_pending:<word>'.
- RQ-17 FIB-13 (should): the state-only replay through the exam months. Until: D-6206 states the hybrid.
- RQ-18 R4-07 and R4-11: the floor reservation under the shared ingest lock. Until: the floor check runs alone, and the race is stated in docs/06-limits.md.
- RQ-19 R4-16's owner: re-specify class (a) against the charter's measured adjustment shapes before any use. FIB-15 never uses it to decide.

(13) BUILD
OWNED:
- crates/runner/src/fib/live.rs: FrozenSetup, the references, LiveState counters, the corrected-reading segment trees, the verdict with judgeability and stability; pure.
- crates/runner/src/fib/brake.rs: pure.
- crates/runner/tests/fib_live.rs
- crates/runner/benches/fib_rows/live.rs
- crates/cli/src/fib/live.rs: the verbs and the judge.
- crates/cli/src/fib/live_files.rs: the kinds and the view-type readers, built on FIB-8's sealing.
- crates/cli/src/fib/live_tests.rs
- crates/cli/benches/fib_rows/live.rs
- crates/api/src/fib_live_routes.rs, fib_live_routes_tests.rs
- web/src/routes/fib/live/+page.svelte, +page.js
- web/src/lib/fib/live.js, live.test.js
- The ledger rows.
OUTSIDE OWNERSHIP: one line each, listed as such in the report.
- crates/runner/src/fib/mod.rs: two mod lines.
- crates/cli/src/fib/mod.rs: the router arm 'live' and its mod lines.
- crates/runner/benches/fib.rs and crates/cli/benches/fib.rs: the #[path] row lines.
- crates/api/src/server.rs: route lines next to FIB-14's /fib routes.
- crates/api/src/lib.rs: one mod line.
- crates/api/src/schedule.rs: the 'fib_live' kind, after R4-12 merges.
- web/src/routes/fib/+page.svelte: one link to /fib/live.
- FIB-14's closing-rank source scan: the three file paths of this part.
Every RQ of section (12) is a requirement on its owner, not an edit by this part.

(14) EXTREME CASES
Every row of the matrix returned with this spec is binding. Each has its handling and its test.

(15) TESTS THAT MUST FAIL BEFORE
crates/runner/tests/fib_live.rs:
- frozen_setup_has_no_default_and_every_term_changes_the_id (golden hex)
- setup_id_excludes_host_clock_and_ledger_sequence
- vix_change_leaves_setup_and_day_ids_unchanged
- setup_id_differs_from_pair_identity_and_both_run_ids
- brake_day_strict_one_paisa (equal does not stop; one paisa below stops), at each size, side and pair
- brake_week_to_date_and_closed_week_like_for_like
- brake_drawdown_accumulates
- brake_one_size_only
- brake_long_side_stops_the_pair_naming_it_and_side_only_line_beside
- brake_any_reading_trips (in-force rates, mixed, corrected, possible halt)
- brake_anchored_decides_rolling_warns_tightest_line_shown
- false_alarm_chance_equals_m_over_n_plus_m
- runs_and_recovery_warn_never_stop
- counters_equal_recompute
- corrected_reading_insert_equals_recompute
- stopped_stays_stopped_without_unstop
- unstop_rearms_drawdown_since_unstop_and_headline_keeps_since_f0
- as_braked_includes_breach_day
- retroactive_breach_dated_at_discovery
- not_yet_judgeable_until_two_complete_quarters (F0 mid-quarter; a not-judged day inside a quarter)
- pro_rated_count_one_below_and_at_q (with a partial year and a calendar hole); fm_f076_d_and_all_days_reading
- three_columns_and_fib7_readings_carried (with the older-rule banner case)
- every_scheme_must_pass_and_failing_scheme_named
- regime_clause_not_proven_until_each_regime_seen
- verdict_needs_primary_and_corrected
- verdict_stability_and_first_pass_labelled
- daily_path_does_no_bootstrap_work
- references_equal_fib7_recompute_from_sealed_learning_rescore
- undefined_reference_uses_recorded_reading
- reference_version_on_rate_change
- looks_and_freezes_enter_trials_retries_add_0
- not_judged_share_outside_learned_coverage_warns
- median_order_statistic_equals_sort
- source scans: no typed numbers on the strategy path; gates 17 and 22; the closing-rank scan
crates/cli/src/fib/live_tests.rs, on a generated store of 3 stocks plus reference indices, with synthetic authority day files, a synthetic exam record, a deep run and a synthetic pinned binary:
- daily_judge_equals_batch_replay, over 3 synthetic months including a holiday, a Muhurat, a halt, a not-judged day of every row of 4.4, a basis change with its rebuild and a 360-minute day, given the exclusion set and rebuild events
- own_past_effect_per_reason (one case per row of 4.4)
- decision_at_bar_n_unchanged_when_later_bars_change
- eligibility_refusal_changes_are_labelled (perturbs later seconds and the DATA-1 and DATA-2 inputs)
- one_record_per_session_day_in_order_hash_chained
- ordinal_is_append_index_and_calendar_revision_does_not_renumber
- f0_first_tradable_session_after_freeze_and_newest_stored_session
- f0_after_newest_stored_session_even_with_clock_behind (host_clock_behind_store)
- bridge_days_commit_and_record_nothing; bridge_pending_blocks_f0
- every freeze refusal by name, and the idempotent second freeze
- scheduler_tick_freezes_once; new_commit_does_not_refreeze
- refreeze_of_stopped_pair_refused; successor_of_stopped_inherits_stopped; successor_of_retired_starts_proving; successor_labelled_everywhere
- freeze_from_exam_every_winner_canonical_order
- zero_winners_sentence
- operator_chosen_label_everywhere
- unmeasured_at_freeze_refused_without_recovery_rule
- one_overtaken_gdfl_day_then_trades_after_remeasure (with a synthetic RQ-2 rule)
- fully_unmeasured_pool_is_not_judged_never_zero_trade
- missing_zerodha_pending_then_overtaken_only_on_vendor_absence
- late_status_independent_of_judge_timing (landing D+1 before D inside one catch-up gives the same primary record as landing in order)
- pending_complete_day_never_overtaken (store unavailable for two sessions; both judged once it is back)
- window_end_is_pending_not_final; zerodha_window_closed_refuses_freeze
- gdfl_after_next_open_sets_brake_not_current
- d_plus_1_before_d_vendor_absent_then_late_reading
- failed_check_final_at_once
- refusal_census_reconciles
- never_filled_from_another_feed (GDFL-folded minutes present and Zerodha missing: stays pending, then late:zerodha)
- missing_vix_still_judges_the_day
- day_landed_through_pull_form_is_judged
- partial_zerodha_day_is_incomplete
- new_member_on_d_is_pulled_before_judging
- calendar_horizon_shows_brake_not_current
- muhurat_next_session_is_next_tradable (a synthetic Sunday Muhurat with no timings)
- operator_close_day_requires_pending_and_who_why
- redelivery_appends_restated_chain; restated_chain_equals_batch_on_corrected_inputs (a day restated 40 sessions back)
- learning_data_restated_shows_both_reference_sets_and_tighter_trips
- provisional_minute_warning_never_stops
- possible_halt_both_readings_and_restate_on_halt_row
- halt_entry_before_halt_kept_at_worst_seen (a 10:00 entry and an 11:40 halt)
- one_sided_book_fill_moves_to_next_second (a synthetic locked book)
- rate_selectors_explicit_replay_equals_daily_across_rate_change
- charges_not_rechecked_label_and_verdict_word
- charges_unchecked_refuses_freeze
- basis_raw_split_across_two_landings_without_attestation_refused
- basis_classes_same_ratio_affine_revised_differs
- basis_measured_step_without_nse_row_refuses_and_latches
- basis_differs_latches_until_rebuild (refused on D and D+1, admitted only after the rebuild seal)
- basis_rebuild_after_nse_row_equals_backtest_on_repulled_series
- pulled_late_adjusted_refused_in_catch_up (a synthetic ex-date inside a five-day catch-up)
- unknown_rename_refused_until_sourced; rekey_keeps_replay_green
- leaver_still_pulled; appending_entrant_leaves_records_byte_identical
- cas_era_label_and_previous_close_rule
- pinned_build_judges; frozen_build_digest_mismatch_pending
- commit_continuity_equal_appends; commit_continuity_different_restates_beside_never_retires; retired_only_when_pinned_build_cannot_run; commit_field_excluded_from_continuity
- sealing_a_month_leaves_verify_and_replay_green
- rerun_byte_identical_conflict_refused
- crash_between_trades_and_day_record_resumes_identically
- second_process_refused
- input_changed_during_run_pending_never_overtaken
- unstop_requires_stopped_and_who_why; unstop_in_operation_audit
- network_seam_untouched; no_credential_or_order_words; schedule_arm_names_no_credential_type
- no_output_names_an_unclosed_session
- writes_store_only_with_out_store
- rehearsal_refused_with_out_store_and_on_exam_months
- disk_floor_before_any_byte; disk_projection_refuses_freeze
- events_per_day_and_per_brake
- forced_exit_unpriceable_at_worst_seen
- zero_trade_day_counts
- pair_overlap_readings
- two_setups_sharing_a_second_equal_their_solo_records
- partial_forward_year_labelled
- exam_breaches_brake_reference_refused_or_cited
- learning_window_not_deep_refused
- prior_look_list_appended
- retries_add_no_looks
crates/api/src/fib_live_routes_tests.rs:
- NOT_FOUND, BAD_REQUEST and CONFLICT on each route;
- paging bounds, and limit above the measured bound refused;
- every_route_reads_bounded_records (three record sizes; O(limit));
- day_json_reads_corrected_reading_in_o1;
- os_user_never_served;
- no bare metric names and no inf or NaN;
- schema digest mismatch refused.
web/src/lib/fib/live.test.js:
- table builders; brake, warning, currency and status text; the not-yet-judgeable text; every label of 10.4 present; schema mismatch refused.

(16) BENCH
- runner: per trade fed (WORST-CASE); per day close with brake and verdict (AMORTISED, through RQ-7's append-a-day leave-one-out, period stated); corrected-reading insert (O(log n)); scheme warnings (O(family size)). Axis: judged days at three sizes spanning >= 100x. p99 and max ratio <= 1.25.
- cli: per-day judge per pool stock-day, BOUNDED by the day's minutes and touched seconds (declared) once RQ-10 lands, otherwise BOUNDED O(month-to-date bars), benched on the last day of a month; append (one small sealed file); late reading; restated chain per day (BOUNDED); replay per day (BOUNDED); freeze re-score (BOUNDED); luck (BOUNDED O(replicates x days)); verify in bytes per second.
- api: each route with n >= 200 GETs at three record sizes spanning >= 100x, ratio <= 1.25, at the measured limit bound, plus a cold first request, on a shadow server on your own port.
Every measured number goes in docs/06-limits.md.

(17) REAL DATA, READ-ONLY
A rehearsal into a NEW scratch root, with at most 4 workers.
1. Pick the pair: a pair from FIB-13's scratch outputs, or, if none passed, the nearest pair from FIB-7's closest-miss segment, labelled 'rehearsal: not a winner'.
2. Pseudo-freeze it at the start of the newest LEARNING month. Never an exam month.
3. Judge it day by day over that month with the real authorities, launched through a pinned copy of the release binary in the scratch root (measures whether a copied binary runs from that volume).
4. Prove daily == batch replay on real data. Run one restated chain over the month. Report the counts, the per-day cost, the bytes per day and per snapshot, and the projection to W0 (labelled EXTRAPOLATION).
5. Take screenshots of /fib/live on the shadow server at desktop and phone width.

(18) STORE WRITES NEEDED
For the main session, after the merge: one at a time, caffeinated, each with its measured time and disk projection.
a. Operator prerequisites: the Zerodha window end moved past 2026-09-30 (R4-12, a docs/05 entry); GDFL's daily delivery confirmed.
b. The ingest chain's per-day authority stages running daily (RQ-16), and the RQ items that the freeze refuses without.
c. After the exam record is finished: <TGT>/release/cli fib live freeze --from-exam <exam-record-id> --out-store, then fib live verify <id>.
d. The 'fib_live' kind rehearsed in config/store-steps.toml, and enabled only on the operator's yes.
e. Daily, through the schedule: the pinned build's fib live judge --out-store, which checks the chain incrementally. Full fib live verify on demand.

(19) LEDGER (docs/05-decisions.md)
- D-6200 what FIB-15 is; 'live' = forward and unseen, after the close, not real-time; real-time is UNVERIFIED and out of v1 (charter rows quoted as in 1.3) and what a later part needs
- D-6201 the frozen setup, the freeze key and the setup id (no host clock, ledger sequence, INDIA VIX or index terms); the surface list bytes
- D-6202 entry: exam winners once per exam record, operator-chosen pairs labelled, pair_already_frozen, the successor rule
- D-6203 F0 after the host clock and the newest stored session; bridge days; the starting state; unmeasured state
- D-6204 one record per session day; append-index ordinals; completeness from store receipts on any path; pending reasons; late = vendor-confirmed absence by ledger sequence; operator close-day; calendar revisions; the per-status own-past table
- D-6205 same code: daily == batch given the exclusion set and rebuild events; the pinned build; data refusals never zero-trade days; eligibility look-ahead labelled
- D-6206 commit continuity; restated-under-commit beside; retirement only when the pinned build cannot run; the hybrid limit
- D-6207 forward charges: today's rates on every date deciding, rates in force beside, explicit selectors, checked-through receipts
- D-6208 forward gate under every scheme and every NIFTY regime; the NIFTY month-close input; FM-F076 D unchanged
- D-6209 judgeability (two complete quarters from R6), the pro-rated count, stability and selected looks, the anytime-valid reading's construction, the O(1) daily figures and the luck verb
- D-6210 references from a deep re-score of the learning window, per scheme and version; stage-A and mixed beside; selection bias; CAS era; the exam check
- D-6211 the brake: clauses, readings, the anchored references deciding (PROPOSED), scheme warnings, false-alarm chance, per-side scope with the side-only line, provisional minute warning
- D-6212 un-stop: cli only, who and why, drawdown since un-stop, since-F0 headline, OS user stored not served
- D-6213 no execution, no credential, no network; the schedule arm's process boundary
- D-6214 brake currency, the next tradable session, the calendar horizon
- D-6215 files (one sealed file per append, shared snapshots, form-independent receipts, bound generations), routes (O(limit), bounded limit) and page
- D-6216 late readings, restated chains, restated learning data, re-deliveries not yet detected
- D-6217 the schedule kind, shipping off, launching pinned builds, freezing once
- D-6218 trials and looks, the prior-look list, concurrency not modelled
- D-6219 basis changes: the same-landing re-read, the ingest basis verdict, measured classes only refuse, the latch, rebuild after an nse row, renames and catch-up
INVARIANTS (docs/04-invariants.md), each beside its test:
- FFC-L01 a frozen setup never changes; any term change makes a new id; the id holds no host clock, ledger sequence, INDIA VIX or index term.
- FFC-L02 the freeze is idempotent by freeze key; a pair gets a second setup only through an operator successor, which inherits STOPPED.
- FFC-L03 one primary record per sourced session day from F0, in order, by append index, hash-chained; calendar revisions never move a record.
- FFC-L04 every judged session opened after the freeze and after the newest session stored at the freeze.
- FFC-L05 the daily judge equals the batch replay given the same exclusion set and rebuild events.
- FFC-L06 decisions read bars 0..N.
- FFC-L07 eligibility refusals use whole-day data, are labelled, and their would-have-traded outcomes enter the mixed reading.
- FFC-L08 'late' is computed from ledger sequences and needs vendor-confirmed absence; this Mac's own failures never overtake a day.
- FFC-L09 each status's own-past effect is the backtest's rule.
- FFC-L10 a not-judged day is never filled; readings never touch the primary record or its own past.
- FFC-L11 only the frozen Generation and the pinned build judge, or a commit with a continuity record.
- FFC-L12 the references equal FIB-7's recompute from the sealed learning re-score, per scheme and version.
- FFC-L13 the brake trips iff a clause breaches strictly on a checked reading against the deciding references, per size, side and pair.
- FFC-L14 the deciding brake is O(1) per day whatever the record length; a corrected insert is O(log n); a restated chain is BOUNDED and declared.
- FFC-L15 STOPPED persists until an UNSTOP record with who and why.
- FFC-L16 not yet judgeable before two complete calendar quarters.
- FFC-L17 every deciding number is net of the precise charges under an explicit rate selector.
- FFC-L18 no execution, no credential, no network.
- FFC-L19 routes read O(limit) bytes and O(1) per row, with a bounded limit.
- FFC-L20 reruns are idempotent and crash-safe; a conflict is refused.
- FFC-L21 the refusal census reconciles; a data refusal never counts as a judged zero-trade day.
- FFC-L22 a basis change is accepted only with an nse-sourced row and a sealed rebuild; every other outcome latches.
- FFC-L23 freezes and looks are counted as trials, retries add 0, and forward-judged months enter the prior-look list.
- FFC-L24 brake currency is shown, including the calendar horizon.
- FFC-L25 no typed numbers on the strategy path.
- FFC-L26 receipts are form-independent and every bound generation stays addressable.
- FFC-L27 INDIA VIX and the reference indices never enter an id, a digest or completeness.

(20) UNVERIFIED
See the list returned with this spec. In short: real-time feeds; Zerodha's post-close completeness, its stream equality and its adjustment timing; GDFL's daily delivery, retention and clock basis; the 2027 calendar; PIT-1 forward revisions; paper-fill realism, one-sided books, price bands and auction close-out; halt losses; concurrent setups; NSE corporate-action fetching and the adjusted base price; forward charge circulars and Dhan's dates; FIB-7's exact downside definitions; the self-declared operator identity; disk per day; the pinned binary on the store volume; exchangeability behind the false-alarm chance.

(21) DONE
- CLAUDE.md section 9 in your worktree: cargo fmt --check; clippy --workspace --all-targets -D warnings; test --workspace --locked, with the documented cli --skip; cargo deny check; 100% line and branch coverage of the new files; no surviving mutant in them.
- Every test in (15) fails before and passes after.
- The bench rows pass, or are declared BOUNDED in docs/06-limits.md.
- The source scans are green.
- The rehearsal in (17) shows daily == batch on real data.
- npm run build and npm test pass.
- The docs rows are present.
- Every RQ of section (12) that has not landed is listed with what FIB-15 does until it lands.
- Everything else is listed as NOT DONE.
```


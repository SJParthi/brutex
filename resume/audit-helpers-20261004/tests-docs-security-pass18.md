# tests-docs-security pass 18 (tag tds18): superseded decisions still cited as current

Checkout: /home/claude/wt/zero3, detached at 1f4de71. Audit only. Nothing was edited and no cargo was run; every finding below is from source.

## Counts

- New findings: **5**, all low. P18-01 to P18-05.
- Not re-reported: P12-06, P13-04, P17-*.
- Supersession links extracted from docs/05: 61 "supersede" lines plus "reverses", "withdraws" and "replaces" lines, giving about 45 superseded-to-successor pairs. Each superseded ID was grepped across crates/, web/src, docs/00-04, docs/06-11, CLAUDE.md, AGENTS.md and README.md.

## P18-01 low: docs/04 AS-11 and AS-14 still state D-0595's bracket rule as current, beside AS-12, which says D-0602 reversed it

- docs/04-invariants.md:3210 (AS-11, marked ✓): "D-0595 removed that objection — the threshold is the trade's own `best - worst` bracket — and D-0596 applied the same principle to a day." Its proof column lists "D-0595 (the trade)" as a closed finding.
- docs/04-invariants.md:3209 (AS-14, marked ✓): "Zero now means either "nothing won" or "nothing won by more than its own uncertainty"". Its proof is "the field's own doc comment".
- Contradiction: AS-12, three lines above at docs/04:3207, says "D-0602, REVERSING D-0595 ... `min_win` is `min(pess)` over trades with `pess > 0`, and nothing else". The field doc that AS-14 cites as proof (crates/cli/src/population_base_evidence_v2.rs:373-376) now says "Zero means only that no trade won". D-1763 rewrote that doc and did not touch AS-14.
- Code follows D-0602. crates/runner/src/grid.rs:4614 and population_base_evidence_v2.rs:1056 apply no bracket test.
- Why it matters: one table holds two rows with ✓ that contradict each other about the 3:1 ranking floor. AS-14's proof now says the opposite of the row. AS-11 also counts the trade finding as closed by D-0595, and D-0602 says that repair made every result flattered.
- Repro (not run): read docs/04:3207-3210 next to population_base_evidence_v2.rs:373.
- Fix: in AS-11, say the trade repair was D-0595 reversed by D-0602 (`min_win = min(pess>0)`), and that the day rule was corrected by V4/D-0605. Rewrite AS-14 to say zero means only that nothing won, or strike it, because `scratch_wins` has nothing left to separate.

## P18-02 low: the autopilot's compile-time grace floor and AU-01b still argue from D-0108's "boot flies" default, which D-0128 reversed

- crates/api/src/autopilot.rs:87-92: "The whole weight of "nothing is contacted before somebody could say no" now rests on that window, because D-0108 made the boot default FLY."
- docs/04-invariants.md:2221, section heading: "The autopilot flies on Run ... — D-0108".
- docs/04:2232 (AU-01b): "Under the old default a served `Control` came up paused ... flying by default moves that whole guarantee onto the countdown".
- Code follows D-0128 (docs/05:12288, "This reverses D-0108's default"). `stays_paused_from` (autopilot.rs:2025-2027) flies only on the exact `BRUTEX_AUTOPILOT=run`. AU-01, the first row under that heading (docs/04:2231), says so and calls it "the D-0128 reversal".
- So the heading and AU-01b present D-0108 as current, while AU-01 in the same table presents D-0128. Under the live default, the consent gate is the pause plus the explicit opt-in, not the countdown alone.
- Repro (not run): read docs/04:2221-2232 and autopilot.rs:87-92 next to autopilot.rs:2025.
- Fix: retitle the section, for example "a halt is probation only where it can be measured — D-0108, boot default reversed by D-0128". Reword AU-01b and the comment on the `GRACE_SECS` floor to "when the operator opts in with `run`, the countdown is the only gate". The `>= 5` floor can stay.

## P18-03 low: a test's doc and a vendor comment still state D-0055's "the month binds every rung" rule, which D-0320/D-1370 removed; the test asserts the opposite

- crates/api/src/server.rs:28685-28710 is the first of two `///` blocks on `each_rung_is_split_by_the_cap_its_own_vendor_published`. It reads: "A DAILY WINDOW IS SPLIT BY THE MONTH AND NOT BY THE ONE-MINUTE CAP ... Neither records a day-level figure, so neither descriptor carries one ... So the daily split must be the month alone. Groww at one minute needs **126** requests ... At the day rung it needs **80** ... D-0054 says "14 at day level", which ... is unreachable regardless."
- The second block on the same test (server.rs:28711-28723) says the month rule was removed by D-0320. The body asserts Groww `(Some(30), 81, Some(180), 14)` (server.rs:28745-28751): 81 minute requests and 14 daily ones, not 126 and 80.
- crates/pull/src/vendor.rs:4762-4764, on Groww's 180-day daily cap: "It changes no request today — `split_window` binds the MONTH at every rung and a month is never 180 days". `split_window` (crates/pull/src/session.rs:1229-1253, D-1370) ends a capped chunk at the cap, not at the month end. So the 180-day cap changes Groww's daily pass from 80 requests to 14.
- Chain: D-0054 said "14"; D-0055 (docs/05:4355) superseded that with a floor of 80 per month; D-0320 and D-1370 removed the month clamp; the descriptor gained the 180-day row. Both comments stop at D-0055.
- Repro (not run): read server.rs:28685-28751 and vendor.rs:4762-4766.
- Fix: delete the first doc block (28685-28710), or rewrite it to the cap-only rule. In vendor.rs, say the 180-day cap cuts Groww's daily pass to 14 requests per 80 months.

## P18-04 low: BT-26 states D-0410's "excursions unrecorded, commission 0.00%" model, which D-0414 superseded, and cites two tests that do not exist

- docs/04-invariants.md:3314 (BT-26, ✓): "statutory commission is `0.00%` for the spot-index scope; ... unrecorded per-trade price and excursion columns are omitted". Its proofs include `*known commission is shown as zero and its unmodeled spread stays visible*` and `*the trade ledger contains only durable per-trade fields*`.
- D-0414 (docs/05:28756) "supersedes ... D-0410/BT-26's statement that per-trade excursions are unrecorded".
- web/tests/backtest-ui.test.js:109-135 (`the chosen-grid trade ledger exposes every durable row fact ...`) now requires the `MAE` and `MFE` headings and `t.adverse_paisa` / `t.favourable_paisa`.
- web/tests/backtest-ui.test.js:98-107 (`commission load comes from the run's charge scope ...`) asserts `doesNotMatch(breakdown, /0\.00%/)` and reads the charge scope, which includes RELIANCE.
- `grep -rn` over web/tests finds neither cited test title. They were renamed, and the row still names the old titles and the old behaviour.
- Repro (ran, grep only): `grep -rln "known commission is shown as zero\|trade ledger contains only durable" web/tests` returns nothing.
- Fix: rewrite BT-26 to say MAE and MFE are shown from the D-0414 receipt, prices and exit cause are omitted, and commission load comes from the run's charge scope. Cite the two current test titles.

## P18-05 low: the stored tiers and Total Market targets are served as "stored, never swept", and docs/04 says "none of them is swept — D-0105"; since D-0506 their F&O members are swept

- crates/api/src/ingest.rs:410-419, `SpotTarget::note()`: Equities is "stored, never swept"; Nifty500/200/100/50 are "the published N constituents — stored, never swept". `/universes.json` serves these strings to the ingest form (crates/api/src/coverage.rs:485). Tests pin them at ingest.rs:2606-2631 and :2646-2665.
- docs/04-invariants.md:2258, section heading: "The four NIFTY tiers are requestable and none of them is swept — D-0105".
- Code follows D-0506 and D-0682. ST-02, the second row under that same heading (docs/04:2271), says "Every member of the four NIFTY tiers is sweepable ... exactly when it belongs to the existing F&O cash snapshot (D-0506/D-0508)". `a_nifty_tier_target_stores_and_sweeps_exactly_its_fno_members` (ingest.rs:2840-2856) asserts `key.is_sweepable() == FNO_INDEX.contains(name)`. RELIANCE is in all four tiers and is swept. Total Market (`Equities`) contains every one of the 208 swept shares.
- The file's own comment on the `Fno` note (ingest.rs:421-428) rejects this kind of wording: ""Stored, never swept" is true of the SET but reads as 'this set excludes the swept pair', which is false". The tier notes do exactly that. An operator reading "NIFTY 50 equities — stored, never swept" is told RELIANCE, HDFCBANK and the others are not swept, and they are.
- Same-theme count drift: ingest.rs:430, the `Fno` note, says "the 213 F&O underlyings — stored, and swept as cash equities since D-0506". D-0682 (docs/05:37754) made the cash half 208 shares, and FINNIFTY, MIDCPNIFTY and NIFTYNXT50 are swept in neither shape.
- Repro (not run): `SpotTarget::Nifty50.note()` against `equity("RELIANCE").is_sweepable()`, which ingest.rs:2850 already asserts true.
- Fix: word the tier and Total Market notes "stored; members that are F&O shares are swept (D-0506)". In the `Fno` note, say "208 F&O shares swept as cash equities, plus NIFTY and BANKNIFTY as indices (D-0682)". Retitle docs/04:2258 to match ST-02, then update the pinned strings and the "never swept" loop's exemption.

## Also seen (not counted as findings)

- crates/indicators/src/column.rs:492, :892-905, :957 and :2297-2300 still say `align::onto_execution` "returns the first execution bar stamped at or after the signal's close". They also say "`align`'s own test asserts `[Some(10), Some(10)]`" and "A hole of one bar is enough to collide two 2-minute signals". D-0401 (docs/05:28220) superseded D-0293's at-or-after rule. runner/src/align.rs now maps only an exact close instant, and no `[Some(10), Some(10)]` assertion exists in align.rs. The duplicate-fill-bar branch is therefore unreachable from the exact aligner. This is the same theme as the findings above. It is listed here because it is comment-only and the code is safe.
- crates/cli/src/pool.rs:52-55: "D-0506 records the cost model as necessary before any stock result is acted on". D-0681 superseded D-0506's wording, but "acted on" is compatible with D-0681's execution refusal, and D-0681 is cited two lines above.

## CLAUDE.md citation check (task 4)

CLAUDE.md at 1f4de71 cites D-0013, D-0017, D-0052, D-0053, D-0169, D-0206, D-0208, D-0210, D-0212, D-0217, D-0225, D-0226, D-0288, D-0453, D-0506, D-0507, D-0509, D-0525, D-0681, D-0682, D-0683, D-1440, D-1448 and D-1764.

| Citation | Verdict | Evidence |
|---|---|---|
| D-0506 (widening) | CURRENT, with its successors cited | §1 also cites D-0681 (cost wording supersession) and D-0682 (208 count) |
| D-0017 | CURRENT | later mentions (D-0248, D-0506:32411, D-0955) restate it, none narrows it |
| D-0052/D-0053 | CURRENT | D-0058 and D-0174 cite them, neither amends them; `.claude/launch.json` is the only tracked `.claude/` file |
| D-0210 | CURRENT | no later amendment |
| D-0225 | CURRENT | D-0436:29278 says "does not gain a tenth term" |
| D-0212 | CURRENT | `PastPrefix` is still without a production caller (D-0520 era, docs/05:50391) |
| D-0507 | CURRENT | rule 7 now carries it (P1-15-01 fix); stored.rs:2063-2068 agrees |
| D-0453 | CURRENT, as corrected | D-0683 corrects its list, and CLAUDE.md cites both |
| D-0013 | CURRENT | D-1374 confirms `totp.rs` exchanges no token |
| D-0169, D-0206, D-0208, D-0217, D-0226, D-0288, D-0683, D-1440, D-1448, D-1764 | CURRENT | no later supersede/withdraw/reverse line names them |

No CLAUDE.md citation was found to be superseded without its successor beside it.

## Superseded pairs checked and clean at 1f4de71

D-0002/D-0005 to D-0031; D-0024 to D-0025; D-0035 to D-0036/D-0040; D-0046 to D-0692/D-0921; D-0076 to D-0079; D-0086 to D-0087; D-0095 to D-0108; D-0108/D-0124 recovery to D-0473; D-0170 to D-0193; D-0181/D-0395/D-0397/D-0427 to D-0421/D-0436; D-0182/D-0433 to D-0687 (AF-07); D-0293 to D-0401 (code clean; comments noted above); D-0309 to D-0310; D-0345 to D-0675; D-0348 to D-0432; D-0404/D-0410/D-0411 to D-0414 (except BT-26, P18-04); D-0464 to D-0470; D-0477 to D-0478; D-0523 to D-0524; D-0595 to D-0602 (except AS-11/AS-14, P18-01); D-0629 to D-0654; D-0679 to D-0689; D-0688 to D-0910 (docs/06:9188 carries "Widened by D-0910"); "near-only" rule to D-0231; D-0015 to D-0802 (fill-pricing only; D-0802 itself defers the UE-row rewrite to the fill-engine entry, so the UE rows standing is per its instruction and is not a contradiction).

## Verification tally

No verification rows were assigned to this pass. Earlier IDs P12-06, P13-04 and P17-* were excluded and not re-checked.

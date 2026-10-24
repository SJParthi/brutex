# CLOUD-CHG1-OPT: complete specification for a cloud build (24 Oct 2026)

Planning material, never merged. Written on the operator's Mac from the build launcher and its
tables so that a cloud session that cannot read any Mac file builds exactly what the Mac builder of
this part would build. Read `reports/db-specs-20261024/README.md` (the cloud rules) and `CLAUDE.md`
first; where this file quotes a Mac path, the README's "What differs in the cloud" applies.

Scope note: this is brutex only. The part touches `crates/costs` of this repository and nothing of
any other project.

**Read section 4 ("What already landed at e851da1ca") before you start.** The package text in
section 2 was written on 20 Oct against `fix/db-CLOUD-CHG1` at `e6d05d708`, before CLOUD-CHG1's
repair rounds and before CHG-0 merged. At the head of `feat/db-fix` (`e851da1ca`) item 1 is already
settled, item 2 is mostly done, item 3 is open and item 4 is a check. The launcher's own rule
applies: where the specification states a fact about merged code that is false at the head, the code
at the head is the fact; build to the specification's intent and say so in your report.

---

## 1. Header

| Field | Value |
|---|---|
| Name | `CLOUD-CHG1-OPT` |
| Title | Charges, options remainder of CHG-1: the FA73061 dated-regime option golden test settled once CHG-0 lands, one dated IPFT authority, and the 2026-03-01 boundary pinned at the stack |
| Kind | `nw` (new work; not an `ia` ingest-automation package, so the design `ingest-automation.md`, its section 16.2 row and the round-7 critic do not apply) |
| Source of the package | `__ARGS.extraPackages[]` entry `CLOUD-CHG1-OPT` (launcher line 6; same JSON in `state/cloud-20261024/args.json` `.extraPackages`), origin `state/options-20261020/decisions.json` (verified decision for CLOUD-CHG1, 20 Oct 2026 options replan) |
| Priority rank | 89 of 137 in `__ARGS.priority` (0-based index 88), between `R4-13` and `IA-2` |
| Critical path | **No** (not among the 24 names of `__ARGS.criticalPath`) |
| In this launch | Yes: listed in `__ARGS.only` (index 82). Not in `skipFix`, `verified`, `pendingRepair`, `parked`, `doneExtra`; no `roundBase` entry. `noCritic` and `noGate` are both `true` for the whole run (booleans, not lists: no design critic and no final workspace gate step in this launch, so the part's own gates in section 7 are the only gates it gets before merge). |
| Package deps | `CHG-0`, `CLOUD-CHG1` |
| `__ARGS.extraDeps["CLOUD-CHG1-OPT"]` | none |
| `__ARGS.dropDeps["CLOUD-CHG1-OPT"]` | none |
| `__ARGS.parked` | empty, removes nothing |
| **PREREQUISITES** (deps + extraDeps - dropDeps) | `CHG-0`, `CLOUD-CHG1` |
| FILES_ONLY links (`__ARGS.filesOnly["CLOUD-CHG1-OPT"]`) | none ("merge first, do not block the start": nothing to list) |
| Parts that wait for this one | `WP-99` (the ledger close) names `CLOUD-CHG1-OPT` in `__ARGS.extraDeps["WP-99"]` and in its specAppend ("every <ID>-OPT package ... wait for them and close the ledgers for stocks, indices and options together") |
| Decision block | `D-6740..D-6759` (section 5) |
| Mac-only facts (for the merge-back) | shadow port 8400 + 97 = **8497**; shared build pool `tgt/pool-5`; effort ladder: fix step at `high` (score 1: spec longer than 1,500 characters; not on the path, not `ia`, under 5 deps; the `-OPT` suffix is not discounted) |

### Prerequisites, checked

Checked with `git -C /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/wt/DBFIX log feat/db-fix --first-parent --merges --format=%s`
at `feat/db-fix` = `e851da1ca`, and against `__ARGS.doneExtra`:

| Prerequisite | State | Evidence |
|---|---|---|
| `CHG-0` | **merged** | `baa74072d Merge CHG-0: NSE index-options exchange charge 0.03553% from 2026-03-01 as a NEW dated regime row (append-only)` (2026-10-23 11:33 IST); also in `__ARGS.doneExtra` |
| `CLOUD-CHG1` | **merged** | `7fe3b6cc8 Merge CLOUD-CHG1: Merge back the cloud charge calculator (CHG-1): precise NSE/SEBI stack + Dhan calculator mode, 49 golden cases` (2026-10-22 13:09 IST); also in `__ARGS.doneExtra` |
| `CLOUD-CHG1-OPT` itself | not merged, never started | no `Merge CLOUD-CHG1-OPT:` subject; no branch `fix/db-CLOUD-CHG1-OPT`; no worktree `wt/P-CLOUD-CHG1-OPT`; no `state/fix/round2/fix-CLOUD-CHG1-OPT*.json` or progress file; no `D-674x`/`D-675x` id on `feat/db-fix` or any `fix/*` / `origin/cloud/*` branch |

Both prerequisites are merged: the part may start.

---

## 2. The package text

Title (from `__ARGS.extraPackages[]`, name `CLOUD-CHG1-OPT`):

```text
Charges, options remainder of CHG-1: the FA73061 dated-regime option golden test settled once CHG-0 lands, one dated IPFT authority, and the 2026-03-01 boundary pinned at the stack
```

Spec, in full and verbatim (`state/cloud-20261024/args.json` `.extraPackages[] | select(.name == "CLOUD-CHG1-OPT") | .spec`; the same object is in the launcher `state/fix/db-fix-round-2-run0738.js.fat` line 6, `__ARGS.extraPackages`; `decisionBlock` `D-6740..D-6759`; `source` `state/options-20261020/decisions.json (verified decision for CLOUD-CHG1, 20 Oct 2026 options replan)`). Paths in it are Mac paths; where it names a Mac file, the content is copied in sections 3 and 4:

```text
DECISION BLOCK (fixed by the 20 Oct 2026 options replan and recorded in state/decision-blocks.json before launch): D-6740..D-6759. Use only ids from this block, in order; this line overrides any "take the next free block" instruction below.

DECISION IDS: only the fixed block above. Before using an id, grep docs/05-decisions.md on feat/db-fix and every fix/* and origin/cloud/* branch; never reuse an id. Every locked choice of this package is one docs/05-decisions.md entry from the fixed block above, in order; nothing is renumbered (append-only). Read CLAUDE.md in wt/DBFIX first: it is law. Work on feat/db-fix in your own worktree (branch fix/db-CLOUD-CHG1-OPT). Start only after BOTH have merged: CLOUD-CHG1 (the merged-back cloud CHG-1 charge stack, fix/db-CLOUD-CHG1 e6d05d708, D-5110..D-5130) and CHG-0 (the NSE index-options exchange charge FA73061 as a new dated regime row from 2026-03-01, D-5100..D-5109; WIP on fix/db-CHG-0 469869254 and b836c174f, which may change before it merges: read what merged, not the WIP). WHY: the operator's 20 Oct decision (stocks, indices and options combined) puts the F&O half of the charges back in scope. The 18 Oct split deferred 'Futures and options segments and the CHG-0-dependent golden case' and 'F&O rows and the one ignored CHG-0 test' (state/split-20261018/final.json .mixed_splits[20] and [21], .cut_links[13]). CLOUD-CHG1 did build the F&O rows, at e6d05d708: crates/costs/src/schedule.rs NSE_FUTURES_EXCHANGE :274, NSE_OPTIONS_EXCHANGE :285, STT_FUTURES :359, STT_OPTIONS :370, STAMP_FUTURES :409 and STAMP_OPTIONS :418, with the FUT reading and the 11 option readings in crates/costs/tests/dhan_calculator.rs priced line by line. CLOUD-CHG1's own spec un-ignores the CHG-0 test only 'if CHG-0 has merged locally', and CHG-0 was parked, so without this package nobody settles it. ITEMS: [1] (D-5115; cloud report reports/cloud-chg1-20261016.md section 6) Settle the_dated_regime_and_todays_table_agree_on_the_live_october_2026_option_cases (dhan_calculator.rs:891-893, #[ignore = "needs CHG-0's FA73061 regime row"]). It prices the 11 live October option readings under RateSelector::Dated and RateSelector::CurrentOnEveryDate on 2026-10-15, 2026-10-16 and 2026-03-01, and asserts that every line and the total are equal. RED BEFORE CHG-0, already measured on the Mac (state/fix/round2/verify-CLOUD-CHG1-r1-logs/test-costs-ignored.log:5-14, 20 Oct 10:10 IST): it panics at dhan_calculator.rs:910 on 2026-10-15 with the exchange line Paisa(501) dated against Paisa(508) today, GST 810 against 812. Re-run it with --ignored on CHG-0's merge first parent and paste the assertion into notes. EXPECTED AFTER CHG-0, BY READING THE CODE (not run; UNVERIFIED until you run it): 2026-10-15 and 2026-10-16 agree (regime 3553 from 2026-03-01 with IPFT 0, and STT 0.15% from 2026-04-01 on both sides). The third date, 2026-03-01, is expected to still fail on the STT line. On that day the dated regime's options STT row is the OCT_2024 row, 0.10% (crates/costs/src/regime.rs STT_OPTIONS), while today's table is 0.15% from 2026-04-01 (schedule.rs STT_OPTIONS, NSE/FATAX/73524, sources row S-03): Rs 8 against Rs 12 on the NIFTY 25300 CE reading (stack.rs:2204 pins Rs 8 under Dated on 2026-02-27). Run it on the tree with CLOUD-CHG1 and CHG-0 merged, remove the ignore, and paste the result. If 2026-03-01 fails by the STT line as read here, the test's claim is false for that date, not the rates. Record a decision entry and replace that date's single equality with exact assertions: the exchange line equals today's from 2026-03-01, and the STT line is the dated 0.10% row against today's 0.15% until 2026-03-31. Keep full line and total equality on 2026-10-15 and 2026-10-16. Never change a rate, a regime row or a golden value to make it pass, and never delete the test (CHG-1 spec: never tune the precise mode to match Dhan). If CHG-0's merge already un-ignored and settled it green on feat/db-fix, record that run and do not redo it. [2] ONE DATED IPFT AUTHORITY. After CHG-0 merges, two rules decide the NSE options IPFT for a date. One is CLOUD-CHG1's crates/costs/src/schedule.rs dated_option_card (e6d05d708 :994-1005): 0 from its own FA73061_EFFECTIVE (:251), else crate::rate::NSE_IPFT, summed into the one exchange rate. The other is CHG-0's crate::regime::ipft_rate (fix/db-CHG-0 regime.rs:725): a dated table, refused before 2023-04-01, Rs 50 per crore from 2023-04-01 to 2026-02-28, and zero from 2026-03-01, where FA73061's Rs 0.01 is folded into the Rs 3,553 row NSE_EXCHANGE_CHARGE_FA73061. CHG-0 also removes rate::ipft. Make dated_option_card read regime::ipft_rate and the regime's own boundary, so the date rule exists once; FA73061_EFFECTIVE either goes or is pinned equal to the regime's boundary by a test. Today's table (schedule.rs NSE_OPTIONS_EXCHANGE, FA73061) is unchanged. No priced figure changes: before 2024-10-01 the exchange charge's refusal row refuses first. FAILS BEFORE: a source-shape test the_dated_option_card_reads_the_ipft_from_the_regime_only (red while schedule.rs keeps its own IPFT match). Add a value test that the card's IPFT equals regime::ipft_rate on 2023-04-01, 2026-02-27 and 2026-03-02. [3] THE BOUNDARY AT THE STACK. Write a new test beside stack.rs:2204, a_dated_option_exchange_line_is_fa64232_plus_ipft_on_2026_02_27_and_fa73061_alone_on_2026_03_02, under RateSelector::Dated. On 2026-02-27 the exchange rate is 3503 (FA64232) + 50 (IPFT). On 2026-03-02 it is 3553 + 0; that is the first session, since 2026-03-01 is a Sunday (CHG-0's MAR_2026 doc). The total is Rs 3,553 per crore on both days, equal to today's NSE_OPTIONS_EXCHANGE, and each line's source string names its own row. The totals already agree before the boundary; what the test pins is which rows make them. Run it red on CHG-0's merge first parent (2026-03-02 reads 3503 + 0 there) and paste the assertion, then green. CHG-0's trip-level tests (fix/db-CHG-0 crates/costs/tests/fa73061.rs:102, :119, :139, :155) pin the regime rows; this test pins the stack's dated card. If CHG-0's merge already landed an identical stack-level test, cite it and add none. [4] KEPT REFUSALS, verified unchanged. Physical-delivery settlement of a stock option stays refused by name (schedule.rs:149 PHYSICAL_DELIVERY_REFUSAL; Settlement::PhysicalDelivery pinned at stack.rs:1957-1983 and ledger.rs:771-772). In state/fno-winners-20261013/sources-charges.json, B-06 (Dhan 0.10% of contract value at settlement) has effective_from UNVERIFIED. S-06 (STT on the delivery leg) is UNVERIFIED in every field: rate, base, side and effective date. Its what_would_settle_it names the NSE Clearing physical-settlement circular and the Finance (No.2) Act 2004 section 98 table (CLAUDE.md section 3 rule 1, no invention). It stays listed as not priced in docs/06-limits.md section CHG-1 and is never marked done. BSE futures and BSE F&O in calculator mode stay refused, because BSE is outside the engine surface (CLAUDE.md section 1). If the operator later charters B-06's effective date and S-06's rate, pricing them is a new package, not this one. OWNED FILES: crates/costs/tests/dhan_calculator.rs (item 1); crates/costs/src/schedule.rs, only dated_option_card and FA73061_EFFECTIVE (item 2); the test module of crates/costs/src/stack.rs (item 3). SEQUENCED EDITS: crates/costs/src/regime.rs only after CHG-0 merges (CHG-0 owns it), one named function, and only if item 2 needs a public accessor for the boundary. LEDGERS: docs/00-charter.md (cite the FA73061 row as CHG-0 recorded it; no new rate); docs/04-invariants.md (a row beside each new or changed test); docs/05-decisions.md (this block: the one IPFT authority, and item 1's corrected 2026-03-01 assertion if the run shows it); docs/06-limits.md section CHG-1 (a new row supersedes the 'stays #[ignore] until package CHG-0 merges' line, which is never edited; the 1-paisa calculator total rule stays UNVERIFIED as measured). TESTS THAT MUST FAIL BEFORE: items 1, 2 and 3 as stated, each run red with the failing assertion pasted into notes, then green. REGRESSION, stays green unchanged: every_calculator_line_matches_the_page_on_every_golden_case (dhan_calculator.rs:559), the_one_paisa_total_misses_are_listed_with_their_direction (:672), the_five_contract_note_vectors_rederive_from_the_rules (stack.rs:1637), a_dated_option_day_before_fa73061_prices_at_the_regime_plus_ipft (stack.rs:2204), the physical-delivery refusal tests above, and CHG-0's crates/costs/tests/fa73061.rs. FAILURE-MATRIX ROWS: none. No row of state/fix/e2e-failure-matrix.json names CHG-0, CHG-1 or CLOUD-CHG1 as owner; the findings closed are D-5115's ignored test and the wave-2 pieces of mixed_splits[20] and [21]. BENCH: no money path changes cost class. Run cargo bench -p costs --locked --bench stack; C-CHG-01..08 p50/p99/max must stay within noise of the CLOUD-CHG1 figures in docs/06-limits.md section CHG-1, recorded with date and host. DONE when: cargo fmt --all --check, cargo clippy --workspace --all-targets -- -D warnings, cargo test --workspace --locked (run as the operator, not root) and cargo deny check are green; line and branch coverage is 100% and no mutant survives on the touched files (cargo mutants --file on each); money is paisa i64 with exact integer rates and never a float; Rust only, never the Py-language or its name; failures reported plainly.
```

`__ARGS.specAppend["CLOUD-CHG1-OPT"]`: **none** (no key). No later amendment (options, one-command cut, no-fresh, ledger follow-ups, R4-09b, priorities) was appended to this part: it was created by the 20 Oct options replan, which wrote its scope into the spec itself. The replan note `__ARGS._note_options_20261020` lists it among the new `-OPT` packages; `WP-99`'s specAppend makes the ledger close wait for it. The package is not in `EMBED.packages`.

An earlier draft of this spec (`state/options-20261020/decisions.json`, `opt_package.spec`) began "DECISION BLOCK: take the next free block from state/decision-blocks.json"; the launched text above replaced that with the fixed block. The replan record for CLOUD-CHG1 that motivated the part said at the time: `options_half_status` "partly_built"; worktree `wt/P-CLOUD-CHG1` on `fix/db-CLOUD-CHG1` one commit ahead (`e6d05d708`, 20 Oct 01:28 IST), "F&O rows built; the CHG-0 test is still #[ignore]". Section 4 says what changed after that record.

---

## 3. What the Mac builder of this part reads, and what each source holds for it

The Mac builder is started with `fixPrompt(p)` (launcher lines 351-372, copied whole in Appendix B).
For a part whose kind is not `ia`, `specStep(p)` (launcher lines 341-349) sends it to the fix-plan
entry, the findings it lists, and the package spec (section 2 above). Every source it names was
read for this part on 24 Oct 2026:

| Source the prompt names | What it holds for CLOUD-CHG1-OPT |
|---|---|
| `EMBED.context` (`C`, launcher line 10) | The shared background every part reads first. Copied whole in Appendix A (one e-mail address redacted for the public repository). |
| `fixPrompt` NIFTY FIRST paragraph (`args.niftyFirst` is unset, so it is included) | Applies in its intent only: this part changes no route, page or import, so there is no NIFTY selection to prove. Its "fix in the SHARED, instrument-agnostic code path" clause binds: the dated card is one path for every NSE option (index and stock), never a NIFTY special case. |
| `fixPrompt` RULES AUDIT: `grep -n '^## .*CLOUD-CHG1-OPT' state/design/rules-audit-20261011.md` | **No section.** The audit (11 Oct) predates the part; no `CHG` owner appears in it at all. The standing rules still bind (README). |
| `specStep` (non-ia): `jq '[.work_packages[] \| select((.name \| split(" ")[0]) == "CLOUD-CHG1-OPT")][0]' state/board/fix-plan.json` | **No entry** (`null`). No entry for `CHG-0`, `CHG-1` or `CLOUD-CHG1` either. |
| `specStep`: findings in `state/board/audit-result.json` (key `confirmed`) | **None listed** (no fix-plan entry, so no finding ids). The findings this part closes are named in its spec: D-5115's once-ignored test and the wave-2 pieces of the 18 Oct split (copied below). |
| `specStep`: "D-number assignments are in WP-02's decisions_needed" | Not used: the spec's first line fixes the block `D-6740..D-6759` and overrides any other id source. |
| `specStep`: fix-plan `summary` / `all_sources_view` "if your package touches them" | Not touched by this part. |
| e2e failure matrix `state/fix/e2e-failure-matrix.json` (`.rows`, owner = the part) | **No rows.** No row names `CLOUD-CHG1-OPT`, `CHG-0`, `CHG-1` or `CLOUD-CHG1` as owner (`CLOUD-CHG1` is mentioned only inside 24 rows owned by `PREC-15`, all carrying the same sentence that lists it among the parts PREC-15 follows; re-checked 24 Oct). The spec says the same. |
| design `ingest-automation.md` 16.2/16.3, critic round 7 | Not applicable (kind `nw`); neither names the part. |
| 16 Oct cloud spec `origin/cloud/db-base-20261016:reports/db-specs-20261016/CLOUD-CHG1-OPT.md` | **Does not exist** (the part was created on 20 Oct). |
| `fixPrompt`: an earlier attempt (`wt/P-CLOUD-CHG1-OPT`, `state/fix/round2/fix-CLOUD-CHG1-OPT.json`, `verify-CLOUD-CHG1-OPT-r<n>.json`, `fix-CLOUD-CHG1-OPT-progress.md`) | **None exist.** But the second check of CHG-0 left one follow-up that names this part; it is copied in section 4 and is part of this package. |

### 3.1 The pieces the spec cites, copied

**The 18 Oct split entries the spec closes** (`state/split-20261018/final.json`, `.mixed_splits[20]`,
`.mixed_splits[21]`, `.cut_links[13]`), verbatim:

```text
{"id":"CHG-1","wave1_piece":"Equity intraday, delivery and MTF segments: precise and Dhan-calculator modes, rate-set selector, SEBI fee, STT, stamp, GST, rounding, 15:10 square-off line, 5 equity contract-note vectors","wave2_piece":"Futures and options segments and the CHG-0-dependent golden case"}
{"id":"CLOUD-CHG1","wave1_piece":"Equity stack in stack.rs/dhan_calc.rs/schedule.rs and the cost_invariants bench fix","wave2_piece":"F&O rows and the one ignored CHG-0 test"}
{"from":"CHG-0","to":"CHG-1","what_the_dependent_must_do":"Land the equity segments now from fix/db-CLOUD-CHG1. The F&O segments and the CHG-0 golden case wait."}
```

**The cloud CHG-1 report, section 6** (`reports/cloud-chg1-20261016.md` on `origin/cloud/chg1-20261016`,
lines 415-442), verbatim:

```text
## 6. What is NOT DONE, or waits on someone else

- **The CHG-0 dependency.** `the_dated_regime_and_todays_table_agree_on_the_live_october_2026_option_cases`
  is `#[ignore = "needs CHG-0's FA73061 regime row"]`.
  - It runs the 11 live October option readings under the dated selector and
    under today's table.
  - Today it fails by exactly the IPFT: ₹5.01 against ₹5.08 on the NIFTY
    25300 CE reading.
  - The dated NSE options line is the regime row plus `NSE_IPFT` before
    2026-03-01, and the regime row alone from that date. **Check at merge:**
    if CHG-0's row is 0.03553% with the IPFT folded in, the test passes once
    un-ignored. If CHG-0 keeps the IPFT separate, it fails and says so.
- **Partly squared delivery days are refused.** These are days with unequal
  buys and sells on both sides; splitting them needs the FIFO order sequence.
- **Not priced at all:**
  - MTF interest;
  - stock F&O physical settlement;
  - BSE futures, under both modes;
  - BSE F&O in calculator mode;
  - exchange or stamp charges on an option exercise (only the exercise STT is
    levied);
  - call-and-trade fees.
- **Workspace-wide gates** were run (§4). What fails is either base's (the
  `cargo deny` advisories, gates 1d, 10, 19, 21 and 23, five tests) or the
  container running as root (19 permission tests). The `cargo test` run as a
  non-root user is NOT DONE.
- **One golden reading is missing.** The brief's 49th was not in the data.

```

**The red run the spec quotes** (`state/fix/round2/verify-CLOUD-CHG1-r1-logs/test-costs-ignored.log`
lines 5-14, Mac, 20 Oct 2026 10:10 IST, on `e6d05d708`), described in words: running
`the_dated_regime_and_todays_table_agree_on_the_live_october_2026_option_cases` with `--ignored`
failed at `crates/costs/tests/dhan_calculator.rs:910` on the first date, 2026-10-15, on the first
option reading (the NIFTY 25300 call, one lot of 65, premium 100 to 120). The dated line list read
exchange transaction charge (incl. IPFT) `Paisa(501)` and GST `Paisa(810)`; today's table read
`Paisa(508)` and `Paisa(812)`. Brokerage 4000, SEBI 1 and STT 1200 paisa agreed. The 7-paisa gap was
the dropped IPFT. (This is a test's own output over a calculator reading, not a vendor data row.)

**The charge-source rows the spec cites** (`state/fno-winners-20261013/sources-charges.json`), in words:

| Row | What it says | Status |
|---|---|---|
| `E-03` | NSE equity options (stock and index), exchange transaction charge: Rs 3,552.99 per crore of premium each side (0.0355299%); with the IPFT leg the NSE total is Rs 3,553 per crore (0.03553%); circular `FA73061`, effective 2026-03-01 (verified), retrieved 2026-10-13. Prior: Rs 3,503 + Rs 50 IPFT = Rs 3,553 from 2024-10-01 (`FA64232`). | verified |
| `S-03` | STT on the sale of an option: 0.15% of premium, seller pays, from 2026-04-01 (verified; NSE `FATAX73524`, Finance Act 2026). Prior rates: 0.10% from 2024-10-01 to 2026-03-31; 0.0625% from 2023-04-01 to 2024-09-30; 0.05% from 2016-06-01 to 2023-03-31. | verified |
| `B-06` | Dhan brokerage on NSE stock F&O that ends in physical delivery: 0.10% of contract value plus the exchange charges, at settlement. `effective_from` is **UNVERIFIED**. | rate read, date UNVERIFIED |
| `S-06` | STT on the delivery leg of physically settled stock derivatives: **UNVERIFIED in every field** (rate, base, side, effective date). `what_would_settle_it`: the NSE Clearing physical-settlement circular on STT and the Finance (No.2) Act 2004 section 98 table as amended. | UNVERIFIED |

---

## 4. What already landed at e851da1ca, item by item (read before building)

Three commits changed the ground under the 20 Oct spec:

- `618924f75` (20 Oct, CLOUD-CHG1 repair 1, **D-5131**): the dated options card adds the IPFT on
  **every** day as `costs::trip` does; the golden test lost its `#[ignore]`. Merged with
  `7fe3b6cc8 Merge CLOUD-CHG1`.
- `683525c15` (CLOUD-CHG1 repair round 2, D-5132): option exercise brokerage (not this part's subject).
- CHG-0 (`baa74072d Merge CHG-0`, **D-5100, D-5101**): the FA73061 row (Rs 3,553 with the IPFT folded
  in) from 2026-03-01, a dated NSE IPFT table (`regime::ipft_rate`), `rate::ipft` removed, and the
  dated card switched to `crate::regime::ipft_rate(exchange, day)?`.

CHG-0's first parent is `0dc8361c3 Merge IA-9` (D-5131 already in it). **There the dated card read
`crate::rate::ipft(exchange)` (Rs 50, undated) beside the old regime row: on 2026-03-02 it read
3503 + 50, not "3503 + 0" as the spec says** (that was `e6d05d708`, before D-5131). The red runs the
spec asks for must be taken where they are honestly red; see each item.

### Item [1]: the golden test. SETTLED at the head; record it, do not redo it

At `e851da1ca`, `crates/costs/tests/dhan_calculator.rs:893-939`
`the_dated_regime_and_todays_table_agree_on_the_live_october_2026_option_cases` has **no**
`#[ignore]`. On 2026-10-15 and 2026-10-16 it asserts full line and total equality; on 2026-03-01 it
asserts every line except index 3 (STT) equal, and the STT exactly: dated
`(sell_value + 500) / 1_000` (0.10%), today `(sell_value * 3 + 1_000) / 2_000` (0.15%). That is the
corrected assertion item 1 asked for. CHG-0's builder ran it and it passed on all three dates
unchanged (`state/fix/round2/fix-CHG-0.json`: `cargo test -p costs --locked` lib 292, adversarial 4,
dhan_calculator 12, fa73061 11 + 1 ignored timing, doctests 55; D-5101 item 3). Invariant `CHG0-09`
cites it.

What item 1 still asks of you:
1. Run it on the head and paste the result line into your report (the spec: "If CHG-0's merge
   already un-ignored and settled it green on feat/db-fix, record that run and do not redo it").
2. **The follow-up CHG-0's second check left for this part** (`state/fix/round2/verify-CHG-0-r1.json`,
   verdict `pass_with_followups`), verbatim:

```text
CLOUD-CHG1-OPT: crates/costs/tests/dhan_calculator.rs:885-892 doc comment still says the dated card is 'FA64232's 0.03503% plus the 0.0005% IPFT' on and after 2026-03-01; since CHG-0 it is the folded 3,553 row plus a zero IPFT. Under mutation M6 the test still passes, because it compares totals only; the split is guarded by fa73061.rs
```

   Correct that doc comment (dhan_calculator.rs:885-892 at the head) to the folded row, and either
   make the test see the split or cite where the split is guarded (`tests/fa73061.rs`). Item 3's
   stack test is the natural place to pin the split.
3. The spec's "the STT line is the dated 0.10% row against today's 0.15% until 2026-03-31": the head
   asserts it on 2026-03-01 only. A pin on 2026-03-31 (still 0.10% dated) and 2026-04-01 (both 0.15%)
   is in the spirit of the item; it is optional, not a gap the head has.
4. A decision entry for item 1 is needed only if you change the test; D-5101 item 3 already records
   the settled assertion.

**Other follow-ups near this part that are NOT its work** (re-checked 24 Oct, so a builder does not
widen the scope by accident; §3 rule 2 of `CLAUDE.md`):
- `verify-CHG-0-r1.json` has seven more follow-ups. They name `CHG-0`, `CHG-1`, `costs` or the main
  session, never this part: a resolved `docs/00-charter.md` merge note; `docs/06-limits.md:1186-1192`
  still calling FA56129 and FA61137 "never officially retrieved"; `docs/06-limits.md` section 27's
  "proves the port agrees" sentence next to the two one-paisa fold differences; the fold's one-paisa
  under-run at legs of Rs 50 lakh and above (already stated and tested); `MAX_LATER_ROWS = 2` now full
  on the NSE exchange-charge table (the next NSE options circular needs the table type widened, not
  a row: do not widen it here); section 25's stale heading; and, builder-reported, two pre-existing
  gate-10 misses plus a duplicate `D-6247` heading for the decision-numbers gate. None is in this
  part's owned files or ledger rows.
- `verify-CLOUD-CHG1-r1.json`'s follow-up "when CHG-0 merges, bump `DATED_RATES_VERSION` to
  `dated-regime/r2`, re-pin `DATED_FINGERPRINT` and un-ignore" the golden test is already done at the
  head (section 8).
- `verify-CLOUD-CHG1-r3.json` (verdict `pass_with_followups`) leaves one disclosed options item open
  under `CHG-1`: Dhan's Rs 20 on an option that expires or is assigned, and the SEBI fee on an
  exercised or assigned option's notional (sources row F-02), are not priced and such positions are
  refused by `NetTrade`. This part's spec does not name it; leave it refused and listed as it is.

### Item [2]: one dated IPFT authority. MOSTLY DONE; three pieces open (2a, 2b, 2c)

Done at the head (D-5101 item 2): `crates/costs/src/schedule.rs:1055 fn dated_option_card` reads
`let ipft_bps = crate::regime::ipft_rate(exchange, day)?.get();` (`:1059`) and adds it to
`exchange_charge_rate(exchange, day)?`; there is no IPFT match of its own and `rate::ipft` no longer
exists. Its NSE source line is the constant `REGIME_EXCHANGE_NSE` (`:987`). Value pins already in
`schedule.rs` tests: `the_dated_option_card_is_the_regime_on_the_trade_day_plus_the_ipft_on_every_day`
(`:1527`; pins (3503, 50) on 2026-02-28 and (3553, 0) on `FA73061_EFFECTIVE`, 2026-10-15 and
`CURRENT_AS_OF`) and `the_dated_option_card_is_the_trip_stacks_exchange_charge_plus_ipft_on_every_day`
(`:1612`; every day 2018-01-01..2027-12-31 at both venues, priced 1,187 x 2 days, refused 2,465 x 2).

Open:
- **(2a) `FA73061_EFFECTIVE` is a second copy of the boundary.** `schedule.rs:249`
  `pub const FA73061_EFFECTIVE: TradeDay = boundary!(2026 - 3 - 1);` is still there, with a stale
  doc comment ("folded into the dated options row package CHG-0 adds"). Outside its own tests nothing
  in `crates/` reads it (`:1324`, `:1326` pin it to the literal `day(2026, 3, 1)`; `:1571` iterates it).
  The regime's own boundary is `regime.rs:346 const MAR_2026` (private; no accessor). No test pins
  the two equal. Per the spec, either remove `FA73061_EFFECTIVE` (it is `pub`; check `web/` and every
  crate for users first; at the head there are none outside `schedule.rs`) or pin it to the regime's
  boundary. A pin needs no `regime.rs` edit if it is behavioural: `ipft_rate(Nse, FA73061_EFFECTIVE)`
  is 0 and the day before is 50, and `exchange_charge_rate(Nse, ...)` moves 3503 -> 3553 exactly there.
  A `pub(crate)` accessor in `regime.rs` is the spec's SEQUENCED EDIT (one named function), allowed
  only if item 2 needs it. Record the choice as a decision from your block.
- **(2b) The source-shape test does not exist.** `the_dated_option_card_reads_the_ipft_from_the_regime_only`
  is absent. The spec wanted it "red while schedule.rs keeps its own IPFT match", but at the head the
  match is already gone, so on the head it is green from its first run. Say so honestly: show the red
  against the old shape (for example run the assertion over the `e6d05d708` text of
  `dated_option_card`, kept as a test-only string, or over CHG-0's first parent `0dc8361c3`, where the
  card called `crate::rate::ipft(exchange)`), then green on the head. The repository's existing
  source-shape style is `include_str!` (`crates/costs/src/scope.rs:232`). The test should refuse any
  IPFT figure in `dated_option_card` other than `crate::regime::ipft_rate`.
- **(2c) The value test on 2023-04-01, 2026-02-27 and 2026-03-02.** Not present for those three
  dates. **Code fact that changes the 2023-04-01 case:** `RateCard::resolve(Options, Nse, 2023-04-01,
  Dated)` refuses (the NSE exchange-charge table's refusal row covers every day before 2024-10-01 and
  is reached first), while `regime::ipft_rate(Nse, 2023-04-01)` answers 50. The card's IPFT cannot be
  read on that day; assert that the card refuses with the exchange-charge refusal and that the IPFT
  table alone answers 50 (invariant `CHG0-04`'s "no priced trip meets that refusal"). On 2026-02-27
  the card is 3503 + 50 and on 2026-03-02 3553 + 0, 3553 in both cases. The refusal half already has
  a model at the head: `schedule.rs:1657
  the_dated_option_card_refuses_the_pre_2024_window_with_the_regimes_own_refusal` asserts, at both
  venues on 2024-09-30, that `RateCard::resolve(Options, exchange, day, Dated)` returns exactly
  `ChargeError::Cost(CostError::Unverified(regime))`, where `regime` is
  `exchange_charge_rate(exchange, day)`'s own refusal. Assert the 2023-04-01 refusal the same way
  (exact equality, not "is an error") and leave that test unchanged.

### Item [3]: the boundary at the stack. OPEN

At the head the only stack-level dated option test is
`crates/costs/src/stack.rs:2419 a_dated_option_day_before_fa73061_prices_at_the_regime_plus_ipft`
(the spec's `:2204`). It opens `ClientSecurityDay` for options, NSE, 2026-02-27, `RateSelector::Dated`,
buys 65 at Rs 100 and sells 65 at Rs 120, and asserts the basis text `rates in force on 2026-02-27`,
the exchange line Rs 5.08 and STT Rs 8 (0.10% of Rs 7,800 = 7.80, rounded to Rs 8). No test pins
2026-03-02 at the stack. CHG-0's trip-level tests are at `crates/costs/tests/fa73061.rs:107`
(`a_trade_dated_2026_02_27_is_priced_at_the_fa64232_row_and_its_50_rupee_ipft`), `:124`
(`a_trade_dated_2026_03_02_is_priced_at_the_fa73061_row`), `:144`
(`the_boundary_day_2026_03_01_is_priced_at_the_fa73061_row`) and `:160`
(`a_trip_straddling_2026_03_01_prices_each_leg_at_its_own_days_row`); the spec's `:102 :119 :139 :155`.

Two code facts the new test must face:
- **The stack sees one summed rate.** `ClientSecurityDay` carries one line, "exchange transaction
  charge (incl. IPFT)", priced from the card's single `Ratio::new(base + ipft, RATE_SCALE)`. The
  "3503 + 50 versus 3553 + 0" split is visible only through `regime::exchange_charge_rate` and
  `regime::ipft_rate` on the same day; the line value is Rs 5.08 on both days for the same orders.
  The test pins the line and total on both days and the split beside them.
- **"Each line's source string names its own row" is false at the head.** The dated NSE card's
  exchange source is the one constant `REGIME_EXCHANGE_NSE` on both sides of the date (it names
  FA64232 + the Rs 50 IPFT to 2026-02-28 and FA73061 from 2026-03-01 in one string). Either make the
  source date-specific inside `dated_option_card` (owned by item 2; a change to a `Cited` source moves
  no figure; the `dated-regime/r2` fingerprint of
  `the_rate_set_versions_are_pinned_to_the_tables_they_name` folds only each card's rate numerators
  and denominators, `schedule.rs:1733 fn fold_card`, not its source strings, so it does not move) or
  assert that the shared string names both rows and record why in a decision. **Two code facts bound
  that choice.** `schedule.rs:1552` (inside the existing value test at `:1527`) asserts
  `before.exchange_charge().source() == REGIME_EXCHANGE_NSE` on 2026-02-28, and `:1598` pins the BSE
  card's source to `REGIME_EXCHANGE_BSE`. And `REGIME_EXCHANGE_NSE` itself (`:987`) is not among this
  part's owned items in `schedule.rs` (only `dated_option_card`, `FA73061_EFFECTIVE` and new tests
  are). So a date-specific source must leave the pre-boundary source equal to `REGIME_EXCHANGE_NSE`,
  so that `:1552` stays true unedited (for example a second source chosen inside `dated_option_card`
  from the boundary day on). Rewording `REGIME_EXCHANGE_NSE` or editing `:1552` is an edit outside
  ownership: keep it minimal and list it in `files_outside_ownership`. Never delete either assertion.

The red run: the spec says "Run it red on CHG-0's merge first parent (2026-03-02 reads 3503 + 0
there)". At `0dc8361c3` it reads **3503 + 50** (D-5131 was in), so a test that pins the split
(3553, 0) on 2026-03-02 is red there with (3503, 50), and the line value is equal. Paste whatever
the red assertion really is. If you cannot build that commit, say NOT PROVED for the red half.

### Item [4]: kept refusals. CHECK ONLY, unchanged at the head

- `schedule.rs:147 PHYSICAL_DELIVERY_REFUSAL` (spec `:149`), with `Settlement::PhysicalDelivery` at
  `schedule.rs:142`; refused at `stack.rs:369-370`; pinned by
  `stack.rs:2132 a_stock_option_settled_by_physical_delivery_is_refused_by_name_and_changes_nothing`
  (spec `:1957-1983`) and `ledger.rs:1009-1010` (spec `:771-772`). Its reason text names rows B-06
  and S-06.
- BSE futures (`charge_error.rs:365`, `schedule.rs:799`) and BSE futures and options in calculator
  mode (`dhan_calc.rs:238`, `ledger.rs:1134`, `:1608`) stay refused.
- `docs/06-limits.md` section CHG-1 keeps B-06's date and S-06 as not priced; never mark them done.

### The ledgers at the head

- `docs/06-limits.md:20015` `## CHG-1. What the contract-note charge stack does not know ... D-5110 to D-5132`.
  Its subsection at `:20092` (copied below) already records D-5131 and "Done by CHG-0 (D-5100,
  D-5101)". **The line the spec wants superseded ("stays #[ignore] until package CHG-0 merges")
  is no longer in the file**: no `docs/` line at the head says the test stays ignored. Add a new row
  only for what this part changes; never edit an old one.
- `docs/04-invariants.md:8513-8521` holds `CHG0-01`..`CHG0-09` (CHG0-09 copied below).
- `docs/05-decisions.md`: `D-5115` at `:73784`, `D-5131` at `:74141`, `D-5100` at `:81528`,
  `D-5101` at `:81614` (copied below). The highest id at the end of the file is `D-5189`; ids are not
  in file order, so grep before every use.
- `docs/00-charter.md:1374` (the options charge row naming `NSE/FA/73061`, row E-03) and `:1687`
  `## NSE charge circulars for options premium ... (D-5100)`: cite these; add no rate.

**D-5101, the CHG-0 completion entry** (`docs/05-decisions.md` at `e851da1ca`, lines 81614-81679), verbatim:

```text
### D-5101 — CHG-0 completed on 22 Oct 2026: the extremes of the `FA73061` row proved, the undated `rate::ipft` stays removed, and the charge stack merged before it reads the one dated IPFT (CHG-0) — 2026-10-22

**Why a second entry.** D-5100 was written on 16 Oct 2026 and its tests were
never run before a usage limit cut the package off. Run on 21 Oct against
`feat/db-fix` at `5ff4956cc`, one of them failed: `tests/adversarial.rs` had
gained the `FA73061` constant as an eighth rate while still asserting
`9 notionals x 7 rates = 63 cells`. The count now says 72 and the "every
notional up to a crore is priced" floor 56. On 22 Oct the cloud charge stack
(CLOUD-CHG1, `Merge CLOUD-CHG1:` at `7fe3b6cc8`) landed on `feat/db-fix`
before this package, which is what items 2 and 3 are about. Nothing in
D-5100's rates or rows changed.

1. **The extremes, each a test** (`crates/costs/tests/fa73061.rs`,
   invariants CHG0-06 to CHG0-08): every month and year boundary from
   2023-03-31 to 2026-04-01 and the last representable day; the 2026-04-01
   STT change, across which the exchange charge and IPFT hold while the STT
   moves; dates that do not exist (2026-02-29 and the rest), refused before
   any rate is looked up; the fold's bound from one paisa to `i64::MAX`, with
   its one real under-run of the half-up split found at exactly ₹50 lakh a
   leg (177,650 paisa folded against 177,651 split); and 14 threads asking
   about the same 563 days at once, getting the single-threaded answers.
2. **`rate::ipft(exchange)` stays removed, and the one caller the merge
   brought reads the dated IPFT instead.** Nothing outside `crates/costs`
   called it. CLOUD-CHG1's dated option card (`costs::schedule`,
   `dated_option_card`) called it once, and beside the ₹3,553 row that line
   would have charged 0.03603% from 2026-03-01; with the function gone it no
   longer compiles, which is the point. It now reads
   `crate::regime::ipft_rate(exchange, day)?`, the one dated IPFT authority
   `trip::Rates::resolve` reads too, and the card's NSE and BSE source lines
   name that function. The card is ₹3,503 + ₹50 to 2026-02-28 and ₹3,553 + a
   folded zero from 2026-03-01: the same 0.03553% on every day from
   2024-10-01, so every figure the card carries is unchanged and
   `dated-regime/r2` keeps its pinned fingerprint
   (`costs::schedule::tests::the_rate_set_versions_are_pinned_to_the_tables_they_name`
   passes without a re-pin). The card's test now pins the rows it adds —
   (3,503, 50) on 2026-02-28 and (3,553, 0) on 2026-03-01, 2026-10-15 and
   today — where it had pinned 3,503 after 2026-03-01 with a message naming
   this package as the one that moves it. Re-adding an undated `ipft` is
   refused by this entry.
3. **The CLOUD-CHG1 golden test runs unchanged.**
   `costs::dhan_calculator::the_dated_regime_and_todays_table_agree_on_the_live_october_2026_option_cases`
   was un-ignored by D-5131 on CLOUD-CHG1's own branch, so there is no
   `#[ignore]` left to lift and `crates/costs/tests/dhan_calculator.rs` is not
   edited. It prices the eleven live October option readings under the dated
   selector and under today's table on 2026-10-15, 2026-10-16 and 2026-03-01.
   With this change it passes on all three, as written: on the two October
   dates every line and the total agree; on 2026-03-01 every line but STT
   agrees and STT differs exactly by the 0.10% against 0.15% rate the test
   asserts (the dated STT row of 2026-04-01 is a month later). Nothing was
   forced green; the result is in the CHG-0 report
   (`state/fix/round2/fix-CHG-0.json`).
4. **Timing**, measured (not a gate) by the ignored
   `the_dated_lookups_cost_the_same_on_every_row_measured`, in release,
   2,000 timed batches of 1,000 calls each per figure, on 22 Oct 2026 on the
   M4 Pro while other builds shared the machine. Each sample is a batch's
   mean per call (one lookup is shorter than the clock's tick), so p99 and max
   are of batch means, not of single calls. Across the five days measured
   (a 2020 refusal, 2024-10-01, 2026-02-27, 2026-03-01, 2100-12-31):
   `exchange_charge_rate` p50 2 ns, p99 2 to 8 ns, max 16 to 52 ns;
   `ipft_rate` p50 2 to 3 ns, p99 2 to 4 ns, max 18 to 28 ns;
   `Rates::resolve` p50 6 to 8 ns, p99 6 to 21 ns, max 17 to 54 ns; the dated
   `RateCard::resolve` p50 9 to 15 ns, p99 14 to 19 ns, max 23 to 68 ns; one
   `price` of a 2026-03-02 NIFTY round trip p50 54 ns, p99 63 ns, max 167 ns.
   The same on the refused, the first and the last row: each lookup is the
   fixed two-slot table walk it was, and the new IPFT table is one more of
   the same shape, so nothing here scans.
```

**Invariant CHG0-09** (`docs/04-invariants.md:8521` at `e851da1ca`), verbatim:

```text
| CHG0-09 | **The dated charge card and the round trip read one dated IPFT, and the fold moves no figure of the card.** Under `RateSelector::Dated` an NSE options card is `regime::exchange_charge_rate` plus `regime::ipft_rate` on the trade day, the two functions `trip::Rates::resolve` reads: ₹3,503 + ₹50 on 2026-02-28, ₹3,553 + 0 on 2026-03-01, 2026-10-15 and today, and 0.03553% in all on every one of those days; on every day from 2018-01-01 to 2027-12-31 at both venues the card and the round trip price together or refuse together, and where they price the card is the trip's exchange rate plus its IPFT; the `dated-regime/r2` fingerprint is unchanged by the 2026-03-01 rows; and the eleven live October option readings priced under the dated selector equal today's table on 2026-10-15 and 2026-10-16, and on 2026-03-01 on every line but STT (D-5101) | `costs::schedule::tests::the_dated_option_card_is_the_regime_on_the_trade_day_plus_the_ipft_on_every_day`; `costs::schedule::tests::the_dated_option_card_is_the_trip_stacks_exchange_charge_plus_ipft_on_every_day`; `costs::schedule::tests::the_rate_set_versions_are_pinned_to_the_tables_they_name`; `the_dated_regime_and_todays_table_agree_on_the_live_october_2026_option_cases` | ✓ |
```

**`docs/06-limits.md` lines 20092-20131 at `e851da1ca`** (the CHG-1 subsection on the dated options
exchange charge), verbatim:

```text
### The dated options exchange charge is the trip stack's, and how CHG-0 must merge (D-5131)

Under `RateSelector::Dated`, an NSE options day is priced at the regime row in
force **plus the IPFT**, read exactly where `costs::trip::Rates::resolve` reads
them, on every day: FA64232's 0.03503% plus the 0.0005% IPFT = 0.03553%, which
on and after 2026-03-01 is the total `FA73061` states (₹3,552.99 + ₹0.01 per
crore, sources row `E-03`). Until D-5131 the card dropped the IPFT by date from
2026-03-01, expecting CHG-0's FA73061 row; that row is not in `regime.rs`, so
every dated NSE options day from 2026-03-01 was priced at 0.03503% — ₹5.01
against ₹5.08 on the NIFTY 25300 CE reading — labelled as cited, and the test
that showed it was `#[ignore]`d. It now runs in the gate
(`costs::dhan_calculator::the_dated_regime_and_todays_table_agree_on_the_live_october_2026_option_cases`),
and `costs::schedule::the_dated_option_card_is_the_trip_stacks_exchange_charge_plus_ipft_on_every_day`
holds the card to `trip` on every day from 2018 to 2027 at both venues.

**What CHG-0's merge must do.** CHG-0 (branch `fix/db-CHG-0`, not merged when
this was written) adds a 0.03553% NSE row from 2026-03-01 and a dated NSE IPFT
that is zero from the same day, and reads the IPFT through
`regime::ipft_rate(exchange, day)` in `trip`. The dated card must then read
the IPFT from the same function: if it kept `rate::ipft` beside CHG-0's row it
would charge 0.03603% from 2026-03-01, and the trip-agreement test, the
October golden test and the r2 fingerprint each fail on that merge. Read where
`trip` reads it, the card's figures — and the `dated-regime/r2` fingerprint —
do not change at that merge, because CHG-0 re-splits the same total.

**The IPFT figure before 2026-03-01 is the one `trip` has always carried.**
`rate::NSE_IPFT` (₹50 per crore) comes from `COSTS_VERIFIED` §5 and its circular
citation is UNVERIFIED (D-1779); the dated card labels the NSE exchange line
cited, as it did before this repair, on the strength of FA64232's row and that
figure. CHG-0's branch reads `FA56129` for it; until that merges, the
pre-2026-03-01 IPFT part is the predecessor's figure, not a circular read here.

**Done by CHG-0 (D-5100, D-5101).** Both paragraphs above are history from
the merge of `fix/db-CHG-0` on: the dated card reads
`regime::ipft_rate(exchange, day)`, the regime holds `FA73061`'s ₹3,553 row and
the folded zero IPFT from 2026-03-01, and the ₹50 before it is the 2023-04-01
row of the dated IPFT table, cited to `FA56129` in `docs/00-charter.md`. The
card's figures and the `dated-regime/r2` fingerprint did not change, and the
October golden test and the trip-agreement test pass unchanged (CHG0-09).

```

**The bench figures to stay within noise of** (same section, `### Measured cost, and what the figures are not`, lines 20236-20271 at the head, in
short): cloud container 16 Oct, 20,000 batches of 16 calls: one executed order p50 26.8-27.3 ns,
p99 28.5-40.7 ns; one client-security-day p50 201.9-202.6 ns, p99 337.8-415.6 ns; one calculator quote
p50 47.8-48.3 ns. Operator's Mac 19 Oct, three runs: order `add` p50 10.3-10.4 ns, p99 13.0 ns;
ledger `add` p50 5.1-5.2 ns, p99 7.8-13.0 ns; a client-security-day p50 122.3-125.0 ns, p99
135.4-153.6 ns; gated ratios C-CHG-01..08 between 0.86x and 1.07x (20 Oct, workspace bench profile:
C-CHG-01..09 0.93-1.09x). Only the ratios are gated; absolute figures belong to their machine.

---

## 5. Files this part owns, and files it must not touch

From the spec's OWNED FILES, SEQUENCED EDITS and LEDGERS clauses:

| File | Ownership | What for |
|---|---|---|
| `crates/costs/tests/dhan_calculator.rs` | owned | item 1: the run record, the stale doc comment at `:885-892`, any optional STT pin |
| `crates/costs/src/schedule.rs` | owned **only** `fn dated_option_card` (`:1055`), `FA73061_EFFECTIVE` (`:249`, with its doc and its test uses at `:1324`, `:1326`, `:1571`), and new tests in its `mod tests` for item 2 | item 2 |
| `crates/costs/src/stack.rs` | owned **only** its test module (`#[cfg(test)] mod tests`, from `:1721`) | item 3's new test beside `:2419` |
| `crates/costs/src/regime.rs` | SEQUENCED EDIT (CHG-0 owns it and has merged): at most **one named function**, and only if item 2 needs an accessor for the boundary (`MAR_2026`, `:346`) | item 2, optional |
| `docs/00-charter.md` | ledger | cite the FA73061 row as CHG-0 recorded it (`:1374`, `:1687`); **no new rate** |
| `docs/04-invariants.md` | ledger | one row beside each new or changed test (next to `CHG0-01`..`CHG0-09`, `:8513-8521`) |
| `docs/05-decisions.md` | ledger | entries from `D-6740..D-6759` only: the one IPFT authority (FA73061_EFFECTIVE's fate), item 3's source-string choice, item 1 only if changed |
| `docs/06-limits.md` | ledger, section CHG-1 (`:20015`) | a new row for what changes; never edit an old row; the 1-paisa calculator total rule stays UNVERIFIED as measured |

**Must not touch:** every other file. In particular the rest of `schedule.rs` (today's table:
`NSE_OPTIONS_EXCHANGE` `:283`, `STT_OPTIONS` `:368`, every `Cited` rate, `current_card`, the
versions and fingerprints), the non-test body of `stack.rs`, `regime.rs` beyond one accessor (no
rate, no row, no table), `rate.rs`, `trip.rs`, `ledger.rs`, `dhan_calc.rs`, `crates/costs/tests/fa73061.rs`
(CHG-0's; regression only), `crates/costs/benches/*` (run only), every other crate, `web/`. Never
change a rate, a regime row or a golden value to make a test pass; never delete a test; never tune
the precise mode to match Dhan.

## 6. Decision-number block

`state/decision-blocks.json`, `.blocks[]`: `{"block": "D-6740..D-6759", "owner": "CLOUD-CHG1-OPT (options replan, 20 Oct)"}`.
Use only these, in order, starting at `D-6740`. Checked 24 Oct: no `D-6740`..`D-6759` appears in
`docs/05-decisions.md` on `feat/db-fix` or on any of the 312 `fix/*` and `origin/cloud/*` refs
(274 local `fix/*` + 38 `origin/cloud/*`); an independent re-check the same day also covered the 123
`origin/fix/*` refs (258 distinct blobs of the file in all): no hit.
The README's `D-TBD-<NAME>-n` rule does not apply: this part has an assigned block.

## 7. NEEDS THE MAC (run at merge-back)

This part changes no route, page, cli verb, import or store file. At the head nothing outside
`crates/costs` calls `costs::schedule`, `costs::stack`, `costs::regime`, `costs::dhan_calc` or
`costs::ledger` (outside the crate only `costs::{day, expiry, fill, lot, moneyness, rate, scope,
strike, trip, venue}` are named, and `runner/src/trade.rs` asserts that `costs::trip::price` is never
called). So there is **no shadow-server, real-data, store-write or :8080 check**, as CHG-0's builder
also recorded ("NOT PROVED on a server, proved by tests"). The Mac runs these at merge-back, in a
worktree of the cloud branch merged onto the newest `feat/db-fix`, holding the shared-pool lock as
the launcher's step 2 describes (`mkdir tgt/pool-5.lock`, `git ls-files -z | xargs -0 touch`, the
cargo sequence, then `rmdir`), with `export PATH=/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/bin:$PATH`
and `export CARGO_TARGET_DIR=/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/tgt/pool-5`:

```text
cargo fmt --all --check                                   # expect exit 0
cargo clippy --workspace --all-targets -- -D warnings     # expect exit 0
cargo test -p costs --locked                              # expect every suite ok; the new tests listed by name
cargo test -p costs --locked --test dhan_calculator -- the_dated_regime_and_todays_table_agree_on_the_live_october_2026_option_cases --exact
                                                          # expect: 1 passed (item 1, on all three dates)
cargo test -p costs --locked --lib -- schedule::tests::the_dated_option_card stack::tests::a_dated_option
                                                          # expect the old and the new item 2 / item 3 tests all pass
cargo test -p costs --locked --test fa73061               # expect 11 passed, 1 ignored (the timing test), unchanged
cargo test --workspace --locked -- --skip the_pool_verb_prints_its_whole_page_on_a_generated_store
                                                          # as the operator, not root; the skipped test is the known slow one
cargo deny check                                          # expect green, as on feat/db-fix since 17 Oct
cargo bench -p costs --locked --bench stack               # C-CHG-01..09 ratios within noise of 0.86-1.09x; record date, host, load
cargo llvm-cov -p costs --branch --locked                 # 100% line and branch on the touched files
cargo mutants -p costs --file crates/costs/src/schedule.rs --file crates/costs/src/stack.rs --file crates/costs/tests/dhan_calculator.rs
                                                          # no surviving mutant on the touched code (add regime.rs if it changed)
```

The red runs, if the cloud could not take them: check out CHG-0's merge first parent `0dc8361c3`
(`feat/db-fix` before `baa74072d`), add only the new item 2 / item 3 tests, and run them: expect
item 3's split pin to fail on 2026-03-02 with (3503, 50) against (3553, 0), and the source-shape test
to fail on the `crate::rate::ipft(exchange)` call. Record each failing assertion, then the green run
on the merged tree.

Expected answers, from the code at the head: NSE dated card 3553 on 2026-02-27 (3503 + 50) and on
2026-03-02 (3553 + 0); `ClientSecurityDay` options NSE Dated, buy 65 at Rs 100 and sell 65 at Rs 120:
exchange line Rs 5.08 on both days, STT Rs 8 (0.10%) on both days (0.15% only from 2026-04-01);
`RateCard::resolve(Options, Nse, 2023-04-01, Dated)` refuses while `ipft_rate(Nse, 2023-04-01)` is 50.

## 8. Code facts at e851da1ca

Checked with `git show e851da1ca:<file>` and `git grep` (not guessed). Line numbers are at the head;
the spec's numbers were at `e6d05d708` and have moved.

| File | Exists | Key items at the head |
|---|---|---|
| `crates/costs/tests/dhan_calculator.rs` (1,216 lines) | yes | `every_calculator_line_matches_the_page_on_every_golden_case` `:559`; `every_calculator_total_is_within_one_paisa_of_the_page` `:659`; `the_one_paisa_total_misses_are_listed_with_their_direction` `:672`; `precise_total` `:700`; `the_precise_and_calculator_modes_differ_where_the_rules_differ_and_agree_where_they_do_not` `:865`; stale doc comment `:885-892`; `#[test]` `:893`, `the_dated_regime_and_todays_table_agree_on_the_live_october_2026_option_cases` `:894-939`, **no `#[ignore]`**; `every_golden_round_trip_is_netted_of_its_entire_precise_charges` `:1108`; `every_golden_case_closes_in_the_day_ledger_exactly_as_both_modes_price_it` `:1177` |
| `crates/costs/src/schedule.rs` (1,829 lines) | yes | `use crate::regime::{exchange_charge_rate, stt_exercise_rate, stt_options_rate}` `:42`; `Settlement::PhysicalDelivery` `:142`; `PHYSICAL_DELIVERY_REFUSAL` `:147`; `CURRENT_AS_OF` `:244` (2026-10-16); `FA73061_EFFECTIVE` `:249`; `NSE_OPTIONS_EXCHANGE` `:283` (355,300 per 100 crore); `STT_OPTIONS` `:368`; `TODAYS_RATES_VERSION` `:599` `todays-rates/2026-10-16/r1`; `DATED_RATES_VERSION` `:608` `dated-regime/r2`; `PRECISE_VERSION` `:653` `precise/r2`; `current_card` `:926`; `REGIME_ROW` `:983`; `REGIME_EXCHANGE_NSE` `:987`; `REGIME_EXCHANGE_BSE` `:995`; `dated_card` `:1011`; `dated_option_card` `:1055` (reads `crate::regime::ipft_rate` at `:1059`); tests from `:1110`: `the_current_option_taxes_are_the_regimes_rows_on_the_as_of_day` `:1312`, `the_dated_option_card_is_the_regime_on_the_trade_day_plus_the_ipft_on_every_day` `:1527`, `the_dated_option_card_is_the_trip_stacks_exchange_charge_plus_ipft_on_every_day` `:1612`, `the_dated_option_card_refuses_the_pre_2024_window_with_the_regimes_own_refusal` `:1657` (exact refusal equality at `:1664`); source pins `:1552` (`REGIME_EXCHANGE_NSE`, 2026-02-28) and `:1598` (`REGIME_EXCHANGE_BSE`); `fold_card` `:1733` (hashes rate numerators and denominators only), `dated_fingerprint` `:1775`, `the_rate_set_versions_are_pinned_to_the_tables_they_name` `:1793`, `DATED_FINGERPRINT` `11_353_183_212_436_303_133`. **Absent:** `the_dated_option_card_reads_the_ipft_from_the_regime_only` |
| `crates/costs/src/stack.rs` (2,961 lines) | yes | `PhysicalDelivery` refusal `:369-370`; `#[cfg(test)] mod tests` `:1721`/`:1730` (helpers `day`, `rs`, `buy`, `sell`); `the_five_contract_note_vectors_rederive_from_the_rules` `:1808`; `a_stock_option_settled_by_physical_delivery_is_refused_by_name_and_changes_nothing` `:2132`; `a_dated_option_day_before_fa73061_prices_at_the_regime_plus_ipft` `:2419` (2026-02-27 only). **Absent:** `a_dated_option_exchange_line_is_fa64232_plus_ipft_on_2026_02_27_and_fa73061_alone_on_2026_03_02` |
| `crates/costs/src/regime.rs` (1,373 lines) | yes | `OCT_2024` `:338`; `APR_2026` `:341`; `MAR_2026` `:346` (private; "A Sunday; the first session it prices is 2026-03-02"); `APR_2023` `:350`; `STT_OPTIONS` table `:353`; `NSE_EXCHANGE_CHARGE` `:421` (3,503); `NSE_EXCHANGE_CHARGE_FA73061` `:440` (3,553); `NSE_EXCHANGE_CHARGE_TABLE` `:457` (refusal, FA64232 2024-10-01, FA73061 2026-03-01: `MAX_LATER_ROWS = 2` is now full); `NSE_IPFT_TABLE` `:535`; compile-time total check `:592-593`; `stt_options_rate` `:627`; `stt_exercise_rate` `:638`; `exchange_charge_rate` `:672`; `exchange_charge_refusal_windows` `:682`; `ipft_rate` `:725`. `RegimeTable`/`RegimeRow` are `pub(crate)` with no boundary accessor |
| `crates/costs/tests/fa73061.rs` (618 lines) | yes (CHG-0's) | trip-level tests `:107`, `:124`, `:144`, `:160`; `every_captured_dhan_option_case_shows_the_exchange_line_of_the_fa73061_row` `:278`; `every_boundary_around_fa73061_prices_its_own_rows_and_nothing_else_moves` `:322`; `the_fold_costs_at_most_one_paisa_a_leg_and_under_50_lakh_is_never_below_the_split` `:429`; `the_dated_rows_answer_the_same_from_fourteen_threads_at_once` `:473`; ignored timing `the_dated_lookups_cost_the_same_on_every_row_measured` `:549` |
| `crates/costs/src/rate.rs` (415 lines) | yes | `NSE_IPFT` `:174` (50; now one row of the dated table); `BSE_IPFT` (UNVERIFIED zero); **no `ipft` function** (removed by CHG-0, D-5101) |
| `crates/costs/src/ledger.rs` (1,694 lines) | yes | physical-delivery refusal pinned `:1009-1010`; BSE F&O calculator refusals `:1134`, `:1608` |
| `crates/costs/benches/stack.rs` (698 lines) | yes | C-CHG-01 `:202` .. C-CHG-09 |
| `crates/costs/src/scope.rs` | yes | `include_str!("scope.rs")` source-shape pattern `:232` |

---

## Appendix A. The shared context every builder reads first (`EMBED.context`, launcher line 10)

Verbatim from `state/cloud-20261024/embed.json` `.context` (identical to the launcher's line 10), except that the two copies of the commit co-author e-mail address are replaced by a pointer, because this file is public. Much of it is about `/db` and ingest; for this part the repo law, the operator's standing rules, the hard rules, the usage-budget rules and the no-deletion rule are what bind. Where it and the README disagree about the cloud, the README wins (no store, no ports, no `:8080` here). The cloud commit trailer is the README's.

```text
CONTEXT (brutex, 7 Oct 2026; RESUMED 9 Oct 2026 on a new Claude account after the weekly usage limit stopped round 1 mid-run). The operator wants /db — and everything it depends on — entirely correct: every selection on Zerodha and GDFL (indices, stocks, expired options with moneyness, intrinsic, extrinsic, IV, greeks, OI) shows exactly the right data; everything searchable, findable, auditable, logged and visualised; O(1) per operation with measured p50/p99; every extreme case covered. A read-only audit confirmed 185 defects (each verified twice); the fix plan splits them into work packages with non-overlapping files.

FILES: plan = /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/board/fix-plan.json ; findings with full evidence = .../state/board/audit-result.json (key "confirmed"). Integration worktree /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/wt/DBFIX on branch feat/db-fix (from feat/app-all plus the Zerodha minute-calendar fix). The operator's real app (:8080) serves wt/APP (feat/app-all) — never touch it; it is redeployed by the main session after merges.

REPO LAW (CLAUDE.md, binding): Rust only outside web/ (web/ is unrestricted, but no crate may need Node to build/test); no invention — vendor/exchange claims need a source or UNVERIFIED; run identity includes feed; O(1) per operation; append-only store (new versions/generations, never in-place mutation); no look-ahead; fallbacks never hide a failure (degrade loudly with the reason, or refuse); no test that asserts nothing; prices are paisa i64; decisions live in docs/05-decisions.md (WP-02 writes D-3316..D-3343 up front; the Zerodha calendar fix uses D-3344+). Never copy raw vendor rows into the repository; fixtures are synthetic.

OPERATOR ANSWERS (defaults the operator can still override; WP-02 records them):
1. The GDFL options import was STOPPED by the operator at 23:23 IST on 6 Oct, mid-day 2020-09-16. Bars after 2019-11-13 are on disk but uncounted; they get counted by the census v4 recount (no vendor reread), then the import resumes from 2020-09-16.
2. Store writes and API rebuild/restart are approved in principle ("fix and resolve everything"); they run in the main session after the code lands (WP-80), each as a new month/generation, never in place. Agents never write the real store.
3. GDFL closing-auction prints (from 2026-08-03): keep them, as a labelled 'auction' field/row kind — never silently dropped, never mixed into ordinary trades.
4. Spot for an option bar: exact second stays the default (D-2807); add an opt-in 'last spot at or before t' that always shows the spot's age. Never a later spot.
5. Yes: import the GDFL index files (FINNIFTY, MIDCPNIFTY, NIFTYNXT50, NIFTYIT, NIFTYCPSE, NIFTYFPI, INDIA VIX) and the old-name stock files (TATAMOTORS, HDFC, ZOMATO and the others) — stored, not swept — so every option underlying gets spot and greeks.
6. Zerodha 2015–2017 index 1day bars: the operator's window is 2018-01-01..2026-09-30. Hide them by default; a visible toggle shows them labelled 'outside your window'. Never delete.
7. Dhan, Groww and TrueData: removed from every picker (the operator uses only Zerodha and GDFL).
8. NSE circulars for the 2020-03-13 / 2020-03-23 halts, renames and index aliases: WP-02 may look them up (WebSearch/WebFetch on nseindia.com / nsearchives); record the circular number and URL if found, else keep UNVERIFIED and show 'suspected'.
9. 30 May 2019 LTQ=1 marker rows and LTQ=0 price rows: flag only; bars unchanged.
10. OI of a 1s bar: the last row of the second (bar-close semantics), applied to new imports; existing months are re-imported as new generations by WP-80.
11. /db opens on NIFTY · Spot · the feed's finest timeframe · newest first.

HARD RULES: never write the real store /Volumes/WD_BLACK/brutex/fresh-20260919/runtime/store; never POST to any server; never start, stop or restart :8080; never kill a process you did not start; never push; never rm -rf outside your own worktree, target dir or shadow dir. Commands that bind ports or write outside the sandbox may need dangerouslyDisableSandbox. The machine is shared by ~12 agents: keep builds to your own CARGO_TARGET_DIR and avoid needless rebuilds.


RESUME FACTS (9 Oct): WP-02 is committed and merged into feat/db-fix (b875f22e), so D-3316..D-3343 exist. WP-70 is done (outside the repo). Every other package's first agent was cut off by the usage limit; six of them (WP-00, WP-01, WP-10, WP-11a, WP-12b, WP-14) left UNCOMMITTED work in their worktrees, backed up as patches in /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/resume-kit/wip-20261009/. The Mac rebooted on 8 Oct; :8080 was restarted by the main session on the same build (tgt/app5) and real store. BUILDS: export PATH=/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/bin:$PATH before any cargo command, so builds go through the machine-wide queue (8 at once, 4 jobs each; queued waiting is normal, not a hang). Start long cargo commands in the background writing a log that ends with a line EXIT <code>, then wait with one blocking `until grep -q '^EXIT ' <log>; do sleep 20; done` call (600000 ms timeout, repeated). SAFETY: never rm -rf "$HOME", "$TMPDIR", ~ or any unquoted variable or glob; use literal absolute paths inside your own worktree, target or shadow dir. Never commit or copy a real GDFL data row (Ticker,DD/MM/YYYY,HH:MM:SS,price...) into the repository: fixtures are invented, with prices that cannot be mistaken for real ones.

ROUND 1c (10 Oct, new account): 17 packages are merged (feat/db-fix 3cd0623c). The 18 defects found on 9 Oct are now findings DB-N01..DB-N18 in audit-result.json, added to WP-50, WP-52, WP-53, WP-56 and a new package WP-39 (bars sort index: the blocking full-history 1s sort). Co-author line for commits: 'Co-Authored-By: Claude Opus 5.5 (1M context) <the noreply address: see reports/db-specs-20261024/README.md, "Commit messages">'.

ROUND 2 (11 Oct 2026, a new account; where this paragraph and the text above disagree, this paragraph is right). 33 packages are merged on local feat/db-fix (WP-00 WP-01 WP-02 WP-10 WP-11a WP-11b WP-12a WP-12b WP-12c WP-13a WP-13b WP-13c WP-14 WP-15 WP-20 WP-30 WP-31 WP-32 WP-33a WP-33b WP-33c WP-34 WP-35 WP-36 WP-37 WP-38a WP-38b WP-50 WP-51 WP-52 WP-53 WP-54 WP-55); read git -C /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/wt/DBFIX log --oneline --grep='^Merge ' feat/db-fix for the true state. The confirmed findings are now 343 (DB-*, DB-N01..N26, DB-A01..A53, DB-B01..B79). The operator's live app :8080 runs a frozen copy of a build (tgt/app<N>) over the real store and is redeployed by the main session after merges, never by you. This run also builds the INGEST AUTOMATION packages (IA-*): the operator's core aim is that the app itself imports GDFL from /ingest, fast and self-checking, with no Claude session in the loop; their specification is state/design/ingest-automation.md.

OPERATOR'S STANDING RULES (11 Oct, binding on every package): (1) O(1) per operation at p99 everywhere, MEASURED: report p50, p99 and max with n and the method; anything that cannot be O(1) is bounded or refused out loud and named for docs/06-limits.md. (2) One common, runtime-dynamic, incremental, scalable code path: no instrument or vendor special case, no hard-coded list the data already knows, no work redone that an index or a sidecar can remember. (3) Every state change is saved, audited, logged, searchable and visible on a page; nothing fails silently. (4) Every extreme case is covered by a test that fails before the fix: empty, single, maximum, duplicate, out-of-order, late, torn, truncated, missing-in-the-middle, renamed, special session, concurrent, killed mid-write, disk full, garbage parameters. (5) No claim without evidence a second agent can repeat; say NOT PROVED otherwise. A SECOND AGENT re-runs your gates, reproduces every 'fixed' finding on a fresh shadow server and attacks your change before it is merged.

PORTS AND SANDBOX (11 Oct): test servers left by earlier runs still listen on 8100-8142, 8231, 8347 and 8372; never trust an answer from a port you did not just start, and never stop a server that is not your own package's. Inside the Bash sandbox a direct connection to 127.0.0.1 is refused ('Operation not permitted'). Reach your own shadow port either through the sandbox proxy - curl --noproxy '' --proxy "$HTTP_PROXY" http://127.0.0.1:<port>/... with 127.0.0.1:<port> listed in that command's allowed_domains - or by running the command with dangerouslyDisableSandbox. Starting a server (binding a port) needs dangerouslyDisableSandbox. Never change permission settings; never ask the operator to approve anything: if a command is refused, find another way that stays inside these rules, or report it in notes. Commit co-author line: 'Co-Authored-By: Claude Opus 5.5 (1M context) <the noreply address: see reports/db-specs-20261024/README.md, "Commit messages">'.

USAGE BUDGET (11 Oct): the account's usage window is small and an agent cut off by it loses everything it has read. Work economically: grep before you read; read by line range, never whole large files; never print a whole log (tail -40, or grep for the failures); do not re-read what you have read; run each gate once after your last edit, not after every edit; keep throwaway exploration out of your context. If an earlier attempt of your package left commits and uncommitted changes in the worktree, they hold up to 30 minutes of an interrupted agent's work from today: start from them. Commit your work in progress on your own branch at least every 15 minutes (message 'WIP <package>: <what>'; squash before your final report), so a cut-off loses little. Aim to have the package committed within about 45 minutes; if a part cannot be finished well in that time, commit what is complete and proved, and list the rest exactly under notes as NOT DONE.

KNOWN SLOW TEST (11 Oct, measured by a second agent): cli's pool::tests::the_pool_verb_prints_its_whole_page_on_a_generated_store ran for more than 95 minutes on this loaded machine and held up every package that runs the cli suite. Unless your package changes crates/cli/src/pool* or what that test reads, run the cli suite as: cargo test -p cli --locked -- --skip the_pool_verb_prints_its_whole_page_on_a_generated_store , and say in your gates line that this one test was skipped as the known slow one. The final Gate run on feat/db-fix runs it. Never wait more than 25 minutes on one test run without looking at which test is still running (the log's last 'test ... has been running for over 60 seconds' lines name it).

NO DELETIONS THAT NEED APPROVAL (12 Oct, learned the hard way): a command with rm -rf, or any deletion run with dangerouslyDisableSandbox, raises an approval card that nobody answers, and the agent that sent it hangs for hours. NEVER run rm -rf. Do not delete browser profiles, scratch worktrees' build output, target directories or shadow stores at the end: leave them where they are and list their paths in your notes; the main session cleans up. This holds inside the sandbox too: find -delete, git worktree remove, git clean and rm of a directory all raised cards. Delete nothing at all; if you need a clean directory, make a NEW one with a new name. Write your report file BEFORE any tidying. The helper scripts state/fix/shadow.sh, serve.sh and stop.sh delete nothing (shadow.sh moves an older shadow of the same name aside): use them as written, and always make a FRESH shadow for a check; never reuse one another agent left. If any command has not returned after 20 minutes with no output, treat it as stuck, do not repeat it, and finish your report without it.

RELAUNCH 13 OCT 2026 (a new session; where this paragraph and the text above disagree, this paragraph is right). 50 packages are merged on feat/db-fix (head 29dc84e6 at launch); git -C /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/wt/DBFIX log --oneline --grep='^Merge ' feat/db-fix is the truth. :8080 runs a frozen build tgt/app18 and is redeployed by the main session only. WP-62 and WP-12f failed their third check (round2/verify-<pkg>-r3.json): their builders repair first. The compressed-store design (state/design/store-compression-20261012.md, section 9) names packages that collide; those are now dependencies (IA-1 after R3-35; R3-06 after SC-0 and SC-2; SC-3 after R3-08; SC-5 after R3-07 and R3-08; R3-22 after SC-4) and each affected package's spec says what to do. DISK (operator's rule): before any step that writes many files (a backfill, an import, a sweep, an index build), project its disk use and file count from a measured sample and write the projection in your report; no import of stock options happens until the compressed form exists. The operator restated the standing rules on 13 Oct, word for word in spirit: O(1) at p99 everywhere, measured; everything in parallel; every state change saved, audited, logged, tracked, searchable, monitored and shown on a page as plain comparison tables; one 100% common, runtime-dynamic, incremental, scalable path; and every extreme permutation and combination of exceptions, errors, situations and conditions covered by a test, including the out-of-the-box ones nobody listed.

ROUND 3 ADDENDUM (14 Oct 2026). The operator's MAIN AIM: one press of Ingest (and a daily timer) runs the whole chain end to end with no human or Claude step: GDFL Dropbox download, verify, compress into the tick store, ticks to 1-second bars; Zerodha daily + 1-minute + the folded 2-60 minute candles; every check; the pages. Everything compressed with O(1) per lookup at p99; a fresh clone plus only the Dropbox link and the daily Kite sign-in rebuilds code and data. Packages R4-01..R4-29 come from /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fix/round3-packages.json (read its notes for SHARED LINES, SEQUENCED EDITS and split items). FAILURE MATRIX /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fix/e2e-failure-matrix.json: every row whose owner is your package, one of your item ids, or (for round-2 packages) your package id is part of your package: make its test exist and pass, or report it NOT DONE with the reason. AMENDMENTS: A1 IA-12a depends on R4-01 and adds its panel as web/src/routes/ingest/panels/GdflImport.svelte with no +page.svelte edit. A2 SC-8 depends on R4-01 and R4-04, puts its band at web/src/routes/ingest/panels/Storage.svelte and its /db badge at web/src/routes/db/badges/Storage.svelte, edits neither +page.svelte, and uses server.rs for route lines only. A3 IA-6 and IA-16 add their codes' rows to crates/pull/src/reasons.rs if R4-05 merged first. A4 SC-2's sealed summary section carries R3-06's month summary bytes with their version; no R4 package edits sealed.rs. A5 R3-32 ships no tracked config/launchd/*.plist (CLAUDE.md section 2, gate 1). A6 decision numbers are taken at landing from the ledger's head (D-4928 is already used; the head is past D-4950). Never delete any file; never write the real store.
```

## Appendix B. The builder's prompt (launcher `state/fix/db-fix-round-2-run0738.js.fat`, lines 341-372)

Verbatim. `specStep(p)` (341-349) and `fixPrompt(p)` (351-372). For this part: `p.kind` is `nw`, so the second `return` of `specStep` applies; `${W}` = `/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925`; `${p.name}` = `CLOUD-CHG1-OPT`; `${p.title}` = the title in section 2; `${p.spec}` = the spec in section 2; `${p.deps}` = `CHG-0, CLOUD-CHG1`; `${p.port}` = 8497; `${POOL(p.name)}` = 5; `${OUT}` = `${W}/state/fix/round2`; `${CO}` = the co-author trailer (in the cloud: the README's); `${C}` = Appendix A; `${drainFirst(...)}` = a Mac-only drain-flag check (no meaning in the cloud: the README's GO / HOLD files replace it); `args.niftyFirst` is unset, so the NIFTY FIRST paragraph is included. In the cloud, steps 2 (worktree, pool lock, node_modules link) and 5 (shadow server on the Mac) are replaced by the README's branch rule and section 7 of this file; steps 3, 4, 5b, 6 and 7 bind as written, except that the cloud pushes its own branch as the README says.

```text
function specStep(p) {
  if (p.kind === 'ia') {
    return `1. Read your package. It is an INGEST AUTOMATION package; its specification is the design ${DESIGN} (6,154 lines; read by line range, never all at once). Start with your row of the table in section 16.2: grep -n '^| ${p.row || p.name} ' ${DESIGN} (rows are single long lines; print them whole with sed -n '<line>p'), the round notes above that table that name ${p.name}, "Lanes" and "The order" below it, and your row in 16.3 (CI). Then read EVERY section, decision (section 15, column "Lands with"), file entry (section 11), route (12), web change (13), test id (14), limit row (10.2), matrix row (8) and edge case (9) your row cites. If ${CRITIC} exists, read its part for ${p.name}: its resolutions are your defaults where the design is silent or contradicts itself. Where the design states a fact about merged code that is false at the head of feat/db-fix, the code at the head is the fact: build to the design's intent and say so in notes. ${p.spec || ''}
   Also read ${W}/state/board/fix-plan.json's entry if one exists: jq '[.work_packages[] | select((.name | split(" ")[0]) == "${p.name}")]' ${W}/state/board/fix-plan.json
   The findings your package answers (ids starting DB-A with package IA, and the gaps F1..F18 your row names) are in ${W}/state/board/audit-result.json (key "confirmed"). Report one findings[] entry per test id, decision and gap your row names (id = the test id, decision number or gap id).`
  }
  return `1. Read your package: jq '[.work_packages[] | select((.name | split(" ")[0]) == "${p.name}")][0]' ${W}/state/board/fix-plan.json
   Read the full text of every finding it lists (observed, expected, evidence, location, fix_hint) from ${W}/state/board/audit-result.json (key "confirmed", match on "id"). Read the plan's summary and all_sources_view in fix-plan.json if your package touches them. D-number assignments are in WP-02's decisions_needed. ${p.spec || ''}`
}

function fixPrompt(p) {
  return `${drainFirst('fix', p.name)}${C}

YOU OWN WORK PACKAGE ${p.name}: "${p.title}".
${(args && args.niftyFirst === false) ? '' : `\nNIFTY FIRST (operator): prove your work first on NIFTY spot and NIFTY options before anything else, on every NIFTY selection your package touches (feed GDFL 1s and Zerodha 1min..1day; segment Spot and expired options; every expiry form 2018-09..2020-09, strikes, CE/PE; dates, time filters, sorts, Go to, paging, export, coverage, moneyness/IV/greeks cells; for import packages: one NIFTY spot day, one NIFTY options day, one special session). Fix in the SHARED, instrument-agnostic code path, never as a NIFTY special case, then repeat the check on one BANKNIFTY selection, one F&O stock and one illiquid strike. The NIFTY evidence so far is in ${W}/state/gdfl-board/out/nifty.json.\n`}
IF ${W}/wt/P-${p.name} ALREADY EXISTS (an earlier attempt of this same package was cut off by a usage limit; it may hold commits ahead of feat/db-fix, "WIP ... to be squashed" commits and UNCOMMITTED changes), reuse it: read git log feat/db-fix..HEAD and git diff first, judge every hunk critically against your package, keep what is right, fix what is wrong, finish what is missing; never discard it blindly, never git checkout/reset away its changes. Changed files under web/build/ are generated output of an earlier npm run build: reset them (git checkout -- web/build && git clean -fdq web/build) before you judge the diff; never commit web/build unless your package's row owns it.
IF ${OUT}/fix-${p.name}.json or ${OUT}/verify-${p.name}-r<n>.json EXIST, an earlier run today already built this package (and a second agent may have checked it): read the newest of each first. Every BLOCKING issue in the newest verify report, and every follow-up in it that names your files, is part of your package now; fix those first, each with a test that fails before, then finish whatever the builder's notes list as NOT DONE. If the branch is committed and nothing is open, re-run the gates once, write your report and return quickly.
SAVE AS YOU GO, RESUME, NEVER REDO (operator 21 Oct 2026: "don't repeat the same work twice; always continue or resume after holding it"). A usage limit, an app restart or a hold can end you at any minute, and whatever is only in your head is then lost. (a) FIRST, if ${OUT}/fix-${p.name}-progress.md exists, read it whole: an earlier attempt of this package wrote it. Continue from its last NEXT line. What it records as done stands for every commit still on the branch (git log feat/db-fix..HEAD): redo only what it does not record, and re-run a recorded gate only as part of your one final gate run. Shadow-server checks it records from before your last code commit do not stand: take them again on the final build before your last commit (a running server tests every file at once). If git log --first-parent --format=%s feat/db-fix shows "Merge ${p.name}:", that earlier attempt was already merged: its entries are history and its NEXT line does not bind you; append an entry "attempt after merge" and do the task below. (b) Keep that file as you go: append (never rewrite) an entry after every finished piece of work, headed by the output of TZ=Asia/Kolkata date '+%F %T IST': what you did and in which files, each gate or test command with its exact result line, each shadow-server check (URL + value), the WIP commit sha, and a NEXT line that says exactly what comes next. (c) Commit on fix/db-${p.name} at least every 20 minutes of editing and always right before any long test, build or gate run: git add <only your files>; git commit -m "WIP ${p.name}: <what> (to be squashed)" -m "${CO}". Never hold more than 20 minutes of edits uncommitted. Do NOT squash these commits yourself, whatever the context says about squashing before the final report (21 Oct): the merge folds every WIP commit into one squash commit with the same tree. Your last commit (step 6) is a normal one. (d) A BRANCH ALREADY MERGED BY A FOLD (review fix 21 Oct): before ANY git merge feat/db-fix into this branch (steps 2 and 4), run B=$(git log -1 --format=%H --grep='^${p.name}: squash of fix/db-${p.name} at ' feat/db-fix). If B is set and git merge-base --is-ancestor $B HEAD fails, take T0=$(git log -1 --format=%s $B | grep -oE '[0-9a-f]{40}' | head -1); if git merge-base --is-ancestor $T0 HEAD succeeds, first run git merge -s ours -m "${p.name}: record fold $B of $T0 (same content, no file changed)" $B. It changes no file (that fold holds exactly the tree of $T0, which your branch already has) and makes the later commits of feat/db-fix merge onto your own lines without conflicts. The commits that fold already merged still show in git log feat/db-fix..HEAD: they are merged, not open work.
RULES AUDIT (11 Oct): ${W}/state/design/rules-audit-20261011.md lists, per owner, the gaps ten agents found against the operator's rules (O(1) at p99, parallel, saved and searchable, common and scalable, extreme cases). Find the section for ${p.name} (grep -n '^## .*${p.name}' that file): every gap listed there is part of your package, with the test or bench it names. If a gap there cannot be closed inside your files, say so in notes with the owner it needs.
${specStep(p)}
2. Set up: git -C ${W}/wt/DBFIX worktree add ${W}/wt/P-${p.name} -b fix/db-${p.name} feat/db-fix   (if the worktree or the branch already exists, reuse it, then git merge feat/db-fix into it, resolving conflicts by keeping BOTH sides' intent). Then: ln -s ${W}/wt/APP/web/node_modules ${W}/wt/P-${p.name}/web/node_modules (skip if present); export CARGO_TARGET_DIR=${W}/tgt/pool-${POOL(p.name)}. [SHARED POOL, OPS-09 18 Oct: hold its lock around every cargo command: mkdir ${W}/tgt/pool-${POOL(p.name)}.lock before, rmdir it after; while it exists wait 30 s and retry; a lock older than 90 minutes is stale: rmdir it. OPS-10 20 Oct (AG-05, R3-24, PIT-1 and PREC-1 checks): cargo names the artifacts of a workspace crate by its path inside the workspace and trusts mtimes, so in a shared pool a newer build from another worktree reads as fresh and runs as yours. Take the lock ONCE for a whole sequence of cargo commands (gates, release build) and rmdir it after the last of them, not after each. Touch the lock directory right before each cargo command of the sequence, and never from a background loop or a timer: a loop that outlives you keeps the lock of a dead holder fresh, and the 90-minute rule never frees it. Right after taking the lock, and again after any git merge, checkout or reset inside the hold, run (cd <the worktree you build> && git ls-files -z | xargs -0 touch) before the first cargo command. A gate result taken without that touch is not counted. A release api or cli you serve or measure is copied under the lock (mkdir -p, then cp) to ${W}/tgt/bin-${p.name}/release/ (the second agent: ${W}/tgt/bin-${p.name}-v/release/) and served from there, never from the pool] Work ONLY in that worktree.
   Dependencies: ${(p.deps || []).join(', ') || 'none'}. All are merged into feat/db-fix, so your branch contains them once you merge feat/db-fix.
3. Do the package completely and precisely, the way its specification says, with tests that fail before and pass after. Edit only your package's owned files (new test files for your own modules are fine). If you truly must touch another file, keep it minimal and list it in files_outside_ownership. /db packages (WP-*) do not edit docs/05-decisions.md, docs/04-invariants.md, docs/06-limits.md, docs/11-findings.md or docs/07-plan.md unless they are WP-99: cite the D-numbers WP-02 assigned in code comments instead. Ingest packages (IA-*) land their own rows in those ledgers in the same change (D-4861): re-grep the highest D-48xx number first and never reuse one.
4. Gates in your worktree (all must pass before you commit; merge feat/db-fix into your branch once more just before them): cargo fmt --all --check; cargo clippy -p <each touched crate> --all-targets -- -D warnings; cargo test -p <each touched crate> --locked; if web/ changed: (cd web && npm run build && npm test), then reset the generated web/build files unless you own them. Anything you add to a hot path must stay O(1) per operation; measure p50, p99 and max for every route or operation you touch (n >= 200 sequential GETs against your shadow server, or the crate's bench) and report them with n and the method. Anything that cannot be O(1) is refused or bounded, never silently slow, and is named for docs/06-limits.md in your notes.
5. Check against REAL data, never on :8080: build in release (cargo build --release -p api -p cli), then
   ${W}/state/fix/stop.sh ${p.name}; ${W}/state/fix/shadow.sh ${p.name} && ${W}/state/fix/serve.sh ${p.name} ${p.port} ${W}/tgt/bin-${p.name} [SHARED POOL, OPS-09 18 Oct: hold its lock around every cargo command: mkdir ${W}/tgt/pool-${POOL(p.name)}.lock before, rmdir it after; while it exists wait 30 s and retry; a lock older than 90 minutes is stale: rmdir it. OPS-10 20 Oct (AG-05, R3-24, PIT-1 and PREC-1 checks): cargo names the artifacts of a workspace crate by its path inside the workspace and trusts mtimes, so in a shared pool a newer build from another worktree reads as fresh and runs as yours. Take the lock ONCE for a whole sequence of cargo commands (gates, release build) and rmdir it after the last of them, not after each. Touch the lock directory right before each cargo command of the sequence, and never from a background loop or a timer: a loop that outlives you keeps the lock of a dead holder fresh, and the 90-minute rule never frees it. Right after taking the lock, and again after any git merge, checkout or reset inside the hold, run (cd <the worktree you build> && git ls-files -z | xargs -0 touch) before the first cargo command. A gate result taken without that touch is not counted. A release api or cli you serve or measure is copied under the lock (mkdir -p, then cp) to ${W}/tgt/bin-${p.name}/release/ (the second agent: ${W}/tgt/bin-${p.name}-v/release/) and served from there, never from the pool] ${W}/wt/P-${p.name}
   This serves your build at http://127.0.0.1:${p.port} over a private shadow of the real store (bar files read through a link, everything else cloned). Confirm YOUR server is the listener: lsof -nP -iTCP:${p.port} -sTCP:LISTEN -t must print the pid in ${W}/shadow/${p.name}/test-server.pid. GET requests only against a shadow whose bars are the real store's link: never POST there, never run an import, pull, repair or backfill against it. For code that WRITES (importer, repair, sidecar or index writer, migration, verdict or alert store): exercise the write path only on a store root you own entirely - a temp dir built from synthetic fixtures, or a shadow in which you REPLACED the bars link by a real directory holding copies of the few months you need (cp -c) - and prove it never touched /Volumes/WD_BLACK/brutex/fresh-20260919/runtime/store. Reproduce each finding's selection and confirm the right answer (record URL + value as evidence a second agent can repeat). If the page is web-only, curl the API and run the web unit tests; you may open http://127.0.0.1:${p.port}/db in the browser pane only if no other agent is using it. Stop your server with ${W}/state/fix/stop.sh ${p.name} when done.
5b. SELF-CHECK BEFORE YOU COMMIT (19 Oct speed rule: a failed second check costs a whole repair round plus a re-check): do the second agent's checks yourself first. (i) Read your own full diff against feat/db-fix and judge every hunk against repo law (O(1) per operation: name every loop over months, files, records or instruments that a request or a per-row step can reach; append-only store; no look-ahead; no fallback that hides a failure; prices in paisa i64; Rust only outside web/; no test that asserts nothing; no real vendor row in a fixture) and the operator's rules (one common runtime-dynamic code path, incremental and scalable, every state change saved, audited, logged and searchable; every failure, refusal and extreme value tracked so it can be found, debugged, monitored and seen where the operator looks (/logs, /db or the part's own page) through a read that stays O(1) at p99). (ii) Add tests for at least 8 extreme cases that fit your change, at least 2 of them combinations of extremes at once and at least 1 out-of-the-box case no list names (empty, one-record and maximum inputs; month, year and expiry boundaries; special sessions; renamed symbols; duplicate, out-of-order and late rows; torn or zero-length files; a missing month mid-range; garbage parameters; concurrent requests; a kill mid-write; two writers). (iii) Re-run every finding you call fixed on your shadow server and record URL + value. Fix whatever this finds before you commit, and list in notes what you checked.
6. Commit on fix/db-${p.name} (git add the files you changed; never git add -A at the root, never commit vendor data, binaries, shadow or target dirs). Message: what was fixed and why, finding ids, D-numbers, ending with the line "${CO}". Never push. Never touch :8080, wt/APP, tgt/app*, the real store, or another package's worktree.
7. If something needs a store write on the real store (re-import, recount, migration, re-pull, backfill), implement and test the code path on fixtures and list the exact command under store_writes_needed; do not run it on the real store.
8. Write your structured report also as JSON to ${OUT}/fix-${p.name}.json (mkdir -p ${OUT}).
Return the structured report. Be honest: a finding is "fixed" only if a test or a shadow-server check proves it; say NOT PROVED where you could not check.`
```

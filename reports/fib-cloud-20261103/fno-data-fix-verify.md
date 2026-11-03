# F&O fix, gaps A-E: stage VERIFY (31 Oct 2026, 02:43-03:3x IST)

**Verdict: PROVED for what this run set out to close.** Every check of the whole run is listed in section 6, and
every difference any of them found has a documented rule. No difference is unexplained.

- **Gaps A and E (Zerodha) are closed.** The 12 new keys hold every owed session at 1min, at all 7 derived rungs and
  at 1day. The one exception is the 2018-11-07 Muhurat 1day bar of the five 2018 shares, which Kite does not serve
  (gap F). MAZDOCK and FORCEMOT hold all 19 October sessions.
- **Gaps B, C and D (GDFL) are closed.** Two kinds of session stay missing, both named:
  - 252 Aug-Sep 2026 sessions of six B stocks. No door in a32722c45 can file them.
  - PATANJALI 2020-01-24, which is not a trading day for it.
- **The app serves exactly what the rehearsed shadows hold.** 720 stock-days were sampled, 599,230 bars, and all are
  equal. In 4 bars the open interest is null on the wire and 0 in the file, by D-3322.
- **Nothing outside the plan changed.** That covers every stock and file the live stages did not plan.
- **Reads are O(1).** The time does not grow with history length or month age.

**Now:**
- **207 of 218** stocks are session-complete in both feeds (was 178).
- **196** of them are also free of known stock defects (was 169).
- 11 stocks are not session-complete, each with its door (section 4).

Rules kept:
- GET only on `:8080`. It was never stopped and is still pid 61553.
- Every script that read the store or a shadow, and all three `find`s, ran under `state/fix/ro.sh`. A few plain shell
  reads of the live store ran without it: `stat`, `md5`, `tail` and `ls` of the audit ledgers and of some month
  files. None of them can write.
- No write to the store, no lock taken, no server started, no cargo, no Py-language.
- No token, password or secret touched. No price printed.
- Nothing deleted or moved, except three outputs of my own scripts that I moved aside (section 7).

All paths are under `state/fno-data-fix-20261030/`. My evidence is in `verify-evidence/`, with every command in
`verify-evidence/COMMANDS.txt`. My scripts are `bin/verify-*.js`.

---

## 1. `/db`

- At the start: `GET :8080/db` returned HTTP 200, 5,950 bytes (`verify-evidence/db-check-1.txt`).
- At the end (03:34 IST): also 200. No writer was running, `:8080` was still pid 61553 and the lock was absent
  (`end.txt`).
- No other writer was running at the start (`writers-start.txt`):
  - `ps` showed no `release/cli` writer;
  - `/pull/run.json` showed running false;
  - `/autopilot.json` showed pull_active false;
  - `live-writer.lock` was absent.

## 2. Every touched stock: sessions held / owed, from the census and from the app, and served bars equal to the shadow

The touched stocks are 37: the 30 of the GDFL track (B, C, D) plus the 14 of the Zerodha track (A, E), less the 7
that are in both (`touched37.txt`).

### 2a. The census, all 218 stocks, live store

- **The census.** The audit's own census, `census-stock.js`, in its CAS copy `bin/census-stock-cas.js` (the dated
  375/360 rule for the 14 pulled stocks), run by `bin/census-shadow-cas.sh` on the live store under `ro.sh`.
- **The window.** Zerodha runs from max(2018-01-01, listing). GDFL runs from max(2018-09-03, listing). Both end on
  2026-10-29.
- **The run.** 95 s, 218 of 218 stocks, 0 failed, 0 census errors (`census-218-live/`, `census-218-live.log`).

### 2b. The app's own answer: `/calendar.json`, the /db page's calendar, against the census, day by day

The script is `bin/verify-touched.js`, with GET requests only, writing `touched-census-vs-app.tsv`. It covered 37
stocks × 10 series (Zerodha 1min, 2/3/5/10/15/30/60min and 1day; GDFL 1s), so 370 series.

For each series it compared the census's held days and session days with the app's `days` and `held` strings, every
day in the window.

**Result: 370 of 370 series EQUAL.** 0 days are held on one side only, and 0 session days differ.

| track | stock | Zerodha 1min | each of 7 rungs | 1day | GDFL 1s | still missing |
|---|---|---|---|---|---|---|
| A+B | KARURVYSYA | 2,173/2,173 | 2,173/2,173 | 2,173/2,173 | 1,964/2,006 | GDFL 2026-08-03..09-30 (42) |
| A+B | UJJIVANSFB | 1,697/1,697 | 1,697 | 1,697 | 1,655/1,697 | same 42 |
| A+B | ANANDRATHI | 1,201/1,201 | 1,201 | 1,201 | 1,159/1,201 | same 42 |
| A+B | DATAPATTNS | 1,193/1,193 | 1,193 | 1,193 | 1,151/1,193 | same 42 |
| A+B | SYRMA | 1,026/1,026 | 1,026 | 1,026 | 984/1,026 | same 42 |
| A+B | ENRIN | 336/336 | 336 | 336 | 294/336 | same 42 |
| A+D | SAGILITY | 483/483 | 483 | 483 | 483/483 | none |
| A | COROMANDEL, CUB, MAHABANK, NAVINFLUOR | 2,173/2,173 each | 2,173 | 2,173 | 2,006/2,006 | none |
| A | ATHERENERG | 368/368 | 368 | 368 | 368/368 | none |
| C | GVT&D, UNOMINDA, NAM-INDIA | 2,173/2,173 | 2,173 | 2,173 | 2,006/2,006 | none |
| C | ANGELONE | 1,495/1,495 | 1,495 | 1,495 | 1,495/1,495 | none |
| C | PATANJALI | 1,666/1,667 | 1,666 | 1,666 | 1,666/1,667 | 2020-01-24 in both feeds: not a session for it (audit C4) |
| D | ADANIPOWER, SUZLON, ADANIENSOL, CGPOWER, JSWENERGY, PGEL, CDSL, COCHINSHIP, PNBHOUSING, DMART, ABB, ABCAPITAL | 2,173/2,173 each | same | same | 2,006/2,006 each | none |
| D | ADANIGREEN | 2,059/2,059 | 2,059 | 2,059 | 2,006/2,006 | none |
| D | 360ONE | 1,752/1,752 | 1,752 | **1,751**/1,752 | 1,752/1,752 | Zerodha 1day 2019-10-04 (gap G, untouched by this run) |
| D | JIOFIN / KPITTECH / MAXHEALTH / POWERINDIA | 784 / 1,853 / 1,525 / 1,623, all held | same | same | same, all held | none |
| E | MAZDOCK | 1,490/1,490 | 1,490 | 1,490 | 1,490/1,490 | none |
| E | FORCEMOT | 1,698/1,773 | 1,698 | 1,698 | 1,699/1,773 | 74 suspension sessions, plus Zerodha 2019-08-19 (gap G) |

**Specials** (Muhurat, weekend and DR-drill sessions; counted apart, as the census does):
- All held in every series, with two exceptions:
  - the 2018-11-07 Muhurat **1day** bar of the five 2018 shares and of every other share listed by then (176 stocks
    now: the audit's 171 plus these 5). Kite does not serve it (gap F).
  - FORCEMOT 2023-11-12 and 2024-01-20, inside its suspension.
- "Owed" counts regular, halt and outage sessions, as the audit's shard 1 did.

**Counts by evidence, not only by the census.** The attackers' own tools, unchanged, were re-run on the live store
under `ro.sh`:
- **GDFL, `bin-attack/ag-owed.js`.** It counts from the calendar of record (CAL-1) and the listing days. 30 stocks:
  - owed to 2026-09-30: 50,528; held: 50,275 on R2 and on live; missing: 253;
  - 0 stray days;
  - to 2026-10-29: 51,098 owed, 50,845 held.

  Its columns equal the attack's own output exactly (`ag-owed-r2-vs-live.tsv`).
- **GDFL, `bin-attack/ag-months.js`** (shadow R2 against live). It covered every one of the 2,473 months of the 30
  stocks. For each it checked the served generation, header slots, block CRCs, pointer and stamp order, then compared
  every 56-byte record (`ag-months-r2-vs-live/summary.txt`):
  - **2,473 unchanged**, 0 failures;
  - 0 days added or lost.
- **Zerodha, `attack-zerodha-evidence/bin/atk-zcheck.js`** (every file, bar and session of the 14 keys). On live:
  - 7,587 files with 0 problems; 14,509,543 bars with 0 problems;
  - 17,323 sessions × 9 rungs;
  - refold 8,050,699 of 8,050,699 equal;
  - `gapLineMismatch` 3, the three listing-day `refused_day` lines (WP-13d, D-3351).

  `zcheck.tsv`, `zcheck.json` and TOTALS are **identical** to the attacker's run on the rehearsal shadow
  (`zcheck-live/`, `zcheck-live-run.log`).

### 2c. Bars the app serves, against the shadow, on sampled stock-days

**How the days were chosen.** `bin/verify-pick-samples.js` and `bin/verify-pick-extra.js` pick them with a fixed seed,
not by hand (`samples.tsv`, `samples-extra.tsv`).
- **GDFL, the 30 stocks.**
  - up to 3 days the run ADDED, in distinct years;
  - one added day in a VERSIONED month, served from a new revision, where the stock has one: 27 sampled days are
    served from a revision;
  - 2 days HELD BEFORE the run.
- **Zerodha, the 12 new keys.** Each key's first day plus one day in each of up to 4 years, at all 9 rungs.
- **Zerodha, MAZDOCK and FORCEMOT.** 3 October days and 2 older days, at all 9 rungs.
- **Coverage.** Every stock-feed spans at least 3 years, except ATHERENERG and ENRIN, which have only 2026 and 2025
  in their life.

**What was compared** (`bin/verify-served-vs-shadow.js` under `ro.sh`).
- **Served side.** For each stock-day, every bar `:8080` serves through the /db page's own route,
  `/bars/window.json?...&from_day=D&to_day=D` (paged to the total).
- **Shadow side.** The same day decoded from the shadow's **served generation**:
  - FNOFIX-1030-R2 for GDFL, the rehearsed after-state;
  - ZFIX-1030-R1 for Zerodha.
- **Live side.** The same day from the live file.
- **Fields.** All seven fields: t, open, high, low, close, volume and oi. The wire carries integer paisa.

| class | stock-days | rows | verdict |
|---|---|---|---|
| GDFL added (B, C, D) | 49 | | EQUAL |
| GDFL added, versioned month | 23 | | EQUAL |
| GDFL held before | 60 (+3 extra years) | | EQUAL |
| Zerodha new key, first day (× 9 rungs) | 108 | | EQUAL |
| Zerodha new key (× 9 rungs) | 387 | | EQUAL |
| Zerodha October, MAZDOCK and FORCEMOT (× 9) | 54 | | EQUAL |
| Zerodha held before (× 9) | 36 | | 32 EQUAL; **4 EQUAL under D-3322** |
| **total** | **720 samples, 44 stock-feeds** | **599,230 bars** | **599,230 equal; 0 differ, 0 missing either side** |

**The 4 bars.** They are MAZDOCK 1day 2020-10-20 and 2024-04-29, and FORCEMOT 1day 2019-10-07 and 2021-01-08. All
four were held before the run and untouched by it.
- **Where they differ.** Only in open interest. The month file stores `0`, in the shadow and in the live file alike,
  and the wire says `null`.
- **The rule.** `crates/api/src/bars_route.rs` `bars_array_of` at a32722c45, lines 38-45 (D-3322): a CASH bar's
  open interest *"is not applicable and is served `null` ... whatever was stored. A Zerodha index or share month
  stores `0` there"*.
- **Confirmed directly.** t, open, high, low, close and volume are equal.
- **The files themselves.** Shadow and live are equal, raw, in all 720 samples.

Per stock: `served-vs-shadow-per-stock.tsv`; every row: `served-vs-shadow.tsv` and `served-vs-shadow-extra.tsv`.

## 3. Everything the live stages did not plan is unchanged

| check | items | differences |
|---|---|---|
| **My re-diff of the GDFL stage's own lists** (gate-2 before, 02:12, against after, 02:15; `bin/verify-rediff.js`) | CASH 144,904→152,125; revisions 449,029→451,589; generations 49,983→50,015; imports 15; manifest 15; audit 3,855→3,856 | 7,221 NEW + 70 GROWN, 2,560 NEW, 32 NEW + 2 GROWN, 2 GROWN + 2 TOUCHED, 1 GROWN, 1 NEW + 3 GROWN: **all planned; 0 unplanned, 0 shrunk, 0 gone** |
| **between the stages**: the Zerodha revisions and pointers in the GDFL after-lists (02:15) against the Zerodha before-list (02:26) | 377,120 | **0** |
| **My re-diff of the Zerodha stage's lists** (stat and md5, 02:26 against 02:39) | 1,448,011 → 1,480,855 | 32,844 NEW, all planned; **0 changed, 0 gone, 0 md5 differ** |
| **Recomputed now**, a seeded random sample of UNPLANNED paths (`bin/verify-unplanned-sample.js`) | 2,000 GDFL CASH, 500 GDFL revision, 300 GDFL pointer and 3,000 Zerodha paths. Size and mtime against the before-lists; content of 4,942 (sha256 live = R2 for GDFL, md5 = the list for Zerodha) | **0 of 5,800** |
| **Whole live store, every path with mtime or ctime after 02:00 IST** (`find`, 11 min; `bin/verify-classify-changed.js`) | 51,510 paths | 12,111 GDFL-stage, 39,154 Zerodha-stage (incl. `manifest/zerodha.man`, the census manifest a pull rewrites), 2 runs ledger, 21 directories, 222 app telemetry: **0 unexplained**. After the Zerodha stage's lock release (02:39:51), only 12 app-telemetry entries from this stage's GETs |
| **Directories changed after 02:41** (a deletion moves its directory's mtime), the whole store re-walked 03:23-03:32 | every directory | 23, **none under `bars/`, `bar-revisions-v1/`, `bar-generations-v1/` or `manifest/`**: the `verify/spool/1793396656` tree and `verify/live` (the 03:14 pull below), and 2 app-telemetry `run` index folders (`store-dirs-changed-since-0241.txt`) |
| **Another writer during this stage**: a Zerodha pull on `:8080` at **03:14:16-03:15:25 IST**, not by this stage (run 1793396656) | every path with mtime or ctime after 03:14:00 under `bars/zerodha`, `bar-revisions-v1`, `bar-generations-v1`, `manifest`, `imports`, `responses`, `audit` and `verify` (`store-changed-since-0314.txt`) | 254 paths: 208 raw answers under `responses/zerodha`, 36 + 2 under `verify/`, `imports/zerodha.landing.lock`, and `audit/` `pull.journal` (+ index), `pull.notes`, `zerodha.gaps`, `runs.ledger` and `runs.claims`. **0 bar, pointer, revision or manifest files.** All 208 chunks for 2026-10-30 "answered with NO ROWS AT ALL" (`pull-notes-appended-0314.txt`), and `zerodha.gaps` gained only 2 lines, NIFTY and BANKNIFTY 2026-10-30 `vendor_empty`. This is the same kind of 30 Oct pass-4 attempt that ran at 01:05 (live-gdfl.md section 1) |
| census rollups `zerodha` CASH and INDEX, `gdfl` CASH | 3, read at 02:47, 03:23, 03:34 and 03:35 | all byte-identical to each live stage's after-state: generation 390,135; and gdfl 2,366,574, 98 months, 3,887,008,643 rows |
| audit ledgers (`pull.journal`, `zerodha.repairs`, `zerodha.gaps`, `pull.notes`, `runs.*`, `gdfl.repairs`) | 7 files, size, mtime and md5 | at 02:47, equal to the Zerodha stage's `live-after.txt`. At the end, five had grown, by the 03:14 pull only (`audit-files-end.txt`); `zerodha.repairs` and `gdfl.repairs` were unchanged |
| `/feeds.json` store totals | 2 feeds | GDFL 2,127,932 month files and 10,836,946,839 bars; Zerodha 261,798 and 481,378,811; equal at start and end |

**Free space.** It rose from 1,210,533,484 KiB at 02:47 to 2,080,226,020 KiB by 03:19 IST. This stage did not cause
it. It wrote only evidence files, about 43 MB (`verify-evidence/`).
- **Inside the store, nothing was removed.** After 02:41 no directory changed under `bars/`, `bar-revisions-v1/`,
  `bar-generations-v1/` or `manifest/`, and every census answer and store total above is unchanged.
- **So the space came from outside the store.** I did not look for its source.

## 4. The audit's per-stock table, rebuilt for all 218 stocks as of now

`per-stock-after.tsv` and its summary `per-stock-after-summary.json` come from `bin/verify-per-stock.js` under
`ro.sh`. They are built from the ONE census of section 2a. The script also makes three direct reads of the live
store:
- the 2021-02-24 15:45-16:59 minutes;
- the 2018-11-07 Muhurat 1day bar;
- the GDFL 2019-05-30 first second.

It uses the audit's own yardsticks:
- **session-complete in both feeds:**
  - a Zerodha key exists;
  - every owed session is held at 1min, at each of the 7 rungs and at 1day;
  - no rung bucket is withheld;
  - GDFL 1s holds every owed session;
  - specials are held, the Muhurat 1day excused as gap F;
  - PATANJALI 2020-01-24 is not counted (C4);
- **free of known stock defects:** session-complete, and not on the adversary's S1-S4 list.

The audit's own files were not edited.

**The rung "fold mismatches" the census flags were each decoded** (`bin/verify-fold-classify.js`,
`fold-mismatch-classified.tsv`):
- **66,858 buckets in 26 stocks** hold only vendor-padded minutes. Under PaddedFold::Absent they have no bar by design
  (D-7270; the audit's C3 class).
- That covers all twelve new keys and the October months of 13 old keys.
- **The only withheld buckets** are PIIND's 27, on 2019-05-06, 08-02, 09-18 and 09-23. That is exactly the audit's
  gap G.

| | audit (30 Oct) | now |
|---|---|---|
| Zerodha key | 206 | **218** |
| session-complete in both feeds | 178 | **207** |
| ... and free of known stock defects | 169 | **196** (194 if the Zerodha attack's F4 09:15 minutes of COROMANDEL and ATHERENERG are counted; the audit counts F4 as minor) |
| minute-perfect | 0 | 0 |
| GDFL sessions missing | 13,027 (32 stocks) | **332** (8 stocks) |
| Zerodha sessions missing | 17,245 + 132 | **94** (FORCEMOT 75, ICICILIFE 19) |

**What changed in the table.**
- **Newly complete: 29.**
  - From gap A: ATHERENERG, COROMANDEL, CUB, MAHABANK, NAVINFLUOR and SAGILITY.
  - From gaps C and D: ABB, ABCAPITAL, ADANIENSOL, ADANIGREEN, ADANIPOWER, ANGELONE, CDSL, CGPOWER, COCHINSHIP,
    DMART, GVT&D, JIOFIN, JSWENERGY, KPITTECH, MAXHEALTH, NAM-INDIA, PATANJALI, PGEL, PNBHOUSING, POWERINDIA, SUZLON
    and UNOMINDA.
  - From gap E: MAZDOCK.
- **Newly free: 27,** the same list less PATANJALI and MAZDOCK, which carry the adversary's S4 and S3 defects.
- **The other 181 stocks are unchanged.** For each, the complete and free verdicts equal the audit's
  (`untouched-vs-audit.txt`).
  - Shard-1 stocks: the counts are identical.
  - Shard-2 and shard-3 stocks: the owed and held counts are lower by exactly the number of specials. Those shards
    counted specials in "owed"; this census counts them apart.
  - No stock lost completeness.

**Not session-complete now: 11. Each gap, with the door that fixes it.**

| stock | what is missing | reason | door |
|---|---|---|---|
| ANANDRATHI, DATAPATTNS, ENRIN, KARURVYSYA, SYRMA, UJJIVANSFB | GDFL 2026-08-03..09-30, 42 sessions each (252) | refused `restated_auction_band`: the journal keys were closed under definition 2 or 3 (`gdfl_import_journal.rs`:1574) | R3-22's restate door with R4-21 (not built in a32722c45, `gdfl_import_guard.rs`:57, :1137), its own rehearsed stage, then the same named `cli gdfl-import` over 2026-08-03..09-30 |
| ICICILIFE | Zerodha 1min..60min 2026-10, 19 sessions (key ICICIPRULI); GDFL 2026-10-22..29, 6 sessions | the rename to ICICILIFE is held (D-7311) | the operator's rename decision, then `POST /pull/spot` 1min 2026-10 and `cli gdfl-import` 2026-10-22..29 |
| FORCEMOT | 74 sessions 2023-10-26..2024-02-13 in both feeds, plus Zerodha 2019-08-19 | a suspension: no vendor day file under any spelling (attack-gdfl 6); its first NSE day is not served (gap G) | none for the 74; `POST /pull/repair` for 2019-08-19 (UNVERIFIED that Kite serves it) |
| PIIND | 27 withheld rung buckets on 4 days in 2019; S1 price-level mix in 2018-01..2020-01 | gaps G and H | re-derive as a new generation (D-3321 item 4, S-47); a Zerodha versioned repair for S1 |
| 360ONE | Zerodha 1day 2019-10-04 | gap G | `POST /pull/repair` (UNVERIFIED) |
| UNITDSPR | Zerodha 1day 2019-11-08 | gap G | `POST /pull/repair` (UNVERIFIED) |

**Complete, but with a known stock defect: 11.** IRFC, ITC, KFINTECH, MAZDOCK, NMDC, PATANJALI, RELIANCE, SIEMENS,
TMPV, VBL and VEDL. These are gap H: the adversary's S1-S4. The doors:
- the corporate-action table R4-26, which is not live;
- Zerodha versioned repairs;
- the read-time skip rules of the audit's `backtest-skip.tsv`.

**Gaps that hit almost every stock** (gap F; minute level, so no stock is minute-perfect):

| gap | stocks now | door |
|---|---|---|
| Zerodha 1day 2018-11-07 Muhurat bar | 176 (the audit's 171, plus the five 2018 shares of gap A) | `POST /pull/repair`, UNVERIFIED that Kite serves it |
| Zerodha 2021-02-24 15:45-16:59 | 190 (the audit's 184, plus 6 new keys listed by then); only CGPOWER, NTPC and TITAN hold it | Zerodha repair of 2021-02, UNVERIFIED; GDFL holds the window |
| GDFL 2019-05-30 morning (the vendor's file starts at about 11:58) | 182 | a GDFL re-delivery ask |
| Kite-hole minutes: Zerodha minutes missing where GDFL printed | 8,091 minutes over 217 keys (this census's count, one method for all 218; the audit's 8,219 mixed three shard methods) | not fixable from Zerodha; GDFL holds the trades. 17 days are market-wide |
| auction-era stock-days with live 15:15-15:29 trades that Zerodha omits by D-3320 | 56: the audit's 44, plus each new key's first eligible day (attack-zerodha 6.1) | by design; the operator should review D-3320's first-day dating with gaps F-H |

**Not gaps:**
- Zerodha 30 Oct minutes, which arrive with pass 4.
- GDFL 30 Oct, which is not uploaded and not owed.
- The 30 Oct Zerodha 1day bar, which is held and provisional. The app shows it outside this table's window.

## 5. Speed: the operator's O(1)-at-p99 rule

**The random run** (`bin/verify-speed.js`, `speed-1800.tsv`, `speed-1800-analysis.json`).
- **Size.** 1,800 GET requests, one at a time, of randomly chosen stock-days (seeded).
- **Routes.** The /db page's own routes, at a32722c45:
  - `/bars/window.json` (`chart.js` `pageRequest`, `bars-plan.js` `windowQuery`, limit 1000), for the stored rungs:
    Zerodha 1min, 2-60min and 1day, and GDFL 1s;
  - `/sources/bars.json`, for the rungs the server folds on request: GDFL 1min-60min and 1day.
- **Spread.** 18 series × 100 requests, stratified evenly over history length (short < 48 months held, medium 48-95,
  long ≥ 96) and month age (2018-19, 2020-22, 2023-24, 2025-26).
- **Answers.** All HTTP 200. Two fold answers were empty: ICICILIFE, whose GDFL key is ICICIPRULI (D-7311).

| | n | p50 | p95 | p99 | max |
|---|---|---|---|---|---|
| **all** | 1,800 | **2.88 ms** | **10.27 ms** | **12.76 ms** | **66.16 ms** |
| Zerodha (window) | 855 | 1.49 | 2.72 | 3.38 | 66.16 |
| GDFL 1s (window) | 95 | 2.86 | 4.95 | 65.97 | 65.97 |
| GDFL folds 1min..1day | 850 | 5.1-8.8 | 9.5-13.4 | 11.6-17.5 | 17.5 |
| history short / medium / long | 608 / 603 / 589 | 2.54 / 2.67 / 3.52 | 8.91 / 9.03 / 11.30 | **12.10 / 12.37 / 12.92** | 66.2 / 66.0 / 16.6 |
| month 2018-19 / 2020-22 / 2023-24 / 2025-26 | 456 / 456 / 456 / 432 | 3.17 / 2.93 / 2.66 / 2.75 | 10.5 / 10.7 / 9.8 / 10.1 | **14.9 / 12.9 / 12.7 / 12.7** | 66.0 / 15.5 / 14.9 / 66.2 |

The app's own server-side timing for the last hour agrees (`app-speed-json-at-0322.json`):
- `/bars/window.json`: n 16,854, p50 0.9 ms, p99 5.6 ms;
- `/sources/bars.json`: n 1,825, p50 5.6 ms, p99 13.3 ms.

**Does time grow with history length or month age? No.** In the random run, time is rank-correlated with months
held inside each series (Spearman about 0.3-0.7). I tested that correlation four ways (total of all timed requests in
this stage: 15,131):

1. **History length, everything else fixed** (`bin/verify-speed-sameday.js`, `speed-sameday.json`). All 218 stocks,
   the same day, the same request, 5 rounds round-robin: 3,270 requests.

   | series | rho | median ms, short / medium / long |
   |---|---|---|
   | Zerodha 10min, 2026-08-20 | 0.043 | 0.210 / 0.210 / 0.217 |
   | Zerodha 1min | -0.016 | 0.443 / 0.444 / 0.446 |
   | GDFL 1s, 2026-07-20 | 0.006 | 0.916 / 0.920 / 0.922 |

   The 5% line is 0.135, so there is **no dependence on history length**.
2. **Month age, everything else fixed** (`bin/verify-speed-monthage.js`, `speed-monthage.json`). 8 long-history stocks
   × every month they hold × 3 series × 3 rounds: 7,440 requests. Median ms from the 2018-19 months to the 2025-26
   months:
   - Zerodha 1min: 0.481 to 0.476, flat;
   - Zerodha 10min: 0.274 to 0.255. A 10-minute day holds 38 bars before 2026-08-03 and 36 after (the 375- and
     360-minute sessions of D-3320), so the payload explains the 7%;
   - GDFL 1s, 1,000-row pages: 1.136 to 1.076, within 6% and not monotone (1.099 and 1.110 between).

   Nothing grows with the month's age.
3. **First touch** (`bin/verify-speed-firsttouch.js`, `speed-firsttouch.txt`). 9 short-history and 27 long-history old
   keys that no stage touched tonight, the same day, asked for the first time:
   - first-touch medians, short against long: 0.87 / 1.01 ms (30min), 0.92 / 1.04 (3min), 1.69 / 1.59 (1day),
     3.82 / 3.83 (GDFL 1s);
   - warm medians: 0.25-0.34 ms for both.
4. **Why the random run correlates** (`bin/verify-speed-locality.js`, `bin/verify-speed-workload.js`).
   - **Cache locality from the stratification.** The short bucket has only 12 stocks, so their files recur: 10-44% of
     their requests re-read a file already read in the run, against 0-3% for long histories. Among the first request
     to each stock, rho against months held is **-0.019** (Zerodha windows, 178 stocks).
   - **The folds track the work.** A fold's time follows the 1s records it folds for that ONE day (rho 0.66-0.88), and
     that work is bounded by the session (at most 22,500 seconds). With that work held fixed, the partial rho against
     months held is **-0.036** and against month age **-0.071** (`speed-fold-partial.json`).

**The maximum: rare pauses.** Over all 15,131 timed requests, 8 took more than 20 ms (0.05%):

| run | request | ms |
|---|---|---|
| random | ATHERENERG Zerodha 1min 2026-08-19 (18 months held) | 66.0 |
| random | ENRIN Zerodha 15min 2026-08-31 (17) | 66.2 |
| random | NYKAA GDFL 1s 2026-08-20 (60) | 66.0 |
| warm re-ask | KFINTECH Zerodha 1min 2024-07-25 (47) | 23.1 |
| warm re-ask | IREDA GDFL 1min fold 2024-07-31 (36) | 21.6 |
| same-day | BANKBARODA Zerodha 10min 2026-08-20 (106) | 48.4 |
| month-age | RELIANCE Zerodha 1min 2018-01-12 | 25.2 |
| month-age | RELIANCE GDFL 1s 2026-08-14 | 76.3 |

- **Where they fall.** They hit short and long histories, old and new months alike, so they are not a history or
  age effect.
- **What they do to the percentiles.** They set the maximum, never the p99.
- **A possible cause.** The random run (03:15:26-03:15:34) started 1 s after another writer's pull ended (03:15:25,
  section 3), and the app may still have been busy with it.
- **What that does not explain.** The later spikes came with no writer running. **The cause of these pauses was not
  identified (UNVERIFIED).**

## 6. Zero-error summary: every check of the whole run

"Diff." is the number of differences each check found. Every one is explained in the last column; **none is
unexplained.**

| stage | check | items checked | diff. | the documented rule that explains each difference |
|---|---|---|---|---|
| rehearse GDFL R2 | import failures classified (`gdfl-rehearsal-r2/analysis/failures-classified.txt`) | 94 failure lines | 94 | 49 `series_not_stored:BL` + 3 `series_not_stored:T0`: `gdfl_cm.rs` SHARE_SERIES (EQ, BE, BZ only). 42 `restated_auction_band`: `gdfl_import_journal.rs`:1574, R3-22/R4-21 not built. 0 unnamed |
| rehearse GDFL R2 | sessions owed held, 30 stocks (`analysis/per-stock.tsv`) | 30 stocks | 253 missing | 252: the no-door days above. 1: PATANJALI 2020-01-24, not a session for it (audit C4) |
| rehearse GDFL R2 | bar-tree joins and other writes (`changed-files.txt`, `other-writes.txt`) | 9,885 changed paths | 0 unplanned | |
| rehearse GDFL R2 | verify-months, live (= before) against R2 | 2,443 months, 623 held days | 0 lost | 12,742 days added, as planned |
| rehearse GDFL R2 | versioning on disk, sidecars | 34 months, 320 revisions | 0 | |
| rehearse GDFL R2 | served on :9937 against a file recount | 13 values | 0 | |
| rehearse Zerodha R1 | 17 calls on the shadow | 17 answers, 291 member failures, 202 drops | 493 | 288 coverage gaps: D-3335 (Kite sent no row). 3 `foreign_level`: WP-13d, D-3351. 202 dropped `AtOrAfterSessionClose`: session.rs DropReason, D-3320 |
| rehearse Zerodha R1 | sessions, census form | 17,169 + 134 | 5 | 2018-11-07 Muhurat 1day, Kite does not serve it (gap F) |
| rehearse Zerodha R1 | D-3320 375/360 | 61 days × 14 stocks | 4 days | COROMANDEL 2026-08-07, 08-19, 09-03, 09-25, 1-2 minutes each: Kite sent no row (D-3335) |
| rehearse Zerodha R1 | refold of derived rungs | 8,050,699 bars | 0 | |
| rehearse Zerodha R1 | other Zerodha files, md5 | 1,448,011 | 0 | 32,844 added, all planned |
| rehearse Zerodha R1 | live store untouched | census and bars | 0 | the live journal grew by another workflow's 30 Oct pass 4 (418 records, 0 bars), not this run |
| attack GDFL | raw-tick recompute (`raw-r2-summary.txt`) | 13,365 stock-days, 39,330,015 s | 0 | |
| attack GDFL | held bars decoded; months; revisions; appended prefixes | 2,473 months, 2,852,399 records, 320 revisions, 22 months | 0 | |
| attack GDFL | live list against R2; determinism P1 = R2 | 1 list; 9,885 paths | 0 | only the store root differs, by design |
| attack GDFL | owed sessions; filing and alias spans | 50,528; 7 renames | 253 | as rehearse |
| attack GDFL | the rehearsal's text | | 3 wording errors, 4 false-STOP risks | no data effect; each risk fails closed (attack-gdfl.md sections 9-10) |
| attack Zerodha | files, bars, sessions, minutes (`zcheck`) | 7,587 files, 14,509,543 bars, 155,907 session-rung checks, 6,467,310 minutes | 5 + 3 | 5: Muhurat 1day (gap F). 3 `gapLineMismatch`: the listing-day `refused_day` lines (WP-13d) |
| attack Zerodha | refold, with a bite test | 8,050,699 bars | 0 | 66,796 padded-only buckets have no bar by design (D-7270) |
| attack Zerodha | Kite's raw rows against stored bars | 6,441,730 + 17,280 rows | 206 not stored | 181 at or after close; 21 between the 2021-02-24 windows (calendar OUTAGE_SESSION); 3 `foreign_level`; 1 impossible OHLC (`pull.decode`) |
| attack Zerodha | calendar, eligibility, listing days, lists ×4, live list, live unchanged | 2,187 days, 854, 12, 1.45 M files ×4, 17, 1,448,011 | 0 | |
| attack Zerodha | data limits named | | 3 classes | D-3320 first eligible day (180 minutes); five 09:15 minutes kept by D-3347 and WP-13d; halt minutes filed as `vendor_empty` (calendar HALTS) |
| attack Zerodha | the rehearsal's text | | 4 corrections | for example 18 stock-sessions counted twice; no data effect |
| live GDFL | precheck gate 1 | full lists | 4 paths in audit/ | another workflow's Zerodha 30 Oct landing, 01:05:25-01:06:35, with no rows and no bar, manifest or import write. Accepted only by the exact diff (`accept-audit-diff-30oct-zerodha-landing.txt`) |
| live GDFL | precheck gate 2 | full lists, journal prefix, census, masters, tick store | 0 | |
| live GDFL | receipt against R2 | 3,084 lines | 5 lines | run id, pid, store root, by design |
| live GDFL | step 2: census 30/30; verify-months 2,443; joins 9,885; rollup 98 months; /db | | 1 | rollup try 1 still answered the previous index (`census_rollup.rs`, `facets.rs` `index_or_previous`; attack risk 10.1). Try 2 is byte-identical to R2 |
| live GDFL | sha256 against R2 | 10,042 paths | 796 + 2 | only landed_at, promoted_at, run id, pid, times, store root, and the CRCs and offsets that follow from them; field-checked (`fieldcheck-summary.txt`) |
| live GDFL | find newer than the run | 10,535 entries | 0 unplanned | |
| live Zerodha | 17 calls against the rehearsal | 17 receipts, 325 journal records | 0 | |
| live Zerodha | lists; live = shadow after; census delta; gap lines | 1,480,855 files, 954 rows, 7,646 lines | 0 | |
| live Zerodha | proofs: refold, bars per stock, first/last, census | 8,050,699; 14 × 9; 14 | 6 stocks' notes | the GDFL cross-check notes changed because the GDFL stage had just imported those stocks' 1s (`census-stock.js` reads GDFL for each short day) |
| **verify** | /db at start and end | 2 | 0 | |
| **verify** | census against the app calendar, day by day (2b) | 370 series, 37 stocks | 0 | |
| **verify** | served bars against the shadow (2c) | 720 stock-days, 599,230 bars | 4 bars | OI `null` against a stored `0`: D-3322, `bars_route.rs`:38-45. The files are equal |
| **verify** | attackers' tools re-run on live: ag-owed, ag-months, atk-zcheck | 30 stocks; 2,473 months; 14 keys, 14,509,543 bars | 253 / 0 / 3 | 253 named above; 3 `refused_day` (WP-13d). Every output equals the shadow run |
| **verify** | re-diff of both stages' lists (3) | 8 list pairs, about 2.4 M paths | 0 unplanned | |
| **verify** | unplanned paths recomputed now | 5,800 paths (4,942 by content) | 0 | |
| **verify** | whole store changed since 02:00; directories since 02:41 | 51,510 paths; every directory | 0 unexplained | the 23 directories after 02:41 are the 03:14 pull's `verify/` folders and app telemetry |
| **verify** | another writer during this stage: a Zerodha pull, 03:14:16-03:15:25 | 254 changed paths since 03:14 | 0 bar, pointer, revision or manifest files | another workflow's 30 Oct pass-4 attempt (208 chunks, NO ROWS AT ALL). It wrote audit ledgers, raw answers, verify spool and the landing lock, as the 01:05 attempt did |
| **verify** | rollups and feed totals, start against end; audit ledgers | 3 + 2; 7 | 0; 5 grown | the 5 ledgers grew by the 03:14 pull only. `zerodha.repairs` and `gdfl.repairs` are unchanged |
| **verify** | per-stock table (4) | 218 stocks | 11 incomplete, 11 with defects | each named with its door. The 181 untouched equal the audit's verdicts; owed and held differ only by the specials basis |
| **verify** | rung fold mismatches decoded | 26 stocks | 66,858 + 27 | padded-only buckets (D-7270); PIIND's withheld buckets (gap G) |
| **verify** | speed (5) | 15,131 timed GETs | 0 | O(1) holds. 8 pauses over 20 ms (max 76 ms) set the maximum, not the p99; their cause is UNVERIFIED, and they are not a data difference |

**proved = true.** Every difference above is explained by a documented rule, and no unexplained one remains.

## 7. Problems, honestly

- **Rule slips in the run, carried from the GDFL live stage.** Both are process slips, not data, and the stage
  reported both.
  - It ran `rm -f` on a scratch copy of its own new script (`bin/precheck-live-gdfl-r2.sh.tmp`). This breaks the
    letter of "never delete anything".
  - A zsh word-splitting slip left four empty files in `live-gdfl-evidence/`: `join-live-.tsv`, `.r2-join-.sorted`,
    `.live-final-` and `.r2-final-`. They are still there.
- **Defects in my own verify scripts, found and corrected.** None had a data effect.
  1. Run 1 of `verify-served-vs-shadow.js` read the wire prices as rupees, but the wire carries integer paisa. Every
     price compared unequal, while t, volume and oi were equal. Its output was moved aside as
     `served-vs-shadow.run1-wrong-unit.tsv`, not deleted.
  2. Run 2 found the 4 D-3322 rows. Its outputs were moved aside as `*.run2-before-d3322.*`. Run 3 names that rule.
  3. Some first-run outputs were overwritten by the corrected reruns. They are logged in `COMMANDS.txt`.
     - The first build of `per-stock-after.tsv` read only the first key on the two direct reads. That made
       TATACONSUM look like it had no 2019-05-30 data, and miscounted the auction days.
     - The first sample pick missed the versioned months. No comparison ever used it.
     - The first run of the changed-path classifier had a wrong epoch for 02:39:51, and the Zerodha manifest unnamed.
- **Another writer ran during this stage.** A Zerodha pull of 2026-10-30 ran on `:8080` at 03:14:16-03:15:25 IST,
  run 1793396656, without the live-writer lock, as at 01:05.
  - Every chunk answered with no rows, and it wrote no bar, pointer, revision or manifest file (section 3).
  - It came after my census (02:47-02:49), sample comparisons (02:57-03:00) and whole-store find (03:00-03:11), and
    before the speed runs.
  - It was not this stage's to stop, and nothing was retried.
- **Free space went up by 869,692,536 KiB (about 891 GB)** between 02:47 and 03:19 IST, from outside the store and
  not by this stage (section 3).
- **Not verified here:**
  - the cause of the 3 requests at 66 ms;
  - whether Kite serves any gap-F or gap-G day (each "UNVERIFIED" door above);
  - the GDFL side of the first eligible day of UJJIVANSFB, ANANDRATHI and ENRIN, 2026-09-29, where GDFL holds no
    month.

## 8. Files

- **The report:** this file, `verify.md`.
- **The table:** `per-stock-after.tsv` (218 rows) and `per-stock-after-summary.json`.
- **Scripts:** `bin/verify-lib.js`, `verify-served-vs-shadow.js`, `verify-touched.js`, `verify-pick-samples.js`,
  `verify-pick-extra.js`, `verify-per-stock.js`, `verify-fold-classify.js`, `verify-rediff.js`,
  `verify-unplanned-sample.js`, `verify-classify-changed.js`, `verify-speed.js`, `verify-speed-analyse.js`,
  `verify-speed-control.js`, `verify-speed-firsttouch.js`, `verify-speed-sameday.js`, `verify-speed-monthage.js`,
  `verify-speed-locality.js` and `verify-speed-workload.js`. All are read-only toward the stores.
- **Evidence** (`verify-evidence/`):
  - the command log, `COMMANDS.txt`;
  - the census: `census-218-live/`, `per-stock-after-summary.json`, `fold-mismatch-classified.{tsv,txt}`,
    `untouched-vs-audit.txt`;
  - census against the app: `touched-census-vs-app.tsv`;
  - samples and served bars: `samples*.tsv`, `served-vs-shadow*.tsv`, `served-vs-shadow-per-stock.tsv`;
  - the attackers' tools on live: `ag-owed-r2-vs-live.tsv`, `ag-months-r2-vs-live/`, `zcheck-live/`;
  - re-diffs and samples: `rediff-*`, `unplanned-sample*`;
  - changed paths: `store-changed-since-0200*`, `store-dirs-changed-since-0241*`, `store-changed-since-0314.txt`,
    `pull-notes-appended-0314.txt` (the other writer's notes: text only, no price);
  - rollups and writers: `rollup-*-{now,end,0335}.json`, `audit-files-now.txt`, `audit-files-end.txt`,
    `writers-start.txt`, `end.txt`;
  - speed: `speed-*`, `app-speed-json-at-0322.json`.

# 07 — The plan, the requirements, and what is actually done

**This file exists because the plan kept living in conversation.** It was stated
a dozen times and re-stated every time it was asked for, which means it was
never anywhere a reader could check it against the code. A plan that is not
tracked is not a plan; it is a memory.

Read with `docs/06-limits.md` beside it. That file records what is *not*
constant-time and what is unmeasured. This one records what is *not built*, in
what order it must be, and why.

Status words mean exactly one thing each:

| Word | Means |
|---|---|
| **DONE** | Landed on `feat/pull`, gates green, and named here with its commit |
| **NEXT** | Nothing blocks it; no permission needed |
| **BLOCKED** | Waiting on a decision or a fact this repository cannot see |
| **OPEN** | Real, measured, not started |

---

## 0. How the operator runs this

Stated first because it is the requirement everything else serves, and because
it changed: until D-0064 and D-0068 there were **two** processes on two ports.

### The whole procedure

1. `git clone`
2. Open the directory in IntelliJ.
3. Press **Run** on the `api` binary — the run configuration named
   *"brutex — the whole application (press Run on THIS one)"*.
4. Open <http://127.0.0.1:8080>.

**No commands. No `npm`. No second process. No Node on the machine.** The
binary serves the front end itself, the store opens, and the autopilot drives
the backfill without being asked — `crates/api/src/autopilot.rs`, and
`/autopilot.json` reports what it is doing.

### Why that works with no Node

`web/build` is **committed** (D-0068), and `crates/api` reads it from disk at
request time rather than embedding it (D-0064). So a clone already contains the
built front end, and `cargo build` never looks at it — CI gate 1e proves that
by building the workspace with `web/` moved aside.

### What is honest about it

* **The first Run compiles the workspace in release and takes minutes.** It is
  one click, not one second. Later runs are incremental.
* **The run configuration itself is not in the clone.** `.claude/` is ignored by
  root `.gitignore:30`, and a tracked IntelliJ `.run/*.xml` is an extension gate
  1 forbids outside `web/`. What the operator actually gets today is IntelliJ's
  own Cargo auto-detection of the `api` binary, which does produce a working Run
  entry — but it is the IDE inferring it, **not this repository promising it**.
  Closing that needs a decision entry and a gate 1 amendment. **OPEN.**
* **`web/build` can be stale.** Nothing ties it to `web/src`; see
  `docs/06-limits.md` §38 and D-0068's cost list.

### The two other configurations, and when they are not what you want

| Configuration | Use it for | Not for |
|---|---|---|
| `brutex — the whole application` | Everything. This is the one. | — |
| `web — Vite dev server` | Front-end development with hot reload, on port 5173. **Needs Node.** | Running the application. It serves only the page and proxies the rest |

---

## 1. The requirement, in the operator's own terms

Restated here so the plan can be checked against it rather than against memory.

| # | Requirement | Where it is enforced |
|---|---|---|
| R-1 | Spot, **every instrument**, bounded at ~800 — 750 NIFTY Total Market + ~35 NSE indices | `catalog::tracked`, and `/instruments.json` shares that exact predicate |
| R-2 | **2020-01-01 → yesterday**, never today | `finished_day_only` in `broker_window`, HTTP path only |
| R-3 | Day-level first, then one-minute | Selectable and landing under `1day/` — D-0055. Open only for Groww, whose daily interval word is unrecorded; see §4 item 4a |
| R-4 | F&O: **NIFTY only** as the first step | Not yet — **OPEN** |
| R-5 | TrueData / GDFL are **F&O CSVs from a folder**: no market hours, no rate limit, no token | `Transport::LocalArchive`, and every vendor rule keys off the transport |
| R-6 | **No vendor comparison anywhere.** One selected feed, always | Feed picker on `/store`; the counter cards and the row column both follow it |
| R-7 | **N feeds** appear everywhere with no edit | True of routing, the picker, budgets and `/feeds.json`. **Not** true of the descriptor table — see §5 for the measured number |
| R-8 | Rust only, **except the front end** | `CLAUDE.md` §2, D-0052, D-0053. Gate 1 by path; gate 1e proves the engine builds with `web/` absent |
| R-9 | **O(1)** everywhere it is claimed | Gate 8 measures at 1×/10×/100× and exits non-zero on a breach |
| R-10 | Feeds not owned must not be offered | `/feeds.json` reports `ready`; the picker disables. **Advisory only** — see §4 |

---

## 2. DONE

Each line names the defect, not the feature, because the defect is what a
reader needs to recognise if it returns.

| Was | Now | Commit |
|---|---|---|
| Census re-imaged and renamed **per window** — 424 GB of writes to maintain a 5.75 MB file | One 64-byte positional append plus a header commit | `a8cadb4` |
| `pull::rate::Governor` had **zero callers** | Held on the `Site`, charged **per request**, ahead of the credential read | `c1ea7ab` |
| A multi-year window sent **whole** to a vendor capped at 30 days | Split to the descriptor's cap, per rung as of D-0055; 2020→yesterday is 126 legal requests at Groww's one-minute cap and 80 at either feed's day rung, credential read once. The 81 this row first claimed predates the month-boundary clamp | `bf8f86e`, `0596621` |
| `read_dir` stripped two extensions unconditionally — **215 calls merged with their own puts per day** | The rule is the extension's *shape*, not a count | `a91026e` |
| A blank `folder` textbox decided which **protocol** to speak | The feed's declared transport decides | `d2fa20c` |
| The page offered two hardcoded feeds, so the archive arm was unreachable | Built from `Feed::ALL` | `f8be537` |
| `run_local` hardcoded `Vendor::Dhan` — **194 instrument-months of GDFL futures filed under `bars/dhan/`** | Archive feeds have their own store prefixes | `44ce731` |
| `/store` showed `Groww rows` beside `Dhan rows` | One feed, selected; cards follow the column | `2efdce7`, `6602a9a` |
| Front end was server-rendered Rust HTML | Svelte 5 + `lightweight-charts` (TradingView's own), 375 real candles | `4a5953f`, `f34875b`, `6dd2e97` |
| `totp` appeared **zero times** in the workspace | RFC 6238 generator, all six Appendix B vectors reproduce | `b96294f` |
| `StoreFilter::keeps` allocated **two Strings per row**, and the comment said it did not | Neither side allocates; needle folded once at construction | `7661a61` |
| `census::filtered` copied the **whole table** to draw 200 rows | `Cow`, borrowed when unfiltered | `7661a61` |
| §32 asserted `/store` was constant-time **while two linear costs sat on it** | Corrected; both measurements recorded | `7661a61` |
| A run that stored **nothing** reported `STORED` | `Outcome::Empty` — "STORED NOTHING", loud, code appended as 4 | `62274e0` |
| `Timeframe::DAY_1` existed in the store and **nothing could reach it** — the spot form had no bar-length control and `land_one` wrote `Timeframe::MINUTE_1` as a literal | The rung is a control on the form, built from the descriptors; it lands under `1day/` | D-0055 |
| The window cap was **one scalar per feed**, qualified by a granularity in prose only | A lookup on (feed, rung). A daily window is no longer split at the one-minute cap | D-0055 |
| A feed with no published cap sent the window **whole**, which `fetch::land` refuses for spanning two months | `split_window` takes an `Option` and the month boundary binds at every rung | D-0055 |
| `BarRequest` stated the rung and the `Cadence` **independently**, and a daily pull left at `Cadence::Minute` drops every bar as `BeforeSessionOpen` and reports a clean zero | One field; the cadence is derived from it | D-0055 |

---

## 3. BLOCKED — and on what, exactly

| Item | Blocked on | Why this repository cannot decide it |
|---|---|---|
| Move the 194 misfiled instrument-months out of `bars/dhan/` | The operator | It is a delete and a manifest rebuild over their data. A backup exists at `~/.brutex/store.backup-1786174250` |
| Wire the TOTP **mint** | Whether `tickvault` still calls Dhan | A broker issues one active token per client. Minting here kills whatever `tickvault` holds, and `tickvault` is not visible from this repository. `CLAUDE.md` §8 |
| Dhan pulls at all | The same | The token in Parameter Store has not been refreshed since 2026-07-25 while Groww's is refreshed daily. This repository never mints, by rule |
| `TrueData` descriptor: `Segment::Index` vs F&O-only | A real bought archive | The sample proved the layout; `MemberPattern::SymbolAtRoot` is wrong for the futures family, which nests under `Contract Futures/` |

**Groww is not blocked.** The backfill can run single-sourced today.

---

## 4. NEXT — in dependency order, no permission needed

| # | Item | Why this position |
|---|---|---|
| 1 | **Drive one Groww pull end to end from the Ingest page** | Nobody has closed the loop once. An 11,200-request backfill on an undriven loop is how a half-written history happens |
| 2 | **Make the readiness gate a rule, not a courtesy** | `/feeds.json` reports `ready` and the picker disables — but `parse_feed` accepts any feed, so a `curl` bypasses it. The first attempt used the census as the signal, which is **circular**: a feed just bought holds nothing and could never be pulled. Needs an entitlement signal that is not the census |
| 3 | ~~**Serve `web/build` from the Rust binary**~~ — **done: D-0064 serves it from a directory read at run time, D-0068 commits the output.** See §0 | This row used to say "assets embed the way `STYLE` already does". **That was wrong and it is worth recording why:** an embed resolves at compile time against a path under `web/`, and CI gate 1e builds with `web/` *moved aside* — so embedding makes the crate fail to compile under the gate's own premise. The assets are read from disk instead |
| 4 | ~~**Day-level mode before one-minute** (R-3)~~ — **selectable and landing under `1day/` as of D-0055.** What is left is one vendor fact, below | The operator's stated first step. The saving is smaller than this row used to claim; see the corrected arithmetic |
| 4a | **Read Groww's daily `candle_interval` word off a live call and write it into one descriptor row** | Its request names the bar length in a parameter and the daily spelling is recorded nowhere — `1day`, `1d` and `day` are all plausible and only one is a request. A daily pull against that feed refuses by name until it is recorded. Dhan needs nothing: its request carries no interval field at all |
| 5 | **The 2020 → yesterday backfill** | The goal |

### The arithmetic behind #5, corrected

2020-01-01 to 2026-08-07 is **2,411 days inclusive**, and — this is the number
that was missing — **80 calendar months**.

| Rung | Cap | Windows / instrument | × 800 instruments |
|---|---|---|---|
| Day-level, Groww | none published | **80** | ~64,000 requests |
| Day-level, Dhan | none published | **80** | ~64,000 requests |
| One-minute, Groww | 30 d | **126** | ~100,800 requests |
| One-minute, Dhan | 90 d | **80** | ~64,000 requests |

**This table replaces one that claimed a 180-day daily cap and 14 windows.**
That cap appears in no source: `docs/00-charter.md` §4 records a day-level
figure for neither vendor, and D-0054 cited §4 for "14 at day level" while §4
said nothing of the kind. Both are corrected here and in D-0055.

**14 was unreachable regardless, and by a bound this repository owns.** The
store addresses one month per file and `pull::ingest` refuses a batch spanning
two, so the floor is one request per month — 80 — whatever a vendor allows. The
real saving of the daily rung is **126 → 80 on Groww and nothing at all on
Dhan**, whose 90-day cap was already wider than any month.

The daily rung is still the right first pass, for the reason that survives the
arithmetic: **one bar per day instead of 375**, so a wrong symbol, a dead
credential or a missing floor surfaces against a store one three-hundred-and-
seventy-fifth the size.

This is arithmetic from `docs/00-charter.md` §4 and from the calendar, **not a
throughput measurement**. No backfill has been run, so no duration is claimed.

---

## 5. R-7, measured rather than asserted

"N feeds appear everywhere with no edit" was said repeatedly in conversation and
is only partly true. A fifth feed was **actually added in a scratch clone and
compiled to green** on 2026-08-08. The result:

| Layer | Edits | Note |
|---|---|---|
| Routing, feed picker, `/feeds.json`, budgets | **0** | All walk `Feed::ALL`. Verified: the N-feed tests passed with the fifth feed present |
| `crates/pull/src/vendor.rs` | 12 | **4 are deliberate** — the file destructures its own arrays so a half-added feed is a compile error, not a runtime surprise |
| `run_local` / `broker_answer` plans | 2 | Accidental duplication |
| + a CSV layout not already in `csv::Columns` | +5 | A second hand-written copy of what the descriptor already says |
| + its own store prefix (`core::Vendor`) | +8 | `VendorSet(u8)` caps the total at 8 feeds |

**20 edit sites to compile.** But the compile count is the least interesting
number: `cargo test --workspace` then failed **29 tests across 7 targets**, and
the compiler could not see the two worst problems.

Three descriptor fields **cannot express an arbitrary broker at all**:

* `HttpSpec::bars_path` is a fixed string concatenated with `base_url`. A vendor
  whose instrument and granularity are *path segments* cannot be described.
* `AuthScheme` is `Raw | Bearer`. A scheme carrying a prefix and a **second**
  secret cannot be described.
* `TimestampEncoding::IsoDateTimeText` documents itself as carrying no zone. A
  vendor returning `+0530` cannot be described.

All three compile as lies. **The table is genuinely N-feed for a vendor shaped
like Dhan or Groww. It is not N-feed for an arbitrary broker**, and saying
otherwise was overstatement.

Two costs that made this worse have since been removed: `pull::config` demanded
a credential table from every vendor (an archive has none), and `api::merge`
required every vendor to agree (an archive publishes no master). Both asked a
question the wrong set could not answer. `44ce731`.

---

## 6. How O(1) is kept, not promised

`CLAUDE.md` §3 rule 4 says a change that makes a named operation scan **fails
the bench gate**. That gate is real and it is the answer to "how is this
ensured":

* `cargo bench --workspace` measures each operation at **1×, 10× and 100×**
  input, divides, and `std::process::exit(1)` on a ratio past **3.0×**.
* Picoseconds and integer permille, so no float is anywhere near the comparison.
* An **unmeasurable baseline is a failure**, not a divide-by-something —
  reporting a ratio nobody measured is what §3 rule 6 forbids.
* Gate 14 separately proves the benches *exist and have the shape of a ratio
  measurement*, so deleting one to make gate 8 pass fails a different gate.

Last run: **`rc=0`**, every ratio inside the ceiling at 100× input.

**Where it does not yet reach.** `engine`, `vocab` and `indicators` have no
implementation, so four of the five operations rule 4 names — condition lookup,
mask evaluation, duplicate rejection, result append — have nothing to measure.
Gate 14 goes red the day they land without benches. That is designed, not
overlooked.

---

## 7. OPEN — real, measured, not started

| Item | Evidence |
|---|---|
| GDFL futures unreadable | `MemberPattern::SymbolAtRoot`, but they nest one level deeper. **All 642 files invisible** |
| 596 decimal-strike contracts per day decode then vanish | `.` is not in the store's legal identifier byte set |
| TrueData 2025 indices store nothing | 88,885 rows read, **0 stored** — `NIFTY 50` and `INDIA VIX` contain spaces |
| The only true 1-minute archive product is unreadable | 8 fields, and the time is `09:15` — the parser needs seconds |
| No zip reader anywhere in the workspace | Every archive must be hand-extracted |
| `PriceScale::Rupees` in both archive descriptors | The decoder already emits paisa. Dormant ×100 error, masked only by `run_local` hardcoding `Paisa` |
| `RecordShape::Snapshot` unenforced | Its doc says a snapshot feed must refuse rather than be "stored as a bar with the price repeated four times — a lie written into the data itself". That is exactly what reaches disk |
| The archive half of the descriptor table has **zero non-test consumers** | Wiring it as-is would activate the ×100 scale and the wrong member pattern. The rows must be corrected first |

---

## 8. What this file is not

It is not a promise of dates, and it names no throughput that was not measured.
Where a number appears it is either arithmetic from a published cap (§4, labelled
as such) or a measurement with its method stated. `CLAUDE.md` §3 rule 6.

---

## 9. The completeness guarantee: nothing missed, nothing corrupted

The requirement, in the operator's terms: **"irrespective of any situation, not a
single bar should be missed or corrupted."** Two different guarantees needing two
different mechanisms, and conflating them is why "be careful" is not an answer.

### 9.1 Not corrupted — achieved by construction

Corruption is prevented at the moment of writing, and every mechanism below is
already in place.

| Threat | Mechanism | Where |
|---|---|---|
| A torn record | CRC-32C per 56-byte record, verified on read | `store::block` |
| A torn header | Two slots, written alternately; recovery takes the newest passing CRC | `store::header` |
| A counter published over absent bars | Entry write, **barrier**, then the slot that counts it | `pull::ingest` |
| A vendor restating history | Overlap verified byte for byte; a differing bar **refuses** | `store::file::suffix_that_follows` |
| Two runs racing | `CensusLock` taken before the read, so read-modify-write is atomic | `pull::ingest` |
| A full disk | Returned, never signalled — no writable mapping, by rule | `CLAUDE.md` §4 |
| A half-written day | Never requested: yesterday is the newest day asked for | `finished_day_only` |

**This half is done.** Kill the process at any instant; the store recovers, the
census recovers, and the rerun resumes rather than refusing.

### 9.2 Not missed — NOT achievable by careful writing

A gap is the absence of a write. Nothing at the write boundary can see it,
because there is nothing there to look at. Retries, backoff and isolation all
reduce how *often* a gap appears; **none of them can tell you none remains.**

The only construction that yields the guarantee:

```
1. EXPECT   — for each (feed, instrument, day) that should exist, say so
2. OBSERVE  — read what the store actually holds. The census is O(1) per probe
3. DIFFER   — expected minus observed. This set IS the remaining work
4. FETCH    — pull only the difference
5. REPEAT   — until the difference is empty TWICE in a row
```

Twice, not once: a round that finds nothing may have found nothing because a
vendor was down, not because nothing is missing. Two consecutive dry rounds
distinguish "complete" from "unreachable".

**This makes every failure mode in §10 a delay rather than a loss.** A timeout, a
5xx, an expired token, a killed process — each leaves a gap, and the next round
finds it and refills it. The pull loop stops needing to be perfect, which is
good, because it cannot be.

### 9.3 What EXPECTED means, and why it is the hard part

Step 1 is where honesty is required, and it is the part with no code yet.

| Question | Answer | Status |
|---|---|---|
| Which instruments? | The tracked universe, ~800 | ✅ `catalog::tracked` |
| Which days? | Trading days between the instrument's floor and yesterday | ❌ **no trading calendar in this build** |
| How many bars in a day? | 375 at one-minute, 09:15–15:29 inclusive (CAS, from 2026-08-03) | ⚠️ constant exists; holidays do not |
| When does an instrument's history start? | Per feed AND per instrument. Groww from 2020; Dhan is a **rolling** ~5 years that moves daily | ❌ no floor recorded |

**Without a trading calendar, "expected" cannot be computed exactly.** A weekend
and an exchange holiday are indistinguishable from a missing pull, and treating
either as a gap makes the loop never terminate.

`docs/06-limits.md` already records the absence honestly: the coverage swatches
on `/store` are quartiles of *the fullest month on the page*, "not of an ideal
month, because no trading calendar exists in this build to say what a full month
is."

So the completeness guarantee has a prerequisite, and it is **a trading calendar
derived from data rather than invented** — the set of days on which some
instrument reported bars is the exchange's own answer to which days were trading
days, and it needs no vendor to publish one.

### 9.4 The honest statement of the guarantee

Stated the way `CLAUDE.md` §3 rule 6 requires:

* **Corruption: guaranteed against, by construction, today.** Every mechanism is
  in place and named above.
* **Completeness: not guaranteed today, and cannot be until §9.2 and §9.3 exist.**
  A pull that is careful is not a pull that is complete.
* What *is* true today: no bar that was successfully written can be silently
  lost or altered, and a rerun cannot corrupt what a previous run wrote.

Anything stronger than that would be a claim without a measurement.

---

## 10. Interruptions during a pull, and what each does today

Enumerated because a 22,400-request run (daily) and a 129,600-request run
(one-minute) will meet most of these at least once.

| # | Interruption | Today | Needed |
|---|---|---|---|
| 1 | Process killed, laptop sleep | Written bars survive; rerun resumes | ✅ |
| 2 | Crash mid bar-file write | Two-slot header recovery | ✅ |
| 3 | Crash mid census write | Barrier before the counter | ✅ |
| 4 | Two runs at once | Lock held across read-modify-write | ✅ |
| 5 | Vendor throttles | AIMD decrease, charged per request | ✅ |
| 6 | Window past the vendor cap | Split automatically | ✅ |
| 7 | Partial trading day | Never requested | ✅ |
| 8 | Bars outside the window | Dropped and counted by reason | ✅ |
| 9 | Vendor restates history | Refused, not swallowed | ✅ |
| 10 | Disk full | Returned, never signalled | ✅ |
| 11 | **Token expires mid-run** | 401 for every later request; no re-read | ❌ **certain to fire** — daily reset 06:00 IST, and no 22,400-request run fits in one window |
| 12 | Network timeout or reset | Chunk errors, whole member fails, remaining chunks abandoned | ❌ retry with backoff |
| 13 | Vendor 5xx | Same | ❌ same path |
| 14 | One instrument fails | No loop yet; when there is one it must not abort the other 799 | ❌ per-instrument isolation |
| 15 | Restart loses progress | Rerun is safe but redoes everything | ❌ census as the progress ledger |
| 16 | Instrument younger than the window | Every request before its listing is wasted budget | ❌ per-instrument floor |
| 17 | Symbol renamed between deliveries | Measured: `NIFTY` (2022) vs `NIFTY 50` (2025) — two directories, one index | ❌ canonical alias, descriptor-driven |
| 18 | Instrument delisted mid-run | 404 read as a failure rather than "history ends here" | ❌ distinguish gone-from-master from transport error |
| 19 | Census disagrees with the bar files | Nothing notices | ❌ reconcile pass |
| 20 | Clock skew across DST | **Not a risk.** India observes no DST | — |

**#11 is the only one certain to fire**, because no run of this size fits inside
one 24-hour token. It is a hard stop today.

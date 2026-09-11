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
* **A run configuration IS in the clone; an *IntelliJ* one is not.** This row
  used to read "The run configuration itself is not in the clone" and say that
  closing it "needs a decision entry and a gate 1 amendment — **OPEN**". Both
  halves are now done and the row was the last thing that did not know it.
  `.claude/launch.json` is **tracked** — `.gitignore:30` ignores `.claude/`
  *except that one file*, and says so — and it carries exactly the two
  configurations the table below names, in the preview tools' format. Gate 1 and
  gate 1b were amended to admit `.json` under `.claude/`, and `CLAUDE.md` §2 and
  D-0210 closed the law half.

  **What is still true, and is the part that mattered:** IntelliJ does not read
  that file. It takes run entries from `.idea/` or `.run/*.xml`, neither tracked.
  An operator opening the clone in IntelliJ still gets that IDE's own Cargo
  auto-detection of the `api` binary — a working Run entry, but **the IDE
  inferring it rather than this repository promising it**. Promising it to
  IntelliJ specifically is still **OPEN**, and is a smaller question than this
  row used to state.
* ~~**`web/build` can be stale.** Nothing ties it to `web/src`~~ — **closed.**
  Gate W1 runs `npm run build` and fails on any diff under `web/build`, so the
  committed bundle is now tied to the source that produced it. This row cited
  `docs/06-limits.md` **§38**, which does not exist and never did; the subject
  lives in **§82**, which records what W1, W2 and W3 close and which two of the
  three are ratchets rather than floors. Both the dead reference and the false
  claim are corrected here. D-0068's cost list still applies to the commit
  itself.

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
| R-9 | **O(1)** everywhere it is claimed | Gate 8 measures at 1×/10×/100× and exits non-zero on a breach. **All eleven crates covered as of D-0103** — `vocab`, `indicators` and `engine` claimed bounds with no bench until 2026-08-11, and gate 14 was red for it. Two of the claims were false; see §6 |
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

### The prediction this section made came true, and is now closed

This section used to end: *"`engine`, `vocab` and `indicators` have no
implementation, so four of the five operations rule 4 names — condition lookup, mask
evaluation, duplicate rejection, result append — have nothing to measure. Gate 14
goes red the day they land without benches. That is designed, not overlooked."*

**They landed without benches and gate 14 went red, exactly as written.** Seventeen
cost claims across the three crates, none measured. Closed 2026-08-11 by D-0103:
three `benches/ratio.rs`, twelve invariant rows `C-V-01…04`, `C-I-01…04`,
`C-E-01…04`, and three rows in gate 14's own coverage table.

Four of the five operations rule 4 names are now measured:

| Rule 4 operation | Measured | Ratio |
|---|---|---|
| Condition lookup | `C-V-04` | 0.965× popcount, 1 bit → 234 bits |
| Mask evaluation | `C-V-01`, `C-V-02`, `C-V-03` | 0.998× hit→miss, **0.996× word 0 → word 5**, 1.037× k=1 → k=234 |
| Duplicate rejection | `C-E-04` | 0.842× per bar, 10,000 → 100,000 bars |
| Result append | **MEASURED** — `C-E-11` | 0.633×–0.811×. `C-04` still carried UNMEASURED long after this landed; both are corrected together |
| Bar lookup | `C-01` | already measured in `store` |

The 0.996× is the one that matters: an early-exit loop would return sooner on a
candidate failing in word 0, so a flat ratio across the six words is what says the
branchless implementation is the one that actually runs.

**Two claims turned out to be false when measured**, which is the return on writing
the benches at all. `isqrt_i128` was documented "O(1) for **every** `i128`" and its
cost varies **217×**, because the Newton loop exits on convergence — bounded, not
flat, now stated as the bound it is with the spread in `06-limits.md` §51. And a
candle's cost varies **1.87×** with its content, recorded in §52.

**Where it still does not reach.** Peak memory (`E-08`) alone — result append was closed by `C-E-11` and this paragraph is corrected with it. `E-08`
have no measurement, and both say so in `docs/04-invariants.md` with the word
UNMEASURED rather than a test name. `E-08` needs a declared budget before it needs a
measurement, and inventing a budget is what §3 rule 1 forbids.

---

## 7. OPEN — real, measured, not started

**This section was re-measured against the code on 2026-08-10 and four of its
eight rows were wrong.** They are corrected below and what each used to say is
kept in §7.4, because a row that was wrong once is the row a reader will
re-derive wrongly again. D-0083.

The rows are now **ordered by whether an operator can meet the defect today**,
which is the distinction this section did not carry and the one a reader needs
first:

* **§7.2 is live.** Reachable now, on the path `api::server::run_local`
  actually drives.
* **§7.3 is latent.** A wrong value in a descriptor field that **nothing
  outside a `#[cfg(test)]` module reads**. Correcting it changes no output
  whatsoever until §7.1 is closed. Three rows here used to read as active bugs
  and are not.

### 7.1 The root cause, first, because it re-reads every row below

| Item | Evidence |
|---|---|
| The archive half of the descriptor table has **zero non-test consumers** | Every read of an `ArchiveSpec` field in `crates/pull/src/vendor.rs` is at a line inside the `#[cfg(test)]` module that opens at **2645** — `archive_spec` (3729), `member_suffix` (3754), `spec.layouts` (3794), `.layout(…)` (3919, 3928, 3933). `Descriptor::record` is the same: read at 4011–4014 and nowhere else. Outside that file **every** consumer matches `Transport::LocalArchive(_)` and discards the spec — `api/src/server.rs:838, 1710, 2330`, `api/src/render.rs:1566`, `pull/src/emit_sites.rs:761`. And the path that actually ingests an archive, `run_local` (`api/src/server.rs:3950`), never asks the descriptor anything: it hardcodes `Columns::Gdfl`, `TimestampEncoding::EpochSecondsUtc`, `PriceScale::Paisa`, `Exchange::Nse` and `Segment::Fno` at 3999–4005 |

This row was already in the table and it was, if anything, **understated**. It
is not one more defect beside the others; it is why the others divide the way
they do.

### 7.2 LIVE — reachable today, on the `run_local` path

| Item | Evidence |
|---|---|
| GDFL futures are **unaddressable except by hand** | Measured on `GFDLNFO_TICK_01072025`: **642** futures CSVs, at `Futures/{-I,-II,-III}/{SYMBOL}-{series}.NFO.csv` — `Futures/-III/FINNIFTY-III.NFO.csv`. Options sit flat under `Options/` and resolve; futures nest a **continuation-series folder**, so the member path has **four** components where `MemberPattern::StemGroupSymbol` documents three — `{archive stem}/{group folder}/{symbol}{suffix}`, `vendor.rs:1700`. "Unreadable" was too strong: an operator who types the deepest folder into the Ingest page **does** get them, which is exactly how the 194 misfiled instrument-months in §3 happened — `ABB-III`, from `ABB-III.NFO.csv` |
| **596** decimal-strike contracts per day are refused — **loudly** | Reproduced on 2025-07-01: **11,490** option members in `Options.zip`, of which **exactly 596** carry a decimal strike (`BANKBARODA31JUL25276.65CE.NFO.csv`). `.` is not in the store's legal identifier byte set — `core/src/symbol.rs:73–77` admits `A–Z 0–9 - _ &` and nothing else — so each is an `InstrumentError::Malformed`. **The coverage loss is real. The silence is not**; see below |
| TrueData 2025 indices store nothing, and the **reason is thin** | 88,885 rows read, **0 stored** — `NIFTY 50` and `INDIA VIX` contain a space, refused by the same byte set. **That count is carried from the original row and was NOT re-measured in this pass**: no `NSE_IDX_TICK_*` archive is on this machine, and `CLAUDE.md` §3 rule 6 forbids restating it as though it were. What WAS re-measured is the refusal path, and it is loud by the same five mechanisms below. The genuine defect is the **wording** — `core/src/error.rs:91` renders every one of them as "malformed instrument identifier", naming neither the offending byte nor its offset. An operator reading "NIFTY 50 — malformed instrument identifier" is not told it was the space |
| The only true 1-minute archive product is unreadable | 8 fields, and the time is `09:15` — the parser needs seconds |
| No zip reader anywhere in the workspace | Every archive must be hand-extracted. Visible in the operator's own tree, where `GFDLNFO_TICK_01072025.zip` sits beside an already-extracted `GFDLNFO_TICK_01072025/` |

**Why the two refusal rows are not `CLAUDE.md` §4 violations.** Each refused
member raises **five** things, and every one of them was reproduced:

1. a telemetry event at **`Error`** on `pull.member` — "did not land" — naming
   the instrument and the reason (`pull/src/ingest.rs:442`, called at 576);
2. an entry in `Ingested::failures`, named (`ingest.rs:577`);
3. `Ingested::balances()` **false**, because it begins `self.failures.is_empty()`
   (`ingest.rs:244–246`);
4. a receipt line reading **"Members failed: 596"**, and the balance line
   spelled `NO — …` (`api/src/server.rs:3717`, 3733);
5. `audit::Outcome::Failed` on the journalled record, forced by
   `!done.failures.is_empty()` (`api/src/audit.rs:588`).

That is "degrade loudly and name the reason". It is **not** "a fallback that
hides a failure", which is what the old wording — *decode then vanish*, *store
nothing* — asserted, and that half is what decides how urgent these rows are.

### 7.3 LATENT — wrong values in fields nothing outside a test reads

Correcting any of these today changes no byte on disk and no line on a page.
They are recorded because wiring §7.1 without correcting them first is what
would make them fire.

| Item | Evidence |
|---|---|
| `PriceScale::Rupees` in both archive descriptors | **`vendor.rs:2517`** — TrueData's `ArchiveSpec` — and **`vendor.rs:2573`** — GDFL's. Re-verified line by line; the numbers this row was previously given were the broker rows and are corrected in §7.4. The decoder already emits paisa, so wiring these as they stand is a ×100 error, masked today only by `run_local` hardcoding `PriceScale::Paisa` at `server.rs:4002` |
| `MemberPattern` cannot express GDFL's futures depth | Three components against the four measured in §7.2. Nothing resolves a `MemberPattern` into a path outside `#[cfg(test)]`, so this is the descriptor half of the GDFL row and it is inert |
| `RecordShape::Snapshot` unenforced | Its doc says a snapshot feed must refuse rather than be "stored as a bar with the price repeated four times — a lie written into the data itself". That is exactly what reaches disk — and `Descriptor::record` is read at `vendor.rs:4011–4014` **and nowhere else**, so nothing consults it before writing |

### 7.4 What these rows used to say, and why it was wrong

Kept rather than deleted: `CLAUDE.md` §3 rule 1 forbids an unsourced claim
standing, and a corrected record is worth more here than a tidy one.

| Row | Used to say | Why that was wrong |
|---|---|---|
| GDFL futures | "GDFL futures unreadable — `MemberPattern::SymbolAtRoot`, but they nest one level deeper. **All 642 files invisible**" | **The stated cause was never true.** `git log -S StemGroupSymbol -- crates/pull/src/vendor.rs` returns exactly two commits; the older, `dbaafd6`, is the one that created the descriptor table, and it introduced GDFL carrying `StemGroupSymbol { suffix: ".NFO.csv" }`. The newer, `10b11b2`, touches only test-side references and leaves the descriptor's value alone — so GDFL has carried it unchanged since the table was born. `SymbolAtRoot` is **TrueData's** value — §3's diagnosis, copied onto the wrong vendor. §3's row is correct and stands unchanged. The count 642 is right; "invisible" and "unreadable" are not, and the 194 misfiled months are the proof |
| 596 decimal strikes | "596 decimal-strike contracts per day **decode then vanish**" | The count is exact and reproduces. "Vanish" does not: it names the one shape `CLAUDE.md` §4 bans, and the five mechanisms above are the opposite of it. Half the row decided the severity and that half was wrong |
| TrueData 2025 indices | "TrueData 2025 indices **store nothing**" | Literally true of the bars, but read beside "vanish" it said *silently*. It is the same loud refusal. The real defect is the message text, which this row never mentioned |
| `PriceScale::Rupees` | The claim behind the row named **lines 2159 and 2324** | Neither is a `PriceScale` line at all. Each sits inside a **broker's** `HttpSpec` literal, seven lines above that broker's own `prices: PriceScale::Rupees` — 2166 for Dhan, 2331 for Groww — where `Rupees` is **correct**, because both vendors send decimal rupee text. A remedy aimed at those lines would have broken the two feeds that work and left the two that do not. No tracked file carried the numbers, which is why the row went unchecked; the archive lines are 2517 and 2573 |

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
| When does an instrument's history start? | Per feed, per RUNG, and per instrument. Groww's day rung from 2020 and its one-minute rung a rolling 3 months; Dhan a **rolling** ~5 years that moves daily | ⚠️ per-feed and per-rung floors are recorded and emitted — `pull::vendor::Descriptor::history`, `/feeds.json`, D-0113. **Per INSTRUMENT is still nowhere**: a scrip listed in 2024 has no bars in 2021 and nothing here knows that |

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

---

## 11. Step 3 authority closure — measured through 2026-09-01

This section records the current drill-down work and does not rewrite the older
pull plan above. The last complete locked workspace test/doctest measurement —
**407/407 CLI**, **438/438 runner**, **86/86 engine** and **816/816 API** library
tests, plus one intentionally ignored real-store API test — predates the current
untracked Population V5 and Execution V3 Stage-A work. It is carry-over evidence
only. No current-tree strict CLI Clippy, locked workspace, coverage, mutation or
benchmark gate is claimed green. Focused earlier suites and the live ratio bench
remain recorded below with their boundaries; none is a completion percentage or
a substitute for the production authority path.

### What Step 3 means end to end

Step 3 is the authoritative computation core for one deep stored-data
drill-down sweep. It starts with exact stored source capabilities and ends with
one reopened global execution receipt whose 200 selected witnesses resolve
back through every earlier receipt. It does not include the operator command,
HTTP route or browser table that invokes/renders that core; those are Step 4.
The frozen full gate matrix is Step 5, and the separately authorized real
Zerodha NIFTY run is Step 6.

| Layer | Complete Step-3 responsibility | Fail-closed boundary |
|---|---|---|
| Stored source | Own exact signal bars, full one-minute context, evaluated one-minute slice, causal prior-day daily reference, feed/commit and complete IST calendars under explicit nonzero load bounds | No caller-authored digest, invented tick, sampled month, silent missing month or foreign feed |
| Candidate Universe | Walk Apriori levels from one condition to natural frontier extinction and expand every closed mask through both directions and both complete dynamic grids | No depth parameter, retained-only family, caller-capped grid or partially closed frontier |
| Pre-Admission | Reconcile that exact Candidate completion with the same measured bar/daily/calendar source before Population IDs, statistics or ranking exist | No Population/admission cycle, substituted execution slice or claimed calendar receipt |
| Statistics V2 | Build one complete NIFTY-then-BANKNIFTY hypothesis family per rung; retain aligned returns and complementary splits; recompute Wilson, CSCV/PBO, White, SPA and Romano--Wolf | No lucky-survivor family, caller-authored score/digest, per-instrument undercount or post-selection evidence cycle |
| Search V4, Admission V3 and Finalization V3 | Bind reopened Candidate/Base/Observation/Statistics authority, anchored search lineage and one canonical Runner V3 decision to every row, then finalize the complete NIFTY-first/BANKNIFTY family receipt-last | No Admission V1 downgrade, detached digest authoring, per-candidate full-ledger rescans or status-only promotion |
| Population V5 and Execution V3 successors | Exact-join authenticated Candidate record bytes to Finalization V3, persist final Population semantics, then rebuild one truthful terminal execution disposition per row from matching stored series, columns and grids | No reinterpretation of Population V4/Admission V1/Execution V2 bytes, dropped policy-refused coordinate, copied capability or foreign completion |
| Selection V5 successor | Recompute one deterministic global Top-25 for each rung from the complete successor admitted-and-authorized NIFTY/BANKNIFTY family; Top-10 is the exact prefix | No externally implementable fake authority, duplicate strategy alias, partial family or structural receipt without full replay |
| Global Replay V3 successor | Resolve all eight Top-25 lists into 200 authority-owned stored witnesses, schedule them under one global inclusive position lock and persist/reopen the exact decision/money/VIX publication chain | No simultaneous long/short/instrument/timeframe position, caller-assembled witness, missing 8×25 topology or inferred money/VIX authority |
| Receipt chain | Sync data rows before each Completion, recover only an exact valid trailing prefix, then reopen/recompute every one-to-one join and identity | No fallback, overwrite, stale same-length handle, corrupt/resealed forgery or success inferred from a controlled fixture |

Most work in this table is input-dependent. Fixed-stride seeks and the named
per-operation primitives remain O(1); universe construction, file validation,
statistics, replay, persistence and total latency do not become O(1).

The newer 2026-08-31 Runner checkpoint is current-tree evidence: after V1
measured-PBO construction/reopen was made fail-closed, Admission V2 also
refuses a decided choice without its exact pessimistic OOS result and refuses a
retained score family shorter than the fold's priced count. The focused V2
slice is **6/6 green**, the complete validate slice **25/25**, the complete
Runner library **474/474**, Runner compile-fail doctests **3/3**, and strict
Runner library/test Clippy reports zero findings. The opaque V2 search
projection is now public but non-constructible/non-`Debug`, while zero resolved
rungs refuse before search. This does not refresh the CLI/API/workspace,
coverage, mutation or benchmark gates.

The current paired stored production-seam checkpoint is also narrower than
Step-3 closure. A deterministic synthetic temporary store drives the actual
NIFTY-then-BANKNIFTY `BarFile` load, Candidate, Pre-Admission, Observation and
Statistics code paths. The orchestrator is **14/14**, Observation **7/7** and
Statistics **17/17** focused-green; exact retry preserves freshly reopened
audits/projections and byte-identical output ledgers. An independent audit found
no bounded-seam bypass. This proves the typed architecture and idempotence, not
real-market correctness, Admission V2, Finalization or the all-rung caller.

The newest successor checkpoint supersedes the last sentence above without
retroactively closing Step 3. Population Admission V3 was previously measured
**10/10**, Population Finalization V3 **9/9**, and the retained orchestrator
**14/14**. Admission authenticates the complete Statistics family once and
joins fixed-offset Base rows in O(Statistics-file-bytes + candidates);
Finalization consumes one bulk Admission projection in
O(Admission-file-bytes + candidates). Population V5 and an Execution V3
fixed-record/receipt kernel now exist, so describing the successor as merely
absent is no longer accurate. The current Execution V3 Stage-A focused suite is
**13/13 green**. Layout V5 proves the independent 4x2 admission/execution
matrix, nonzero PolicyRefused run provenance, one common rung, unique Candidate
base rows, exact resolved Stop/Target/Trail/ratio-pair cardinalities, forced-stop
identity and all five exact exit-coordinate slots. This focused result still
did not by itself make Execution V3 a production authority. D-0487 now adds a
sole production door that consumes `CommittedStoredPopulationV5`, retains that
source beside the durable Execution capability, and reconstructs the exact
run/grid/column/context evidence before and after persistence without copying
Population ranking facts into layout 5. The extended stored architectural
fixture proves first write, fresh reopen, exact reuse and refusal after its
retained Finalization source changes. This closes the one-rung retained-source
door; it does not create the absent all-rung non-test caller, Selection V5
production constructor or Global Replay V3 authority.

The first canonical eight-rung Population V5 coordinator is **5/5 focused
green**, including canonical topology, missing-rung and symlink refusal, but an
independent adversarial review proved that its original raw-array successor
escape permitted sibling code to reorder rungs. That escape was removed. The
coordinator is therefore intentionally non-callable until a sealed Execution
successor owns the canonical topology and the retained full resolved grids.
The review also proved that natural one-family extinction was not end-to-end.
Pre-Admission Data V2 now seals the exact Candidate extinction proof with a
**13/13** focused V1+V2 suite, and Observation V2 consumes only that opaque
production/commit to persist one family-specific `NaturallyExtinct` authority
with zero observation rows; its focused V1+V2 suite is **11/11 green**.
Statistics V3 is now the first typed mixed/extinct successor and its focused
suite is **6/6 green**. It accepts an evaluated Observation V1 family plus one
authenticated Observation V2 extinction in either orientation, or two exact
extinction receipts; its terminal shape forbids numeric evidence for an empty
family and distinguishes a real one-Candidate `InsufficientForCscv` family.
This closes Statistics only. Admission, Finalization and Population still
cannot consume that opaque mixed/extinct authority, so the paired production
chain and Step 3 remain open.
Execution V3's
authenticated fixed-offset lookup also rehashes all held files, making the
authenticated operation O(total bounded file bytes), not O(1). The retained
all-rung Execution and Selection joins are the next open seams, and no current strict CLI or
workspace gate is claimed green. The latest strict CLI Clippy attempt reached
the new module and failed on its production dead-code warnings, which is
measured confirmation that no non-test caller exists; these warnings will not
be suppressed and called production wiring. Existing Population V4/Admission V1,
Execution V2, Selection V4 and Global Replay V2 remain valid legacy audit
formats and tests; genuine V3 evidence may not be coerced into them.

Two resource limits remain explicit. The coordinator can retain sixteen
bounded stored execution contexts at once; each context has a ceiling, but no
aggregate all-rung memory ceiling has yet been admitted. Root capabilities and
pre/post generation checks fail closed for stable missing, symlinked, aliased
or replaced paths, but pathname-based lower I/O is not atomic with those checks.
A hot-unplug followed by mount-point replacement can therefore touch the
replacement before the post-check refuses. Proving that an internal disk is
never touched requires capability-relative lower I/O (an `openat`-equivalent
design), not another pathname preflight.

The 2026-09-01 host inventory confirms useful routing but not confinement:
`~/.brutex`, `target`, `logs`, `.claude/worktrees` and `mutants.out*` currently
resolve onto `/Volumes/WD_BLACK`, whose followed device differs from the
repository device. None of those symlinks is a durable authority. Replacing one
with an internal directory redirects ambient pathname I/O before the existing
post-check can refuse. Closing that gap requires a safe Rust directory-
capability layer, a marker/device policy whose expected value is supplied
independently of the drive, and bottom-up conversion of Store, result/Step-3
ledgers, pull audit, telemetry and API locks/readers to relative child opens,
renames, unlinks and syncs. `std::fs` alone exposes no capability-relative
child operation; a reviewed safe Rust facade may keep every brutex crate under
`forbid(unsafe_code)`, while forbidding even a dependency's OS syscall boundary
would make the requested confinement impossible. Cargo target/TMP routing is a
separate host-preflight concern because a repository build check runs only
after Cargo has already selected its output locations.

No dependency is authorized at this checkpoint. `cap-std` has no locally
auditable source or index metadata and therefore remains `UNVERIFIED`.
`rustix` 1.1.4 is locally auditable and exposes the required macOS relative-I/O
primitives without native C/C++ source, but its unconditional `build.rs`
launches `rustc`; that violates the repository's no-external-process build-
script law even though existing CI does not inspect registry build scripts.
Adding either package without first resolving those facts would replace one
unproved guarantee with another.

The later Global Replay V2 checkpoint is narrower and newer: its focused module
is **6/6 green** and its public test carries eight Selection V4 receipts,
sixteen Population V4/admission/Execution V2 completions and 200 exact witnesses
through preparation, receipt-last append, reopen and scheduler reconstruction.
Strict CLI Clippy reported zero diagnostics from that module; the package-wide
test-target run still had four fixture-only diagnostics in sibling modules at
that checkpoint, so the complete Clippy gate remains open. V2 is explicitly
execution-only: it persists no admitted money rows and no VIX stamps.

The newer Execution Disposition V2 checkpoint is **8/8 focused tests green**.
It persists exactly one terminal row for every Population V4 row, including
policy-refused coordinates that V1 could not represent, and reconstructs its
admission marginals and complete 4×2 admission/execution matrix on reopen. The
suite includes missing/foreign authority, semantic matrix redistribution,
orphan/torn/corrupt and post-open same-length mutation attacks. This supersedes
the stale V1 cardinality blocker only; strict package Clippy and the non-test
stored producer/caller remain separate open gates.

| Surface | Measured state | Remaining dependency |
|---|---|---|
| Live mask support | Release ratio bench green on the live owned-row path: k=1→4 **0.971x**, k=1→8 **0.996x**, k=1→384 **0.942x**; each bar performs one fixed-six-word hit operation; post-edit engine and complete workspace regressions are green | Total support remains O(bars), and frontier/sweep/replay work is not O(1) |
| Population V4 | Focused suite green; fixed receipt persists full requested-span signal and independent one-minute calendar coverage | Non-test stored producer for both indices and all eight rungs |
| Stored-data completeness V1 | **5/5** focused authority tests green plus **10/10** institutional-evidence regressions; exact signal, full one-minute context, evaluated execution and daily-reference bytes/policies/calendar receipts reconcile to one Population V4 before a sealed receipt can reopen as typed `Complete`; foreign/corrupt/ragged/stale authority refuses | No non-test stored-run caller produces/commits/reopens this authority; vendor integrity remains explicitly `UnverifiedNoReceipt`; preparation/open are O(data/file) |
| Institutional evidence and exact family statistics | The current institutional-evidence slice is **14/14 focused tests green**: anchored walk-forward decided/profitable/OOS facts remain measured, while all three genuine-PBO V1 fields stay `Unmeasured`; its 44-field source matrix and **seven** blockers name the missing authority. Runner Admission V1 fails closed at construction and canonical reopen for every measured PBO-named field; its focused suite is **32/32 green** and strict Runner library/test Clippy reports zero findings. Candidate Observation V1 plus the zero-family Observation V2 successor are **11/11 focused tests green**. V2 embeds the exact sealed Pre-Admission V2 Data record and records only `NaturallyExtinct` with zero rows. Population Statistics V2 remains the paired-nonempty authority. Statistics V3 is **6/6 focused green** for both mixed orientations, truthful all-extinct, one-Candidate CSCV insufficiency, exact receipt reuse and corrupt/stale/path-replacement refusal; it never creates numeric evidence for an extinct family | The opaque Statistics V3 result still has no non-test Admission successor consumer, so Admission/Finalization/Population cannot yet carry a mixed/extinct pair through the all-rung chain. Global `full_precision_statistics_complete` remains `Unmeasured`; no non-test uncapped real family exists. Projection, resampling, authentication and persistence are input/file/system proportional, not total O(1) |
| Population + admission producer | **11/11** focused tests green; uncapped closed-mask expansion, one-pass grid validation, receipt-last two-ledger commit/reopen and exact reuse are proved | Invoke it from the real stored pipeline with authoritative evidence; no non-test caller exists |
| Population V5 to Execution V3 | A source-retaining `CommittedStoredPopulationV5` capability and sole layout-5 Execution V3 commit door now exist. Runner full-instrument digest **1/1**, Candidate replay **1/1**, the extended stored write/reopen/reuse/stale-source fixture **1/1**, and Execution V3 **13/13** are focused-green. Ranking facts remain owned by retained Population V5 rather than copied into Execution bytes. The canonical eight-rung Population V5 coordinator remains **5/5 focused-green** at its isolated topology/root boundary | The new door's only exercised caller is the focused `#[cfg(test)]` stored fixture; the all-rung coordinator still has no non-test caller, Selection V5 has no production constructor, natural zero-family extinction has not crossed every successor, and authenticated reads still perform O(file-bytes) generation checks |
| Global Selection V3 + Top-N policy | Selection V3 focused 8-test suite green; Top-N focused **14/14** green; the corrected 76-byte policy identity includes all eleven runtime weights; both Population V4/admission completions are cohort-bound and only recomputed joined verdicts enter Top-N | Non-test all-rung caller plus a resolvable exact execution authority contract for every population row |
| Execution disposition V2 | **10/10** focused tests green; exactly one `Authorized` or `PolicyRefused` terminal persists per Population V4 row, with sparse capability/refusal partition, exact admission binding and 4×2 matrix; append/reopen/reuse/orphan/corrupt/resealed-matrix/stale attacks refuse, and an absent/non-directory authority root is never recursively recreated | V1 remains unchanged but its cardinality blocker is superseded. No non-test stored all-rung producer/caller reconstructs these authorities from real grids/runs yet; initial pathname safety is not physical-device identity or hot-unplug atomicity |
| India VIX reference | Nine focused tests green; same-feed/month exact-minute or typed-absent lookup is fail-closed and remains reference-only | Load required months in the production replay path; no non-test caller exists |
| Global replay V1 | Seven focused tests green: prepared-fixture persistence proves fixed 200-stream codecs, inclusive occupancy, replay/VIX identity separation, append/reopen/reuse and corruption/stale refusal; direct public preparation proves 8 selections + 16 execution authorities + 200 witnesses reach scheduling and missing/foreign authorities refuse | Replace controlled fixtures with real stored selected reconstruction, invoke it from production and append/reopen/replay that exact public result; no non-test caller exists |
| Global replay V2 execution authority | **7/7** focused tests green: exact Selection V4 reproduction, sixteen distinct Population V4/admission/Execution V2 completions, 200 fresh reclassifications, shared global occupancy, five-file receipt-last append/reopen/exact reuse, valid orphan-gap continuation, corrupt/reordered/stale refusal | Controlled fixtures only; no non-test stored caller. The completion deliberately says no money/no VIX, so admitted `TradeRow` and exact-or-absent VIX publication remain separate open authorities |
| Step-3 comparison read model | At its earlier isolated checkpoint the Rust read model was **9/9** focused-green and opened only Population V4/admission V1/Execution V2/stored-data V1/Selection V4/Global Replay V2 authorities, reconciled exact IDs/counts, preserved `READY`/`BLOCKED`/`UNMEASURED`/`REFUSED`, proved Top-10=80/Top-25=200/8×25=200 and rendered one deterministic CommonMark table | That checkpoint predates the current successor edits and does not make current CLI Clippy green. The model remains library-only with no CLI/API/audit/dashboard caller; Selection remains blocked until exact authority replay, and no real stored authority set was compared |
| Operator surface and real sweep | Not authoritative and not run | Reopened replay receipt must drive CLI/API/audit/dashboard comparison; then complete gates, then explicitly authorized stored Zerodha NIFTY sweep |

### Exact remaining Step-3 sequence

| Order | Required proof | Current state |
|---:|---|---|
| 1 | Resolve the population/execution cardinality law: every persisted Population V4 row must have one truthful terminal representation without treating a policy-refused coordinate as selected | **Closed at the versioned V2 kernel/ledger boundary** — 10/10 focused tests prove one `Authorized` or `PolicyRefused` record per row, including missing-row, semantic-matrix and missing/non-directory-root attacks. This is not production wiring or Step-3 completion |
| 2 | Real stored authorities produce the complete Candidate→Pre-Admission→Observation→Statistics→Search V4→Admission V3→Finalization V3 chain, then exact-join literal Candidate bytes into Population V5 for both indices at every canonical rung | **In progress, not gate-proven** — Pre-Admission V2 and Observation V2 carry authenticated zero-family extinction through independent receipt-last formats; Observation V2 is **11/11** focused green. D-0492 Statistics V3 now joins one exact nonempty Observation V1 family to one authenticated Observation V2 extinction in either orientation, or two truthful extinctions, and is **6/6** focused green. It fabricates no empty-side rows or statistics. Admission/Finalization/Population still lack the typed mixed/extinct successors, and no production all-rung caller exists. The retained `CommittedStoredPopulationV5` and isolated topology fixtures prove only the current nonempty controlled path |
| 3 | Population V5 rows produce exact Execution V3 terminal authorities, and Selection V5 winners reproduce their sealed selected exits from matching training columns/runs and stored one-minute series | **In progress; the one-rung Population V5→Execution V3 door is now focused-green, while the all-rung and Selection joins remain open** — D-0487 proves full-instrument identity, exact Candidate run/grid/column/context replay, layout-5 write/fresh-reopen/exact-reuse and stale retained-source refusal. The capability retains Population ranking facts beside, not inside, Execution bytes. Its current exercised caller is test-only. Closure still requires the non-test eight-rung caller plus an Execution-to-Selection V5 production projection and exact selected-exit replay. Authenticated whole-file generation checks remain honestly O(file bytes), not O(1) |
| 4 | Global Replay V3 consumes all eight real Selection V5 receipts, sixteen successor execution authorities and 200 authority-owned stored witnesses, then writes/reopens/replays the same decision, admitted-money and exact-or-absent VIX publication chain | **Open** — Global Replay V2 proves an execution-only 8/16/200 scheduler/ledger over controlled fixtures. It is hard-bound to legacy authorities, has no production caller and truthfully contains neither money rows nor VIX stamps; its scheduling kernel may be reused but its bytes may not be relabelled V3 |
| 5 | CLI, API, audit and dashboard expose the same receipt identities, counters, decisions, trades, typed absences and comparison rows | **Earlier comparison kernel measured; current successor callers open** — the six-row Rust comparison model's prior 9/9 checkpoint predates the current edits. It still has no non-test CLI/API/audit/dashboard caller for V5/V3 authorities, and no current strict-Clippy claim is made |
| 6 | Formatting, strict Clippy, locked workspace tests, deny/security, adversarial, benchmark, coverage and mutation gates | **Open with current evidence** — owned formatting and repository diff checks are clean; the D-0487 Runner/Candidate/stored-seam/Execution focused runs are **1/1 + 1/1 + 1/1 + 13/13**, Pre-Admission V1+V2 is **13/13**, Observation V1+V2 is **11/11**, and all-rung topology/root safety remains **5/5**. Focused test builds succeed but most recently report 11 successor dead-code warnings from the unreachable all-rung coordinator and not-yet-consumed Pre-Admission V2; they are not a strict-Clippy success. Locked workspace tests, deny/security, full adversarial suites, benchmarks, coverage and mutation remain to run after implementation freezes; no current-tree complete green gate is claimed |
| 7 | Review manifest, commit/push `feat/pull`, then run the separately authorized real stored Zerodha NIFTY sweep | **Blocked by open orders 2–6** |

Step 3 is therefore **not finished**. Its kernels and durable formats have
advanced, but green submodules, fixed fixtures and stored primitives do not
prove the missing production joins. Step 4 starts only after orders 1–4 produce
one reopened replay authority. Step 5 is the complete gate matrix, and Step 6
is the first authorized real stored sweep—not evidence retroactively used to
mark the earlier steps complete.

The statistics claim is also deliberately bounded. Each rung's uncapped NIFTY
plus BANKNIFTY pair is the complete family that competes for that rung's one
Selection V4 Top-25; Top-10 is its exact prefix. Those eight per-rung families
are not advertised as one cross-rung FWER-controlled family, because Global
Replay V2 consumes their completed lists without jointly re-ranking the source
families. D-0464 requires every CLI/API/dashboard comparison to retain that
label. A future joint cross-rung statistical selection needs a new versioned
policy rather than a stronger claim over these bytes.

### 2026-09-01 Global Replay V3 authority checkpoint — D-0493

The stale order-4 row above is superseded at the isolated library seam, not
closed end to end. Global Replay V3 now consumes D-0494's named all-rung
Selection successor plus exactly 200 opaque Runner OOS replay capabilities,
exact-joins their selected-exit/instrument/direction/OOS identities, schedules
all candidates under one global inclusive position lock and writes/reopens a
new receipt-last Witness/Candidate/Decision/Money/Completion authority. The
focused V3 suite is **5/5 green** and the Runner witness-mint proof is **1/1
green**. The typed cross-authority join compiles, but the five V3 tests use
private controlled witnesses; no one focused fixture yet constructs the
complete all-rung Selection successor and all 200 opaque Runner capabilities
together.

| Global Replay V3 subproof | Measured state | Honest remaining work |
|---|---|---|
| Canonical 8x25 Selection handoff | Typed all-rung Selection successor preflights eight authorities and visits exactly 200 winners in literal rung/rank order | Natural zero-family extinction still requires its separately versioned upstream successor; no rows or statistics may be invented |
| Authority-owned OOS witnesses | Runner's opaque mint binds canonical full instrument, feed, direction, first OOS, run, selected exit and exact replay candidates; the mint test is 1/1 green | A non-test Step-3 orchestrator must obtain and move these capabilities from the real stored successor chain |
| Global lock and publication semantics | V3 5/5 proves one inclusive position lock, deterministic simultaneous/occupied outcomes, pricing-refused occupancy, admitted-only money and admitted-only same-feed exact-or-absent VIX | Controlled private fixtures only; no real pulled/stored bars were swept |
| Receipt-last V3 ledger | Five new fixed-stride codecs, explicit bounds, exact reuse, fresh semantic replay and torn/corrupt/foreign refusal are focused-green. Its sole crate-level commit door returns an opaque capability only after fresh reopen reproduces the prepared identity, counts and counters. Runner strict library/test Clippy is green and the latest strict CLI attempt reports no V3-file diagnostic | The CLI package gate remains red in sibling unfinished successors; full workspace tests, deny/security, benchmarks, coverage, mutation and physical storage-failure injection remain open |
| Operator surfaces | The crate-private production door exists; no upstream stored orchestrator or operator caller invokes it | Move the real successor capabilities into that door, then wire its reopened audit projection into the authoritative CLI/API/database/audit/monitoring/dashboard comparison without copying facts |
| Final real sweep | Not run | Only after the production chain, operator surfaces, complete gates, manifest review and commit/push are green may the separately authorized stored Zerodha NIFTY sweep run |

Therefore Step 3 is still open, but Global Replay V3 is no longer the missing
library authority described by the earlier row. The principal remaining seams
are the non-test stored orchestrator, natural-extinction successor completion,
operator publication and the complete gate matrix. This checkpoint neither
claims profitability nor upgrades fixed topology into impossible universal
O(1) time, space or latency.

### 2026-09-01 Statistics V3 to Admission V4 checkpoint — D-0495

The earlier order-2 statement that Statistics V3 has no Admission successor is
superseded at the isolated library boundary only. Admission V4 now consumes the
typed mixed/extinct Statistics projection and retains a fresh-reopen authority
with a complete Finalization-facing projection. Admission V3 and Finalization
V3 bytes/paired-nonempty semantics remain unchanged.

| Admission successor subproof | Measured state | Honest remaining work |
|---|---|---|
| Statistics handoff | Statistics V3 exposes exact common authority, literal NIFTY/BANKNIFTY family projections and only real Candidate projections; evaluated Candidates carry a genuine Runner V3 draft, an insufficient singleton carries none, and extinction emits no row. Focused Statistics V3 is **7/7 green** | No one production fixture constructs the complete stored upstream chain and transfers the opaque source into Admission V4 |
| Candidate/Base/Search authentication | The sole production preparation signature requires both reopened Candidate audits, one bound paired Base reader, retained Search V4, authenticated durable Search V4 lineage and one Runner policy. It exact-joins cohort, ordinal, semantic, Observation, signal, long/short grid and validation facts; it accepts no caller digest, family, status or row | Exercise this entire typed door from the non-test stored orchestrator, including adversarial upstream crosswire fixtures |
| Mixed terminal policy | Evaluated families recompute one canonical Runner exact-grid decision per real Candidate. `InsufficientForCscv` preserves one Candidate lineage with zero decisions/PBO invention. `NaturallyExtinct` preserves the Observation V2 proof with zero Candidate/statistic/decision rows. NIFTY is always first | Finalization V3 cannot represent this honestly and must never receive relabelled V4 facts |
| Admission V4 ledger | Independent sealed 4,096-byte Data/Family/Decision/Completion records, explicit bounds, evidence-before-Completion sync, exact reuse, one exact-prefix retry, bounded fresh reopen and corrupt/resealed/stale/path refusal are **5/5 green** | Physical ENOSPC/power-loss/kernel-crash/hot-unplug testing and full workspace gates remain open |
| Finalization handoff | Only the retained fresh-reopen Admission authority can return the complete typed common/family/decision projection; every Runner decision is reverified during projection | Implement a version-separated Finalization V4 codec, receipt-last persistence/fresh authority and its typed Population successor; do not fake Admission/Finalization V3 compatibility or invent zero-family rows |
| Focused compile/lint state | `cargo check -p cli --lib --locked` is green. The substantive CLI library Clippy diagnostic (`-D warnings -A dead-code`) reports **0 findings** in `population_admission_v4.rs` and **0** in `population_statistics_v3.rs` | That command remains red on **38 sibling findings**, and the full strict command is still open. Full formatting, locked workspace tests, deny/security, benchmarks, coverage and mutation are not green by implication |
| Operator/real-run state | No CLI/API/database/audit/monitoring/dashboard caller and no real stored sweep was added | Finish Finalization V4 plus the all-rung production join, then publish the same reopened identities on every operator surface, complete the full gate matrix, review/commit/push, and only then run the separately authorized stored sweep |

Step 3 therefore remains **in progress**. This checkpoint closes the
Statistics→Admission mixed-terminal library seam and makes its exact projection
available to a future Finalization V4 authority; it does not close
Finalization, Population, all-rung orchestration, Global Replay integration or
operator publication. Preparation, decision evaluation, hashing, persistence
and reopen are input/file/system proportional. Fixed-stride offset arithmetic
alone is worst-case O(1), and no universal O(1) time, space or latency promise
is made.

### 2026-09-01 Admission V4 to Finalization V4 checkpoint — D-0496

The D-0495 row that names Finalization V4 as absent is superseded at the
isolated library boundary only. A version-separated Finalization V4 now owns a
fixed Data/NIFTY Family/BANKNIFTY Family/evaluated-Decision/Completion grammar
and a source-retaining typed Population handoff. Finalization V3 and Population
V5 bytes remain unchanged.

| Finalization successor subproof | Measured state | Honest remaining work |
|---|---|---|
| Admission-only mint | The sole production door moves retained Admission V4 and obtains its fresh projection internally; no detached receipt, digest, row, status, terminal or policy enters | Exercise the complete non-test stored upstream chain and all eight rungs |
| Mixed terminal preservation | Exact NIFTY/BANKNIFTY Family records carry evaluated, insufficient and extinct shapes; only evaluated Candidates emit decisions, and Statistics sequence preserves an insufficient singleton gap. Focused suite is **5/5 green** with **729 filtered** | Construct and move the complete non-test upstream capability chain rather than controlled sources |
| Finalization V4 ledger | Independent 64-byte header and sealed 4,096-byte Data/Family/Decision/Completion records; explicit bounds, evidence-before-Completion sync, exact reuse, every exact-prefix retry, fresh reopen and generation checks. Focused CLI library check is green | Strict library/test diagnostic has zero V4 findings but remains package-red on **105/117 sibling findings**; physical storage-failure injection and full workspace gates remain open |
| Population handoff | The retained capability reauthenticates Admission before and after a complete Finalization read and returns exact common/family/Runner decision projections | Existing Population V5 is hard-bound to V3 snapshots and cannot represent zero-decision families. Implement a new Population byte version, rather than widen/relabel V5 |
| Operator/real-run state | No CLI/API/database/audit/monitoring/dashboard caller and no real stored sweep is added | Complete the new Population→Execution→Selection→Global Replay production joins, publish the same reopened facts, finish gates, then run the separately authorized sweep |

Finalization V4 closes neither Step 3 nor the mixed-terminal all-rung chain by
itself. Authentication, decision replay, hashing, persistence and reopen remain
input/file/system proportional. Fixed-record offset arithmetic alone is
worst-case O(1); no total O(1) time, space or latency promise is made.

### 2026-09-01 stored post-training OOS and terminal-aware Replay V4 pivot — D-0497

The earlier D-0493 checkpoint remains true for Global Replay V3's legacy fixed
eight-by-Top-25 topology, but it cannot honestly close current Step 3 after the
mixed-terminal chain admitted natural extinction and insufficient statistical
evidence. Those states yield an actual zero-through-twenty-five winners per
rung, so the production successor is Selection V6 plus Global Replay V4. V3 is
not changed or relabelled.

| Boundary | Current precise state | Honest remaining work |
|---|---|---|
| Stored OOS request | `StoredPostTrainingOosRequestV1` accepts only an inclusive civil month range and explicit signal/minute/daily record ceilings; the controlled suite is **3/3 green** and the focused CLI library check is green | Strict shared-tree Clippy remains red; four direct cohort/orchestrator findings from the measured attempt were corrected statically and need a post-correction rerun |
| Source derivation | The retained stored Candidate transaction now reuses the common bounded loader and derives vendor, full instrument, rung, commit, evaluator, horizon, ladder, dynamic Long/Short grids and training boundary without caller-authored market facts | No real pull or sweep is part of this phase; complete workspace/coverage/mutation/security gates remain open |
| OOS cohort identity | The opaque cohort owns complete signal/daily/exact-minute evidence and a held root; its seal binds root generation, source/calendar/policy/data/grid identities, exact training/OOS boundaries and load ceilings. Empty, incomplete, overlap, crosswire and root replacement paths are fail-closed in code | Physical hot-unplug/ENOSPC/power-loss and live-current per-file generation are not proved; §169 records the boundary |
| Witness mint | Runner remains the only producer of an opaque replay universe. The stored adapter builds the exact causal OOS column/run and binds cohort ID plus run/selected-exit/universe identity; it exposes no raw bars or loose feed/family/direction/mask/price fields. Exact reuse, cross-family and root-replacement paths are included in the **3/3 green** controlled suite | Selection V6 must retain the matching opaque Execution dispositions rather than only copy their digests |
| Selection V6 dependency | Separate lane is defining terminal-aware actual winner prefixes (0..=25 per named rung) | Freeze the opaque all-rung move type and its preflight semantics before Replay V4 code begins |
| Global Replay V4 | Contract map below is locked; no V4 codec/coordinator has been invented | Implement only after Selection V6 freezes, then add receipt-last persistence, fresh reopen, global no-overlap replay and full adversarial tests |
| Operator and real-run state | No new CLI/API/database/audit/monitoring/dashboard caller and no real stored sweep | Publish one reopened V4 authority everywhere, complete the full gate matrix and manifest review/commit/push, then run the separately authorized stored Zerodha NIFTY sweep as the final step |

#### Global Replay V4 contract map

| Stage | Required opaque input/output | Mandatory refusal before any durable append |
|---:|---|---|
| 1. Selection handoff | Consume one freshly authenticated Selection V6 capability containing eight named rungs in 60/120/180/300/600/900/1,800/3,600-second order, each with its actual canonical 0..=25 rank prefix and terminal reason | Missing/duplicate/reordered rung; non-prefix/duplicate rank; stale retained Population/Execution/Selection source; any caller-authored replacement row |
| 2. OOS handoff | Consume the eight matching `StoredPostTrainingOosCohortV1` capabilities by value | Missing/duplicate cohort; family/rung/feed/instrument/commit/calendar mismatch; empty/incomplete/overlapping OOS; changed root capability |
| 3. Full preflight | For every actual winner, exact-join its retained Execution disposition and mint one `StoredPostTrainingOosWitnessV1`; retain all successes in canonical rung/rank order before writing | Missing/duplicate/reordered winner or disposition; selected-exit/evaluator/side mismatch; stale/crosswired witness; partial winner prefix; any replay/pricing refusal that invalidates capability mint |
| 4. Global scheduler | Schedule every reachable candidate under one inclusive global position lock across NIFTY/BANKNIFTY, Long/Short and all rungs; deterministic same-minute priority binds rank plus strategy identity | Any overlap, ambiguous ordering, out-of-range time, unbound money or noncanonical decision terminal |
| 5. Receipt-last ledger | Version-new Witness/Candidate/Decision/Money/Completion bytes bind actual per-rung counts and total P, not fixed 200; Completion syncs last | Ragged/torn/corrupt/resealed/foreign/orphan evidence, exceeded explicit bounds, nonexact retry, stale named path |
| 6. Fresh authority | Drop writer, reopen read-only, decode/authenticate every bounded record, replay scheduler/publication and compare the complete semantic projection before returning an opaque committed V4 capability | Any counter/order/decision/money/VIX/identity difference, hidden unreferenced record, retained upstream generation change |

The all-rung-empty publication rule and exact Selection V6 move type remain
open and must be decided by a later append-only entry, not guessed in V4. The
three controlled-fixture tests are **3/3 green**, and the focused CLI library
check is green. Strict shared-tree Clippy remains red at 115 library and 126
library-test diagnostics from the last measured attempt; four direct
cohort/orchestrator findings were corrected statically afterward and await a
post-correction rerun. Step 3 therefore remains open. Complete store loading, causal derivation, witness
replay, scheduling, hashing, persistence and reopen are input/file/system
proportional; only fixed-field and admitted fixed-offset primitives can be
described as worst-case O(1).

### 2026-09-01 Candidate reachability and Population/Execution successor checkpoint — D-0498

The earlier rows calling `InsufficientForCscv` one real Candidate V1 row are
superseded for production reachability. Candidate V1 stores
mask×direction×exit execution-coordinate rows, and every nonempty production
has both Long and Short; one closed mask therefore yields at least two rows.
The insufficient enum/ledger shape remains append-only history, but no current
Candidate V1 production can originate it. Population V6 admits four reachable
E/X pairs and refuses the five singleton-containing pairs rather than inventing
evidence or relabelling V3/V4.

| Step-3 successor subproof | Current exact state | Remaining proof/work |
|---|---|---|
| Candidate reachability | Candidate receipt validation now recomputes exact `closed_masks * (Long + Short)` rows and rejects nonzero `< 2`; the focused invariant test also proves both directions | Run the focused Candidate test under the serialized Cargo slot |
| Statistics V3 E/E | New production door consumes two genuine evaluated Observation/Pre-Admission sources, requires NIFTY first, reuses existing family validation and V3 bytes, and has E/E/swapped/crosswired/stale coverage | Run the focused Statistics V3 suite and strict diagnostic; no V3 byte migration is allowed |
| Population V6 source | Four typed source doors retain Finalization V4 plus Candidate authorities only for evaluated families: E/E=two, E/X=NIFTY, X/E=BANKNIFTY, X/X=none. No caller terminal/count/digest enters | Genuine E/E chain is present; full genuine stored mixed/X/X orchestration and all eight rungs remain to be composed |
| Population V6 ledger | Independent fixed Data/NIFTY/BANKNIFTY/Candidate/Completion grammar; truthful zero rows; exact lineage and Runner bytes; receipt-last sync/recovery/reuse/fresh reopen; duplicate/gap/crosswire/corruption/ragged/stale/symlink/path-replacement tests are written | Focused compile/test/check/strict diagnostic are pending serialized Cargo; physical fault injection, coverage and mutation remain open |
| Execution V4 | Static version-separated implementation consumes and retains Population V6, persists 0/2/4 active-family parameters, percentiles, dispositions and Completion, and exposes only a fresh-reopen Selection V6 source. Seventeen codec/topology/recovery/corruption/matrix tests are written | Module compile/test/strict proof is pending serialized Cargo; no downstream Selection V6 implementation is claimed |
| Downstream compatibility | Population V5 and Execution V3 remain unchanged. Terminal families and zero rows require Population V6→Execution V4→Selection V6; D-0497 already requires Global Replay V4 actual winner prefixes | Implement Selection V6, then its all-rung coordinator and Global Replay V4; do not fake V5/V3 compatibility |
| Operator state | No new CLI/API/database/audit/logging/monitoring/dashboard surface and no real stored sweep | Publish only freshly reopened final authorities after downstream freeze and gates; then run the separately authorized real stored sweep |

This is a static implementation checkpoint. `rustfmt --check` is green for the
touched Rust files; no new Cargo command has run. Step 3 remains in progress.
Authentication, exact joins, hashing, persistence and reopen are
input/file/system proportional; only admitted fixed-record address arithmetic
is worst-case O(1). No universal O(1) space/latency or customer-scale guarantee
is inferred.

## Cash-stock research checkpoint — 2026-09-05

The operator has not started the full Zerodha cash-stock pull and requires an
explicit go-ahead. **Bulk pull approval is withheld.** No vendor pull or server
was started by this implementation session; the existing NIFTY sweep was left
running on its already-built binary.

Implemented preparation:

- `cli research-plan VENDOR`: read-only inventory for all eight intraday
  timeframes plus daily reference, from 2020-01-01 through yesterday IST.
- `ResearchWindow`: one clock resolution, exact half-open timestamp bounds,
  clipping helper and versioned requested-date identity fields.
- Cash enumeration positively intersects existing F&O and total-market
  membership instead of treating every F&O name as an equity.
- Runner opt-in SL+TP, SL+TTP and their union, excluding other variants before
  pricing. Legacy callers retain their previous exit population.
- Three incorrect imports in the already-existing untracked `pool.rs` draft
  were corrected so focused CLI tests could compile. A table-format warning
  and unchecked candidate access were fixed; missing candidates are named
  rather than panicking. Its strategy logic was not otherwise certified.

Required before calling the full workflow implemented or starting research:

1. Verify the live cash-only `/pull/spot` request, actual external-store root,
   and current Zerodha entitlement/session evidence without exposing secrets.
2. Bring the authoritative session calendar beyond its current 2026-08-21
   boundary using verified sources; audit through the frozen requested date.
3. Verify exact-day OHLCV/volume, folds, listing/corporate-action handling and
   distinguish the current membership snapshot from historical membership.
4. Integrate the frozen date and exit-family policy into the actual search,
   replay, full run identity and durable research manifests/checkpoints.
5. Replace the draft pool's shortlist-only discovery, correct its coarse-bar
   execution call and compute portfolio drawdown from chronological sized
   trades. Its current maximum individual drawdown is NOT a pooled bound.
6. Persist/reopen pooled ranked rows and every selected row's trade details;
   expose matching UI/API results and coverage instead of independent lists.
7. Benchmark the exact mode, enforce explicit resource/cancellation policy,
   and measure whether the requested workload fits 24 hours. No guarantee yet.
8. Complete full workspace tests, Clippy, formatting, deny, coverage and
   mutation gates. Do not merge/deploy by bypassing the dirty-tree commit stamp.

No charges and no statistical validation are the requested discovery policy,
not a claim that live execution is costless or future winners are assured.

Verification at this checkpoint: 11 focused CLI tests passed (nine research
window/inventory cases, the missing pooled-candidate regression and command
listing); 513 runner library tests passed, including the new exit-family
enumeration/equivalence cases. Strict library Clippy passed for `cli` and
`runner`; workspace formatting and `git diff --check` passed. These are
component checks, not a full-workspace test result, coverage/mutation proof,
live vendor entitlement check, deployed workflow or completed stock sweep.
No release binary was replaced and no new research result was persisted.

## Historical sweep completion follow-through — 2026-09-06 (D-0524)

The highlighted index sweep work now includes the stored AND continuation,
canonical AND/OR/NOT cursor, signal-only and priced expression commands,
exact evaluated-candidate trade catalogs, bounded detail readers, Selection V6
and the explicit later-period Global Replay V4 command. Contracts and scoped
tests are in `docs/19` through `docs/22`; the current human comparison is
`docs/14-sweep-readiness-20260906.md`. Earlier absence statements describe their
earlier checkpoint, not a prohibition against this authorized implementation.

The common API/browser integration now passes its scoped checks. A separate
clean source snapshot ran two real historical expression searches through
8→16 candidate restart, and an independent reader verified all 64 trade files
and 31,279 exact trades. A real AND run reached extinction and recovered its
saved checkpoint without duplicating the parent. Exact scope, identities,
zero cell-rule admissions and missing independent scrub receipt are in
`docs/23-historical-sweep-verification.md`. Final stable workspace gates are
reported separately by the repeatable verifier. The production acquisition
service and unrelated sessions' changes were preserved.

On7 September the operator delegated the37 remaining policy choices. D-0535
supplies an explicit versioned research profile and an automated explanation;
no fixture threshold is promoted into a claim of pre-existing approval.
The profile must be selected for the actual process and every candidate still
needs its required measured evidence. Configuration is not market admission.
Full touched-crate line/branch coverage, mutation closure, visual verification
and customer-scale measurements remain evidence requirements, not automatic
consequences of source implementation. Whole-sweep O(1) time and unlimited
durable history in O(1) total space are not implementable requirements.

The later operator-authorized clean backend activation closed the new-route
404 mismatch. Read-only HTTP checks confirmed both routes, the existing asset
version, a still-paused automatic scheduler and resumed existing recovery.
This is separately recorded in `docs/23-historical-sweep-verification.md`.

Local parallel work now uses the operator-supplied MacBook baseline recorded
in `docs/25-macbook-runtime-budget.md`: M4 Pro, 14 CPU cores and 48 GiB memory,
with current headroom checked at runtime. This sizes operating concurrency;
it does not replace engine admissions with machine-specific magic constants.

The D-0526 continuation completes all20 VWAP truth/known mappings, adds strict
multi-month admission to the shared pricing kernel, and tests actual source,
receipt and ranked-child loss before parent publication. `docs/26-vwap-mapping.md`
contains the cash/index/futures distinction and independent real-row evidence.
`audit-audited-range` is additive; futures are still storage-only, no prices or
volumes are synthesized, and the current acquisition remains undisturbed.
Later clean-build executions and combined gates retain their actual snapshot
under `target/sweep-audit-20260906/` and the automated comparison check panel.

The7 September fixed-training qualification continuation is documented in
`docs/29-fixed-training-qualification.md` and D-0542–D-0544. It connects a
zero-conservative full-family procedure, a predeclared finite eight-timeframe
allocation, original frozen exits, complete genuinely later windows, durable
qualification comparisons and restartable slot reservations. Test fixtures,
clean real-data runs, combined verification and live activation must retain
separate source-stamped evidence. Passing a research policy is not Selection V6
authority, a claim about arbitrary additional grammar batches, or trading approval.

The D-0549 continuation adds a distinct declared-search command and observer.
It fixes a testing-budget gap between individually qualified finite batches and
the whole declared grammar search: one exact countable allocation now binds
every batch/timeframe before pricing. Reservations, retries, complete child links,
all-coordinate projections, independent history/decoding bounds and a retained
parent guard are integrated. The new dashboard compares original and corrected
results without rewriting either source evidence or older formats. See
`docs/30-search-wide-qualification.md` for the easy comparison and limitations.

Final source-stamped tests, real retained-OHLCV continuation, direct handler and
frontend replay, full coverage/mutation obligations and safe service activation
remain separate steps. The current acquisition and shared checkout are preserved.
An initial implementation or previous release's green checks do not clear new
source automatically. D-0548's seven exact allocation/guard mutation replays all
failed their tests as intended; fresh campaign coverage remains395/412 lines and
69/84 branches, so it does not close the whole-crate100% requirement.

# Crash/edge pass 14 (ce14): operator inputs

Head: `1f4de71` (`/home/claude/wt/zero3`). IDs CE-75 upward. Release profile: overflow-checks on, panic = abort.

Scope covered:
- cli argument parsing (`dispatch`, the stored-month, span, pool, sweep-all, screen, boolean, expression, checksum and audited arms)
- instrument admission (`stored::swept_index`, `InstrumentKey::is_sweepable`)
- the §8 credential file (`pull/src/config.rs`)
- every `BRUTEX_*` environment reader (`core::knob`, `cli::knobs`, api budgets, autopilot, log level)
- the api write bodies (`/backtest/run`, `/backtest/descend`, `/engine/command`, the boolean launch, `/pull/spot` members)

Two new findings. Both were proven in a throwaway worktree, and the worktree was then removed.

## CE-75 (low): the ordinary path silently clamps `BRUTEX_GRID_RUNGS`; the strict path refuses the same value

- **Where:** `crates/cli/src/lib.rs:5387-5388` (`grid_rungs`)
  ```rust
  if let Some(n) = crate::knobs::count_usize("BRUTEX_GRID_RUNGS") {
      return n.min(rungs_within_cell_budget()).max(2);
  }
  ```
- **Why it is wrong:**
  - `count_usize` accepts any positive value, so it records no refusal. The value is then replaced, with no `KNOB REFUSED` line:
    - `1` becomes 2.
    - Anything above the cell budget becomes the budget.
  - This reader serves screen, elite, range-all, range-rung, audit-range, audit-stored and the browser's `/backtest/run` `grid_rungs` field. The browser sends that field to this knob through `api/src/sweeprun.rs:533`.
  - The strict reader (`strict_range_knobs.rs:73-75`, `machine_count(raw, rungs_within_cell_budget()).is_some_and(|count| count >= 2)`) refuses both values. Its own doc says it refuses "values the existing readers would clamp".
  - The result is one knob with two outcomes, and on the ordinary path the operator's stated value is silently replaced (CLAUDE.md §4). This is the CE-6/CE-44/P8-04 class on a knob none of those covered. `r1-slices/slice03 F2` was about the old `fold_rungs` reader, not this clamp.
- **Repro (ran):** a throwaway test with `knobs::set("BRUTEX_GRID_RUNGS", v)` and `grid_rungs(&[])`, then `knobs::refused()`:
  - `ASKED 1 USED 2 REFUSED None STRICT_OK false`
  - `ASKED 100000 USED 7 REFUSED None STRICT_OK false`
- **Minimal fix:** when `n < 2 || n > rungs_within_cell_budget()`, call `knobs::refuse_value("BRUTEX_GRID_RUNGS", &raw)` before clamping, so the banner names it. Better: use the strict predicate and fall back to the derived count, with the refusal recorded.

## CE-76 (medium): the results ledger stores the instrument word as the operator typed it, so a case-variant rerun of the same run is refused and filters split

- **Where:**
  - The raw word reaches the ledger row at `crates/cli/src/lib.rs:3751` (sweep-stored and sweep-audited-stored), `:6335` (audit-stored), `:7209` and `:16443` (audit-range, range-all, range-rung, and the api routes that call them).
  - The row is built at `:17847` (`underlying: results::field(into.underlying)`).
  - The comparison is at `:17692-17696` (`same_run_answer` compares the whole `Record`, `underlying` included), and the refusal is raised at `:17729-17733`.
  - The run identity uses the canonical key: `instrument: &loaded.key`, from `stored::swept_index`. `Symbol::new` upper-cases the word, so `nifty`, `NiFtY` and `NIFTY` all resolve to `NSE-NIFTY` (pinned by `stored.rs:3961-3973`).
- **Why it is wrong:**
  - The same computation gets one identity and two possible ledger rows. The first spelling wins the row.
  - A later rerun spelt differently finds that identity already held. `verify` then fails `same_run_answer` on the `underlying` bytes alone and refuses:
    `run … is already in the ledger, but its deterministic fields differ from this exact rerun. The existing row was kept and the new answer was refused`
  - That message falsely tells the operator the engine is nondeterministic, and a byte-identical computation is refused for good. This breaks §3 rule 5 (idempotence).
  - `cli top/results FEED UNDERLYING` (`lib.rs:8310-8315` and `8476-8477`) and `latest_for` (`:15975-15983`) compare the raw bytes. So `cli top dhan NIFTY` prints "NO COMPLETE RUN matches … or nothing has been recorded yet" over a run recorded as `nifty`, and exits 0. The ledger listing also shows `nifty` and `NIFTY` as two instruments.
  - The api body `{"underlying":"nifty"}` reaches this path the same way: `sweeprun.rs:1061` passes the raw word through.
- **Repro (ran):** a throwaway test in `stored_month_publication_tests.rs`. One fixture identity was built from `swept_index("NIFTY")`, then `record_swept_run` was called three times with `underlying` set to `"nifty"`, `"NIFTY"` and `"nifty"`:
  - `FIRST: Ok(..)`
  - `SECOND: Err("run 99e9…c6 is already in the ledger, but its deterministic fields differ from this exact rerun. …")`
  - `THIRD: Ok(..)`
- **Minimal fix:**
  - Record `key.underlying.as_str()` (the canonical symbol from the loaded key) in `Recording.underlying` at the four sites, not the argv/body word.
  - Canonicalise the `top`/`results` filter through `stored::swept_index` and refuse an off-surface word.
  - Optionally canonicalise `underlying` once at the api boundary.

## What held

| Area | Case(s) | Result |
|---|---|---|
| cli dispatch | missing / extra args, unknown word, wrong arity | Refused by name with usage. A known word with the wrong arity is named as such. A non-UTF-8 argv word is refused by position (`text_args`). |
| cli numbers | negative, `+`, overflow, 0 for SESSIONS / MIN_HITS / SUPPORT_PPM / TOP / MAX_POINTS / MIN_RR | Typed parse plus range checks; refused. SESSIONS is capped at 3650. |
| cli dates | month 0 / 13 / 255, backwards range, span > max, year past u16 | `months_between` (`stored.rs:2564-2606`) refuses each by name. `pool_arm`, `expression`, boolean and `checksum` refuse too. A 2026-02-30 day is not an input anywhere (months only); boolean month checks go through `session::Day::new`. |
| Instrument surface | BSE word, MCX, `INDIAVIX`, `SENSEX`, `FINNIFTY` / `MIDCPNIFTY` / `NIFTYNXT50`, a contract string, a non-F&O stock, a space or control char, a 10k-character word | `swept_index`: one sentence, clipped and escaped, "Nothing was read". Lower case folds to the canonical index or share; see CE-76 for what the ledger does with the original word. |
| Rungs | `1day`, `1s`, a typo | `swept_rung`, `stored::rung`, `batch` and the api `rung_from` refuse. An empty `rungs:[]` is refused, not widened. |
| §8 config file | missing, FIFO, oversize, empty, a dup top key, a dup vendor key, a dup vendor table, an unknown key or table, a missing org / env / region, wrong region, `/`, `..`, upper case, a secret-like segment, an inline comment, non-UTF-8 | Each halts with the line number and no default (`config.rs:828-900`, `1040-1165`). CRLF is handled by `str::lines`. A BOM makes line 1 `UnknownKey`/`Unparseable`: loud, though the BOM itself is not named. The caller (`server.rs:10282-10299`) refuses an unset, empty or relative HOME. |
| Env knobs | empty / blank folders, HOME, switches, non-UTF-8 | `core::knob` (CE-5/33/36-39 fixes held). A count knob with an unusable value is named under `KNOB REFUSED` (except CE-75). `BRUTEX_CEILING` refuses. The api boolean budgets are strict, with no default. `BRUTEX_AUTOPILOT`: any value except `run` stays paused and is announced. `BRUTEX_LOG_LEVEL` names unreadable clauses. |
| api JSON bodies | dup known key, `null`, fraction, array, wrong type, u64::MAX year, backwards span, month 0/13, `screen_budget_ms`, a known field the route does not read | Strict serde with `WireField` (no null-as-absent). Year is bounded before the multiply. Each refusal is named (D-0685, D-1972). `sweep-stored` refuses a multi-month span instead of narrowing it. Generated-bar commands are refused with the reason. |
| api boolean launch | oversize, symbol whitespace / commas, non-canonical bits, zero digest, values past their limits | Refused (`booleanlaunch.rs:101-171`). Duplicate symbols are refused by `ResearchScopeV1::new`. |
| `/pull/spot` members | an unknown member, one outside the target, a non-symbol, too many | Refused before network work (`ingest.rs:1480-1495`, `server.rs:7432-7470`). |

Not re-reported, already filed: argument-error exit codes and the bare `ParseIntError` text (P8-03); descend PER_WEEK and elite MAX_POINTS wording (P8-02); policy knobs with two readers (P8-04); `/engine/top.json` feed validation (P1-02-06); `/universe/resolve` defaulting to Groww (P1-02-04).

# Pass 8: docs against behaviour (final/all-fixes-zero @ 1f4de71)

Read-only audit of `/home/claude/wt/zero3`, detached at `1f4de71` ("docs: CE-19's decision (D-1981) and invariant (ZE-02)"). No cargo was run: every finding below is low and is argued from source. Ids already in `tests-docs-security*.md` and `crash-edge*.md` are not repeated (P1-01-*, P1-02-*, P1-06-*, P2-02-01, P3-01-*, P3-02-03/04/05/06/07, CE-6/CE-44 and the others).

## Verdict

**1f4de71: 5 new findings, all low. There are no medium or high findings.**

- The cli dispatcher, `USAGE` and `COMMANDS` agree on all 35 command words and on every arity. Every documented `cli …` invocation in `docs/`, `AGENTS.md` and `README.md` has a matching arity.
- Every one of the 50 API paths that `web/src` names has a route in `route_table`. Every detail JSON route checks its query keys against a strict allowlist, and those lists match what the page sends.
- Every grep or git check in `docs/09-verify.md` §2–§6 prints what the page says it prints.

The new defects are in four places:
- exit-code semantics
- refusal wording that contradicts the grammar the parser actually accepts
- one `USAGE` legend example
- two drifted readers of the same environment knob

## Theme 1: the cli subcommands

Source: `crates/cli/src/lib.rs:2186-2315` (`dispatch`), `USAGE` (`lib.rs:354-606`), `COMMANDS` (`lib.rs:2355`) and each arm. Every arm is a fixed-length slice pattern. Too many or too few words falls to `unmatched`, which names the arity and exits 2. An unknown word, `--help` included, exits 2 and prints the usage. There are no flags: every input is a positional word or an environment knob. A non-UTF-8 argument is refused with exit 2 (`run_durable_os`).

The **Refusal exit** column gives the code when the arguments parse but the work refuses. **Bad-arg exit** is the code for a malformed argument. The documented meaning (`lib.rs:324-331`) is 1 for "reasonable but could not do it" and 2 for "does not understand".

| Command | Args (parsed) | USAGE matches? | Numeric parse | Refusal exit | Bad-arg exit | Notes |
|---|---|---|---|---|---|---|
| sweep | SESSIONS MIN_HITS | yes | i64 1..=3650 ; u64 ≥1 | 1 (also on NOTHING MEASURED) | 2 | |
| auto | SESSIONS | yes | i64 1..=3650 | **0 always** | 2 | P8-01 |
| audit | SESSIONS MIN_HITS | yes | as sweep | 1 | 2 | |
| sweep-stored / audit-stored | 6 | yes | u16, u8, u64 ≥1 | **2** | 2 | P8-03 |
| checksum-audit-stored | 7 | yes | u16, u8 via YearMonth, u64 | 1 | **1** | P8-03 |
| sweep-audited-stored | 9 | yes | u16/u8/u64 | 1 | **1** | P8-03 |
| audit-audited-range | 11 | yes | YearMonth, u64>0 | 1 | **1** | P8-03 |
| expression-stored | 6 | yes | – | 2 | 2 | P8-03 |
| expression-search-stored / -backtest-stored | 9 / 12 | yes | own `Args::parse` | **2** | 2 | P8-03 |
| audit-range | 8 | yes | u16,u8; months validated in `stored::months_between` | 2 | 2 | P8-03 |
| screen | 12 | yes | u16,u8,u64 ppm,i64 >0,i64 ≥0,usize >0 | 2 | 2 | |
| elite | 10 | yes | i64 ≥0 (0 = no ceiling), usize >0 | 2 | 2 | P8-02 (wording) |
| descend | 10 | yes | u64 ppm, CADENCE `n`, `n/y`, **`n/w`** | 2 | 2 | P8-02 |
| range-all / range-rung / pool | 7 / 8 / 7 | yes | ppm or `auto` | 2 | 2 | |
| research-plan | 1 | yes | – | 2 | 2 | |
| policy-check | 2 | yes | – | 2 | 2 | |
| boolean-* (6 commands) | 11/11/12/15/14/17-18 | yes (18th since P3-02-03) | own parsers | **2, plus the full USAGE** | 2 | P8-03 |
| ledger-all / ledger-v6 | 8 | yes | u64 ppm, u64 >0 | 2 | 2 | |
| ledger-v6-replay | 12 | yes | month 1..=12 checked | 2 | 2 | |
| verify | 2 | yes | – | 1 | – | |
| fold-audit | 6 | yes | u16,u8 | 1 | 2 | |
| auto-stored | 7 | yes | u16,u8 | 2 on a refusal line; **0 on nothing measured / halted** | 2 | P8-01 |
| top / results | 0 or 2 | yes | – | 1 | 2 | |
| sweep-all | 3 | yes | u64 ≥1 | 2 | 2 | |

Environment knobs: about 40 `BRUTEX_*` names are read through `crate::knobs`. They were already enumerated by conc-pass7. The new item here is P8-04: two readers of the same knobs accept different values.

## Theme 2: the api routes

Source: `crates/api/src/server.rs:16509-16843` (`route_table`) plus `/inspection.json` at 16132. Of 66 registered routes, `web/src` calls 50 by literal path. `boolean-statistics` and `boolean-admission` are built dynamically (`lib/boolean-evidence.js:87`). None of the paths the page calls is missing from the router.

| Route (method) | Query keys the page sends | Server accepts | Unknown key | Refusal status | Prior / new |
|---|---|---|---|---|---|
| /boolean-campaign.json GET | identity, pin | allowlist identity, pin | refused | 4xx | ok |
| /boolean-candidates.json GET | identity, kind, offset, limit (+completion, candidate) | allowlist | refused | 4xx | ok |
| /boolean-oos.json GET | identity, kind, offset, limit, completion, candidate | allowlist | refused | 4xx | ok |
| /boolean-qualified-search.json GET | identity, pin, batch, rung, offset, limit, completion | allowlist | refused | 4xx | ok |
| /boolean-qualified-campaign.json GET | identity, pin | allowlist | refused | 4xx | ok |
| /boolean-{statistics,admission,qualification}.json GET | identity, completion, kind, offset, limit, setting, period | allowlist | refused | 4xx | ok |
| /candidate-trades.json GET | identity, attempt, model, tier, offset, limit, digest, rank, direction | allowlist | refused | 4xx | ok |
| /expression-search.json GET | identity, limit, snapshot(_seal), cursor(_seal) | allowlist | refused | 4xx | ok |
| /index-stop.json GET | identity, kind, offset, limit, completion, setting | allowlist | refused | 4xx | ok |
| /index-stop-candles.json GET | identity, pin, setting, kind, trade, before, after | allowlist | refused | 4xx | ok |
| /index-stop-qualification.json GET | identity, kind, offset, limit, completion, setting | allowlist (+period) | refused | 4xx | ok |
| /index-stop-ranking.json GET | identity, timeframe, filter, offset, limit, sequence, completion | allowlist | refused | 4xx | ok |
| /index-stop-vix.json GET | identity, pin, setting, offset, limit | allowlist | refused | 4xx | ok |
| /sweep-evidence.json GET | identity, kind, page, limit, attempt | allowlist | refused | 4xx | ok |
| /engine/top.json GET | feed, underlying | allowlist | refused | 400 | P1-02-06 |
| /engine/boolean-launch.json GET | max_points | exactly `max_points=` | refused | 4xx | ok |
| /backtest/run.json GET | attempt | `requested_attempt` | 400 | 400/503 | ok |
| /instruments.json, /universes.json, /store.json GET | feed (+encoding, month) | absent feed = Dhan | – | 400 named | known (P1-02-04 context) |
| /frontier.json, /trades.json GET | selector/page | lenient | ignored | – | P1-01-04 |
| /bars/window.json, /logs.json, /backtest.json | … | … | ignored | – | P1-01-01..03, P1-02-02 |
| /autopilot/control POST | action | `action`; other fields ignored | – | 400 | P5-06 class |
| /engine/command POST | JSON command | EVERY_COMMAND (8) | 400 | 400 | P3-02-06 |
| /universe/resolve POST | feed | default Groww, 200 refusals | – | 200 | P1-02-04 |
| /pull/recovery, /ingest/queue, /autopilot/resume, /audit/page | (not called by web/src) | – | – | – | unused by the page, no defect |

## Theme 3: `docs/09-verify.md` checks that need no build (ran)

| § | Command | Doc says | Printed at 1f4de71 | Holds |
|---|---|---|---|---|
| 1 | `find web/src -type f -newer web/build/index.html` | nothing, except in a fresh worktree (stated caveat) | many files: fresh worktree, mtime in checkout order | yes, under the stated caveat |
| 2 | `find crates -name build.rs \| grep -vx crates/cli/build.rs` | nothing | nothing (rc 1) | yes |
| 2 | `grep -rlE 'Command::new\("(npm\|npx\|node\|yarn\|pnpm)"' crates/*/src` | nothing | nothing | yes |
| 3 | theme.css `--(raise\|panel2\|accdeep):` count | 3 | 3; Picker.svelte reads them at 4 sites, as stated | yes |
| 4 | `feeds.active =` in terminal, markets, ingest, db | 0 ×4; one hit in all routes, in /backtest, in a comment | 0 0 0 0; one hit `backtest/+page.svelte:6520` (comment); writer `lib/feed-startup.js` | yes |
| 5 | blackScholes/normCdf in db; raw hex in markets, db | 0 0 0 | 0 0 0 | yes |
| 5 | terminal / ingest raw hex | 47 / 3 | 47 / 3 | yes |
| 5 | `web/src/routes/+page.svelte` absent, `+page.js` present | yes | `+page.js` only | yes |
| 6 | ISIN count in universe.rs / `nse_isin` in constituents.rs | 1812 / 25 | 1812 / 25 | yes |

## New findings

### P8-01 low: `cli auto` always exits 0, and `cli auto-stored` exits 0 when it measured nothing or halted. probeapi-6 fixed this for `sweep` only.

- `crates/cli/src/lib.rs:2306-2312`:
  ```rust
  ["auto", sessions] => match parse_sessions(sessions) {
      Ok(s) => {
          out.push_str(&auto(s));
          OK
      }
  ```
  `auto_with` (`lib.rs:3915-3931`) can return `refused: …`, and `render_auto` (`runner/src/report.rs:977-1008`) prints `threshold chosen NONE <- no rung measured anything` with the `outcome NOTHING MEASURED` verdict. Neither one changes the exit code. The arm two cases above reads that same verdict (`lib.rs:2197`: `if carries_refusal(&text) || nothing_measured(&text) { FAILED }`), with a comment saying exiting 0 "let `cli sweep 1 10 && <next>` proceed".
- `auto_stored_arm` (`lib.rs:1161-1183`) uses `carries_refusal` only. Meanwhile `auto_stored_kernel` records `attempt.finish(sweep_completion(found.affordable && found.outcome.is_complete(), …))` (`lib.rs:4097-4100`), which writes `Completion::Refused` or `Halted` to the sweep evidence. The same run exits 0, and `run_durable` therefore writes `operation_audit::Phase::Completed` (`lib.rs:2124-2128`). Two durable records of one invocation disagree.
- The test `a_valid_sweep_and_a_valid_auto_both_render_and_exit_zero` (`lib.rs:21254-21256`) pins `run(["auto","1"]) == OK`. At 1 session, `sweep` measures nothing for every threshold (`a_sweep_that_measured_nothing_exits_non_zero`, `lib.rs:21217`).
- Repro (not run): `cli auto 1; echo $?` should print the NOTHING MEASURED verdict and then 0. Likewise for `cli auto-stored …` on a span too short to warm.
- Fix: in both arms, use `if carries_refusal(&text) || nothing_measured(&text) || threshold_none(&text) { FAILED } else { OK }`. Change the pinned test to `auto 6`.

### P8-02 low: two refusal sentences contradict the grammar their own parser accepts and `USAGE` documents (`descend` CADENCE, `elite` MAX_POINTS). `/w` is parsed and documented nowhere.

- `lib.rs:1721-1724` (`descend_arm`):
  ```rust
  (_, _, _, _, _, Err(_)) => refuse(out,
      "PER_WEEK must be a whole number of trades per week, at least 1."),
  ```
  It throws away `parse_cadence`'s own `BAD` text (`lib.rs:14161`: "CADENCE must be a whole number of trades per week (`3`), or per year with a `/y` suffix (`6/y`)"). That function's doc says the sentence "names both spellings rather than the one that failed, because an operator who typed the wrong unit cannot tell which one this accepts". So `cli descend … 6/yr` is told the argument is PER_WEEK and must be a whole number, which is the opposite of what `USAGE` says CADENCE is. `parse_cadence` also accepts a `/w` suffix (`lib.rs:14171`), which `USAGE`, D-0594 and the `BAD` text do not mention.
- `lib.rs:1324-1327` (`elite_arm`): an unparsable MAX_POINTS gets "MAX_POINTS must be a whole number of index points, 1 or more". The same arm accepts 0 (`lib.rs:1263-1290`), and `USAGE` tells the operator to "pass ZERO for no ceiling".
- Repro (not run, the code is direct): `cli descend zerodha NIFTY 15min 2024 1 2024 6 200000 6/yr` prints the PER_WEEK sentence. `cli elite zerodha NIFTY 15min 2024 1 2024 6 x 5` says "1 or more".
- Fix: `(.., Err(why)) => refuse(out, why)` in `descend_arm`, and document or drop `/w`. In `elite_arm`, say "0 for no ceiling, or a positive whole number of index points".

### P8-03 low: exit codes 1 and 2 are used the opposite way from their documented meaning across most commands

- `lib.rs:324-331`: `FAILED = 1` is "asked for something reasonable and could not do it". `MISUSED = 2` is "an unknown word, a malformed argument". The `results` doc block (`lib.rs:1137-1158`) restates the rule: "`FAILED` and not `MISUSED` because the arguments were fine; what could not be done was the work".
- Work refusals that exit 2: `stored_month_arm` (`lib.rs:1866-1870` `if refused { MISUSED }`), `range_all_arm`, `range_rung_arm`, `pool_arm`, `descend_arm`, `audit_range_arm`, `screen`, `elite`, `sweep_all_arm`, `ledger_all_arm`, `ledger_replay_arm`, `auto_stored_arm`. Every boolean command and both expression-search commands also exit 2, through `crate::refuse` (for example `boolean_catalog_command.rs:35-40`, `expression_search.rs:89-113`), and these print the whole 250-line `USAGE` after an I/O or store failure. Examples of work refusals: a month missing from the store, an unstamped build, a ledger write failure.
- Malformed arguments that exit 1, with no usage: `command_report` (`lib.rs:3270-3281` `Err(why) => … FAILED`) serves `checksum-audit-stored`, `sweep-audited-stored` and `audit-audited-range`. So `cli checksum-audit-stored zerodha NIFTY 1min x 1 /r 10` prints "CHECKSUM AUDIT REFUSED: invalid digit…" and exits 1.
- Effect: a script cannot tell "fix your arguments" from "the store or build could not do it", which is the distinction the two codes exist to make. `results`, `top`, `verify` and `fold-audit` follow the documented rule. Most other commands do the opposite.
- Repro (not run): `cli sweep-stored zerodha NIFTY 1min 1999 1 10` on a store without 1999 exits 2.
- Fix: split each arm's `refused` into parse errors (2, with usage) and work refusals (1, without usage). Make `command_report` return `MISUSED` for argument-shape errors.

### P8-04 low: the same eleven policy knobs have two readers that accept different values

- `expression_pricing.rs:80-114` (`validate_overrides`) refuses `BRUTEX_MIN_WIN_RATE_BP > 10_000` and any `BRUTEX_PROTECTED_EXITS` other than exactly `"0"` or `"1"` ("no default was substituted").
- `Rules::operator` (`lib.rs:9344-9384`) is the reader behind screen, range-all, range-rung, pool, elite, ledger-* and the browser sweep. It goes through `Rules::stated` → `knobs::nonnegative_floor`, which accepts any `i64 ≥ 0`. `protective_exits_required` (`lib.rs:9403-9405`) is `Self::stated("BRUTEX_PROTECTED_EXITS").unwrap_or(1) != 0`.
  - With `BRUTEX_MIN_WIN_RATE_BP=20000` (a 200% win rate), that path runs without a word. The floor admits no row, and `screen_cascade` then relaxes to tiers, so the operator's typo is replaced by a looser policy rather than refused.
  - `BRUTEX_PROTECTED_EXITS=2` or `=01` is "on" there, while `expression-backtest-stored` refuses it.
- This is the CE-6 / CE-44 "one rule, two readers" class, on knobs those findings did not cover.
- Repro (not run): `BRUTEX_MIN_WIN_RATE_BP=20000 cli screen …` runs, while `BRUTEX_MIN_WIN_RATE_BP=20000 cli expression-backtest-stored …` refuses.
- Fix: one shared validator in `knobs`, called by both `Rules::stated` and `validate_overrides`. It should take the per-knob upper bounds and treat PROTECTED_EXITS as exactly 0 or 1.

### P8-05 low: operator-facing text names a rung and a command that the binary refuses

- `USAGE` legend (`lib.rs:600`): `RUNG the bar length as its directory word -- 1min, 1day`. Every command that sweeps a RUNG refuses `1day`. Both `swept_rung` copies (`lib.rs:8633-8642` and `batch.rs:264-275`) say "`1day` is not a rung this engine sweeps … It is stored and readable and it is not swept". The legend's own example therefore fails on sweep-stored, audit-stored, screen, elite, descend, range-rung, pool, auto-stored, audit-range, sweep-all and both expression commands. Only `checksum-audit-stored` accepts it.
- `lib.rs:8082-8087` (`no_frontier`), printed by `cli top`: "Rows are written by runs made after `cli frontier` landed". `frontier` is not in `COMMANDS` (`lib.rs:2355-2391`), so an operator who types it gets "`frontier` is not a command this build knows".
- `crates/cli/src/main.rs:12-19` still has a heading "# The four commands", listing four of the 35.
- Repro (ran, grep only): `grep -n '1min, 1day' crates/cli/src/lib.rs` → 600. `grep -c '"frontier"' <COMMANDS block>` → 0.
- Fix: change the legend to `1min … 60min (the eight intraday rungs)`, say "runs recorded since frontier rows were added (D-xxxx)", and point the main.rs heading at `USAGE` instead of listing commands.

## Checked and holding (no finding)

- `COMMANDS`, `dispatch` and `USAGE` name the same 35 words, guarded by `every_command_is_listed_in_both_places`. Each arity in `USAGE` equals the slice pattern, and that includes `boolean-qualified-search-stored`'s optional 18th argument (P3-02-03 is fixed).
- No `--flag` is parsed anywhere, and none is documented. `--help` is refused as an unknown word, with usage and exit 2. No document claims `--help`.
- `USAGE` no longer contains `%%` (P3-02-04 is fixed). VENDOR legend: `Vendor::ALL` holds 5, and the legend names those 5.
- Every documented `cli …` invocation in `docs/`, `AGENTS.md` and `README.md` (28 distinct forms) has an arity that matches dispatch.
- Numeric parsing is `str::parse::<uN/iN>`, which refuses overflow. No wrapping or saturating parse was found on an argument. `stored::months_between` refuses month 0 and months 13-255, a backwards span, and spans over 1,200 months. `Rules::operator`'s `usize::try_from(..).unwrap_or(25)` and `u64::try_from(..).unwrap_or(0)` cannot fire, because `stated` returns only values ≥ 0.
- All 50 web-called paths are routed. `/backtest/run.json?attempt=` is parsed (`sweeprun.rs:2322`). The detail routes refuse unknown, repeated or empty keys. `/engine/boolean-launch.json` accepts exactly `max_points=`.
- `index-stop-qualified-search-stored` exists only in the api's `EVERY_COMMAND` and is documented as an "internal worker operation" (`index_stop_launch.rs:8`), so its absence from cli dispatch is deliberate.

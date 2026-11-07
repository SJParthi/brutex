//! The text of `golden/FORMAT.md`: how a separate program reads a scenario.

/// `FORMAT.md`.
pub const FORMAT_MD: &str = r"# FORMAT: how to read a FIB-PREVIEW-REF golden scenario

PREVIEW: an independent reference build on seeded synthetic data; not a result, says nothing about profit.

## The file

Each scenario is one `<ID>.md` file. Its records are the lines of the one fenced
block that opens with three backticks and `text` and closes with three backticks.
Read only those lines. Split each on single spaces.

| First token | Meaning |
|---|---|
| `fibref-golden` | the format and its version, `1` |
| `id` | the scenario id, the file's stem |
| `kind` | which rule family: `rank`, `ladder`, `e1`, `e2`, `e3`, `gate`, `walk`, `charges`, `account`, `walkforward`, `gate`, `luck`, `lots`, `verdict`, `e2e` |
| `seed` | the SplitMix64 seed the synthetic inputs came from (the inputs are written out in full: nothing needs regenerating) |
| `readings` | `headline`, or the readings that differ from it, `Axx=name` joined by commas (report section 4) |
| `title` | plain words |
| `cols <record> <names...>` | the column names of the positional `in <record>` lines that follow |
| `in <record> <values...>` | a positional input row under its `cols` line |
| `in <record> k=v ...` | a keyed input |
| `out <record> k=v ...` | an expected output; every one must match |
| `end` | the last record |

## Values

- Prices and money: paise, integers. `_milli` fields: milli-paise (a rung price is exact there).
- Times: `HH:MM:SS` IST. Dates: `YYYY-MM-DD`. Months: `YYYY-MM`.
- Booleans: `1` and `0`. An absent value: `-`. A refusal or status: its name (`day_cap` and `month_cap` print with an underscore in keyed fields).
- `in minute m o h l c v held padded g`: session minute `m` (0 = 09:15), Zerodha OHLCV, `held` = Zerodha holds it, `padded` = vendor-padded, `g` = the last GDFL trade inside it or `-`. A hole minute is `held = 0`, or `padded = 1` with a `g`.
- `in second t o h l c v`: one GDFL second; `v = 0` printed nothing tradable.
- `in top by_candle=0101...`: the rank clause (in top N for the recipe's side) at each candle's close, candle 0 first.

## Rules the outputs follow (the report's section 3 holds every rule with its source)

- Candles of `tf` minutes are labelled from 09:15 (A04 headline; under `A04=clock` on multiples of `tf` from midnight, the first running from 09:15 to the first multiple); a candle labelled t closes at `min(t + tf, 15:30:00)`; a candle with a hole minute is incomplete. A candle is formed only when every minute it covers is in the data.
- `out ladder before_candle=i`: the running ladder of candles `0..i-1` (the one candle `i` is judged against). `rungs` lists the twelve rung prices in milli-paise in the order 0, 0.236, 0.382, 0.5, 0.618, 0.786, 1, 1.272, 1.618, 2, 2.618, 3.
- Status order: when several clauses fail, the first in this order is printed. E1 close: `incomplete_candle`, `tainted_ladder`, `ladder_undetermined`, `holding`, `height`, `colour`, `not_in_top_n`, `after_last_entry`, `ok`. E1 second: `no_fill_data_window`, `no_trigger_ratio` at the arming, then at the trigger `tainted_ladder`, `ladder_undetermined`, `colour`, `not_in_top_n`, `after_last_entry`, `ok`. E2 close: `touch_order_unsourced`, `touch_number`, `not_in_top_n`, `after_last_entry`, `ok`. E3 close: `incomplete_candle`, `break_ordinal`, `not_in_top_n`, `after_last_entry`, `ok`. Account: `while_open`, `day_cap`, `month_cap`, `slots_full`, `margin_cap` (leverage), then the margin and exposure checks (`margin_cap`, `exposure_cap`, `size_refused`), then the walk's own refusal.
- Second readings: an `e2.signal` of `reading=second` with `t=-` is an arming that ended (`end=<time>:<why>`) without a trigger; its clauses are read only at a trigger, so its status stays `ok`, and it is no entry. E1S and E3S write a record only for an arming that triggered or was refused at the arming.
- `out gate.clause rr_at_least=k`: the clause S-RR at least k (seeds 2, 3, 5) on the official and the worst charges: `ok`, `rr_band`, or the S-RR refusal's own name.
- `*.levels`: `stop_milli`, `t1_milli`, `t2_milli`, `t3_milli` in Zerodha space; `stop_g` and `t*_g` mapped by R1 (`L x G / Z` with Z, G of the last 1-minute candle closed by the decision) and rounded against the position to the 5-paise step; `no_level_ratio` when R1's minute has no price; `no_ladder_target` when no usable ladder.
- Walk fills: `order=entry`, `exit` (the strategy's exit order) or `square_off` (the broker's 15:10 order). `flags` lists the walk's rule bits joined by `|`.
- Charges: `charges.dated_official` and `charges.today_official` are the official reading on the two clocks; the headline is the dearer. `charges.*_worst` is the worst reading, a stress line that decides nothing. `estimator.*` replicates Dhan's order estimator screen.
- Net = money in - money out - the headline charges; a win is net above zero.
";

# SLICE apir: api render.rs / ingest.rs / web number handling

**Verdict:** two new low-severity display defects in `render.rs`, both confirmed by a probe that ran, plus one info-level latent sign bug. `ingest.rs` number parsing is clean. The web/ paisa and u64 handling is clean: it uses integer quotient and remainder with the sign kept separate, and BigInt or decimal strings for u64. No look-ahead surface exists in this slice. Every render loop does constant work per row.

Probe: `/tmp/claude-0/-home-claude-brutex/e43249a0-a963-5273-8703-91766f1116e2/scratchpad/probes/apir/` (path deps on api, core, store, pull; built with the shared target dir).

## Findings

### apir-1 (low): the store grid's shading scale is taken from a feed the page does not show
- `crates/api/src/render.rs:2354-2366` (`page_peak`) together with `:2374`, `:2397-2402` and `:2466-2470`:
  ```rust
  fn page_peak(rows: &[Coverage]) -> u64 {
      let mut peak = 1u64;
      for row in rows {
          for &(_, n) in &row.rows {      // every vendor
  ```
  The table, however, draws only `row.rows.iter().filter(|(v, _)| *v == view.feed)`.
- Why it is wrong: the page shows exactly one feed (D-era `StoreView::feed`: "it shows exactly one"). The swatch quartile and the sentence "quartiles of {peak} — the fullest month on this page" use a maximum taken across all vendors. Two things go wrong. The page prints a number that appears in no cell. And the fullest month of the shown feed is shaded as the lowest quartile, so the "readable without reading it" grid misranks months.
- Repro (ran): one row with `Groww=Some(10)` and `Dhan=Some(100)`, `feed: Vendor::Groww`, rendered with `store_page`:
  ```
  P1 [quartiles of] ...quartiles of 100 — the fullest month on this page — and not of an ideal month, because
  P1 [class="sw q] ...class="sw q1" viewBox="0 0 12 12" width="12" height="12" role="img" aria-label="held, 10 r
  P1 [>10<] ...>10</a></td></tr></tbody></table></div>...
  P1 [>100<] ABSENT
  ```
  Expected: "quartiles of 10" and swatch `q4`. Actual: "quartiles of 100" (absent from the page) and `q1`.
- Fix shape: compute the peak over `n` where `v == view.feed`. Related: `filled` and the strip use `Coverage::is_held`, which is also cross-vendor, but the prose there says "some vendor holds", so that one is stated honestly.

### apir-2 (low): the share bar's rounding remainder is drawn as a segment of a reason with zero drops
- `crates/api/src/render.rs:1376-1391` (`share_bar`):
  ```rust
  let width = if last {
      100u64.saturating_sub(at)
  } else {
      drops.of(reason).saturating_mul(100) / total
  };
  if width > 0 { ... <title>{label}: {count}</title> ... }
  ```
- Why it is wrong: each non-last segment is floored, and the last reason absorbs the remainder (up to 3 points) whatever its own count is. When the last reason (`AtOrAfterSessionClose`) counted 0, the bar still draws a coloured slice for it. Its tooltip reads ": 0", which contradicts the bar. The doc comment's promise of "no gap that is really a division remainder" is kept by charging the remainder to a reason that did not happen.
- Repro (ran): `Drops { before_window: 1, after_window: 1, before_open: 1, after_close: 0 }` rendered through `audit_page`:
  ```
  ...<rect class="r2" x="66" y="0" width="33" height="10"><title>before the session open: 1</title></rect><rect class="r3" x="99" y="0" width="1" height="10"><title>at or after the session close: 0</title></rect></svg>
  ```
  Expected: no r3 rect, or the remainder given to a non-zero reason (largest remainder). Actual: a 1% r3 slice labelled 0.

### apir-3 (info, latent): `kind_cells` drops the sign of a strike in (-1.00, 0) rupees
- `crates/api/src/render.rs:517-522`:
  ```rust
  "<td>Option {}</td><td>{}</td><td class=\"num\">{}.{:02}</td>",
  side.as_str(), expiry, strike.rupees_trunc(), strike.paisa_part().abs()
  ```
- Why: `rupees_trunc` truncates toward zero, so -5 paisa gives `0` and `5`, rendered "0.05". -105 renders correctly as "-1.05". `api::bars::rupees` and every web formatter (`money.js::rupee`, `candidateMoney`, `indexStopPoints`, `strikeExact`, `paisaText`) take the sign separately and get this right.
- Repro (ran, same arithmetic on `core::price::Paisa`):
  ```
  P3 raw=-5 kind_cells-format=0.05
  P3 raw=-105 kind_cells-format=-1.05
  ```
- Reachability: not reachable today. Every production constructor of a strike refuses values <= 0: `core::vendor::parse_strike` (D-1311) and `lake::contract::parse_strike`. It becomes reachable only if a future constructor admits a non-positive strike. Severity info.

## Checked and clean
- `render.rs` integer display math: `hhmm`, `elapsed` (truncates for display, no float), `quartile` (saturating, clamps 1..=4, `whole.max(1)`), dashboard bar `(v*100/peak).max(3)` with a peak floor of 1. The dashboard values are plain `usize::to_string`, so the `parse::<u64>` fallback branch is never hit (server.rs:1024). The capture panel bars use `Drops::peak` (>= 1). `count_cell` uses an em dash for "unmeasured", never 0.
- `render.rs` per-row cost: `table`, `universe_cell`, `vendor_cell`, `isin_cell`, `kind_cells`, `coverage_table` and `coverage_strip` all do constant work per row over a fixed set of vendors, universe bits and reasons. `page_peak` is O(rows on page × vendors), the same order as drawing the page. There is no lookup inside the loops.
- `ingest.rs`: `parse_day` (digits only, rejects the `+` sign), `parse_window`, `epoch_secs` (checked and saturating conversions). The rate is parsed `f64` and passed through `pull::pricing::Rate::measured`, which refuses NaN and inf (`is_finite`) and |rate| > 1.0. JSON in `waiting_json` and `queue_answer` emits only small counters and epoch seconds. No price passes through ingest.
- web/ paisa display: `money.js::rupee` (refuses non-safe integers with an em dash; quotient and remainder on the magnitude), `candidate-trades.js::candidateMoney` and `index-stop-results.js::indexStopPoints` (BigInt on decimal strings, sign separate), `instrument.js::strikeExact`, and `db/+page.svelte::paisaText`. All are correct for negatives, e.g. -5 renders as "-0.05" / "−0.05", never "0.-5".
- web/ u64: mask words and offsets are decimal strings decoded with BigInt (`canonicalU64`, `decodeMaskWords`), consistent with the prior report `webcontract.md`. Bare-number i64 prices on `/bars.json` and `/bars/window.json` (server.rs:2046, 3421) are not guarded in `paisaText`. This is the same unreachable-not-guarded class as docs/06-limits.md §41.5 (2^53 paisa is about ₹90 lakh crore), so it is not reported.
- web/ `money.js::whole` uses `Math.round` (half toward +inf) for averages displayed on /audit. It is display only and never stored, so it is not a §7 storage-rounding issue.
- Look-ahead: not applicable to this slice. Render and ingest compute no signal; the ingest window clamp narrows toward the past and says so (`clamped_from`).

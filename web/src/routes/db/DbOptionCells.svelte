<script module>
  /**
   * /db — ONE OPTION BAR'S CONTRACT CELLS
   *
   * Type, strike, moneyness, intrinsic, extrinsic, IV and the five greeks: the
   * eleven cells from column 15 of one bar row. Rendered inside DbBarsGrid's
   * `<tr>`, so the grid's positional column CSS still counts these cells.
   *
   * Moved out of `routes/db/+page.svelte` by WP-00; owned by WP-53, which
   * fixed here:
   *
   * - DB-009: every bar's spot, rung and greeks are asked in the bar's OWN
   *   IST month — a Close-desc page over 2018-09..2019-03 asked every bar in
   *   2019-03 and drew 50 false blanks.
   * - DB-037, DB-G04, DB-G62, DB-G73: the rung is `/chain.json`'s, at the
   *   bar's own instant, over the strikes stored by then (D-3329).
   * - DB-038: a refused cell shows its reason WORD, styled apart from `n/a`
   *   (`—`, not an option) and pending (`…`); the page tallies the words.
   * - DB-G64: priced cells carry the server's flags — `below_band`,
   *   `above_rate_tenor` — and the tenor in years (D-3330).
   * - DB-068: with "last spot at or before t" opted in, every spot-derived
   *   cell shows the spot's age (D-3328).
   */
  import { readSpots, readRungs, optionCells, spotKey, rungKey, ownMonth } from '$lib/moneyness.js';
  import { feeds } from '$lib/feeds.svelte.js';
  import { ask } from '$lib/ask.js';
  import { parseKey } from '$lib/instrument.js';
  import { readGreeks, greekWhyText, greekCell, greekKey, GREEK_FLAG } from '$lib/greeks.js';
  import { fmt } from '$lib/db/notes-labels.js';
  import { strikeText } from '$lib/db/decoration.js';
  import {
    spotWhyText, rungWhyText, refusalWord, PENDING, GREEK_FIELDS, greekInputs, spotAgeText, tallyOf, rungMatches
  } from '$lib/db/notes-options.js';
  import { paisaText } from '$lib/db/bars-plan.js';

  /**
   * THE OPTION BAR'S SPOT, RUNG AND GREEKS — read per page of rows from
   * `/spot.json`, `/chain.json` and `/greeks.json`, each bar in its own month
   * and at its own instant, under the page's spot rule (`db.spotAsof`).
   *
   * Only the rows ON SCREEN are asked about — the page as the server sent it,
   * before the Moneyness pick keeps some of them — so the cost follows the
   * page size and never the store.
   *
   * @param {any} db the page's shared state object (see the shell)
   */
  export function optionCellsState(db) {
    /** The option rows of the page the server sent, before any rung pick. */
    const optionPage = $derived(
      (db.view === 'bars' ? (db.barPageRead ?? db.barPage ?? []) : []).filter((/** @type {any} */ r) => r.side)
    );

    /* ---------------------------------------------------------------------
       SPOT — D-2807, D-3328. One request per underlying, rung and OWN month,
       each instant one bounded lookup of one file (docs/06-limits.md).
       --------------------------------------------------------------------- */
    /** @type {Map<string, import('$lib/moneyness.js').SpotAnswer>} */
    let spotMap = $state(new Map());
    $effect(() => {
      const feed = feeds.active;
      const rows = optionPage;
      const asof = db.spotAsof ?? 'exact';
      if (!feed || rows.length === 0) {
        spotMap = new Map();
        return;
      }
      const controller = new AbortController();
      readSpots(feed, rows, controller.signal, ask, asof)
        .then((m) => {
          if (!controller.signal.aborted) spotMap = m;
        })
        .catch(() => {
          /* Aborted by a newer page: that page's own read files its answers. */
        });
      return () => controller.abort();
    });

    /* ---------------------------------------------------------------------
       THE RUNG — D-3329. At each bar's own instant, over the strikes of its
       expiry stored by then; and the chain at the page's newest instant.
       --------------------------------------------------------------------- */
    /** @type {Map<string, import('$lib/moneyness.js').RungAnswer>} */
    let rungMap = $state(new Map());
    /** @type {Map<string, any>} */
    let ladders = $state(new Map());
    $effect(() => {
      const feed = feeds.active;
      const rows = optionPage;
      const asof = db.spotAsof ?? 'exact';
      if (!feed || rows.length === 0) {
        rungMap = new Map();
        ladders = new Map();
        return;
      }
      const controller = new AbortController();
      readRungs(feed, rows, controller.signal, ask, asof)
        .then((answer) => {
          if (controller.signal.aborted) return;
          rungMap = answer.rungs;
          ladders = answer.ladders;
        })
        .catch(() => {
          /* Aborted by a newer page: that page's own read files its answers. */
        });
      return () => controller.abort();
    });

    /* ---------------------------------------------------------------------
       IV AND GREEKS — D-2830, D-3330. Solved on the SERVER, per bar, one
       request per contract, rung and OWN month. The page formats what comes
       back and computes no model.
       --------------------------------------------------------------------- */
    /** @type {Map<string, import('$lib/greeks.js').GreekCell>} */
    let greekMap = $state(new Map());
    $effect(() => {
      const feed = feeds.active;
      const rows = optionPage;
      const asof = db.spotAsof ?? 'exact';
      if (!feed || rows.length === 0) {
        greekMap = new Map();
        return;
      }
      const controller = new AbortController();
      readGreeks(feed, rows, controller.signal, ask, asof)
        .then((answer) => {
          if (!controller.signal.aborted) greekMap = answer.cells;
        })
        .catch(() => {
          /* Aborted by a newer page: that page's own read files its answers. */
        });
      return () => controller.abort();
    });

    /** A row's three spot cells, and its greek cell. */
    const cellsOf = (/** @type {any} */ r) => optionCells(r, spotMap.get(spotKey(r)), rungMap);
    const greekOf = (/** @type {any} */ r) => greekCell(r, greekMap.get(greekKey(r)));

    /** The refusals on this page, counted (DB-038). */
    const optionTally = $derived(tallyOf(optionPage, cellsOf, greekOf));

    /* ---------------------------------------------------------------------
       AN UNDERLYING WITH NO SPOT FILE, NAMED ONCE (DB-069, DB-G53): every
       option bar of it refuses `no_spot_file`; the page groups them by
       underlying and asks `/aliases.json` once per name for the cause.
       --------------------------------------------------------------------- */
    const noSpotFamilies = $derived.by(() => {
      /** @type {Map<string, {underlying: string, segment: string|null, key: string|null, months: Set<string>, bars: number}>} */
      const by = new Map();
      for (const r of optionPage) {
        const s = spotMap.get(spotKey(r));
        if (!s || s.why !== 'no_spot_file') continue;
        const u = String(parseKey(r.instrument).underlying);
        let e = by.get(u);
        if (!e) by.set(u, (e = { underlying: u, segment: s.segment, key: s.key, months: new Set(), bars: 0 }));
        e.months.add(ownMonth(r));
        e.bars += 1;
      }
      return [...by.values()].map((e) => ({ ...e, months: [...e.months].sort() }));
    });
    /** `/aliases.json?symbol=` answers, by name; `null` for one that did not read. */
    let aliasAnswers = $state(new Map());
    $effect(() => {
      const missing = noSpotFamilies.map((f) => f.underlying).filter((u) => !aliasAnswers.has(u));
      if (missing.length === 0) return;
      const controller = new AbortController();
      Promise.all(
        missing.map((u) =>
          ask(`/aliases.json?${new URLSearchParams({ symbol: u })}`, { signal: controller.signal })
            .then((res) => res.json())
            .catch(() => null)
            .then((body) => /** @type {[string, any]} */ ([u, body]))
        )
      ).then((got) => {
        if (controller.signal.aborted) return;
        const next = new Map(aliasAnswers);
        for (const [u, body] of got) next.set(u, body);
        aliasAnswers = next;
      });
      return () => controller.abort();
    });

    /**
     * THE MONEYNESS PICK, AS A TEST OF ONE BAR (D-3329, DB-075): `null` when
     * nothing is picked, else whether the bar's own rung is the one picked.
     * The grid keeps the bars it passes; the pager still counts the server's.
     */
    const rungKeep = $derived(
      db.mnyPick.size === 0
        ? null
        : (/** @type {any} */ b) => Boolean(b.side) && rungMatches(db.mnyPick, rungMap.get(rungKey(b))?.rung)
    );
    /** How many of the page's bars the pick set aside, and how many of those are unread. */
    const rungHidden = $derived.by(() => {
      if (!rungKeep) return { hidden: 0, unread: 0, of: 0 };
      const page = db.barPageRead ?? [];
      let hidden = 0;
      let unread = 0;
      for (const b of page) {
        if (rungKeep(b)) continue;
        hidden += 1;
        if (b.side && !rungMap.has(rungKey(b))) unread += 1;
      }
      return { hidden, unread, of: page.length };
    });

    Object.defineProperties(db, {
      optionPage: { get: () => optionPage },
      spotMap: { get: () => spotMap, set: (v) => (spotMap = v) },
      rungMap: { get: () => rungMap, set: (v) => (rungMap = v) },
      ladders: { get: () => ladders },
      greekMap: { get: () => greekMap, set: (v) => (greekMap = v) },
      optionTally: { get: () => optionTally },
      noSpotFamilies: { get: () => noSpotFamilies },
      aliasAnswers: { get: () => aliasAnswers },
      rungKeep: { get: () => rungKeep },
      rungHidden: { get: () => rungHidden }
    });
  }
</script>

<script>
  /** @type {{ db: any, b: any }} */
  let { db, b } = $props();

  const oc = $derived(optionCells(b, db.spotMap.get(spotKey(b)), db.rungMap));
  const gc = $derived(greekCell(b, db.greekMap.get(greekKey(b))));
  /** The spot's age, drawn on every cell it priced when it is not this bar's own second (D-3328). */
  const age = $derived(oc && typeof oc.age === 'number' && oc.age > 0 ? oc.age : null);
  const gAge = $derived(gc && typeof gc.age === 'number' && gc.age > 0 ? gc.age : null);
  /** `T` and `B` on the IV cell, for the server's two flags (D-3330). */
  const FLAG_TAG = /** @type {Record<string, string>} */ ({ above_rate_tenor: 'T', below_band: 'B' });
</script>

<td class="bt">
  {#if b.side === null}
    <span class="unk" title="not an option — this instrument has no side.">—</span>
  {:else}
    <span class="chip {b.side === 'CE' ? 'ce' : 'pe'}">{b.side}</span>
  {/if}
</td>
<td class="bn">
  {#if b.strike === null}
    <span class="unk" title="not an option — this instrument carries no strike."
      >—</span
    >
  {:else}
    <span title="{fmt(b.strike)} paisa, as stored">{strikeText(b.strike)}</span>
  {/if}
</td>

<!-- MONEYNESS (D-3329): the server's rung at THIS bar's own instant, over
     the strikes of its expiry stored by then — or its reason word. -->
<td class="bt">
  {#if oc === null}
    <span class="unk" title="n/a — moneyness belongs to an option.">—</span>
  {:else if oc.mny !== null}
    <span
      title="{oc.mny} at this bar’s own instant: {fmt(oc.n ?? 0)} strike(s) of this expiry stored by then, ATM {oc.atm === null ? '—' : strikeText(oc.atm)}, local step {oc.step === null ? '—' : strikeText(oc.step)}; spot {paisaText(oc.spot)} {spotAgeText(oc)}"
      >{oc.mny}</span
    >{#if age !== null}<span class="age" title="spot from the underlying’s bar {fmt(age)} s before this one (last at or before t, opted in)">{fmt(age)}s</span>{/if}
  {:else if PENDING.has(oc.mnyWhy ?? '')}
    <span class="pend" title={rungWhyText(oc.mnyWhy, oc)} aria-label="Moneyness pending">…</span>
  {:else}
    <span class="ref" title="{oc.mnyWhy}: {rungWhyText(oc.mnyWhy, oc)}" aria-label="Moneyness refused: {rungWhyText(oc.mnyWhy, oc)}"
      >{refusalWord(oc.mnyWhy ?? '')}</span
    >
  {/if}
</td>
{#each [['intr', 'Intrinsic'], ['extr', 'Extrinsic']] as [k, label] (k)}
  <td class="bn">
    {#if oc === null}
      <span class="unk" title="n/a — {label.toLowerCase()} value belongs to an option.">—</span>
    {:else if oc.why !== null && PENDING.has(oc.why)}
      <span class="pend" title={spotWhyText(oc.why, oc)} aria-label="{label} pending">…</span>
    {:else if oc.why !== null}
      <span class="ref" title="{oc.why}: {spotWhyText(oc.why, oc)}" aria-label="{label} refused: {spotWhyText(oc.why, oc)}"
        >{refusalWord(oc.why)}</span
      >
    {:else}
      <span
        title={k === 'intr'
          ? `${fmt(oc.intr)} paisa against spot ${paisaText(oc.spot)} ${spotAgeText(oc)}`
          : `${fmt(oc.extr)} paisa: close less intrinsic, spot ${paisaText(oc.spot)} ${spotAgeText(oc)}`}
        class:aged={age !== null}>{paisaText(k === 'intr' ? oc.intr : oc.extr)}</span
      >{#if age !== null && k === 'intr'}<span class="age" title="spot from the underlying’s bar {fmt(age)} s before this one (last at or before t, opted in)">{fmt(age)}s</span>{/if}
    {/if}
  </td>
{/each}
<!-- IV AND THE FIVE GREEKS (D-2830, D-3330): solved on the server from this
     bar's own close, flagged where the model's inputs are out of range, or
     the server's named reason there is none — never a blank. -->
{#each GREEK_FIELDS as g (g.key)}
  <td class="bn">
    {#if gc === null}
      <span class="unk" title="n/a — {g.label} belongs to an option.">—</span>
    {:else if gc.why !== null && PENDING.has(gc.why)}
      <span class="pend" title={greekWhyText(gc)} aria-label="{g.label} pending">…</span>
    {:else if gc.why !== null}
      <span class="ref" title="{gc.why}: {greekWhyText(gc)}" aria-label="{g.label} refused: {greekWhyText(gc)}"
        >{refusalWord(gc.why)}</span
      >
    {:else}
      <span
        class:flag={gc.flags.includes('below_band')}
        class:aged={gAge !== null}
        title="{g.label} {g.text(gc)} — {greekInputs(gc.row)}"
        >{g.text(gc)}{#if g.key === 'iv'}{#each gc.flags as f (f)}<sup
              class="tag"
              title="{f}: {/** @type {Record<string, string>} */ (GREEK_FLAG)[f] ?? f}">{FLAG_TAG[f] ?? f}</sup
            >{/each}{/if}</span
      >
    {/if}
  </td>
{/each}

<style>
  /* Also in DbBarsGrid.svelte, where its reasons are written. */
  :global(.brow.dayfirst) > td {
    box-shadow: inset 0 1px 0 0 var(--line);
  }
  /* Also in DbBarsGrid.svelte, where its reasons are written. */
  :global(.brow.landed) > td {
    background: var(--acc-soft);
  }
  /* Also in DbCoverageGrid.svelte, where its reasons are written. `n/a`:
     the cell does not apply to this instrument. */
  .unk {
    color: var(--faint);
    cursor: help;
    border-bottom: 1px dotted currentColor;
    padding-bottom: 1px;
  }
  /* A REFUSAL IS NOT A BLANK (DB-038): the reason's word, in the warning
     ink and in italics so it never reads as a figure, dashed so it reads as
     a link to its sentence, and apart from `n/a` (`—`, faint) and pending
     (`…`). */
  .ref {
    color: var(--warn);
    cursor: help;
    font-size: 0.88em;
    font-style: italic;
    border-bottom: 1px dashed currentColor;
  }
  .pend {
    color: var(--faint);
    cursor: progress;
  }
  /* A FIGURE OUTSIDE THE BAND THE GREEKS ARE VALIDATED IN (D-3330): shown,
     and marked as not an ordinary one. */
  .flag {
    color: var(--warn);
    border-bottom: 1px dotted currentColor;
    cursor: help;
  }
  .tag {
    margin-left: 2px;
    font-size: 0.68em;
    font-weight: var(--w-bold);
    color: var(--warn);
    cursor: help;
  }
  /* THE SPOT'S AGE, opted in (D-3328): a badge on the rung and the
     intrinsic, always drawn when the spot is not this bar's own second; the
     other cells it priced are tinted and say the age on their title. */
  .age {
    display: inline-block;
    margin-left: 4px;
    padding: 0 3px;
    border-radius: 3px;
    font-size: var(--fs-micro);
    line-height: 1.35;
    color: var(--warn);
    background: color-mix(in srgb, var(--warn) 14%, transparent);
    cursor: help;
  }
  .aged {
    color: var(--warn);
  }
  /* Also in DbBarsGrid.svelte, where its reasons are written. */
  :global(.bgrid > tbody > tr) > td:nth-child(20) {
    border-left: 1px solid var(--line);
  }
  :global(.bgrid.nofuture > tbody > tr) > td:nth-child(15) {
    border-left: 1px solid var(--line);
  }
  /* Also in DbBarsGrid.svelte, where its reasons are written. */
  :global(.bgrid.lean.nooption > tbody > tr) > td:nth-child(n + 15) {
    display: none;
  }
  :global(.brow) > td {
    padding: 0 var(--s4);
    height: var(--dbrow);
    border-bottom: 1px solid var(--line-soft);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    background: var(--bg-2);
  }
  :global(.brow.odd) > td {
    background: var(--panel-2);
  }
  /* Also in DbBarsGrid.svelte, where its reasons are written. */
  :global(.brow:hover) > td {
    background: color-mix(in srgb, var(--acc) 7%, var(--bg-2));
  }
  @media (prefers-reduced-motion: no-preference) {
    :global(.brow) > td {
      transition: background-color var(--d-hover) var(--ease-out);
    }
  }
  .bt {
    font-family: var(--mono);
    color: var(--ink-2);
  }
  .bn {
    text-align: right;
    font-family: var(--mono);
    color: var(--ink);
  }
  .chip {
    display: inline-block;
    padding: 1px 6px;
    border-radius: var(--r1);
    font-family: var(--mono);
    font-size: var(--fs-micro);
    font-weight: var(--w-bold);
  }
  .chip.ce {
    color: var(--up);
    background: color-mix(in srgb, var(--up) 14%, transparent);
  }
  .chip.pe {
    color: var(--down);
    background: color-mix(in srgb, var(--down) 14%, transparent);
  }
</style>

<script module>
  /**
   * /db — ONE OPTION BAR'S CONTRACT CELLS
   *
   * Type, strike, moneyness, intrinsic, extrinsic, IV and the five greeks: the
   * eleven cells from column 15 of one bar row. Rendered inside DbBarsGrid's
   * `<tr>`, so the grid's positional column CSS still counts these cells.
   *
   * Every line below was MOVED out of `routes/db/+page.svelte` by WP-00 (the /db
   * page split). The only edits to a moved line are the ones the move needs: a
   * page-level name that now lives in another file is imported, or, for state,
   * read as `db.<name>`; a plain module exports what it declares; and a callback
   * over such a value marks its parameter `any`, the way the page already did
   * elsewhere. Nothing was fixed in the move: every defect the audit found is
   * still present byte for byte, and is fixed by the package that owns this file
   * next.
   *
   * Owner after WP-00: WP-53 (option cells: own-month pricing, point-in-time moneyness, refusal words).
   */
  import { chainsFrom, readSpots, optionCells, spotKey } from '$lib/moneyness.js';
  import { feeds } from '$lib/feeds.svelte.js';
  import { readGreeks, greekWhyText, greekCell, greekKey } from '$lib/greeks.js';
  import { fmt } from '$lib/db/notes-labels.js';
  import { strikeText } from '$lib/db/decoration.js';
  import { spotWhyText, GREEK_FIELDS, greekInputs } from '$lib/db/notes-options.js';
  import { paisaText } from '$lib/db/bars-plan.js';

  /**
   * THE OPTION BAR'S SPOT AND GREEKS — read per page of rows from `/spot.json`
   * and `/greeks.json`.
   *
   * Creates the page's effects that stood at old lines 5502 and 5528; the shell calls it
   * in that position of the order.
   *
   * @param {any} db the page's shared state object (see the shell)
   */
  export function optionCellsState(db) {
    /* ---------------------------------------------------------------------
       SPOT FOR THE OPTION ROWS ON THIS PAGE — D-2807.

       The underlying's stored close at each option bar's OWN instant, on the
       same feed and rung, from `/spot.json`: one request per underlying, rung
       and month on the page, each instant answered by the server in a bounded
       bisection of one file (docs/06-limits.md). Only the rows ON SCREEN are
       asked about, so the cost follows the page size and never the store.
       A missing spot comes back NAMED and is drawn as its reason — never as
       the nearest or the previous close.
       --------------------------------------------------------------------- */
    /** Every listed strike per underlying and expiry, from the whole census. */
    const spotChains = $derived(chainsFrom(db.deco));
    /** @type {Map<string, {close: number|null, why: string|null, detail?: string}>} */
    let spotMap = $state(new Map());
    $effect(() => {
      const feed = feeds.active;
      const rows = db.barPage.filter((/** @type {any} */ r) => r.side);
      if (db.view !== 'bars' || !feed || rows.length === 0) {
        spotMap = new Map();
        return;
      }
      const controller = new AbortController();
      readSpots(feed, rows, controller.signal)
        .then((m) => {
          if (!controller.signal.aborted) spotMap = m;
        })
        .catch(() => {
          /* Aborted by a newer page: that page's own read files its answers. */
        });
      return () => controller.abort();
    });
    /* ---------------------------------------------------------------------
       IV AND GREEKS FOR THE OPTION ROWS ON THIS PAGE — D-2830.

       Solved on the SERVER, per bar, by `/greeks.json`: one request per
       contract, rung and month on the page. The page formats what comes back
       and computes no model. A cell with no figure shows the server's reason.
       --------------------------------------------------------------------- */
    /** @type {Map<string, import('$lib/greeks.js').GreekCell>} */
    let greekMap = $state(new Map());
    $effect(() => {
      const feed = feeds.active;
      const rows = db.barPage.filter((/** @type {any} */ r) => r.side);
      if (db.view !== 'bars' || !feed || rows.length === 0) {
        greekMap = new Map();
        return;
      }
      const controller = new AbortController();
      readGreeks(feed, rows, controller.signal)
        .then((answer) => {
          if (!controller.signal.aborted) greekMap = answer.cells;
        })
        .catch(() => {
          /* Aborted by a newer page: that page's own read files its answers. */
        });
      return () => controller.abort();
    });

    Object.defineProperties(db, {
      spotChains: { get: () => spotChains },
      spotMap: { get: () => spotMap, set: (v) => (spotMap = v) },
      greekMap: { get: () => greekMap, set: (v) => (greekMap = v) }
    });
  }
</script>

<script>
  /** @type {{ db: any, b: any }} */
  let { db, b } = $props();

  /* THE ROW'S TWO `{@const}`s, MOVED WITH THE CELLS THAT READ THEM. In the
     page they were `{@const oc = …}` and `{@const gc = …}` inside the bar row's
     `{#each}`; a `{@const}` is a derived, so these are the same two values. */
  const oc = $derived(optionCells(b, db.spotMap.get(spotKey(b)), db.spotChains));
  const gc = $derived(greekCell(b, db.greekMap.get(greekKey(b))));
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

<!-- MONEYNESS, INTRINSIC, EXTRINSIC (D-2807): from the
     underlying's close at THIS bar's own instant, or the
     named reason there is none — never a nearest close. -->
<td class="bt">
  {#if oc === null}
    <span class="unk" title="not an option — moneyness belongs to an option.">—</span>
  {:else if oc.mny === null}
    <span class="unk" title={spotWhyText(oc.mnyWhy, oc)} aria-label="Moneyness unknown: {spotWhyText(oc.mnyWhy, oc)}">—</span>
  {:else}
    <span title="spot {paisaText(oc.spot)} at this bar’s instant">{oc.mny}</span>
  {/if}
</td>
<td class="bn">
  {#if oc === null}
    <span class="unk" title="not an option — intrinsic value belongs to an option.">—</span>
  {:else if oc.intr === null}
    <span class="unk" title={spotWhyText(oc.why, oc)} aria-label="Intrinsic unknown: {spotWhyText(oc.why, oc)}">—</span>
  {:else}
    <span title="{fmt(oc.intr)} paisa against spot {paisaText(oc.spot)}">{paisaText(oc.intr)}</span>
  {/if}
</td>
<td class="bn">
  {#if oc === null}
    <span class="unk" title="not an option — extrinsic value belongs to an option.">—</span>
  {:else if oc.extr === null}
    <span class="unk" title={spotWhyText(oc.why, oc)} aria-label="Extrinsic unknown: {spotWhyText(oc.why, oc)}">—</span>
  {:else}
    <span title="{fmt(oc.extr)} paisa: close less intrinsic">{paisaText(oc.extr)}</span>
  {/if}
</td>
<!-- IV AND THE FIVE GREEKS (D-2830): solved on the
     server from this bar's own close, or the server's
     named reason there is none — never a blank. -->
{#each GREEK_FIELDS as g (g.key)}
  <td class="bn">
    {#if gc === null}
      <span class="unk" title="not an option — {g.label} belongs to an option.">—</span>
    {:else if gc.why !== null}
      <span
        class="unk"
        title={greekWhyText(gc)}
        aria-label="{g.label} unknown: {greekWhyText(gc)}">—</span
      >
    {:else}
      <span title="{g.label} {g.text(gc)} — {greekInputs(gc.row)}">{g.text(gc)}</span>
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
  /* Also in DbCoverageGrid.svelte, where its reasons are written. */
  .unk {
    color: var(--faint);
    cursor: help;
    border-bottom: 1px dotted currentColor;
    padding-bottom: 1px;
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

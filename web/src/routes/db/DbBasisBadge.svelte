<script>
  /**
   * /db — THE PRICE-BASIS BADGE, beside the instrument in the anchor (WP-55).
   *
   * Findings DB-017, DB-033, DB-035, DB-G38 to DB-G43, DB-G58 and DB-G75;
   * decisions D-3320, D-3323 and D-3324.
   *
   * # The defect it removes
   *
   * For 101 of 208 F&O shares the same day reads at two prices that differ by a
   * constant factor across the two feeds — HDFCBANK on 2019-06-03 opens on
   * Zerodha at a quarter of GDFL's first print (DB-017) — and nothing on /db
   * said which one had
   * been rewritten; the % change note even cited D-0018's refusal to
   * back-adjust, which reads as "these prices are raw". A Zerodha day close was
   * likewise shown as if it were the day's last print, and a CASH selection
   * after 2026-08-03 gave no hint that Zerodha's minutes stop at 15:14 by
   * design or that GDFL's 15:28 print is the closing auction's.
   *
   * # What it shows, per store segment of the selection
   *
   * - the PRICE basis: "back-adjusted" (Zerodha shares, D-3323), "raw, as
   *   traded" (GDFL shares and options) or "index level";
   * - the VOLUME basis where it is the vendor's own (Zerodha shares, DB-033);
   * - the CLOSE: "official close" for a day bar, "last print" for an intraday
   *   one, and both rung by rung when the selection spans every rung (D-3324);
   * - the closing auction for a CASH selection that reaches 2026-08-03
   *   (D-3320);
   * - for options, that spot and greeks come from this feed's own underlying
   *   and never another feed's (D-3323 item 3).
   *
   * Every word is `/feeds.json`'s (`basis`, `close_source`, `closing_auction`,
   * from the descriptor in `pull::vendor`), composed by
   * `$lib/db/notes-labels.js`. A feed or segment the server names no basis for
   * says so; nothing is borrowed from another feed.
   *
   * # Cost
   *
   * Reads `db.kinds` (at most three segments, already counted by the selection
   * model) and the active feed's basis rows (at most thirty). No request, no
   * walk of the census.
   *
   * Owner after WP-00: WP-55.
   */
  import { selectionSegments, priceBadge, volumeBadge, closeBadge, auctionNote, spotNote } from '$lib/db/notes-labels.js';

  /** @type {{ db: any }} */
  let { db } = $props();

  const feed = $derived(db.activeFeed ?? null);
  const segments = $derived(selectionSegments(db.kinds, db.kind));
  const rung = $derived(typeof db.timeframe === 'string' ? db.timeframe : '');
  /* THE SELECTION'S LAST DAY WHEN IT HAS ONE, else its last month: a window
     of 2019-06-03 inside a 2018-01..2026-09 month range never reaches the
     closing auction, and saying it does would be a label about rows the grid
     is not showing. */
  const through = $derived(
    typeof db.toDay === 'string' && db.toDay !== '' ? db.toDay : typeof db.toMonth === 'string' ? db.toMonth : ''
  );

  /* A BACK-ADJUSTED SHARE'S RUNS ARE NAMED ROW BY ROW (R3-13 goal 4): the
     badge says where, since a selection of many days spans runs with
     different factors and only the rows' own days can name theirs. */
  const RUNS_WHERE =
    ' Each row\'s adjusted or quantised run is named, with its first and last day and the factor /sources/day.json serves, on its Source line below the grid and in its Details.';
  /** @param {string} seg */
  const priceOf = (seg) => {
    const p = priceBadge(feed, seg, rung);
    return p.tone === 'adjusted' ? { ...p, title: `${p.title}${RUNS_WHERE}` } : p;
  };

  const groups = $derived(
    segments.map((seg) => ({
      seg,
      price: priceOf(seg),
      volume: volumeBadge(feed, seg, rung),
      close: closeBadge(feed, seg, rung),
      auction: auctionNote(feed, seg, through),
      spot: spotNote(feed, seg)
    }))
  );
</script>

{#if feed && groups.length > 0}
  <span class="basis" role="group" aria-label="What these prices are">
    {#each groups as g (g.seg)}
      <span class="bgroup">
        {#if groups.length > 1}<span class="bseg">{g.seg}</span>{/if}
        <span class="bb {g.price.tone}" title={g.price.title}>{g.price.short}</span>
        {#if g.volume}<span class="bb {g.volume.tone}" title={g.volume.title}>{g.volume.short}</span>{/if}
        {#if g.close}<span class="bb {g.close.tone}" title={g.close.title}>{g.close.short}</span>{/if}
        {#if g.auction}<span class="bb {g.auction.tone}" title={g.auction.title}>{g.auction.short}</span>{/if}
        {#if g.spot}<span class="bb {g.spot.tone}" title={g.spot.title}>{g.spot.short}</span>{/if}
      </span>
    {/each}
  </span>
{/if}

<style>
  .basis {
    display: inline-flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--s2, 6px) var(--s3, 10px);
    min-width: 0;
  }
  .bgroup {
    display: inline-flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
  }
  .bseg {
    font-family: var(--mono);
    font-size: var(--fs-micro);
    color: var(--faint);
    letter-spacing: 0.04em;
  }
  .bb {
    display: inline-block;
    padding: 1px 7px;
    border-radius: var(--r1, 4px);
    border: 1px solid var(--line);
    font-family: var(--mono);
    font-size: var(--fs-micro);
    color: var(--ink-2);
    background: var(--panel-2);
    white-space: nowrap;
    cursor: help;
  }
  /* A REWRITTEN PRICE IS THE ONE THAT MUST NOT BE MISSED: the warning tone. */
  .bb.adjusted {
    color: var(--warn);
    border-color: color-mix(in srgb, var(--warn) 45%, transparent);
    background: color-mix(in srgb, var(--warn) 10%, transparent);
  }
  .bb.raw,
  .bb.level {
    color: var(--ink-2);
  }
  .bb.official,
  .bb.mixed,
  .bb.cas,
  .bb.spot {
    color: var(--acc);
    border-color: color-mix(in srgb, var(--acc) 40%, transparent);
  }
  .bb.unverified,
  .bb.unknown {
    color: var(--down);
    border-color: color-mix(in srgb, var(--down) 40%, transparent);
  }
</style>

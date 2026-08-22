<script>
  /**
   * THE MAPPING CONSOLE — what the exchange says about the feed's own symbols.
   *
   * # What this closes
   *
   * D-0275 built the join between a vendor's index symbol and the name NSE
   * publishes, and said in its own last paragraph that the module had no
   * caller. D-0276 gave it `/indexmap.json` and said, in ITS last paragraph,
   * that nothing in `web/` fetched it. This is the reader, and the sentence
   * about a surface that exists and cannot be reached being the same as absent
   * does not need writing a third time.
   *
   * # The refusals are the page, not a footnote
   *
   * A symbol that resolves confirms what an operator already assumed. A symbol
   * the exchange does not confirm is one whose bars are being filed under a
   * name no authority backs — and there was no list of those anywhere: not a
   * page, not a log line, not a count. So the refused rows are not filtered
   * out, not collapsed behind a toggle and not sorted to the bottom. They open
   * the table, and the census counts them beside the resolutions.
   *
   * # Two strengths, never merged
   *
   * `published` means the vendor's symbol IS the name NSE prints — proof.
   * `abbreviation` means it shortens exactly one published name — inference,
   * and strong, and still a rule's opinion. The server sends them as separate
   * bases for the reason `CLAUDE.md` §3 rule 1 exists, and a page that painted
   * both the same colour would be undoing that on the last hop. They carry
   * different hues here, from the console's own semantic ramp.
   *
   * # Why the candidates are shown for an ambiguous row
   *
   * "Ambiguous" alone is a dead end; the operator cannot act on it. The names
   * that collided are what makes it actionable — `NIFTY EV` matches nine
   * published indices, and seeing the nine is the difference between a refusal
   * and a decision waiting to be made.
   *
   * # O(1)
   *
   * The filter is a prefix probe into a Map built once per payload — the same
   * `$lib/prefix.js` `/db` and `/backtest` use — never a scan per keystroke and
   * never a request per character. The outcome chips partition a list already
   * in memory. The payload is one request per feed change and nothing polls.
   *
   * # What is deliberately NOT drawn
   *
   * **No suggested name for a refused symbol.** Fifteen of the seventeen
   * absences have an obvious published name beside them — obvious to a reader,
   * not to the rule. Printing a guess in the same column that elsewhere holds a
   * resolution would make inference and proof indistinguishable at a glance,
   * which is the whole thing this surface exists to keep apart. The REASON is
   * printed instead, because a reason is checkable and a guess is not.
   *
   * **No equities, futures or options.** This is the index join only — the rows
   * a feed marks as indices on NSE. Everything else is a different join on a
   * different key and is not claimed by these numbers.
   */
  import { untrack } from 'svelte';
  import { feeds } from '$lib/feeds.svelte.js';
  import { ask } from '$lib/ask.js';
  import * as prefix from '$lib/prefix.js';

  /* ====================================================================
     THE PAYLOAD
     ==================================================================== */

  /** @type {{ phase: 'loading'|'ready'|'failed', body: any, why: string }} */
  let load = $state({ phase: 'loading', body: null, why: '' });

  /** @param {string|null} feed */
  async function fetchJoin(feed) {
    if (!feed) return;
    load = { phase: 'loading', body: null, why: '' };
    try {
      const response = await ask(`/indexmap.json?feed=${encodeURIComponent(feed)}`, {
        cache: 'no-store'
      });
      if (!response.ok) {
        // THE BODY CARRIES THE REASON AND IS READ RATHER THAN DISCARDED.
        // A 500 here is almost always the operator's NSE catalogue file being
        // absent, and the server puts its path in the message. Throwing that
        // away and printing the status code would turn a one-line fix into a
        // search.
        let why = `/indexmap.json answered ${response.status}.`;
        try {
          const body = await response.json();
          if (body?.error) why = String(body.error);
        } catch {
          /* Not JSON. The status line above is all there is to say. */
        }
        load = { phase: 'failed', body: null, why };
        return;
      }
      load = { phase: 'ready', body: await response.json(), why: '' };
    } catch (error) {
      load = {
        phase: 'failed',
        body: null,
        why:
          error instanceof Error
            ? error.message
            : 'The request for /indexmap.json failed and threw a value that is not an Error.'
      };
    }
  }

  $effect(() => {
    const feed = feeds.active;
    untrack(() => fetchJoin(feed));
  });

  /* ====================================================================
     WHAT THE PAYLOAD SAYS
     ==================================================================== */

  const join = $derived(load.body);
  const rows = $derived(join?.rows ?? []);
  const publishedCount = $derived(join?.published ?? 0);
  const listed = $derived(join?.listed ?? 0);
  const resolved = $derived(join?.resolved ?? 0);
  const verbatim = $derived(join?.verbatim ?? 0);
  const aliased = $derived(join?.aliased ?? 0);
  const abbreviated = $derived(join?.abbreviated ?? 0);
  const refused = $derived(join?.refused ?? 0);

  /**
   * One row's outcome as a single token, so the chips, the pill and the row
   * tint all read the same field rather than re-deriving it three ways.
   *
   * @param {any} row
   * @returns {'published'|'aliased'|'abbreviation'|'ambiguous'|'absent'}
   */
  function outcome(row) {
    if (row.basis === 'published') return 'published';
    if (row.basis === 'aliased') return 'aliased';
    if (row.basis === 'abbreviation') return 'abbreviation';
    return row.why === 'ambiguous' ? 'ambiguous' : 'absent';
  }

  const tagged = $derived(rows.map((/** @type {any} */ r) => ({ ...r, kind: outcome(r) })));

  /* ====================================================================
     NARROWING
     ==================================================================== */

  const CHIPS = /** @type {const} */ ([
    { key: 'all', label: 'All', tone: '' },
    { key: 'published', label: 'Confirmed', tone: 'p' },
    { key: 'aliased', label: 'Renamed', tone: 'r' },
    { key: 'abbreviation', label: 'Abbreviated', tone: 'i' },
    { key: 'ambiguous', label: 'Ambiguous', tone: 'a' },
    { key: 'absent', label: 'Absent', tone: 'x' }
  ]);

  let chip = $state('all');
  let query = $state('');

  const byPrefix = $derived(prefix.build(tagged));
  const searched = $derived(prefix.probe(byPrefix, tagged, query));
  const shown = $derived(
    chip === 'all' ? searched : searched.filter((/** @type {any} */ r) => r.kind === chip)
  );

  /** How many rows each chip would show, so a zero is visible before it is clicked. */
  const tally = $derived.by(() => {
    /** @type {Record<string, number>} */
    const n = {
      all: searched.length,
      published: 0,
      aliased: 0,
      abbreviation: 0,
      ambiguous: 0,
      absent: 0
    };
    for (const r of searched) n[r.kind] += 1;
    return n;
  });

  /** @param {any} row */
  function note(row) {
    if (row.kind === 'ambiguous') {
      return `${row.candidates} published names accept this abbreviation. Picking one would be a guess.`;
    }
    if (row.kind === 'absent') {
      return 'No published name accepts it. Either the vendor reordered the words, dropped a number the name carries, or this is not an NSE index at all.';
    }
    if (row.kind === 'abbreviation') return 'Shortens exactly one published name.';
    if (row.kind === 'aliased') {
      return `This engine renamed it. The exchange publishes the vendor's own name for it, and the store keys on ${row.symbol} instead.`;
    }
    return 'The symbol is the published name.';
  }
</script>

<svelte:head><title>Mapping · brutex</title></svelte:head>

<section class="page">
  <header class="head">
    <div>
      <h1>Instrument mapping</h1>
      <p class="sub">
        Every index symbol {feeds.active ?? 'this feed'} lists, joined to the name NSE publishes for
        it. Nothing here was compared against another vendor — two feeds agreeing proves they bought
        the same upstream, not that either is right.
      </p>
    </div>
  </header>

  {#if load.phase === 'loading'}
    <p class="state">Asking the exchange list…</p>
  {:else if load.phase === 'failed'}
    <div class="refusal">
      <span class="rlabel">The join did not run</span>
      <p>{load.why}</p>
      <p class="rhint">
        The exchange list is operator data and is not tracked: it lives beside the vendor masters as
        <code>nse_indices.csv</code>, one <code>index_name,category</code> row per published index.
        Nothing is guessed when it is missing and nothing is served from a stale copy.
      </p>
    </div>
  {:else}
    <div class="census">
      <div class="cell">
        <span class="n">{publishedCount}</span><span class="k">Published by NSE</span>
        <span class="s">Index names on the exchange's own list.</span>
      </div>
      <div class="cell">
        <span class="n">{listed}</span><span class="k">Listed by feed</span>
        <span class="s">Index symbols in this feed's master.</span>
      </div>
      <div class="cell p">
        <span class="n">{verbatim}</span><span class="k">Confirmed</span>
        <span class="s">The symbol is the published name. Proof.</span>
      </div>
      <div class="cell r">
        <span class="n">{aliased}</span><span class="k">Renamed</span>
        <span class="s">This engine renamed it; the vendor's name is published.</span>
      </div>
      <div class="cell i">
        <span class="n">{abbreviated}</span><span class="k">Abbreviated</span>
        <span class="s">Shortens exactly one name. Inference.</span>
      </div>
      <div class="cell x">
        <span class="n">{refused}</span><span class="k">Refused</span>
        <span class="s">{resolved} of {listed} resolved. These did not.</span>
      </div>
    </div>

    <div class="controls">
      <div class="chips" role="group" aria-label="Filter by outcome">
        {#each CHIPS as c (c.key)}
          <button
            type="button"
            class="chip {c.tone}"
            class:on={chip === c.key}
            aria-pressed={chip === c.key}
            onclick={() => (chip = c.key)}>{c.label}<span class="cn">{tally[c.key]}</span></button
          >
        {/each}
      </div>
      <input
        class="find"
        type="search"
        placeholder="Filter by symbol…"
        aria-label="Filter by symbol"
        bind:value={query} />
    </div>

    {#if shown.length === 0}
      <p class="state">
        No symbol matches. {#if query}The filter is a prefix on the symbol, not a search of the
          published names.{/if}
      </p>
    {:else}
      <div class="tw">
        <table>
          <thead>
            <tr>
              <th scope="col">Symbol</th>
              <th scope="col">Outcome</th>
              <th scope="col">NSE publishes</th>
              <th scope="col">Why</th>
            </tr>
          </thead>
          <tbody>
            {#each shown as row (row.symbol)}
              <tr class={row.kind}>
                <td class="sym">{row.symbol}</td>
                <td>
                  <span
                    class="pill {row.kind === 'published'
                      ? 'p'
                      : row.kind === 'aliased'
                        ? 'r'
                        : row.kind === 'abbreviation'
                          ? 'i'
                          : row.kind === 'ambiguous'
                            ? 'a'
                            : 'x'}">
                    {row.kind === 'abbreviation' ? 'abbreviated' : row.kind}
                  </span>
                </td>
                <td class="nse">{row.nse ?? '—'}</td>
                <td class="note">{note(row)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
      <p class="foot">
        Showing {shown.length} of {listed}. A refused symbol is not a broken feed — it is a name this
        engine will not adopt until the exchange confirms it.
      </p>
    {/if}
  {/if}
</section>

<style>
  .page {
    padding: var(--s5) var(--s5) var(--s8);
    max-width: 1180px;
    margin: 0 auto;
  }
  .head {
    margin-bottom: var(--s5);
  }
  h1 {
    font: var(--w-semi) var(--fs-xl) / 1.15 var(--sans);
    letter-spacing: -0.015em;
    color: var(--n11);
    margin: 0 0 var(--s2);
  }
  .sub {
    font-size: var(--fs-sm);
    color: var(--n9);
    line-height: 1.55;
    max-width: 78ch;
    margin: 0;
  }
  .state {
    font-size: var(--fs-sm);
    color: var(--n9);
    padding: var(--s5) 0;
  }
  .refusal {
    border: 1px solid var(--n6);
    border-left: 3px solid var(--down);
    background: var(--n3);
    border-radius: var(--r2);
    padding: var(--s4);
  }
  .rlabel {
    display: block;
    font: var(--w-semi) var(--fs-micro) / 1 var(--sans);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--down);
    margin-bottom: var(--s2);
  }
  .refusal p {
    font-size: var(--fs-sm);
    color: var(--n10);
    margin: 0 0 var(--s2);
    max-width: 76ch;
  }
  .rhint {
    color: var(--n9) !important;
    margin-bottom: 0 !important;
  }

  .census {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(158px, 1fr));
    gap: 1px;
    background: var(--n6);
    border: 1px solid var(--n6);
    border-radius: var(--r2);
    overflow: hidden;
    margin-bottom: var(--s4);
  }
  .cell {
    background: var(--n3);
    padding: var(--s3) var(--s4);
    display: flex;
    flex-direction: column;
  }
  .cell .n {
    font: var(--w-semi) var(--fs-data-xl) / 1 var(--num);
    font-variant-numeric: tabular-nums;
    color: var(--n11);
  }
  .cell .k {
    font: var(--w-semi) var(--fs-micro) / 1 var(--sans);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--n8);
    margin-top: var(--s2);
  }
  .cell .s {
    font-size: var(--fs-xs);
    color: var(--n9);
    margin-top: 4px;
    line-height: 1.4;
  }
  .cell.p .n {
    color: var(--up);
  }
  .cell.r .n {
    color: var(--acc);
  }
  .cell.i .n {
    color: var(--info);
  }
  .cell.x .n {
    color: var(--down);
  }

  .controls {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s3);
    align-items: center;
    margin-bottom: var(--s3);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chip {
    font: var(--w-mid) var(--fs-xs) / 1 var(--sans);
    color: var(--n9);
    background: var(--n3);
    border: 1px solid var(--n6);
    border-radius: var(--r-full);
    padding: 6px 12px;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 7px;
    transition: background var(--d-hover) var(--ease-out);
  }
  .chip:hover {
    background: var(--n4);
  }
  .chip:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }
  .chip .cn {
    font: var(--w-semi) var(--fs-micro) / 1 var(--num);
    font-variant-numeric: tabular-nums;
    color: var(--n8);
  }
  .chip.on {
    border-color: var(--n10);
    color: var(--n11);
  }
  .chip.p.on {
    border-color: var(--up);
    color: var(--up);
  }
  .chip.r.on {
    border-color: var(--acc);
    color: var(--acc);
  }
  .chip.i.on {
    border-color: var(--info);
    color: var(--info);
  }
  .chip.a.on {
    border-color: var(--warn);
    color: var(--warn);
  }
  .chip.x.on {
    border-color: var(--down);
    color: var(--down);
  }
  .find {
    font: var(--w-reg) var(--fs-sm) / 1 var(--sans);
    color: var(--n11);
    background: var(--n3);
    border: 1px solid var(--n6);
    border-radius: var(--r1);
    padding: 8px 12px;
    min-width: 220px;
    flex: 0 1 280px;
  }
  .find:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 1px;
  }

  .tw {
    overflow-x: auto;
    border: 1px solid var(--n6);
    border-radius: var(--r2);
    background: var(--n3);
  }
  table {
    border-collapse: collapse;
    width: 100%;
    font-size: var(--fs-xs);
  }
  th {
    position: sticky;
    top: 0;
    background: var(--n2);
    text-align: left;
    font: var(--w-semi) var(--fs-micro) / 1 var(--sans);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--n8);
    padding: 9px var(--s3);
    border-bottom: 1px solid var(--n6);
    white-space: nowrap;
  }
  td {
    padding: 7px var(--s3);
    border-bottom: 1px solid var(--n5);
    vertical-align: top;
    color: var(--n10);
  }
  tbody tr:last-child td {
    border-bottom: 0;
  }
  tbody tr:hover td {
    background: var(--n4);
  }
  td.sym {
    font: var(--w-semi) var(--fs-xs) / 1.4 var(--mono);
    color: var(--n11);
    white-space: nowrap;
  }
  td.nse {
    color: var(--n10);
  }
  td.note {
    color: var(--n9);
    max-width: 46ch;
    line-height: 1.45;
  }
  .pill {
    display: inline-block;
    font: var(--w-semi) var(--fs-micro) / 1 var(--sans);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    padding: 3px 7px;
    border-radius: var(--r1);
    white-space: nowrap;
  }
  .pill.p {
    background: var(--up-soft);
    color: var(--up);
  }
  .pill.r {
    background: var(--acc-soft);
    color: var(--acc);
  }
  .pill.i {
    background: var(--info-soft);
    color: var(--info);
  }
  .pill.a {
    background: var(--warn-soft);
    color: var(--warn);
  }
  .pill.x {
    background: var(--down-soft);
    color: var(--down);
  }
  .foot {
    font-size: var(--fs-xs);
    color: var(--n9);
    margin: var(--s3) 0 0;
    max-width: 80ch;
  }
</style>

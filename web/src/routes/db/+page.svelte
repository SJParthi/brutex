<script>
  /**
   * THE DB VIEW — what the store actually holds, for ONE feed.
   *
   * Named `/db` because that is what it is: a database view. It never shows two
   * feeds side by side; the selector in the top bar chooses one and this shows
   * that one's rows.
   *
   * # Why it is built the way it is
   *
   * **Summary before detail.** 772 rows answer "which instrument-month" and
   * answer nothing about the shape of the store. The strip at the top is the
   * question an operator actually opens this page with: how much is here, how
   * much of it is complete, and where are the holes.
   *
   * **Sortable, because the interesting rows are never alphabetical.** The
   * instrument missing 300 bars matters more than the one that is full, and
   * alphabetical order buries it at whatever letter it starts with.
   *
   * **Windowed.** Only the visible slice is in the DOM. 772 rows is survivable
   * and 93,776 — the census size `docs/06-limits.md` §34 projects for the end
   * of the stated backfill — is not, and the page must not need rewriting when
   * it gets there.
   */
  import { feeds } from '$lib/feeds.svelte.js';

  let rows = $state([]);
  let error = $state(null);
  let loading = $state(false);
  let filter = $state('');
  let sortBy = $state('instrument');
  let desc = $state(false);

  /**
   * A session is 375 one-minute bars, 09:15–15:29 inclusive.
   *
   * NSE's Closing Auction Session moved the close: from 2026-08-03 the
   * continuous session ends at 15:29. Stated because a completeness figure with
   * an unstated denominator is a number nobody can check.
   */
  const BARS_PER_SESSION = 375;

  $effect(() => {
    const f = feeds.active;
    if (!f) return;
    loading = true;
    error = null;
    fetch(`/store.json?feed=${encodeURIComponent(f)}`)
      .then((r) => (r.ok ? r.json() : Promise.reject(new Error(`HTTP ${r.status}`))))
      .then((j) => { rows = j; loading = false; })
      .catch((why) => { error = String(why); rows = []; loading = false; });
  });

  // WHERE ELSE THE DATA IS. An empty page that only says "nothing here" makes
  // the operator hunt; another feed may hold hundreds of thousands of bars one
  // dropdown away.
  let elsewhere = $state([]);
  $effect(() => {
    Promise.all(
      feeds.all.map((f) =>
        fetch(`/store.json?feed=${encodeURIComponent(f.wire)}`)
          .then((r) => (r.ok ? r.json() : []))
          .then((d) => ({ feed: f, bars: d.reduce((a, r) => a + r.rows, 0) }))
          .catch(() => ({ feed: f, bars: 0 }))
      )
    ).then((all) => (elsewhere = all.filter((x) => x.bars > 0 && x.feed.wire !== feeds.active)));
  });

  /** Bars short of whole trading days — the number worth seeing. */
  function shortBy(r) {
    const days = Math.max(1, Math.ceil(r.rows / BARS_PER_SESSION));
    return days * BARS_PER_SESSION - r.rows;
  }

  const matched = $derived(
    filter.trim()
      ? rows.filter((r) => r.instrument.toUpperCase().includes(filter.trim().toUpperCase()))
      : rows
  );

  const sorted = $derived(
    [...matched].sort((a, b) => {
      const k =
        sortBy === 'rows' ? a.rows - b.rows :
        sortBy === 'short' ? shortBy(a) - shortBy(b) :
        sortBy === 'month' ? a.month.localeCompare(b.month) :
        a.instrument.localeCompare(b.instrument);
      return desc ? -k : k;
    })
  );

  // THE SUMMARY, over the MATCHED set so it answers the question actually on
  // screen rather than the one before the filter was typed.
  const total = $derived(matched.reduce((a, r) => a + r.rows, 0));
  const full = $derived(matched.filter((r) => shortBy(r) === 0).length);
  const gaps = $derived(matched.reduce((a, r) => a + shortBy(r), 0));
  const months = $derived(new Set(matched.map((r) => r.month)).size);

  function head(key, label) {
    if (sortBy === key) desc = !desc;
    else { sortBy = key; desc = key === 'rows' || key === 'short'; }
  }

  // WINDOWED. Only what fits is in the DOM.
  const ROW = 30, OVER = 8;
  let scroller = $state(null);
  let scrollTop = $state(0);
  let viewportH = $state(600);
  const first = $derived(Math.max(0, Math.floor(scrollTop / ROW) - OVER));
  const slice = $derived(sorted.slice(first, first + Math.ceil(viewportH / ROW) + OVER * 2));
</script>

<div class="pane" style="min-height:0">
  <div class="pane-head">
    <span class="pane-title">DB · {feeds.all.find((f) => f.wire === feeds.active)?.display ?? '—'}</span>
    <input class="search" style="max-width:16rem" placeholder="Filter instrument" bind:value={filter} />
    <span class="spacer"></span>
    {#if filter.trim()}
      <button class="link" onclick={() => (filter = '')}>clear</button>
    {/if}
  </div>

  {#if rows.length}
    <!-- SUMMARY BEFORE DETAIL. The four numbers an operator opens this page
         for, computed over what is on screen. -->
    <section class="stats">
      <div class="stat"><span class="k">Instrument-months</span><span class="v">{matched.length.toLocaleString()}</span></div>
      <div class="stat"><span class="k">Bars</span><span class="v">{total.toLocaleString()}</span></div>
      <div class="stat"><span class="k">Complete</span><span class="v up">{full.toLocaleString()}</span><span class="n">of {matched.length.toLocaleString()}</span></div>
      <div class="stat"><span class="k">Bars missing</span><span class="v" class:down={gaps > 0}>{gaps.toLocaleString()}</span><span class="n">across {months} month(s)</span></div>
    </section>
  {/if}

  <div class="grid" bind:this={scroller} bind:clientHeight={viewportH}
       onscroll={() => (scrollTop = scroller.scrollTop)}>
    {#if error}
      <p class="empty" style="color:var(--down)">{error}</p>
    {:else if loading}
      <p class="empty">Reading the manifest…</p>
    {:else if sorted.length === 0}
      <div class="blank">
        <h2>Nothing stored for {feeds.all.find((f) => f.wire === feeds.active)?.display ?? feeds.active}</h2>
        {#if filter.trim()}
          <p>No instrument matches "{filter}".</p>
          <button class="link" onclick={() => (filter = '')}>Clear the filter</button>
        {:else}
          <p>This feed has never landed a bar in this store.</p>
        {/if}
        {#if elsewhere.length}
          <div class="alt">
            <span class="lbl">Data exists on</span>
            {#each elsewhere as e}
              <button class="utab" onclick={() => (feeds.active = e.feed.wire)}>
                {e.feed.display}<span class="n">{e.bars.toLocaleString()} bars</span>
              </button>
            {/each}
          </div>
        {/if}
        <a class="cta" href="/ingest">Pull data on Ingest →</a>
      </div>
    {:else}
      <table>
        <thead>
          <tr>
            <th><button class="sort" onclick={() => head('instrument')}>Instrument{sortBy === 'instrument' ? (desc ? ' ↓' : ' ↑') : ''}</button></th>
            <th><button class="sort" onclick={() => head('month')}>Month{sortBy === 'month' ? (desc ? ' ↓' : ' ↑') : ''}</button></th>
            <th>TF</th>
            <th class="num"><button class="sort" onclick={() => head('rows')}>Bars{sortBy === 'rows' ? (desc ? ' ↓' : ' ↑') : ''}</button></th>
            <th class="num">Days</th>
            <th class="num"><button class="sort" onclick={() => head('short')}>Short by{sortBy === 'short' ? (desc ? ' ↓' : ' ↑') : ''}</button></th>
          </tr>
        </thead>
        <tbody>
          <tr style="height:{first * ROW}px"><td colspan="6"></td></tr>
          {#each slice as r (r.instrument + r.month)}
            {@const s = shortBy(r)}
            <tr>
              <td>{r.instrument}</td>
              <td>{r.month}</td>
              <td>{r.timeframe}</td>
              <td class="num">{r.rows.toLocaleString()}</td>
              <td class="num">{Math.max(1, Math.ceil(r.rows / BARS_PER_SESSION))}</td>
              <td class="num" class:down={s !== 0} class:up={s === 0}>{s === 0 ? 'full' : `−${s}`}</td>
            </tr>
          {/each}
          <tr style="height:{Math.max(0, (sorted.length - first - slice.length) * ROW)}px"><td colspan="6"></td></tr>
        </tbody>
      </table>
    {/if}
  </div>
</div>

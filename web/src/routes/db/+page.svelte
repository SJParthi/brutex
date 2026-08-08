<script>
  /**
   * THE DB PAGE — what the store actually holds, for ONE feed.
   *
   * Named `/db` rather than `/store` because that is what it is: a database
   * view. It never shows two feeds side by side; the selector in the top bar
   * chooses one and this shows that one's rows.
   */
  import { feeds } from '$lib/feeds.svelte.js';

  let rows = $state([]);
  let error = $state(null);
  let loading = $state(false);
  let filter = $state('');

  // WHICH OTHER FEEDS HAVE DATA. An empty page that only says "nothing here"
  // makes the operator hunt: Groww may hold 8,922 bars one dropdown away and
  // the page said nothing about it. Knowing where the data IS is the useful
  // half of knowing it is not here.
  let elsewhere = $state([]);
  $effect(() => {
    Promise.all(
      feeds.all.map((f) =>
        fetch(`/store.json?feed=${encodeURIComponent(f.wire)}`)
          .then((r) => (r.ok ? r.json() : []))
          .then((d) => ({ feed: f, bars: d.reduce((a, r) => a + r.rows, 0) }))
          .catch(() => ({ feed: f, bars: 0 }))
      )
    ).then((all) => (elsewhere = all.filter((x) => x.bars > 0)));
  });

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

  const shown = $derived(
    filter.trim()
      ? rows.filter((r) => r.instrument.toUpperCase().includes(filter.trim().toUpperCase()))
      : rows
  );
  const totalRows = $derived(shown.reduce((a, r) => a + r.rows, 0));

  /**
   * A SESSION IS 375 ONE-MINUTE BARS, and the completeness figure below is
   * against that rather than against a calendar this build does not have.
   *
   * NSE's Closing Auction Session moved the close: from 2026-08-03 the
   * continuous session ends at 15:29, so 09:15..15:29 inclusive is 375 bars.
   * Stated because a percentage with an unstated denominator is a number
   * nobody can check.
   */
  const BARS_PER_SESSION = 375;
</script>

<div class="pane" style="min-height:0">
  <div class="pane-head">
    <span class="pane-title">DB · {feeds.active ?? '—'}</span>
    <input class="search" style="max-width:18rem" placeholder="Filter instrument" bind:value={filter} />
    <span class="spacer"></span>
    <span class="pane-title">{shown.length} instrument-month(s) · {totalRows.toLocaleString()} rows</span>
  </div>

  <div class="grid">
    {#if error}
      <p class="empty" style="color:var(--down)">{error}</p>
    {:else if loading}
      <p class="empty">Reading the manifest…</p>
    {:else if shown.length === 0}
      <!-- AN EMPTY STATE THAT SAYS WHAT TO DO, and where the data actually is.
           This was one grey sentence in a full screen of black. -->
      <div class="blank">
        <h2>Nothing stored for {feeds.all.find((f) => f.wire === feeds.active)?.display ?? feeds.active}</h2>
        {#if filter.trim()}
          <p>No instrument matches "{filter}". <button class="link" onclick={() => (filter = '')}>Clear the filter</button></p>
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
            <th>Instrument</th>
            <th>Month</th>
            <th>TF</th>
            <th class="num">Rows</th>
            <th class="num">Sessions</th>
            <th class="num">Complete</th>
          </tr>
        </thead>
        <tbody>
          {#each shown as r (r.instrument + r.month)}
            {@const sessions = Math.floor(r.rows / BARS_PER_SESSION)}
            {@const remainder = r.rows % BARS_PER_SESSION}
            <tr>
              <td>{r.instrument}</td>
              <td>{r.month}</td>
              <td>{r.timeframe}</td>
              <td class="num">{r.rows.toLocaleString()}</td>
              <td class="num">{sessions}</td>
              <!-- A PARTIAL SESSION IS NAMED, not rounded away. `375 · 2 + 4`
                   says a day is four bars short far more usefully than 99.6%. -->
              <td class="num" class:down={remainder !== 0} class:up={remainder === 0 && sessions > 0}>
                {remainder === 0 ? 'whole' : `+${remainder}`}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
</div>

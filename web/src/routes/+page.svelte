<script>
  import { catalogue, search } from '$lib/index.svelte.js';
  import { feeds } from '$lib/feeds.svelte.js';
  import Chart from '$lib/Chart.svelte';

  let typed = $state('');
  let picked = $state(null);

  // O(1) PER KEYSTROKE — one Map probe, not a scan. See lib/index.svelte.js.
  const hits = $derived(search(typed));

  // ONLY THE VISIBLE SLICE IS RENDERED. 800 rows is fine to hold in memory and
  // wasteful to put in the DOM; the window is what the viewport can show plus a
  // small overscan, so scrolling cost does not grow with the universe.
  let scroller = $state(null);
  let scrollTop = $state(0);
  const ROW = 30;
  const OVERSCAN = 8;
  let viewportH = $state(600);
  const first = $derived(Math.max(0, Math.floor(scrollTop / ROW) - OVERSCAN));
  const count = $derived(Math.ceil(viewportH / ROW) + OVERSCAN * 2);
  const window_ = $derived(hits.slice(first, first + count));
</script>

<div class="split">
  <section class="pane">
    <div class="pane-head">
      <input
        class="search"
        placeholder="Search {catalogue.ready ? catalogue.rows.length : '…'} instruments"
        bind:value={typed}
        autocomplete="off"
        spellcheck="false"
      />
    </div>

    <div
      class="vlist"
      bind:this={scroller}
      bind:clientHeight={viewportH}
      onscroll={() => (scrollTop = scroller.scrollTop)}
    >
      {#if catalogue.error}
        <p class="empty" style="color:var(--down)">Instrument index unavailable — {catalogue.error}</p>
      {:else if !catalogue.ready}
        <p class="empty">Loading the universe…</p>
      {:else if hits.length === 0}
        <p class="empty">Nothing matches “{typed}”.</p>
      {:else}
        <div style="height:{hits.length * ROW}px; position:relative">
          <div style="position:absolute; top:{first * ROW}px; left:0; right:0">
            {#each window_ as row (row.key)}
              <div
                class="vrow"
                role="option"
                tabindex="0"
                aria-selected={picked?.key === row.key}
                onclick={() => (picked = row)}
                onkeydown={(e) => e.key === 'Enter' && (picked = row)}
              >
                <span class="sym">{row.symbol}</span>
                <span class="meta">{row.kind}</span>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  </section>

  <section class="pane">
    <div class="pane-head">
      <span class="pane-title">{picked ? picked.key : 'No instrument selected'}</span>
      <span class="spacer"></span>
      <span class="pane-title">{feeds.active ?? '—'}</span>
    </div>
    {#if picked}
      <Chart instrument={picked} feed={feeds.active} />
    {:else}
      <p class="empty">Pick an instrument to chart it.</p>
    {/if}
  </section>
</div>

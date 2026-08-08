<script>
  /**
   * INGEST — the form adapts to the SELECTED FEED's transport.
   *
   * A broker needs a window and nothing else; an archive needs a folder and has
   * no window rules at all. Showing both sets to everyone is what made the old
   * page confusing: a folder box beside a broker, and window warnings beside a
   * folder of CSVs that has no session to be mid-way through.
   */
  import { feeds } from '$lib/feeds.svelte.js';

  const active = $derived(feeds.all.find((f) => f.wire === feeds.active));
  const isBroker = $derived(active?.transport === 'broker');

  let from = $state('');
  let to = $state('');
  let folder = $state('');
  let busy = $state(false);
  let receipt = $state(null);

  // Yesterday, because a broker is never asked for a day that has not finished
  // — a running session yields a partial day the append-only store cannot
  // correct later. The server enforces it; this only stops you asking.
  const yesterday = new Date(Date.now() - 86_400_000).toISOString().slice(0, 10);

  async function start(e) {
    e.preventDefault();
    busy = true;
    receipt = null;
    const body = new URLSearchParams({ target: 'swept', vendor: feeds.active, from, to });
    if (!isBroker) body.set('folder', folder);
    try {
      const r = await fetch('/pull/spot', {
        method: 'POST',
        headers: { 'content-type': 'application/x-www-form-urlencoded' },
        body
      });
      // The Rust route answers HTML — the receipt page. Its status is the
      // verdict; the text is shown rather than parsed, because a receipt this
      // build renders is more trustworthy than one this page reconstructs.
      receipt = { ok: r.ok, status: r.status, text: await r.text() };
    } catch (why) {
      receipt = { ok: false, status: 0, text: String(why) };
    }
    busy = false;
  }
</script>

<div class="pane">
  <div class="pane-head">
    <span class="pane-title">Ingest · {active?.display ?? '—'}</span>
    <span class="spacer"></span>
    <span class="pane-title">{active?.transport ?? ''}</span>
  </div>

  <div class="grid" style="padding:16px; max-width:640px">
    {#if !active}
      <p class="empty">Pick a feed in the top bar.</p>
    {:else}
      <form onsubmit={start} style="display:grid; gap:12px">
        {#if isBroker}
          <p style="color:var(--dim); font-size:12px">
            {active.display} is a broker. The window is split to its per-request cap
            automatically, the rate budget is charged per request, and the newest day
            it will answer for is <b>{yesterday}</b> — a session still running yields a
            partial day this store cannot correct later.
          </p>
          <label>
            <span class="pane-title">From (inclusive)</span>
            <input class="search" type="date" bind:value={from} max={yesterday} required />
          </label>
          <label>
            <span class="pane-title">To (inclusive)</span>
            <input class="search" type="date" bind:value={to} max={yesterday} required />
          </label>
        {:else}
          <p style="color:var(--dim); font-size:12px">
            {active.display} reads CSV files you already bought. No token, no rate
            budget, no market hours and no "not today" — a file's last day is not a
            question about the clock.
          </p>
          <label>
            <span class="pane-title">Folder</span>
            <input class="search" bind:value={folder} placeholder="/path/to/vendor/csvs" required />
          </label>
          <label>
            <span class="pane-title">From (inclusive)</span>
            <input class="search" type="date" bind:value={from} required />
          </label>
          <label>
            <span class="pane-title">To (inclusive)</span>
            <input class="search" type="date" bind:value={to} required />
          </label>
        {/if}

        <button class="feed" style="justify-self:start; padding:8px 18px" disabled={busy}>
          {busy ? 'Running…' : `Start ${isBroker ? 'broker' : 'archive'} pull`}
        </button>
      </form>

      {#if receipt}
        <div style="margin-top:16px">
          <p class="pane-title" style="color:{receipt.ok ? 'var(--up)' : 'var(--down)'}">
            HTTP {receipt.status}
          </p>
          <!-- The server's own receipt, rendered in a frame rather than
               re-parsed. A number this page recomputed could disagree with the
               one the run recorded, and the recorded one is the truth. -->
          <iframe
            title="Pull receipt"
            srcdoc={receipt.text}
            style="width:100%; height:60vh; border:1px solid var(--line); border-radius:8px; background:#fff"
          ></iframe>
        </div>
      {/if}
    {/if}
  </div>
</div>

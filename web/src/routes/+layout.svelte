<script>
  import '$lib/theme.css';
  import { page } from '$app/state';
  import { feeds, loadFeeds } from '$lib/feeds.svelte.js';
  import { loadCatalogue } from '$lib/index.svelte.js';

  let { children } = $props();

  // Both indexes load once, for the session. Everything after is in-memory.
  $effect(() => { loadFeeds(); loadCatalogue(); });

  const tabs = [
    { href: '/', label: 'Markets' },
    { href: '/ingest', label: 'Ingest' },
    { href: '/db', label: 'DB' },
    { href: '/audit', label: 'Audit' }
  ];
</script>

<div class="shell">
  <header class="topbar">
    <span class="brand">bru<b>tex</b></span>
    {#each tabs as t}
      <a class="tab" href={t.href} aria-current={page.url.pathname === t.href ? 'page' : undefined}>{t.label}</a>
    {/each}

    <span class="spacer"></span>

    <!-- A SELECT, NOT PILLS.
         Pills stop working past about four feeds — they wrap, they push the
         nav off screen, and every feed added makes the bar worse. A select
         holds N without changing shape, which is the requirement: adding a
         broker must cost nothing in the UI.
         Options come from /feeds.json, which is built from DESCRIPTORS, so no
         file here names a vendor. -->
    <label class="feedsel">
      <span class="lbl">Feed</span>
      <select bind:value={feeds.active} aria-label="Feed">
        {#each feeds.all as f}
          <option value={f.wire} disabled={!f.ready}>
            {f.display} · {f.ready ? f.transport : 'no data'}
          </option>
        {/each}
      </select>
    </label>
    {#if feeds.error}
      <span class="lbl" style="color:var(--down)">feeds unavailable — {feeds.error}</span>
    {/if}
  </header>

  {@render children()}
</div>

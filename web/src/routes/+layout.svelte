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

    <!-- ONE FEED, SELECTED. Never two side by side — see lib/feeds.svelte.js. -->
    <nav class="feeds" aria-label="Feed">
      <span class="lbl">Feed</span>
      {#if feeds.error}
        <span class="lbl" style="color:var(--down)">feeds unavailable — {feeds.error}</span>
      {/if}
      {#each feeds.all as f}
        <button
          class="feed"
          aria-pressed={feeds.active === f.wire}
          onclick={() => (feeds.active = f.wire)}
        >{f.display}<span class="kind">{f.transport}</span></button>
      {/each}
    </nav>
  </header>

  {@render children()}
</div>

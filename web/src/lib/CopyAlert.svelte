<script>
  /* NO COPY OF THIS STORE, SAID ON EVERY PAGE (rules audit RA-163; DB-B73,
     WP-81). The /ingest disk line already said it; a page that is not /ingest
     did not. Read once when the app opens: `/disk.json` measures at most every
     30 seconds itself and answers from memory (p99 under a millisecond), so
     this costs one small request a load. A read that fails is said, never
     taken for "a copy exists". */
  import { ask } from '$lib/ask.js';
  import { backupAlert } from '$lib/disk.js';

  /** @type {{text: string, detail: string} | null} */
  let alert = $state(null);

  $effect(() => {
    const stop = new AbortController();
    void (async () => {
      try {
        const response = await ask('/disk.json', { cache: 'no-store', signal: stop.signal });
        if (!response.ok) throw new Error(`/disk.json answered HTTP ${response.status}`);
        const body = await response.json();
        if (!stop.signal.aborted) alert = backupAlert(body, Date.now());
      } catch (why) {
        if (stop.signal.aborted) return;
        alert = { text: 'Whether a copy of this store exists was not read.',
          detail: why instanceof Error ? why.message : String(why) };
      }
    })();
    return () => stop.abort();
  });
</script>

{#if alert}
  <div class="copy-alert" role="alert" data-copy-alert>
    <strong>{alert.text}</strong> <span>{alert.detail}</span>
  </div>
{/if}

<style>
  .copy-alert {
    flex: 0 0 auto;
    padding: 6px 16px;
    border-bottom: 1px solid var(--line);
    background: var(--panel);
    color: var(--bad, var(--ink-2));
    font-size: 12px;
    line-height: 1.5;
  }
  .copy-alert span { color: var(--ink-2); }
</style>

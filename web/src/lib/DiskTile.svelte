<script>
  /* THE DISK LINE (DB-B73, DB-B59; WP-81). Read once when the page opens and
     on Refresh: `/disk.json` measures at most every 30 seconds itself, so a
     page cannot make it cost more. A failed read is said, never blank. */
  import { ask } from '$lib/ask.js';
  import { diskSummary } from '$lib/disk.js';

  /** @type {ReturnType<typeof diskSummary> | null} */
  let summary = $state(null);
  /** @type {string | null} */
  let failed = $state(null);

  async function read() {
    try {
      const response = await ask('/disk.json', { cache: 'no-store' });
      if (!response.ok) throw new Error(`/disk.json answered HTTP ${response.status}`);
      summary = diskSummary(await response.json());
      failed = null;
    } catch (why) {
      failed = why instanceof Error ? why.message : String(why);
    }
  }

  $effect(() => { void read(); });
</script>

{#if failed}
  <span class="disk bad" role="status" title={failed}>disk: not read — {failed}</span>
{:else if summary}
  <span class="disk" class:bad={summary.alarm} role="status" title={summary.detail}>
    <span>{summary.free}</span> · <span>{summary.store}</span> · <span>{summary.kept}</span> ·
    <span class="backup">{summary.backup}</span> · <span>{summary.scrub}</span>
    <button class="btn ghost" onclick={() => { void read(); }}>Refresh</button>
  </span>
{/if}

<style>
  .disk {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
    font-size: 12px;
    color: var(--ink-2);
    font-variant-numeric: tabular-nums;
  }
  .disk.bad .backup, .disk.bad { color: var(--bad, var(--ink-2)); }
  .disk button { padding: 2px 8px; font-size: 11px; }
</style>

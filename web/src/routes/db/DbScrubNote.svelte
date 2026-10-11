<script>
  /**
   * /db — WHAT THE FULL-BLOCK SCRUB FOUND IN THE SERIES ON THE PAGE (DB-B74,
   * DB-B65; WP-81).
   *
   * One `/scrub.json` per change of the selection's series and rung — never
   * per page, sort or scroll — answered from an index the server keeps of its
   * scrub ledger: one hash probe per series, no bar file opened. A failed
   * read is said in the note; it never blanks it.
   *
   * Mounted under the grid, beside the source lines, so WP-52's grid columns
   * are untouched.
   */
  import { ask } from '$lib/ask.js';
  import { feeds } from '$lib/feeds.svelte.js';
  import { seriesPlan } from '$lib/db/bars-plan.js';
  import { scrubUrl, scrubNote } from '$lib/db/scrub.js';

  /** @type {{db: any}} */
  let { db } = $props();

  const plan = $derived(db.view === 'bars' ? seriesPlan(db.barPlan?.all ?? []) : null);
  const url = $derived(scrubUrl(feeds.active, plan));
  const asked = $derived(plan?.keys?.length ?? 0);

  /** @type {{tone: 'warn' | 'muted', head: string, lines: string[]} | null} */
  let note = $state(null);
  /** @type {string | null} */
  let failed = $state(null);

  $effect(() => {
    const target = url;
    const count = asked;
    note = null;
    failed = null;
    if (!target) return;
    const controller = new AbortController();
    (async () => {
      try {
        const response = await ask(target, { cache: 'no-store', signal: controller.signal });
        const body = await response.json();
        if (controller.signal.aborted) return;
        if (!response.ok) throw new Error(String(body?.error ?? `/scrub.json answered HTTP ${response.status}`));
        note = scrubNote(body, count);
      } catch (why) {
        if (controller.signal.aborted) return;
        failed = why instanceof Error ? why.message : String(why);
      }
    })();
    return () => controller.abort();
  });
</script>

{#if failed}
  <p class="bnote warn" role="status">Scrub findings not read: {failed}</p>
{:else if note}
  <p class="bnote" class:warn={note.tone === 'warn'} role="status">{note.head}</p>
  {#each note.lines as line (line)}
    <p class="bnote warn">{line}</p>
  {/each}
{/if}

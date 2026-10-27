<script module>
  /* THE PANEL CONTRACT (`$lib/ingest-panels.js`): an order and a title. */
  export const order = 95;
  export const title = 'Extra copies: bytes, reader, proof, hard links to live archives';
</script>

<script>
  /*
   * THE EXTRA-COPIES REGISTER (CC-5). `GET /disk/copies.json` walks the
   * folders `disk.extra_copy_roots` names and serves what it found from
   * memory, walking again only when a root's own stat changes. This panel
   * shows it as plain tables: the second names of live vendor files first, in
   * red, then each root, the live roots, and the retirement list with the
   * server's own text offered as a file. The page adds no command of its own.
   *
   * NOTHING OF IT ON /ingest's FIRST LOAD: the page mounts every panel at
   * once (`import.meta.glob(..., { eager: true })`) and R3-15 holds the
   * page to a byte budget (`$lib/speed.js`), so the reading code
   * (`$lib/copies.js`) is imported on the first poll and the tables
   * (`$lib/CopiesTables.svelte`) when there is a walk to draw. A part that
   * fails to load is said, with the reason, and asked for again: never a
   * blank panel.
   *
   * FEED-AGNOSTIC: the copy roots are the machine's, not a feed's, so the
   * selected feed changes nothing here.
   */
  import { onMount } from 'svelte';
  import { ask } from '$lib/ask.js';
  import { answerLine, createPoller } from '$lib/ingest-panels.js';

  /** @type {{ ctx?: any }} */
  let { ctx: _feed } = $props();

  /** `$lib/copies.js`, once its first import has landed. @type {typeof import('$lib/copies.js') | null} */
  let lib = null;
  let view = $state(/** @type {ReturnType<typeof import('$lib/copies.js').copiesView> | null} */ (null));
  let line = $state('Reading /disk/copies.json');
  /** `$lib/CopiesTables.svelte`, once a walk has asked for it. @type {any} */
  let Tables = $state(null);
  /** Why the tables did not load, or '' when they did or were not asked for. */
  let tablesWhy = $state('');

  const walked = $derived(view && view.fault === null && view.state === 'walked' ? view : null);
  const waiting = $derived(view && view.fault === null && view.state === 'waiting' ? view : null);

  /** @param {unknown} why */
  const words = (why) => (why instanceof Error ? why.message : String(why));

  const poll = createPoller({
    every: 10_000,
    hidden: () => typeof document !== 'undefined' && document.hidden,
    run: async () => {
      try {
        lib ??= await import('$lib/copies.js');
      } catch (why) {
        view = null;
        line = `The Copies panel's reading code did not load (asked again at the next poll): ${words(why)}`;
        return;
      }
      const { ROUTE, copiesView } = lib;
      try {
        const r = await ask('/disk/copies.json', { ms: 15_000 });
        const text = await r.text();
        /** @type {any} */
        let body = null;
        try {
          body = JSON.parse(text);
        } catch {
          body = null;
        }
        if (!r.ok) {
          view = null;
          line = answerLine(ROUTE, r.status, body?.error ?? text.slice(0, 200));
          return;
        }
        const read = copiesView(body);
        view = read;
        line = read.fault ?? '';
      } catch (why) {
        line = answerLine(ROUTE, 0, words(why));
      }
      if (walked && Tables === null) {
        try {
          Tables = (await import('$lib/CopiesTables.svelte')).default;
          tablesWhy = '';
        } catch (why) {
          tablesWhy = `${words(why)} (asked again at the next poll)`;
        }
      }
    }
  });

  /** Hand the server's retirement text to the browser as a plain-text file. */
  function save() {
    if (!walked || !lib) return;
    const blob = new Blob([walked.retirementText], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = lib.retirementFileName(walked.walkedAtMicros);
    a.click();
    URL.revokeObjectURL(url);
  }

  onMount(() => {
    const wake = () => poll.wake();
    poll.start();
    poll.wake();
    document.addEventListener('visibilitychange', wake);
    return () => {
      document.removeEventListener('visibilitychange', wake);
      poll.stop();
    };
  });
</script>

<div class="copies">
  {#if line}<p class="line" class:bad={view?.fault}>{line}</p>{/if}
  {#if waiting}
    <p class="line">{waiting.line}</p>
    {#if waiting.degraded !== null}<p class="line warn">Settings degraded: {waiting.degraded}</p>{/if}
  {/if}
  {#if walked}
    <p class="line dim">
      Walked {walked.walkedAt}, {walked.ageS} s ago, in {walked.walkMs} ms; {walked.fresh ? 'fresh' : 'not fresh'}{#if walked.rewalking}, walking again now{/if}: {walked.why}.
      {walked.note}
    </p>
    {#if Tables}
      <Tables {walked} />
    {:else if tablesWhy}
      <p class="line bad">The register's tables did not load: {tablesWhy}</p>
    {:else}
      <p class="line dim">Drawing the register's tables</p>
    {/if}
    <button type="button" title="Save the server's retirement list text as a plain-text file; it runs nothing" onclick={save}>Save the list as text</button>

    <p class="line">Extra copies, in all: {walked.extraCopiesBytes}</p>
    {#if walked.wholeLine !== null}<p class="line warn">{walked.wholeLine}</p>{/if}
  {/if}
</div>

<style>
  .copies {
    max-width: 100%;
    min-width: 0;
  }
  .line {
    margin: 6px 0;
    font-size: var(--fs-xs, 0.8rem);
  }
  .bad {
    color: var(--bad, var(--down));
  }
  .warn {
    color: var(--warn);
  }
  .dim {
    color: var(--faint);
  }
  button {
    font-size: var(--fs-xs, 0.8rem);
  }
</style>

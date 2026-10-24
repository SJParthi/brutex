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
   * FEED-AGNOSTIC: the copy roots are the machine's, not a feed's, so the
   * selected feed changes nothing here.
   */
  import { onMount } from 'svelte';
  import { ask } from '$lib/ask.js';
  import { answerLine, createPoller } from '$lib/ingest-panels.js';
  import { ROUTE, copiesView, retirementFileName } from '$lib/copies.js';

  /** @type {{ ctx?: any }} */
  let { ctx: _feed } = $props();

  let view = $state(/** @type {ReturnType<typeof copiesView> | null} */ (null));
  let line = $state('Reading /disk/copies.json');

  const walked = $derived(view && view.fault === null && view.state === 'walked' ? view : null);
  const waiting = $derived(view && view.fault === null && view.state === 'waiting' ? view : null);

  const poll = createPoller({
    every: 10_000,
    hidden: () => typeof document !== 'undefined' && document.hidden,
    run: async () => {
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
        line = answerLine(ROUTE, 0, why instanceof Error ? why.message : String(why));
      }
    }
  });

  /** Hand the server's retirement text to the browser as a plain-text file. */
  function save() {
    if (!walked) return;
    const blob = new Blob([walked.retirementText], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = retirementFileName(walked.walkedAtMicros);
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

    <h4 class="bad">Second names of live vendor files ({walked.hazardCount})</h4>
    {#if walked.hazards.length === 0}
      <p class="line dim">No name under a copy root is a second hard link to a live vendor file.</p>
    {:else}
      <div class="scroll">
        <table aria-label="Hazards">
          <thead>
            <tr>
              <th scope="col" title="The second name, under a copy root">Path</th>
              <th scope="col" title="The live vendor file it is a hard link to">Live file</th>
              <th scope="col" title="The live root that file belongs to">Kind</th>
              <th scope="col" title="Device number both names share">Dev</th>
              <th scope="col" title="Inode both names share">Inode</th>
              <th scope="col" title="Hard links the inode has">Links</th>
              <th scope="col" title="Whether this name is a partial-download name, which a resumed write changes in place">Partial</th>
            </tr>
          </thead>
          <tbody>
            {#each walked.hazards as h, i (i)}
              <tr class="bad">
                <td class="mono">{h.path}</td>
                <td class="mono">{h.live}</td>
                <td>{h.kind}</td>
                <td>{h.dev}</td>
                <td>{h.ino}</td>
                <td>{h.nlink}</td>
                <td>{h.partial}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}

    <h4>Copy roots</h4>
    {#if walked.roots.length === 0}
      <p class="line dim">The setting names no copy root.</p>
    {:else}
      <div class="scroll">
        <table aria-label="Copy roots">
          <thead>
            <tr>
              <th scope="col">Path</th>
              <th scope="col" title="walked, partial (not walked whole) or refused, with the reason">State</th>
              <th scope="col">Files</th>
              <th scope="col">Bytes</th>
              <th scope="col" title="Bytes that are copies of what a live root already holds">Extra bytes</th>
              <th scope="col" title="What opens this folder, or none found">Reader</th>
              <th scope="col" title="The receipt that proves it, or none">Proof</th>
              <th scope="col" title="Whether a twin of these bytes is proven">Twin</th>
              <th scope="col" title="Whether any name in it is a second name of a live file">Duplicates live</th>
              <th scope="col" title="The live root it lies inside, if any">Inside</th>
              <th scope="col" title="Entries that could not be read">Faults</th>
              <th scope="col" title="Names hard-linked to files outside every live root">Linked elsewhere</th>
            </tr>
          </thead>
          <tbody>
            {#each walked.roots as r, i (i)}
              <tr>
                <td class="mono">{r.path}</td>
                <td class:warn={r.state === 'partial'} class:bad={r.state === 'refused'}>{r.state}{#if r.refused !== null}: {r.refused}{/if}</td>
                <td>{r.files}</td>
                <td>{r.bytes}</td>
                <td>{r.extraBytes}</td>
                <td>{r.reader}</td>
                <td class="mono">{r.proof}</td>
                <td>{r.twin}</td>
                <td class:bad={r.duplicatesLive === 'yes'}>{r.duplicatesLive}</td>
                <td class="mono">{r.inside}</td>
                <td class:warn={r.faults > 0}>{r.faults}</td>
                <td>{r.linkedElsewhere}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}

    <h4>Live roots</h4>
    {#if walked.live.length > 0}
      <div class="scroll">
        <table aria-label="Live roots">
          <thead>
            <tr>
              <th scope="col">Kind</th>
              <th scope="col">Path</th>
              <th scope="col" title="The setting or variable that names it">Named by</th>
              <th scope="col" title="Whether its files were walked for second names">Walked</th>
              <th scope="col">Why</th>
              <th scope="col">Files</th>
            </tr>
          </thead>
          <tbody>
            {#each walked.live as l, i (i)}
              <tr>
                <td>{l.kind}</td>
                <td class="mono">{l.path}</td>
                <td>{l.namedBy}</td>
                <td>{l.walked ? 'yes' : 'no'}</td>
                <td>{l.why}</td>
                <td>{l.files}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {:else}
      <p class="line dim">No live root is named.</p>
    {/if}
    {#if walked.unsetLive.length > 0}
      <p class="line dim">Live roots not named:</p>
      <ul class="line">
        {#each walked.unsetLive as u, i (i)}<li>{u}</li>{/each}
      </ul>
    {/if}

    <h4>Retirement list</h4>
    <p class="line">{walked.retirementSentence}</p>
    {#if walked.retirement.length > 0}
      <div class="scroll">
        <table aria-label="Retirement list">
          <thead>
            <tr>
              <th scope="col">Path</th>
              <th scope="col" title="Bytes freed">Bytes</th>
              <th scope="col">Files</th>
              <th scope="col">Why</th>
              <th scope="col">Twin</th>
              <th scope="col">Caution</th>
              <th scope="col" title="A second name of a live file">Hazard</th>
            </tr>
          </thead>
          <tbody>
            {#each walked.retirement as t, i (i)}
              <tr class:bad={t.hazard}>
                <td class="mono">{t.path}</td>
                <td>{t.bytes}</td>
                <td>{t.files}</td>
                <td>{t.why}</td>
                <td>{t.twin}</td>
                <td>{t.caution}</td>
                <td>{t.hazard ? 'yes' : 'no'}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {:else}
      <p class="line dim">The list is empty.</p>
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
  h4 {
    margin: 10px 0 4px;
    font-size: var(--fs-sm, 0.9rem);
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
  .mono {
    font-family: var(--mono, monospace);
    overflow-wrap: anywhere;
  }
  .scroll {
    overflow-x: auto;
    max-width: 100%;
    margin: 6px 0;
  }
  table {
    border-collapse: collapse;
    font-size: var(--fs-xs, 0.8rem);
  }
  th,
  td {
    padding: 2px 8px;
    border-bottom: 1px solid var(--line-soft, var(--line));
    text-align: left;
    vertical-align: top;
  }
  button {
    font-size: var(--fs-xs, 0.8rem);
  }
</style>

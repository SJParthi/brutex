<script>
  /*
   * THE EXTRA-COPIES REGISTER'S TABLES (CC-5), drawn for the /ingest Copies
   * panel (`routes/ingest/panels/Copies.svelte`), which imports this file
   * when it first has a walked register to show. So `/ingest`'s first load
   * carries none of it (R3-15's page budget, `$lib/speed.js`). It draws what
   * `$lib/copies.js` read from `GET /disk/copies.json` and nothing else: the
   * second names of live vendor files first, in red, then each copy root, the
   * live roots, and the retirement list. It holds no control; the panel holds
   * the one it has (saving the list as text), under the panel contract.
   */

  /** @type {{ walked: any }} */
  let { walked } = $props();
</script>

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

<style>
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
</style>

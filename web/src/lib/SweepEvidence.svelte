<script>
  import { ask } from './ask.js';
  import { fetchSweepEvidence, evidenceComparison, canInspectCandidateTrades } from './sweep-evidence.js';
  import CandidateTrades from './CandidateTrades.svelte';
  import BooleanCatalog from './BooleanCatalog.svelte';
  import BooleanEvidence from './BooleanEvidence.svelte';
  import BooleanLater from './BooleanLater.svelte';

  let { identity, vocabulary = /** @type {any} */ (null) } = $props();
  let refresh = $state(0);
  let loaded = $state(/** @type {any} */ ({ phase: 'loading', result: null, why: '' }));
  const comparison = $derived(evidenceComparison(loaded.result));
  const candidateDetail = $derived(canInspectCandidateTrades(loaded.result));

  $effect(() => {
    const id = identity;
    void refresh;
    let active = true;
    loaded = { phase: 'loading', result: null, why: '' };
    fetchSweepEvidence(id, (url) => ask(url, { cache: 'no-store' })).then(
      (result) => { if (active) loaded = { phase: 'ready', result, why: '' }; },
      (why) => { if (active) loaded = { phase: 'failed', result: null, why: why instanceof Error ? why.message : String(why) }; }
    );
    return () => { active = false; };
  });
</script>

<div class="evidence">
  <div class="heading">
    <div><span class="eyebrow">Saved sweep evidence</span><h3>What this run can actually prove</h3></div>
    <button onclick={() => refresh += 1}>Refresh evidence</button>
  </div>
  <p class="scope">Latest saved attempt for this run identity. Completion, checked rules and institutional admission are separate facts.</p>
  {#if loaded.phase === 'loading'}
    <p role="status">Reading the saved attempt and every available depth…</p>
  {:else if loaded.phase === 'failed'}
    <p class="problem" role="alert"><b>Evidence unavailable.</b> {loaded.why} No partial depth list is displayed.</p>
  {:else if loaded.result.status === 'missing'}
    <p class="notice">{loaded.result.why}</p>
  {:else}
    {@const e = loaded.result.evidence}
    <div class="facts">
      <span>Attempt <b>{e.attempt}</b></span><span>Operation <b>{e.operation}</b></span>
      <span>Lifecycle <b>{e.completion === 'running' ? 'Start recorded; no terminal receipt' : e.completion}</b></span>
      <span>Saved levels <b>{e.depth_rows}</b></span>
      <span>Retained signal rows <b>{e.ranked_available ? e.ranked_rows : 'not published'}</b></span>
    </div>
    {#if e.completion === 'running'}
      <p class="notice">A recorded start does not prove the process is still alive. Command status observes only the latest correlated command; other parallel commands and uncorrelated legacy events are not a complete process census.</p>
    {/if}
    {#if loaded.result.rows.length > 0}
      <div class="scroll"><table>
        <caption>Every saved depth, with exact candidate accounting</caption>
        <thead><tr><th scope="col">Depth</th><th scope="col">Generated</th><th scope="col">Duplicates</th><th scope="col">Excluded</th><th scope="col">Subset pruned</th><th scope="col">Below support</th><th scope="col">Frequent</th><th scope="col">Cumulative admitted</th><th scope="col">Cumulative pairs</th><th scope="col">Accounting</th></tr></thead>
        <tbody>{#each loaded.result.rows as row (row.k)}<tr>
          <th scope="row">{row.k}</th><td>{row.generated}</td><td>{row.duplicates}</td><td>{row.excluded}</td><td>{row.pruned}</td><td>{row.infrequent}</td><td>{row.frequent}</td><td>{row.admitted}</td><td>{row.pairs}</td><td class:problem={!row.reconciles}>{row.reconciles ? 'Balances' : 'Mismatch'}</td>
        </tr>{/each}</tbody>
      </table></div>
    {:else}
      <p class="notice">No depth rows were published for this attempt. This is not a measured zero-candidate search.</p>
    {/if}
    {#if candidateDetail}
      <CandidateTrades {identity} attempt={e.attempt} model={e.operation === 'expression' ? 'expression' : 'and-mask'} {vocabulary} />
    {/if}
    {#if e.operation === 'boolean-candidates'}
      <BooleanCatalog initialIdentity={identity} />
    {/if}
    {#if ['boolean-statistics','boolean-admission','boolean-qualification'].includes(e.operation)}
      <BooleanEvidence initialIdentity={identity} initialModel={e.operation==='boolean-statistics'?'statistics':e.operation==='boolean-qualification'?'qualification':'admission'} />
    {/if}
    {#if e.operation==='boolean-oos'}<BooleanLater initialIdentity={identity} />{/if}
  {/if}
  <div class="scroll"><table class="comparison">
    <caption>Research readiness comparison</caption>
    <thead><tr><th scope="col">Area</th><th scope="col">What is established</th><th scope="col">Meaning and remaining limits</th></tr></thead>
    <tbody>{#each comparison as row (row.area)}<tr><th scope="row">{row.area}</th><td><span class="state">{row.status}</span></td><td>{row.detail}</td></tr>{/each}</tbody>
  </table></div>
</div>

<style>
  .evidence{margin:1.25rem 0;padding:1.25rem;border:1px solid #364554;border-radius:12px;background:linear-gradient(125deg,#17263233,transparent)}
  .heading{display:flex;align-items:center;justify-content:space-between;gap:1rem}.eyebrow{font-size:.7rem;letter-spacing:.12em;text-transform:uppercase;color:#72b7c5}h3{margin:.3rem 0;font-size:1.15rem}.scope{opacity:.72;font-size:.85rem;max-width:75ch}
  button{padding:.55rem .75rem;border:1px solid #506879;border-radius:6px;background:transparent;color:inherit;cursor:pointer}button:focus-visible{outline:2px solid #8cd8e5;outline-offset:3px}
  .facts{display:flex;flex-wrap:wrap;gap:.7rem 1.5rem;font-size:.85rem;margin:1rem 0}.facts span{display:flex;gap:.5rem}.notice{padding:.7rem;border-left:3px solid #bfa45e;font-size:.85rem}.problem{color:#ef998e}
  .scroll{overflow:auto;margin-top:1rem}table{width:100%;border-collapse:collapse;font-size:.8rem;font-variant-numeric:tabular-nums;text-align:left}caption{text-align:left;font-weight:650;margin:.5rem 0}th,td{padding:.65rem .7rem;border-bottom:1px solid #65708033;vertical-align:top}thead{font-size:.7rem;color:#9ab1bd}tbody th{font-weight:550}td{white-space:nowrap}.comparison td{white-space:normal}.comparison td:last-child{min-width:28ch;line-height:1.5}.comparison th{min-width:20ch}.state{display:inline-block;padding:.25rem .45rem;border:1px solid #65708066;border-radius:4px;color:#c2c7b0}
  @media(max-width:600px){.heading{align-items:flex-start;flex-direction:column}.evidence{padding:.8rem}.facts span{flex-wrap:wrap}}
</style>

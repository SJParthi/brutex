<script>
  import { ask } from './ask.js';
  import { fetchBooleanEvidence } from './boolean-evidence.js';
  import { candidateMoney } from './candidate-trades.js';
  import BooleanCatalog from './BooleanCatalog.svelte';
  import QualificationDetails from './QualificationDetails.svelte';
  let { initialIdentity='', initialModel='statistics', initialCompletion=/** @type {string|null} */ (null), autoLoad=false }=$props();
  let input=$state(''), model=$state('statistics');
  let loaded=$state(/** @type {any} */ ({phase:'idle',body:null,why:''}));
  let catalog=$state(/** @type {any} */ (null));
  let generation=0;
  $effect(()=>{input=initialIdentity;model=initialModel;generation+=1;loaded={phase:'idle',body:null,why:''};catalog=null;
    const completion=initialCompletion;
    if(autoLoad&&initialIdentity)void load({model:initialModel,identity:initialIdentity,completion});
  });
  /** @param {any} selection */
  async function load(selection) {
    const ticket=++generation;catalog=null;loaded={phase:'loading',body:null,why:''};
    try {const body=await fetchBooleanEvidence(selection,url=>ask(url,{cache:'no-store'}));if(ticket===generation){model=body.model;input=body.identity;loaded={phase:'ready',body,why:''};}}
    catch(why){if(ticket===generation)loaded={phase:'failed',body:null,why:why instanceof Error?why.message:String(why)};}
  }
  /** @param {any} body @param {any} changes */
  const select=(body,changes)=>({model:body.model,identity:body.identity,completion:body.completion,kind:body.kind,offset:'0',limit:body.limit,...changes});
  /** @param {string} text */
  const label=text=>text.replaceAll('_',' ').replaceAll('-',' ');
  /** @param {any} value */
  const exact=value=>value.decimal===null?'Non-finite; unavailable as a number':value.decimal;
  /** @param {any} row */
  function openCatalog(row){catalog={identity:row.source.identity,completion:row.source.completion,candidate:row.coordinate};}
</script>

<details class="evidence" open={autoLoad}>
  <summary>Explore saved statistics and research checks</summary>
  <p>Follow an exact research receipt to its source catalogs, full statistics, every saved policy check, and the setting’s own trades. Pages stay pinned to one completion.</p>
  <form onsubmit={event=>{event.preventDefault();void load({model,identity:input.trim()});}}>
    <label>Evidence <select bind:value={model}><option value="statistics">Statistics</option><option value="admission">Research policy comparison</option><option value="qualification">Fixed-training later qualification</option></select></label>
    <label class="identity">Saved ID <input bind:value={input} placeholder="64-character evidence identity" autocomplete="off" spellcheck="false" /></label><button>Read saved evidence</button>
  </form>
  <p class="scope">The command’s output folder must match this server’s configured evidence folder. These are saved observations of a finite supplied program catalog. Returns shown here exclude costs. Reading them grants no Selection V6 authority, fresh market-data attestation, live-trading approval or future-return guarantee.</p>
  {#if loaded.phase==='loading'}<p role="status">Checking saved evidence and its source receipts…</p>
  {:else if loaded.phase==='failed'}<p role="alert" class="problem"><b>Evidence unavailable.</b> {loaded.why} No partial or replacement page is displayed.</p>
  {:else if loaded.phase==='ready'}
    {@const body=loaded.body}
    {@const s=body.summary}
    <div class="facts"><b>{body.model==='qualification'?'Saved fixed-training later qualification':body.model==='admission'?'Saved research policy comparison':'Saved statistical evidence'}</b><span>{s.families} source catalogs</span><span>{s.candidates} candidate settings</span><span>{s.periods} aligned training sessions</span></div>
    {#if body.model==='qualification'}<QualificationDetails {body} />{/if}
    <p>Training statistics: White Reality Check, SPA and Romano–Wolf were configured with {s.procedure.draws} draws, seed {s.procedure.seed}, block length {s.procedure.block_length}. Training Romano–Wolf availability: <b>{label(s.romano_availability)}</b>. PBO splits: {s.contributing_splits} rankable of {s.splits}; {s.bottom_half_splits} ranked in the bottom half.</p>
    <div class="scroll"><table><caption>Saved family test values</caption><thead><tr><th>Test</th><th>Statistic</th><th>Probability</th><th>Exact fraction</th><th>Bootstrap matches</th></tr></thead><tbody>
      {#each [['White Reality Check',s.white],['SPA',s.spa]] as [name,test]}<tr><th>{name}</th><td title={'IEEE-754 bits '+test.statistic.bits}>{exact(test.statistic)}</td><td title={'IEEE-754 bits '+test.probability.bits}>{exact(test.probability)}</td><td>{test.exact_probability.numerator} / {test.exact_probability.denominator}</td><td>{test.matched_or_exceeded} of {test.draws}</td></tr>{/each}
    </tbody></table></div>
    <details><summary>Exact identities, bounds and statistical encodings</summary><pre>{JSON.stringify({identity:body.identity,completion:body.completion,admitted_bytes:body.admitted_bytes,summary:s},null,2)}</pre><p>Admitted bytes count serialized receipts and bodies, including linked sources. They do not measure total memory. Cold admission reads the complete bounded evidence; later reads still check linked source generations.</p></details>
    <div class="actions"><button onclick={()=>load(select(body,{kind:'candidates'}))}>Every candidate</button>
      {#if body.model==='statistics'}<button onclick={()=>load(select(body,{kind:'sources'}))}>Source catalogs</button><button onclick={()=>load(select(body,{kind:'splits'}))}>Every PBO split</button>
      {:else}<button onclick={()=>load({model:'statistics',identity:body.statistics_identity,completion:body.statistics_completion})}>Linked statistics</button>{/if}
    </div>
    {#if body.model!=='statistics'}
      <details><summary>All 39 saved policy settings</summary><p>Policy digest <code>{body.policy.digest}</code>. These are the actual saved settings; a disabled Boolean requirement is shown as false.</p><div class="scroll"><table><thead><tr><th>Setting and unit</th><th>Exact saved value</th></tr></thead><tbody>{#each body.policy.values as row (row.name)}<tr><th>{label(row.name)}</th><td>{String(row.value)}</td></tr>{/each}</tbody></table></div></details>
      {#each body.rows as row (row.index)}
        <details class="candidate"><summary>Candidate {row.index} · {row.statistics.source.instrument} · {row.statistics.side} · {label(row.status)}</summary>
          <p>Saved comparison status is <b>{label(row.status)}</b>. Unmeasured and refused checks remain separate from failed checks. This status applies to this policy and evidence snapshot.</p>
          <p>Training: {row.statistics.trades} trades, {row.statistics.wins} wins; pessimistic return {candidateMoney(row.statistics.return_paisa)} per unit. Source setting {row.statistics.coordinate}; program {row.statistics.program_index}; grid ordinal {row.statistics.ordinal}.</p>
          <button onclick={()=>openCatalog(row.statistics)}>This setting’s training trades, sessions and exact exits</button>
          {#if body.model==='qualification'}<QualificationDetails {body} {row} />{/if}
          <div class="scroll"><table><caption>All 44 recorded checks</caption><thead><tr><th>Check</th><th>Saved outcome</th></tr></thead><tbody>{#each row.checks as check (check.index)}<tr><th>{label(check.name)}</th><td>{label(check.state)}</td></tr>{/each}</tbody></table></div>
          <div class="scroll"><table><caption>All 44 evidence values</caption><thead><tr><th>Evidence and unit</th><th>Availability</th><th>Exact saved value</th></tr></thead><tbody>{#each row.values as value (value.name)}<tr><th>{label(value.name)}</th><td>{label(value.state)}</td><td>{value.value??'—'}</td></tr>{/each}</tbody></table></div>
          <details><summary>Exact source links, statistics and reason masks</summary><pre>{JSON.stringify({identity:row.identity,source_index:row.source_index,failed:row.failed,unmeasured:row.unmeasured,refused:row.refused,statistics:row.statistics},null,2)}</pre></details>
        </details>
      {/each}
    {:else}
      <div class="scroll"><table>
        {#if body.kind==='candidates'}
          <caption>Every candidate’s saved statistics</caption><thead><tr><th>Candidate</th><th>Source / exact setting</th><th>Trades / wins</th><th>Cost-excluded return per unit</th><th>Wilson lower bound</th><th>Romano–Wolf</th><th>Detail</th></tr></thead><tbody>{#each body.rows as row (row.index)}<tr><th>{row.index}</th><td>{row.source.instrument} · {row.side}<br />Program {row.program_index}; grid {row.ordinal}</td><td>{row.trades} / {row.wins}</td><td>{candidateMoney(row.return_paisa)}</td><td title={'IEEE-754 bits '+row.wilson_lower.bits}>{exact(row.wilson_lower)}</td><td>{#if row.romano}Adjusted {row.romano.adjusted.numerator} / {row.romano.adjusted.denominator}<br />Stepdown rank {row.romano.stepdown_rank}{:else}{label(row.romano_availability)}{/if}</td><td><button onclick={()=>openCatalog(row)}>Trades and sessions</button><details><summary>All exact statistics</summary><pre>{JSON.stringify(row,null,2)}</pre></details></td></tr>{/each}</tbody>
        {:else if body.kind==='sources'}
          <caption>Every authenticated source catalog</caption><thead><tr><th>Source</th><th>Instrument</th><th>Settings</th><th>Exact identity and completion</th><th>Detail</th></tr></thead><tbody>{#each body.rows as row (row.index)}<tr><th>{row.index}</th><td>{row.instrument}</td><td>{row.coordinates}</td><td><code>{row.identity}</code><br /><code>{row.completion}</code></td><td><button onclick={()=>{catalog={identity:row.identity,completion:row.completion,candidate:null};}}>Open exact catalog</button><details><summary>Full source metadata</summary><pre>{JSON.stringify(row,null,2)}</pre></details></td></tr>{/each}</tbody>
        {:else}
          <caption>Every saved PBO split</caption><thead><tr><th>Split</th><th>Training segment mask</th><th>Test segment mask</th><th>Rankable</th><th>Bottom half</th><th>Exact score digest</th></tr></thead><tbody>{#each body.rows as row (row.index)}<tr><th>{row.index}</th><td>{row.train_mask}</td><td>{row.test_mask}</td><td>{row.rankable?'Yes':'No'}</td><td>{row.rankable?(row.bottom_half?'Yes':'No'):'Unavailable'}</td><td><code>{row.scores_digest}</code></td></tr>{/each}</tbody>
        {/if}
      </table></div>
    {/if}
    <div class="actions"><span>{body.rows.length} of {body.total} rows; page starts at {body.offset}</span><button disabled={body.offset==='0'} onclick={()=>load(select(body,{offset:String(BigInt(body.offset)>BigInt(body.limit)?BigInt(body.offset)-BigInt(body.limit):0n)}))}>Previous</button><button disabled={body.next===null} onclick={()=>load(select(body,{offset:body.next}))}>Next</button></div>
    {#if catalog}<BooleanCatalog initialIdentity={catalog.identity} initialCompletion={catalog.completion} initialCandidate={catalog.candidate} autoLoad={true} />{/if}
  {/if}
</details>

<style>
  .evidence{border:1px solid #566d7955;border-radius:10px;margin:1rem 0;padding:1rem}summary{font-weight:650;cursor:pointer}p{font-size:.85rem;line-height:1.55}.scope{opacity:.75;max-width:105ch}form,.facts,.actions{display:flex;flex-wrap:wrap;align-items:center;gap:.65rem;margin:1rem 0}label{display:flex;align-items:center;gap:.5rem}.identity{flex:1;min-width:260px}input{flex:1;min-width:160px}input,select,button{background:transparent;color:inherit;border:1px solid #647985;border-radius:5px;padding:.55rem}button{cursor:pointer}button:disabled{opacity:.4;cursor:default}button:focus-visible,input:focus-visible,select:focus-visible{outline:2px solid #79cad8;outline-offset:3px}.problem{color:#e99587}.scroll{overflow:auto}table{border-collapse:collapse;width:100%;font-size:.8rem;text-align:left;font-variant-numeric:tabular-nums}caption{text-align:left;font-weight:650;margin:.6rem 0}th,td{border-bottom:1px solid #61717e44;padding:.65rem;vertical-align:top}code{overflow-wrap:anywhere;font-size:.75rem}pre{font-size:.75rem;white-space:pre-wrap;overflow-wrap:anywhere;max-height:30rem;overflow:auto}.candidate{border-left:3px solid #6bb8c4;padding:.7rem 1rem;margin:.8rem 0}
</style>

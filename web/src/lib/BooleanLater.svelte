<script>
  import { ask } from './ask.js';
  import { fetchBooleanLater } from './boolean-oos.js';
  import { candidateMoney, candidateTime } from './candidate-trades.js';
  import { catalogDay } from './boolean-catalog.js';
  import BooleanCatalog from './BooleanCatalog.svelte';
  let { initialIdentity='', initialCompletion=/** @type {string|null} */ (null), initialCandidate=/** @type {string|null} */ (null), autoLoad=false }=$props();
  let input=$state('');let loaded=$state(/** @type {any} */ ({phase:'idle',body:null,why:''}));
  let original=$state(/** @type {any} */ (null));let generation=0;
  $effect(()=>{input=initialIdentity;generation+=1;loaded={phase:'idle',body:null,why:''};original=null;
    if(autoLoad&&initialIdentity)void load({identity:initialIdentity,completion:initialCompletion,kind:initialCandidate===null?'coordinates':'trades',candidate:initialCandidate});});
  /** @param {any} selection */
  async function load(selection){const ticket=++generation;original=null;loaded={phase:'loading',body:null,why:''};
    try{const body=await fetchBooleanLater(selection,url=>ask(url,{cache:'no-store'}));if(ticket===generation)loaded={phase:'ready',body,why:''};}
    catch(why){if(ticket===generation)loaded={phase:'failed',body:null,why:why instanceof Error?why.message:String(why)};}}
  /** @param {any} body @param {any} changes */
  const select=(body,changes)=>({identity:body.identity,completion:body.completion,kind:body.kind,candidate:body.candidate,offset:'0',limit:body.limit,...changes});
  /** @param {any} body @param {string|null} candidate */
  function openOriginal(body,candidate){original={...body.parent,candidate};}
</script>

<details class="later" open={autoLoad}>
  <summary>Compare fixed settings on a later period</summary>
  <p>Read a separately recorded later-period comparison. Every original coordinate keeps its training exit settings; later results do not select or promote a strategy.</p>
  <form onsubmit={event=>{event.preventDefault();void load({identity:input.trim()});}}><label>Later comparison ID <input bind:value={input} placeholder="64-character later comparison identity" autocomplete="off" spellcheck="false" /></label><button>Read saved comparison</button></form>
  {#if loaded.phase==='loading'}<p role="status">Authenticating the later comparison and its original catalog…</p>
  {:else if loaded.phase==='failed'}<p role="alert" class="problem"><b>Later evidence unavailable.</b> {loaded.why} No original results are substituted.</p>
  {:else if loaded.phase==='ready'}
    {@const body=loaded.body}
    <div class="facts"><strong>{body.instrument}</strong><span>{body.coordinate_count} original settings</span><span>{body.training_session_count} training sessions</span><span>{body.session_count} later sessions</span></div>
    <div class="scroll"><table><caption>The two recorded observation periods</caption><thead><tr><th>Period</th><th>Actual execution span</th><th>Source</th></tr></thead><tbody><tr><th>Training</th><td>{candidateTime(body.grids[0].first_micros)} to {candidateTime(body.grids[0].last_micros)}</td><td><button onclick={()=>openOriginal(body,null)}>Original programs, exits and results</button></td></tr><tr><th>Later comparison</th><td>{candidateTime(body.later.first_micros)} to {candidateTime(body.later.last_micros)}</td><td>{body.later.from} through {body.later.to}; {body.later.bars} real execution bars recorded</td></tr></tbody></table></div>
    <p class="scope">Returns exclude costs. These readers authenticate saved comparison bodies and the original receipt, not current raw-market files. Completing this replay does not establish statistical admission, full-campaign completion, live trading approval or future profitability.</p>
    <div class="actions"><button onclick={()=>load(select(body,{kind:'coordinates',candidate:null}))}>Every original setting</button>
      {#if body.selected}<button onclick={()=>load(select(body,{kind:'trades'}))}>This setting’s later trades</button><button onclick={()=>load(select(body,{kind:'sessions'}))}>Every later session</button><button onclick={()=>openOriginal(body,body.selected.index)}>This setting’s original trades</button>{/if}
    </div>
    {#if body.selected}<p><b>Setting {body.selected.index}</b> · {body.selected.later.expression} · {body.selected.later.side}. Original {body.selected.training.cell.trades} trades; later {body.selected.later.cell.trades}. Later execution refusals: {body.selected.later.execution_refusals.length?body.selected.later.execution_refusals.join('; '):'None recorded; no research admission is implied'}.</p><details><summary>Exact frozen exit levels and both observations</summary><pre>{JSON.stringify(body.selected,null,2)}</pre></details>{/if}
    <div class="scroll"><table>
      {#if body.kind==='coordinates'}
        <caption>Original settings, observed on both periods</caption><thead><tr><th>Setting and program</th><th>Training trades / wins</th><th>Later trades / wins</th><th>Training / later return per unit</th><th>Later execution state</th><th>Detail</th></tr></thead><tbody>{#each body.rows as row (row.index)}<tr><th>{row.index} · {row.later.side}<br />{row.later.expression}</th><td>{row.training.cell.trades} / {row.training.cell.wins}</td><td>{row.later.cell.trades} / {row.later.cell.wins}</td><td>{candidateMoney(row.training.cell.pessimistic)} / {candidateMoney(row.later.cell.pessimistic)}</td><td>{row.later.execution_refusals.length?row.later.execution_refusals.join('; '):'No execution refusal recorded'}</td><td><button onclick={()=>load(select(body,{kind:'trades',candidate:row.index}))}>Later trades and sessions</button><details><summary>Exact paired facts</summary><pre>{JSON.stringify(row,null,2)}</pre></details></td></tr>{/each}</tbody>
      {:else if body.kind==='trades'}
        <caption>This original setting’s own later trades</caption><thead><tr><th>Trade</th><th>Stored source / entry / exit indices</th><th>Entry / exit bar opens</th><th>Pessimistic / optimistic per unit</th><th>Worst adverse / best favourable</th></tr></thead><tbody>{#each body.rows as row (row.index)}<tr><th>{row.index}</th><td>{row.signal_bar} / {row.entry_bar} / {row.exit_bar}</td><td>{candidateTime(row.entry_micros)}<br />{candidateTime(row.exit_micros)}</td><td>{candidateMoney(row.worst)} / {candidateMoney(row.best)}</td><td>{row.adverse_ppm} / {row.favourable_ppm} ppm<br />{candidateMoney(row.adverse_paisa)} / {candidateMoney(row.favourable_paisa)} per unit</td></tr>{/each}</tbody>
      {:else}
        <caption>Every later session, including zero-trade days</caption><thead><tr><th>Session</th><th>Trades / wins</th><th>Return per unit</th></tr></thead><tbody>{#each body.rows as row (row.index)}<tr><th>{catalogDay(row.day)}</th><td>{row.trades} / {row.wins}</td><td>{candidateMoney(row.return_paisa)}</td></tr>{/each}</tbody>
      {/if}
    </table></div>
    {#if body.kind==='trades'}<p class="scope">Stored source is the projected execution-fill index, not the original completed signal bar’s source-series index. Times are bar opens: the 15:09 minute bar closes at the 15:10 intraday deadline. OHLCV does not reveal within-bar event order.</p>{/if}
    {#if body.kind==='trades' && body.total==='0'}<p>No later trades were recorded for this exact setting. Its session rows remain available; no winner trace is substituted.</p>{/if}
    <div class="actions"><span>{body.rows.length} of {body.total} rows; starts at {body.offset}</span><button disabled={body.offset==='0'} onclick={()=>load(select(body,{offset:String(BigInt(body.offset)>BigInt(body.limit)?BigInt(body.offset)-BigInt(body.limit):0n)}))}>Previous</button><button disabled={body.next===null} onclick={()=>load(select(body,{offset:body.next}))}>Next</button></div>
    <details><summary>Exact identities, spans and admitted bytes</summary><pre>{JSON.stringify({identity:body.identity,completion:body.completion,parent:body.parent,cohort:body.cohort,later:body.later,admitted_bytes:body.admitted_bytes,grids:body.grids},null,2)}</pre><p>The byte ceiling counts both serialized bodies and their receipts; it is not a total-memory measurement.</p></details>
    {#if original}<BooleanCatalog initialIdentity={original.identity} initialCompletion={original.completion} initialCandidate={original.candidate} autoLoad={true} />{/if}
  {/if}
</details>

<style>
  .later{border:1px solid #70838f66;border-radius:10px;margin:1rem 0;padding:1rem}summary{font-weight:650;cursor:pointer}p{font-size:.85rem;line-height:1.55;max-width:80ch}.scope{opacity:.8}form,.facts,.actions{display:flex;align-items:center;flex-wrap:wrap;gap:.6rem;margin:1rem 0}label{display:flex;align-items:center;gap:.6rem;flex:1;min-width:260px}input{flex:1;min-width:160px}input,button{background:transparent;color:inherit;border:1px solid #70838f;border-radius:5px;padding:.55rem}button{cursor:pointer}button:disabled{opacity:.4;cursor:default}button:focus-visible,input:focus-visible{outline:2px solid #a2cadd;outline-offset:3px}.problem{color:#eea28f}.scroll{overflow:auto}table{width:100%;border-collapse:collapse;text-align:left;font-size:.8rem;font-variant-numeric:tabular-nums}caption{text-align:left;font-weight:650;margin:.6rem 0}th,td{padding:.65rem;vertical-align:top;border-bottom:1px solid #70838f44}td{min-width:12ch}pre{max-height:30rem;overflow:auto;white-space:pre-wrap;overflow-wrap:anywhere;font-size:.75rem}
</style>

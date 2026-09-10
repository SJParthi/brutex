<script>
  import { ask } from './ask.js';
  import { fetchBooleanCampaign, campaignState } from './boolean-campaign.js';
  import { createCampaignMonitor } from './campaign-monitor.js';
  import BooleanCatalog from './BooleanCatalog.svelte';
  import BooleanEvidence from './BooleanEvidence.svelte';
  let { initialIdentity='', autoLoad=false }=$props();
  let input=$state('');
  let loaded=$state(/** @type {any} */ ({phase:'idle',body:null,why:''}));
  let detail=$state(/** @type {any} */ (null));
  const monitor=createCampaignMonitor((selection,signal)=>fetchBooleanCampaign(selection,url=>ask(url,{cache:'no-store',signal})),state=>{loaded=state;});
  $effect(()=>{input=initialIdentity;monitor.stop();loaded={phase:'idle',body:null,why:''};detail=null;
    if(autoLoad&&initialIdentity)load({identity:initialIdentity});
    return ()=>monitor.stop();
  });
  /** @param {any} selection */
  function load(selection){
    detail=null;
    try{monitor.start(selection,selection.pin==null);}
    catch(why){loaded={phase:'failed',body:null,why:why instanceof Error?why.message:String(why)};}
  }
  /** @param {string} model @param {any} link */
  function open(model,link){detail={model,...link};}
</script>

<details class="campaign" open={autoLoad}>
  <summary>Compare the eight timeframes</summary>
  <p class="intro">Follow one finite program campaign from waiting work to recorded outputs. Open a timeframe’s exact evidence to inspect its settings, trades and research checks.</p>
  <form onsubmit={event=>{event.preventDefault();void load({identity:input.trim()});}}>
    <label>Campaign ID <input bind:value={input} placeholder="64-character campaign identity" autocomplete="off" spellcheck="false" /></label>
    <button>Read current snapshot</button>
  </form>
  {#if loaded.phase==='loading'}<p role="status">Checking the acknowledged snapshot and recorded completion receipts…</p>
  {:else if loaded.phase==='failed'}<p role="alert" class="problem"><b>Campaign unavailable.</b> {loaded.why} No incomplete comparison is displayed. {loaded.watching?'An automatic retry is scheduled; at most three consecutive attempts.':'Automatic watching stopped; refresh explicitly to retry.'}</p>
  {:else if loaded.phase==='ready'}
    {@const body=loaded.body}
    <div class="current"><strong>{campaignState(body.state,body.owner_active)}</strong><span>Snapshot {body.sequence}</span><button onclick={()=>load({identity:body.identity})}>Refresh to latest snapshot</button><button onclick={()=>load({identity:body.identity,pin:body.pin})}>Recheck this snapshot</button></div>
    <p role="status">{loaded.watching?'Watching automatically; one request at a time.':'Automatic watching stopped for this saved state or explicit snapshot check.'} Paused, refused and completed campaigns do not keep polling.</p>
    {#if loaded.why}<p role="alert" class="problem">The latest refresh failed: {loaded.why} The last acknowledged snapshot remains displayed; it is not a new live observation. {loaded.watching?'A bounded retry is scheduled.':'Refresh explicitly to retry.'}</p>{/if}
    <p>{body.program_count} supplied programs · {body.from} through {body.to} · {body.horizon_bars}-bar execution horizon · {body.rows[0].expected_catalogs.length} expected source families per timeframe.</p>
    <p class="meaning">“Recorded complete” means the saved campaign stage finished. This overview checks completion receipts; it does not open every child body. Detail readers authenticate those bodies when opened. An observed owner is not a progress heartbeat.</p>
    <div class="scroll"><table>
      <caption>All eight declared intraday timeframes</caption>
      <thead><tr><th scope="col">Timeframe</th><th scope="col">Recorded stage</th><th scope="col">Exact saved evidence</th><th scope="col">Reason or next evidence</th><th scope="col">Later period</th></tr></thead>
      <tbody>{#each body.rows as row (row.rung)}<tr>
        <th scope="row">{row.rung}</th>
        <td><span class="state" class:refused={row.state==='refused'} class:completed={row.state==='completed'}>{campaignState(row.state,body.owner_active)}</span></td>
        <td><p>{row.catalogs.length} of {row.expected_catalogs.length} source completions recorded</p><div class="links">{#each row.catalogs as catalog (catalog.identity)}<button onclick={()=>open('catalog',catalog)}>{catalog.instrument}: settings and trades</button>{/each}
          {#if row.statistics}<button onclick={()=>open('statistics',row.statistics)}>Statistics</button>{/if}
          {#if row.admission}<button onclick={()=>open('admission',row.admission)}>Every research check</button>{/if}
          {#if !row.catalogs.length && !row.statistics && !row.admission}<span>No completed output linked</span>{/if}
        </div></td>
        <td>{row.reason??(row.state==='completed'?'Open a detail to verify its complete saved body.':row.state==='waiting'?'No work start recorded for this timeframe.':'Only the saved state is established.')}</td>
        <td>Not linked in this campaign snapshot. Use a separate later comparison ID; no passing later result is implied.</td>
      </tr>{/each}</tbody>
    </table></div>
    <p class="scope">The output folder must match this server’s configured evidence folder. Returns exclude costs. Finishing a finite catalog does not exhaust every Boolean expression or establish Selection V6, later-period acceptance, live trading approval or future profitability.</p>
    <details class="exact"><summary>Exact snapshot and recorded child links</summary><pre>{JSON.stringify(body,null,2)}</pre></details>
    {#if detail}
      {#key detail.model+detail.identity+detail.completion}
        {#if detail.model==='catalog'}<BooleanCatalog initialIdentity={detail.identity} initialCompletion={detail.completion} autoLoad={true} />
        {:else}<BooleanEvidence initialIdentity={detail.identity} initialCompletion={detail.completion} initialModel={detail.model} autoLoad={true} />{/if}
      {/key}
    {/if}
  {/if}
</details>

<style>
  .campaign{margin:1rem 0;padding:1.1rem;border:1px solid #70838f66;border-radius:10px}summary{font-weight:650;cursor:pointer}.intro,.meaning,.scope{max-width:80ch;font-size:.85rem;line-height:1.55}.meaning{border-left:3px solid #82a6be;padding-left:.8rem}.scope{opacity:.8}form,.current,.links{display:flex;align-items:center;flex-wrap:wrap;gap:.6rem}form,.current{margin:1rem 0}label{display:flex;align-items:center;gap:.6rem;flex:1;min-width:240px}input{flex:1;min-width:160px}input,button{background:transparent;color:inherit;border:1px solid #70838f;border-radius:5px;padding:.55rem}button{cursor:pointer}button:focus-visible,input:focus-visible{outline:2px solid #a2cadd;outline-offset:3px}.current strong{font-size:1.05rem;margin-right:.5rem}.current span{font-variant-numeric:tabular-nums}.scroll{overflow:auto}table{width:100%;border-collapse:collapse;font-size:.82rem;text-align:left}caption{text-align:left;font-weight:650;margin:.7rem 0}th,td{padding:.8rem .6rem;vertical-align:top;border-bottom:1px solid #70838f44}tbody th{font-size:1rem;font-variant-numeric:tabular-nums;white-space:nowrap}td:last-child{min-width:23ch;max-width:35ch;line-height:1.5}.state{display:inline-block;padding:.3rem .5rem;border:1px solid #8c97a377;border-radius:4px;white-space:nowrap}.state.refused,.problem{color:#eea28f}.state.completed{color:#a5d6bd}.links{align-items:flex-start;min-width:22ch}.links button{font-size:.78rem}pre{max-height:30rem;overflow:auto;white-space:pre-wrap;overflow-wrap:anywhere;font-size:.75rem}.exact{margin-top:1rem}@media(max-width:600px){.campaign{padding:.75rem}.current{align-items:flex-start}.current strong{width:100%}}
</style>

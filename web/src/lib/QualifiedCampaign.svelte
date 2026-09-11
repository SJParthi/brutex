<script>
 import {ask} from './ask.js';
 import {fetchQualifiedCampaign} from './qualified-campaign.js';
 import {campaignState} from './boolean-campaign.js';
 import {createCampaignMonitor} from './campaign-monitor.js';
 import BooleanEvidence from './BooleanEvidence.svelte';
 let {initialIdentity='',autoLoad=false}=$props();
 let input=$state(''),loaded=$state(/** @type {any} */ ({phase:'idle',body:null,why:''})),detail=$state(/** @type {any} */ (null));
 const monitor=createCampaignMonitor((selection,signal)=>fetchQualifiedCampaign(selection,url=>ask(url,{cache:'no-store',signal})),state=>{loaded=state;});
 $effect(()=>{input=initialIdentity;monitor.stop();loaded={phase:'idle',body:null,why:''};detail=null;if(autoLoad&&initialIdentity)load({identity:initialIdentity});return()=>monitor.stop();});
 /** @param {any} selection */
 function load(selection){detail=null;try{monitor.start(selection,selection.pin==null);}catch(why){loaded={phase:'failed',body:null,why:why instanceof Error?why.message:String(why)};}}
</script>
<details class="qualified" open={autoLoad}>
 <summary>Follow qualification across the selected timeframes</summary>
 <p>Every timeframe keeps its predeclared slot. This overview reads the complete acknowledged history; open a completed unit to authenticate all saved qualification details.</p>
 <form onsubmit={event=>{event.preventDefault();load({identity:input.trim()});}}><label>Qualified campaign ID <input bind:value={input} placeholder="64-character campaign identity" autocomplete="off" spellcheck="false" /></label><button>Read current snapshot</button></form>
 {#if loaded.phase==='loading'}<p role="status">Authenticating the acknowledged timeframe history…</p>
 {:else if loaded.phase==='failed'}<p role="alert"><b>Campaign unavailable.</b> {loaded.why} {loaded.watching?'A bounded automatic retry is scheduled.':'Refresh explicitly to retry.'}</p>
 {:else if loaded.phase==='ready'}
  {@const body=loaded.body}
  <p><b>Saved timeframes:</b> {body.timeframes.join(' · ')}. Unselected positions keep their reserved statistical allowance.</p>
  <p><b>{campaignState(body.state,body.owner_active)}</b> · snapshot {body.sequence} · {body.history_records} acknowledged records.</p>
  <p role="status">{loaded.watching?'Following new snapshots automatically; one request at a time.':'Automatic watching stopped for this saved state or explicit snapshot check.'}</p>
  {#if loaded.why}<p role="alert">Refresh failed: {loaded.why}. This is the previous acknowledged snapshot, not a new live observation.</p>{/if}
  <button onclick={()=>load({identity:body.identity})}>Refresh latest</button><button onclick={()=>load({identity:body.identity,pin:body.pin})}>Recheck this exact snapshot</button>
  <div class="scroll"><table><caption>Every selected qualification unit</caption><thead><tr><th>Timeframe</th><th>Recorded state</th><th>Reason</th><th>Exact saved detail</th></tr></thead><tbody>{#each body.rows as row (row.rung)}<tr><th>{row.rung}</th><td>{campaignState(row.state,body.owner_active)}</td><td>{row.reason??(row.state==='waiting'?'No acknowledged start for this unit.':'No refusal recorded.')}</td><td>{#if row.qualification}<button onclick={()=>{detail=row.qualification;}}>All settings, checks and later folds</button>{:else}No completed qualification link{/if}</td></tr>{/each}</tbody></table></div>
  <p>“Recorded complete” describes work completion, not passing strategies. Child links are recorded here; their completion receipts, bodies and ancestors authenticate only in the detail view. An observed owner is not a heartbeat. No raw-market freshness, Selection V6, live trading or future profitability is certified.</p>
  <details><summary>Exact history and unit identities</summary><pre>{JSON.stringify(body,null,2)}</pre><p>Cold history checking scales with recorded checkpoints. The byte allowance includes serialized checkpoint records and retained-record bookkeeping; it is not total memory.</p></details>
  {#if detail}{#key detail.identity+detail.completion}<BooleanEvidence initialIdentity={detail.identity} initialCompletion={detail.completion} initialModel="qualification" autoLoad={true} />{/key}{/if}
 {/if}
</details>
<style>
 .qualified{border:1px solid #566d7955;border-radius:10px;margin:1rem 0;padding:1rem}summary{font-weight:650;cursor:pointer}p{font-size:.85rem;line-height:1.55}form{display:flex;gap:.7rem;flex-wrap:wrap;margin:1rem 0}label{display:flex;gap:.5rem;flex:1;align-items:center}input{flex:1;min-width:220px}button,input{background:transparent;color:inherit;border:1px solid #647985;border-radius:5px;padding:.55rem}button{cursor:pointer}button:focus-visible,input:focus-visible{outline:2px solid #79cad8}table{width:100%;border-collapse:collapse;font-size:.83rem;text-align:left}th,td{border-bottom:1px solid #61717e44;padding:.6rem;vertical-align:top}caption{text-align:left;font-weight:650;margin:.6rem 0}.scroll{overflow:auto}pre{font-size:.75rem;white-space:pre-wrap;overflow-wrap:anywhere;max-height:30rem;overflow:auto}
</style>

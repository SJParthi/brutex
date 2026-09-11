<script>
 import {onDestroy} from 'svelte';
 import {ask} from './ask.js';
 import {createIndexStopLaunch,indexStopLaunchPlan,validateIndexStopMetadata,indexStopSplitProposal,indexStopServerMonth,indexStopContextMonth,indexStopRewardRiskCaption,indexStopQualificationHref} from './index-stop-launch.js';
 import IndexConsistencyPolicy from './IndexConsistencyPolicy.svelte';
 import IndexStopQualification from './IndexStopQualification.svelte';
 import IndexStopReadiness from './IndexStopReadiness.svelte';
 import IndexStopRanking from './IndexStopRanking.svelte';
 let {active=false,feed='',symbols=/** @type {string[]} */([]),rungs=/** @type {string[]} */([]),from='',to='',initialRun=/** @type {any} */(null),blockedReason='',onbusy=/** @type {(busy:boolean)=>void} */(()=>{}),onTimeframes=/** @type {(values:string[])=>void} */(()=>{})}=$props();
 let config=$state.raw(/** @type {any} */({phase:'idle',body:null,why:''})),serverMonth=$state(/** @type {string|null} */(null));
 let settings=$state({maxLossPoints:'',batchPrograms:'',nodeAllowance:'',batchAllowance:''});
 let dates=$state({from:'',to:'',laterFrom:'',laterTo:''}),splitEdited=$state(false),proposal=$state.raw(/** @type {any} */(null)),proposalWhy=$state('');
 let run=$state.raw(/** @type {any} */({phase:'idle',attempt:null,plan:null,qualifications:[],why:''})),localWhy=$state('');
 let savedTimeframe=$state('');
 let generation=0,configAbort=/** @type {AbortController|null} */(null),restored=/** @type {any} */(null);
 const fields=/** @type {const} */([['maxLossPoints','max_loss_points','Research loss limit (points)'],['batchPrograms','batch_programs','Expressions per batch'],['nodeAllowance','node_allowance','Grammar choices per batch'],['batchAllowance','batch_allowance','Batches in this work allowance']]);
 const controller=createIndexStopLaunch({request:ask,changed:value=>{run=value;onbusy(['starting','running','unknown'].includes(value.phase));}});
 const busy=$derived(['starting','running','unknown'].includes(run.phase));
 const rewardRiskCaption=$derived(indexStopRewardRiskCaption(config.body?.policy??null));
 const contextMonth=$derived.by(()=>{try{return indexStopContextMonth(dates.from);}catch{return null;}});
 const prepared=$derived.by(()=>{
  try{
   if(!active)throw new Error('Choose this index workflow to prepare another run.');
   if(!config.body)throw new Error('Read the live server configuration first.');
   if(!serverMonth)throw new Error('The server month is unavailable. Recheck configuration before confirming complete calendar months.');
   if(dates.from<from||dates.laterTo>to)throw new Error('The training and later periods must stay inside the selected full span.');
   if(dates.to>=serverMonth||dates.laterTo>=serverMonth)throw new Error('A selected evaluation month is still current or in the future according to the server. Use complete earlier months.');
   const timeframes=[...rungs].sort((a,b)=>config.body.timeframes.indexOf(a)-config.body.timeframes.indexOf(b));
   return {plan:indexStopLaunchPlan({feed,symbols,timeframes,...dates,bits:'all',...settings},config.body),why:''};
  }catch(why){return {plan:null,why:why instanceof Error?why.message:String(why)};}
 });
 const titles=/** @type {Record<string,string>} */({idle:'Index brute-force sweep',starting:'Submitting one sweep request',running:'Sweep in progress',done:'This work allowance finished',refused:'This attempt was refused',unknown:'Execution status is unconfirmed'});
 const stageLabels=/** @type {Record<string,string>} */({preparing:'Checking stored candles',training:'Evaluating training trades',later:'Evaluating later trades',institutional:'Checking research acceptance',saved:'Child evidence saved; awaiting batch acknowledgement',refused:'This timeframe refused'});
 const progressRows=$derived(run.qualifications?.length?run.qualifications:(run.plan?.timeframes??rungs).map((/** @type {string} */ timeframe)=>({timeframe,stage:null,identity:null,completion:null})));
 const selectedComparison=$derived(run.latestSavedBatch?.qualifications.find((/** @type {any} */ row)=>row.timeframe===savedTimeframe)??run.latestSavedBatch?.qualifications[0]??null);
 $effect(()=>{if(active&&config.phase==='idle')void readConfiguration();});
 $effect(()=>{if(!active||splitEdited)return;try{const next=indexStopSplitProposal(from,to,serverMonth);proposal=next;proposalWhy='';dates={from:next.from,to:next.to,laterFrom:next.laterFrom,laterTo:next.laterTo};}catch(why){proposal=null;proposalWhy=why instanceof Error?why.message:String(why);}});
 $effect(()=>{if(initialRun&&initialRun!==restored){restored=initialRun;void controller.restore(initialRun);}});
 onDestroy(()=>{generation++;configAbort?.abort();controller.dispose();onbusy(false);});
 async function readConfiguration(){
  const ticket=++generation;configAbort?.abort();const abort=new AbortController();configAbort=abort;config={phase:'loading',body:null,why:''};
  const query=new URLSearchParams();for(const [key,wire] of fields.slice(0,2))if(/^[1-9]\d{0,19}$/.test(settings[key]))query.set(wire,settings[key]);
  try{const response=await ask('/engine/index-stop-launch.json'+(query.size?'?'+query:''),{cache:'no-store',signal:abort.signal});if(!response.ok)throw new Error(`The live app does not provide single-stop configuration details (HTTP ${response.status}). No sweep was submitted.`);const body=validateIndexStopMetadata(await response.json());if(ticket!==generation||abort.signal.aborted)return;config={phase:'ready',body,why:''};serverMonth=indexStopServerMonth(response.headers.get('date'));onTimeframes([...body.timeframes]);for(const [key,wire] of fields)if(settings[key]===''&&typeof body.configured[wire]==='string')settings[key]=body.configured[wire];}
  catch(why){if(ticket===generation&&!abort.signal.aborted)config={phase:'failed',body:null,why:why instanceof Error?why.message:String(why)};}
 }
 async function start(){if(!active||busy||blockedReason||!prepared.plan)return;localWhy='';try{await controller.start(prepared.plan);}catch(why){localWhy=why instanceof Error?why.message:String(why);}}
 /** @param {any} value */const elapsed=value=>typeof value==='string'?`${String(BigInt(value)/1000000n)} seconds`:'Unavailable';
</script>

{#if active||run.attempt||run.phase==='unknown'}
 <section class="index-stop-launch" aria-label="Index single-stop sweep">
  <h3>{titles[run.phase]??'Index sweep'}</h3>
  <p>Test AND, OR and NOT combinations in <b>both long and short directions</b>. The completed signal candle fixes
   the stop at its low for a long trade or its high for a short trade. Enter at the next printed one-minute open;
   exit at that stop or <b>15:10 IST</b>. Multiple trades per day are allowed, with one position open at a time per setting.</p>
  <p>Every completed trade retains <b>pessimistic and optimistic</b> fill readings. Acceptance uses the pessimistic result,
   the existing institutional checks, and the required daily/weekly rule. Costs are excluded.</p>
  <p>This preview checks only the app’s configuration. The worker must still admit the exact stored OHLCV and calendar evidence before evaluating trades.</p>
  {#if active}
   <div class="scope"><span><b>Selected full span</b>{from||'Choose first month'} → {to||'Choose last month'}</span><span><b>Instrument</b>{symbols.join(', ')||'Choose NIFTY or BANKNIFTY'}</span><span><b>Feed</b>{feed||'Choose a feed'}</span><span><b>Timeframes</b>{rungs.join(' · ')||'Choose at least one'}</span></div>
   <IndexConsistencyPolicy policy={config.body?.index_consistency_policy??null}/>
   <p><b>Inherited institutional research gate.</b> {rewardRiskCaption}</p>
   <div class="split">
    <h4>{splitEdited?'Your edited chronological split':'Proposed chronological split'}</h4>
    <p>The proposal assigns the oldest 80% of complete selected months to training and the remaining 20% to later evaluation.
     It is visible and editable before Run sweep. Later data is part of research selection; it is not claimed to be an untouched final holdout.</p>
    {#if proposal&&!splitEdited}<p>{proposal.completeMonths} complete calendar months in this proposal.
     {#if proposal.excludedCurrentOrFuture}{proposal.excludedCurrentOrFuture} current or future months are excluded from the proposal, using the server month {proposal.serverMonth}.{/if}
     Stored-data completeness is still checked by the native worker.</p>{/if}
    {#if proposalWhy&&!splitEdited}<p role="alert">{proposalWhy}</p>{/if}
    <div class="dates">{#each [['from','Training starts'],['to','Training ends'],['laterFrom','Later evaluation starts'],['laterTo','Later evaluation ends']] as [key,label]}<label>{label}<input type="month" bind:value={dates[/** @type {keyof typeof dates} */(key)]} disabled={busy} oninput={()=>{splitEdited=true;}}/></label>{/each}</div>
    {#if contextMonth}<p><b>Earlier context also required: {contextMonth}.</b> The same feed must contain original one-minute and daily candles for that month before training begins. This date is a requirement, not confirmation that those files are present. Missing context refuses the run; the selected history stays unchanged.</p>{/if}
    {#if splitEdited}<button disabled={busy} onclick={()=>{splitEdited=false;}}>Use the proposed split</button>{/if}
   </div>
   <details><summary>Advanced work limits and acceptance settings</summary>
    <p>The values come from the live server. The research loss limit is an acceptance setting; it does not change the candle stop or assume an investment amount.</p>
    <div class="dates">{#each fields as [key,wire,label]}<label>{label}<input type="text" inputmode="numeric" bind:value={settings[key]} disabled={busy} onchange={()=>{if(wire==='max_loss_points'||wire==='batch_programs')void readConfiguration();}}/></label>{/each}</div>
    {#if config.body?.policy.ready}<div class="scroll"><table><caption>Original native acceptance thresholds</caption><thead><tr><th>Rule</th><th>Value</th></tr></thead><tbody>{#each config.body.policy.values as field (field.name)}<tr><th>{field.name.replaceAll('_',' ')}</th><td>{String(field.value)}</td></tr>{/each}</tbody></table></div>{/if}
    {#if config.body?.work_model}<p>Up to {config.body.work_model.parallel_timeframe_workers_max??'unconfigured'} configured timeframe workers, also bounded by available CPU capacity. Saved batches limit this invocation’s work; they do not describe the full search population. Runtime has not been measured.</p>{/if}
   </details>
   <div class="controls"><button onclick={readConfiguration} disabled={busy||config.phase==='loading'}>Check configuration</button><button class="launch" onclick={start} disabled={busy||!!blockedReason||!prepared.plan}>Run sweep</button></div>
   {#if blockedReason}<p role="alert">{blockedReason}</p>{/if}
   {#if config.phase==='loading'}<p role="status">Reading the live app’s execution policy and configured limits…</p>{:else if config.phase==='failed'}<p role="alert">{config.why}</p>{:else}<IndexStopReadiness metadata={config.body}/>{/if}
   {#if !prepared.plan&&config.phase==='ready'&&config.body?.ready}<p role="alert">{prepared.why}</p>{/if}
  {/if}
  {#if localWhy}<p role="alert">{localWhy}</p>{/if}{#if run.why}<p role="alert">{run.why}</p>{/if}
  {#if run.attempt}
   <div class="scope" role="status"><span><b>Acknowledged batches</b>{run.completedBatches??'Unavailable'}</span><span><b>Acknowledged expressions</b>{run.completedPrograms??'Unavailable'}</span><span><b>Acknowledged grammar work</b>{run.completedWork??'Unavailable'}</span><span><b>Elapsed wall-clock time</b>{elapsed(run.elapsedMicros)}</span></div>
   {#if run.phase==='done'}<p>{run.exhausted===true?'The native journal reports this declared grammar exhausted.':'This work allowance ended. The declared search is not exhausted.'} A completed search does not guarantee any setting passed.</p>{/if}
   <div class="scroll"><table><caption>Selected timeframe boundaries and saved comparisons</caption><thead><tr><th>Timeframe</th><th>Latest observed boundary</th><th>Saved comparison</th></tr></thead><tbody>{#each progressRows as row (row.timeframe)}<tr><th>{row.timeframe}</th><td>{row.stage===null?'No work boundary observed':stageLabels[row.stage]??'Unrecognized boundary'}</td><td>{#if row.identity}<a href={indexStopQualificationHref(row.identity,row.completion)}>Compare results and trades</a>{:else}<span>No completed comparison recorded</span>{/if}</td></tr>{/each}</tbody></table></div>
   <p>A child stage is progress, not a completed batch. Empty or refused evidence is never shown as a profitable result.</p>
   {#if run.searchIdentity&&run.latestSavedBatch}
    <IndexStopRanking searchIdentity={run.searchIdentity} timeframes={run.plan?.timeframes??rungs} refreshKey={run.completedBatches??''} autoLoad={true}/>
   {/if}
   {#if run.latestSavedBatch}
    <div class="scroll"><table><caption>Latest acknowledged saved batch {run.latestSavedBatch.batch} (numbering starts at 0)</caption><thead><tr><th>Timeframe</th><th>Saved results retained while further work runs</th></tr></thead><tbody>{#each run.latestSavedBatch.qualifications as row (row.timeframe)}<tr><th>{row.timeframe}</th><td><a href={indexStopQualificationHref(row.identity,row.completion)}>Compare this saved batch and inspect trades</a></td></tr>{/each}</tbody></table></div>
    <p>These exact saved comparisons belong to batch {run.latestSavedBatch.batch}; they are not relabelled as the pending batch or the full search.</p>
    <nav class="controls" aria-label="Choose saved results timeframe">{#each run.latestSavedBatch.qualifications as row (row.timeframe)}<button aria-pressed={selectedComparison?.timeframe===row.timeframe} onclick={()=>{savedTimeframe=row.timeframe;}}>{row.timeframe} results</button>{/each}</nav>
    {#if selectedComparison}
     <p><b>{selectedComparison.timeframe} results · saved batch {run.latestSavedBatch.batch}</b>. The first page opens automatically; choose a timeframe to compare another saved table.</p>
     <IndexStopQualification initialIdentity={selectedComparison.identity} initialCompletion={selectedComparison.completion} autoLoad={true}/>
    {/if}
   {/if}
   {#if ['running','unknown'].includes(run.phase)}<button onclick={()=>controller.recheck()}>Recheck this exact attempt</button>{/if}
   <details><summary>Exact submitted scope and saved progress</summary><p>Attempt: <code>{run.attempt}</code></p><p>Search: <code>{run.searchIdentity??'Not yet recorded'}</code></p><p>{run.plan?.index} · {run.plan?.feed} · {run.plan?.timeframes.join(' · ')}</p><pre>{JSON.stringify(run.plan,null,2)}</pre>{#if run.report}<pre>{run.report}</pre>{/if}</details>
  {/if}
 </section>
{/if}
<style>
 .index-stop-launch{padding:1rem;border:1px solid var(--line,#cbd6df);border-radius:8px;margin:1rem 0;color:var(--n11,#23374c)}h3{font-size:1.1rem;margin:.2rem 0}h4{font-size:.95rem;margin:.4rem 0}p{font-size:.82rem;line-height:1.6}.scope,.controls,.dates{display:flex;flex-wrap:wrap;gap:.7rem;margin:.8rem 0}.scope span{border:1px solid var(--line,#cbd6df);padding:.65rem;border-radius:5px;font-size:.8rem}.scope b{display:block;margin-bottom:.3rem}.split{border-left:3px solid var(--acc,#34849e);padding:.6rem 1rem;margin:1rem 0}.dates label{display:flex;flex-direction:column;gap:.3rem;min-width:150px;font-size:.8rem;flex:1}input,button{font:inherit;font-size:.82rem;color:inherit;background:transparent;border:1px solid var(--line,#cbd6df);border-radius:5px;padding:.55rem .7rem}button{cursor:pointer}button:disabled{opacity:.45;cursor:default}.launch{background:var(--acc,#287e96);color:white;font-weight:650}button:focus-visible,input:focus-visible,summary:focus-visible{outline:2px solid var(--acc,#34849e);outline-offset:2px}summary{cursor:pointer;font-size:.82rem}details{padding:.7rem 0}code,pre{overflow-wrap:anywhere;white-space:pre-wrap;font-size:.75rem}.scroll{max-width:100%;overflow:auto}table{width:100%;border-collapse:collapse;font-size:.8rem;text-align:left;font-variant-numeric:tabular-nums}th,td{padding:.65rem;border-bottom:1px solid var(--line,#cbd6df);white-space:nowrap}thead{background:var(--subtle,#f1f6f9)}caption{text-align:left;font-weight:600;padding:.6rem 0}[role=alert]{color:var(--down,#ad4944)}a{color:var(--acc,#287e96)}@media(max-width:520px){.index-stop-launch{padding:.65rem}.split{padding:.5rem}.dates label{min-width:100%}}
</style>

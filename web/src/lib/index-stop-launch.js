import {ask} from './ask.js';
import {createPageRequests} from './page-requests.js';
import {validateNativeResearchPolicy} from './boolean-launch.js';
import {validateIndexConsistencyPolicy} from './index-consistency.js';
import {CAMPAIGN_RUNGS} from './boolean-campaign.js';
import {sweepOutcome} from './sweep.js';
// @ts-expect-error Node strips erasable TypeScript; the browser resolves this source too.
import {liveAttemptKey} from './live-progress.ts';

export const INDEX_STOP_COMMAND='index-stop-qualified-search-stored';
export const INDEX_STOP_INDICES=Object.freeze(['NSE-NIFTY','NSE-BANKNIFTY']);
const MAX=(1n<<64n)-1n, REQUEST_BYTES=16384;
const KNOBS=['max_loss_points','batch_programs','node_allowance','batch_allowance'];
const WIRE=['command','feed','index','timeframes','from_year','from_month','to_year','to_month','later_from_year','later_from_month','later_to_year','later_to_month','bits',...KNOBS,'expected_policy_digest'];
const INPUT=['feed','symbols','timeframes','from','to','laterFrom','laterTo','bits','maxLossPoints','batchPrograms','nodeAllowance','batchAllowance'];
const PHYSICAL=['checksum_bytes','checksum_records','capture_programs','capture_records','capture_bytes','qualification_bytes','qualification_memory_bytes','history_bytes','replay_nodes','qualification_replay_bootstrap_work','qualification_replay_split_work','qualification_replay_memory_bytes'];
const metas=new WeakSet(),plans=new WeakSet(),observations=new WeakSet();
const utf8=new TextEncoder();
/** @param {any} v */const object=v=>v!==null&&typeof v==='object'&&!Array.isArray(v);
/** @param {any} v @param {readonly string[]} names */const keys=(v,names)=>object(v)&&Object.keys(v).length===names.length&&names.every(k=>Object.hasOwn(v,k));
/** @param {any} v @param {boolean} [positive] */const uint=(v,positive=true)=>typeof v==='string'&&v.length<=20&&/^(0|[1-9]\d*)$/.test(v)&&(!positive||v!=='0')&&BigInt(v)<=MAX;
/** @param {any} v */const hex=v=>typeof v==='string'&&v.length===64&&/^[0-9a-f]{64}$/.test(v)&&v!=='0'.repeat(64);
/** @param {any} v */const text=v=>typeof v==='string'&&v.trim().length>0&&utf8.encode(v).length<=4096;
/** @param {any} v */function freeze(v){if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;}
/** @param {any} v */const size=v=>utf8.encode(JSON.stringify(v)).length;
/** @param {any} a @param {any} b @returns {boolean} */function same(a,b){return a===b||Array.isArray(a)&&Array.isArray(b)&&a.length===b.length&&a.every((v,n)=>same(v,b[n]))||object(a)&&object(b)&&Object.keys(a).length===Object.keys(b).length&&Object.keys(a).every(k=>Object.hasOwn(b,k)&&same(a[k],b[k]));}
/** @param {any} value @returns {string[]} */
function timeframes(value){
 if(!Array.isArray(value)||value.length===0||value.length>CAMPAIGN_RUNGS.length)throw new Error('Choose at least one intraday timeframe.');
 let previous=-1;for(const rung of value){const next=CAMPAIGN_RUNGS.indexOf(rung);if(next<=previous)throw new Error('Timeframes must be unique intraday choices in the server’s order.');previous=next;}return [...value];
}
/** One index is an exact request scope; the browser never drops a second selection.
 * @param {string[]} symbols */
export function indexStopInstrument(symbols){
 if(!Array.isArray(symbols)||symbols.length!==1)throw new Error('Select exactly one index: NIFTY or BANKNIFTY. Each request keeps its own complete result history.');
 const index=symbols[0]?.startsWith('NSE-')?symbols[0]:`NSE-${symbols[0]}`;
 if(!INDEX_STOP_INDICES.includes(index))throw new Error('The single candle-stop model applies to NIFTY and BANKNIFTY.');return index;
}
/** Describe one effective research gate without selecting a threshold, measuring
 * trades or changing launch admission. The fixed policy has only 39 fields.
 * @param {any} policy @returns {string} */
export function indexStopRewardRiskCaption(policy){
 const unavailable='The effective reward/risk threshold is unavailable. The server must supply its resolved research policy; no default is assumed.';
 try{
  validateNativeResearchPolicy(policy);
  if(!policy.ready)return unavailable;
  const fields=policy.values.filter((/** @type {any} */ field)=>field.name==='min_worst_reward_risk_ppm');
  if(fields.length!==1||!uint(fields[0].value,false))return unavailable;
  const ppm=BigInt(fields[0].value),fraction=String(ppm%1000000n).padStart(6,'0').replace(/0+$/,'');
  const ratio=`${ppm/1000000n}${fraction?'.'+fraction:''}`;
  return `The smallest positive pessimistic trade in the later evaluation must be at least ${ratio} times the largest losing trade. This inherited research threshold is separate from the winning-day rule and the signal-candle stop. Without an observed loss, this ratio stays unmeasured.`;
 }catch{return unavailable;}
}
/** @param {any} v */
export function validateIndexStopMetadata(v){
 const fields=['schema_version','model','command','ready','readiness_scope','refusal','selected_timeframes_supported','indices','timeframes','execution_policy','execution_policy_digest','execution_rules','index_consistency_policy','configured','policy','procedure','limits','field_help','work_model'];
 if(!keys(v,fields)||v.schema_version!==1||v.model!=='index-stop-qualified-search-launch'||v.command!==INDEX_STOP_COMMAND||typeof v.ready!=='boolean'||v.selected_timeframes_supported!==true||!same(v.indices,INDEX_STOP_INDICES)||!same(v.timeframes,CAMPAIGN_RUNGS)||v.execution_policy!=='signal_candle_stop_v1'||!hex(v.execution_policy_digest)||v.readiness_scope!=='configuration-only; source admission occurs in the audited worker'||!(v.refusal===null||text(v.refusal))||size(v)>REQUEST_BYTES)throw new Error('This server has not supplied the required single candle-stop launch contract. Update the live app before starting an index sweep.');
 const fixed={signal:'Completed signal candle',long_stop:'Signal candle low',short_stop:'Signal candle high',entry:'Immediately following printed one-minute open',gap_invalid_entry:'Skip entry at or beyond the stop',forced_exit:'15:10 IST using the unique accepted 15:09 close',exits:['risk stop','15:10 IST'],directions:['long','short'],readings:['pessimistic','optimistic'],costs_included:false,maximum_open_positions_per_setting:1};
 if(!same(v.execution_rules,fixed))throw new Error('The server’s execution rule is not both directions with one candle stop and the15:10 close.');
 validateNativeResearchPolicy(v.policy);validateIndexConsistencyPolicy(v.index_consistency_policy);
 if(!object(v.configured)||Object.keys(v.configured).some(k=>!KNOBS.includes(k))||Object.entries(v.configured).some(([k,x])=>x!==null&&(!uint(x)||k==='max_loss_points'&&BigInt(x)>MAX/100n))||!keys(v.limits,['request_bytes','max_loss_points_max','physical'])||v.limits.request_bytes!==REQUEST_BYTES||v.limits.max_loss_points_max!==String(MAX/100n)||!(v.limits.physical===null||keys(v.limits.physical,PHYSICAL)&&PHYSICAL.every(k=>uint(v.limits.physical[k])))||!(v.procedure===null||keys(v.procedure,['draws','seed','block_length'])&&uint(v.procedure.draws)&&uint(v.procedure.seed,false)&&uint(v.procedure.block_length))||!keys(v.field_help,['max_loss_points','batch_allowance'])||!Object.values(v.field_help).every(text))throw new Error('Single-stop configuration limits or statistical procedure are malformed.');
 const work=v.work_model;
 if(!keys(work,['parallel_timeframe_workers_max','parallelism','within_timeframe','total_runtime','estimated_seconds','estimate_status'])||!(work.parallel_timeframe_workers_max===null||Number.isSafeInteger(work.parallel_timeframe_workers_max)&&work.parallel_timeframe_workers_max>0)||![work.parallelism,work.within_timeframe,work.total_runtime].every(text)||work.estimated_seconds!==null||work.estimate_status!=='unmeasured')throw new Error('The saved work model cannot claim an unmeasured completion time.');
 if(v.ready? v.refusal!==null||!v.policy.ready||!KNOBS.every(k=>uint(v.configured[k]))||v.limits.physical===null||v.procedure===null||work.parallel_timeframe_workers_max===null : !text(v.refusal))throw new Error('Single-stop readiness contradicts its configured values.');
 const result=freeze(structuredClone(v));metas.add(result);return result;
}
/** @param {any} value @param {string} label */
function month(value,label){if(typeof value!=='string'||!/^[0-9]{4}-(0[1-9]|1[0-2])$/.test(value)||value.startsWith('0000'))throw new Error(`${label} needs an explicit month.`);return {year:Number(value.slice(0,4)),month:Number(value.slice(5)),key:value};}
/** Server response time only; the browser clock never certifies a full month.
 * @param {any} header */
export function indexStopServerMonth(header){if(typeof header!=='string')return null;const parsed=Date.parse(header);if(!Number.isFinite(parsed)||new Date(parsed).toUTCString()!==header)return null;const ist=new Date(parsed+19800000),year=ist.getUTCFullYear();return year>=1&&year<=9999?`${String(year).padStart(4,'0')}-${String(ist.getUTCMonth()+1).padStart(2,'0')}`:null;}
/** Display the preceding minute/daily context month required by the strict
 * native loader. This does not certify that its files or candles exist.
 * @param {string} first */
export function indexStopContextMonth(first){
 const start=month(first,'Training start'),previous=start.year*12+start.month-2;
 if(previous<12)throw new Error('The preceding context month cannot be represented.');
 return `${String(Math.floor(previous/12)).padStart(4,'0')}-${String(previous%12+1).padStart(2,'0')}`;
}
/** Visible proposal only; never submits or overwrites an operator-edited split.
 * @param {string} from @param {string} to @param {string|null} serverMonth */
export function indexStopSplitProposal(from,to,serverMonth){
 const first=month(from,'Selected first month'),last=month(to,'Selected last month');
 if(!serverMonth)throw new Error('The server month is unverified. Recheck readiness before proposing complete-month periods.');
 const now=month(serverMonth,'Server month'),at=(/** @type {{year:number,month:number}} */ date)=>date.year*12+date.month-1;
 const start=at(first),selectedEnd=at(last),end=Math.min(selectedEnd,at(now)-1);
 if(start>selectedEnd)throw new Error('The selected date span is reversed.');
 const count=end-start+1,training=Math.floor(count*4/5),later=count-training;
 if(training<2||later<2)throw new Error('The proposed chronological 80%/20% split needs at least two complete months in both periods. Widen the selected span or edit both periods.');
 const key=(/** @type {number} */ n)=>`${String(Math.floor(n/12)).padStart(4,'0')}-${String(n%12+1).padStart(2,'0')}`;
 return {selectedFrom:from,selectedTo:to,from:key(start),to:key(start+training-1),laterFrom:key(start+training),laterTo:key(end),completeMonths:count,excludedCurrentOrFuture:Math.max(0,selectedEnd-end),serverMonth};
}
/** @param {any} value */
function bits(value){if(value==='all')return value;if(typeof value!=='string'||value.length===0||value.length>4096)throw new Error('Choose all conditions or canonical increasing condition IDs.');let previous=-1n;for(const item of value.split(',')){if(!uint(item,false)||BigInt(item)>(1n<<32n)-1n||BigInt(item)<=previous)throw new Error('Condition IDs must be canonical, unique and increasing; live eligibility remains checked by the server.');previous=BigInt(item);}return value;}
/** @param {any} wire */
function validateWire(wire){
 if(!keys(wire,WIRE)||wire.command!==INDEX_STOP_COMMAND||typeof wire.feed!=='string'||!/^[a-z][a-z0-9_-]{0,63}$/.test(wire.feed)||!INDEX_STOP_INDICES.includes(wire.index)||!hex(wire.expected_policy_digest)||!KNOBS.every(k=>uint(wire[k]))||BigInt(wire.max_loss_points)>MAX/100n||size(wire)>REQUEST_BYTES)throw new Error('The exact single-stop request is incomplete or changed its model.');
 timeframes(wire.timeframes);bits(wire.bits);
 const dates=[];for(const prefix of ['from','to','later_from','later_to']){const year=wire[`${prefix}_year`],m=wire[`${prefix}_month`];if(!Number.isInteger(year)||year<1||year>9999||!Number.isInteger(m)||m<1||m>12)throw new Error('Saved month fields must be exact numeric calendar months.');dates.push(year*12+m);}
 if(dates[1]-dates[0]<1||dates[3]-dates[2]<1||dates[1]>=dates[2])throw new Error('Training and later periods each need at least two months, ordered and nonoverlapping.');return wire;
}
/** Metadata-backed plans alone authorize a click; a restored request is observation-only.
 * @param {any} input @param {any} metadata */
export function indexStopLaunchPlan(input,metadata){
 if(!metas.has(metadata)||!metadata.ready)throw new Error('Read ready single-stop configuration before starting.');
 if(!object(input)||Object.keys(input).some(k=>!INPUT.includes(k)))throw new Error('Only the declared single-stop inputs are accepted.');
 const from=month(input.from,'Training start'),to=month(input.to,'Training end'),laterFrom=month(input.laterFrom,'Later start'),laterTo=month(input.laterTo,'Later end');
 const wire=validateWire({command:INDEX_STOP_COMMAND,feed:input.feed,index:indexStopInstrument(input.symbols),timeframes:timeframes(input.timeframes),from_year:from.year,from_month:from.month,to_year:to.year,to_month:to.month,later_from_year:laterFrom.year,later_from_month:laterFrom.month,later_to_year:laterTo.year,later_to_month:laterTo.month,bits:bits(input.bits??'all'),batch_programs:input.batchPrograms,node_allowance:input.nodeAllowance,batch_allowance:input.batchAllowance,max_loss_points:input.maxLossPoints,expected_policy_digest:metadata.policy.digest});
 if(wire.max_loss_points!==metadata.configured.max_loss_points||wire.batch_programs!==metadata.configured.batch_programs)throw new Error('Recheck configuration for the exact loss limit and batch size before starting.');
 if(BigInt(wire.node_allowance)>BigInt(metadata.limits.physical.capture_records))throw new Error('The grammar allowance exceeds the configured capture limit.');
 const plan=freeze(wire);plans.add(plan);return plan;
}
/** @param {any} running */
function recordedPlan(running){if(running?.command!==INDEX_STOP_COMMAND||running.index_stop?.execution_policy!=='signal_candle_stop_v1')throw new Error('The saved attempt is not this execution model.');const plan=freeze(structuredClone(validateWire(running.index_stop.request)));observations.add(plan);return plan;}
/** @param {any} running @param {any} plan @param {string} attempt @param {any} [previous] */
export function indexStopLaunchObservation(running,plan,attempt,previous=null){
 const value=running?.index_stop;
 if((!plans.has(plan)&&!observations.has(plan))||!uint(attempt)||running?.where!=='browser'||running?.kind!=='command'||running?.command!==INDEX_STOP_COMMAND||liveAttemptKey(running)!==attempt||!keys(value,['schema_version','command','execution_policy','request','search_identity','completed_batches','completed_programs','completed_work','current_batch','elapsed_micros','elapsed_basis','exhausted','qualifications','latest_saved_batch'])||value.schema_version!==1||value.command!==INDEX_STOP_COMMAND||value.execution_policy!=='signal_candle_stop_v1'||!same(value.request,plan))throw new Error('Status does not match the exact request and attempt. No completion is inferred.');
 if(!(value.search_identity===null||hex(value.search_identity))||!(value.completed_batches===null||uint(value.completed_batches,false))||!(value.exhausted===null||typeof value.exhausted==='boolean')||(value.search_identity===null)!==(value.completed_batches===null)||(value.exhausted===null)!==(value.completed_batches===null)||previous?.searchIdentity&&previous.searchIdentity!==value.search_identity||previous?.completedBatches!=null&&(value.completed_batches===null||BigInt(value.completed_batches)<BigInt(previous.completedBatches))||previous?.exhausted===true&&value.exhausted!==true)throw new Error('Saved search identity or acknowledged progress changed or moved backwards.');
 if(!Array.isArray(value.qualifications)||value.qualifications.length!==plan.timeframes.length)throw new Error('Status must retain every selected timeframe and no others.');
 for(const [n,row] of value.qualifications.entries()){if(!keys(row,['timeframe','rung','identity','completion','stage'])||row.timeframe!==plan.timeframes[n]||row.rung!==CAMPAIGN_RUNGS.indexOf(row.timeframe)||(row.identity===null)!==(row.completion===null)||!(row.identity===null||hex(row.identity)&&hex(row.completion))||!(row.stage===null||['preparing','training','later','institutional','saved','refused'].includes(row.stage)))throw new Error('A selected timeframe lost its exact saved comparison link or work boundary.');}
 for(const [wire,prior] of [['completed_programs','completedPrograms'],['completed_work','completedWork']]){if(!(value[wire]===null||uint(value[wire],false))||(value[wire]===null)!==(value.search_identity===null)||previous?.[prior]!=null&&(value[wire]===null||BigInt(value[wire])<BigInt(previous[prior])||value.completed_batches===previous.completedBatches&&value[wire]!==previous[prior]))throw new Error('Acknowledged program or work counters changed without a completed batch.');}
 if(!(value.current_batch===null||uint(value.current_batch,false)&&value.current_batch===value.completed_batches)||value.exhausted===true&&value.current_batch!==null||!(value.elapsed_micros===null||uint(value.elapsed_micros,false))||value.elapsed_basis!=='wall-clock')throw new Error('Current reservation or measured elapsed time is malformed.');
 const hasLinks=value.qualifications.some((/** @type {any} */ row)=>row.identity!==null);
 const links=(/** @type {any[]} */ rows)=>rows.map(({timeframe,rung,identity,completion})=>({timeframe,rung,identity,completion}));
 const saved=value.latest_saved_batch;
 if(saved!==null&&(!keys(saved,['batch','qualifications'])||!uint(saved.batch,false)||value.completed_batches===null||BigInt(saved.batch)>=BigInt(value.completed_batches)||!Array.isArray(saved.qualifications)||saved.qualifications.length!==plan.timeframes.length||saved.qualifications.some((/** @type {any} */ row,/** @type {number} */ n)=>!keys(row,['timeframe','rung','identity','completion'])||row.timeframe!==plan.timeframes[n]||row.rung!==CAMPAIGN_RUNGS.indexOf(row.timeframe)||!hex(row.identity)||!hex(row.completion))))throw new Error('The retained comparison changed its saved batch or selected timeframe receipts.');
 if(hasLinks&&(saved===null||saved.batch!==String(BigInt(value.completed_batches)-1n)||!same(saved.qualifications,links(value.qualifications)))||previous?.latestSavedBatch&&(saved===null||BigInt(saved.batch)<BigInt(previous.latestSavedBatch.batch)||saved.batch===previous.latestSavedBatch.batch&&!same(saved,previous.latestSavedBatch)))throw new Error('Previously acknowledged comparisons cannot disappear, change receipts or be relabelled as pending work.');
 if(hasLinks&&(value.current_batch!==null||value.qualifications.some((/** @type {any} */ row)=>row.identity===null))||(value.completed_batches===null||value.completed_batches==='0')&&(hasLinks||value.exhausted===true)||previous?.completedBatches!=null&&value.completed_batches===previous.completedBatches&&(value.exhausted!==previous.exhausted||value.current_batch===previous.currentBatch&&!same(links(value.qualifications),links(previous.qualifications))))throw new Error('A completed batch changed its exact timeframe evidence or claims premature completion.');
 if(!(running.report===null||typeof running.report==='string'&&running.report.trim())||!(running.refusal===null||text(running.refusal))||running.report!==null&&running.refusal!==null||running.finished_micros!=null&&(typeof running.finished_micros!=='number'||!Number.isInteger(running.finished_micros))||running.in_flight===true&&(running.report!==null||running.refusal!==null||running.finished_micros!=null)||running.in_flight===false&&running.finished_micros===null)throw new Error('Attempt lifecycle contains contradictory terminal evidence.');
 const outcome=sweepOutcome(running);
 if(!['running','done','failed'].includes(outcome.phase)||outcome.phase==='done'&&(!hex(value.search_identity)||value.completed_batches===null)||previous?.phase==='done'&&outcome.phase!=='done'||previous?.phase==='refused'&&outcome.phase!=='failed'||running.status!==undefined&&!['running','completed','refused','failed','cancelled'].includes(running.status)||running.status==='running'&&outcome.phase!=='running'||running.status==='completed'&&outcome.phase!=='done'||['refused','failed','cancelled'].includes(running.status)&&outcome.phase!=='failed')throw new Error(outcome.why||'This attempt has no consistent running or terminal outcome.');
 return {phase:outcome.phase==='failed'?'refused':outcome.phase,attempt,plan,searchIdentity:value.search_identity,completedBatches:value.completed_batches,completedPrograms:value.completed_programs,completedWork:value.completed_work,currentBatch:value.current_batch,elapsedMicros:value.elapsed_micros,exhausted:value.exhausted,qualifications:structuredClone(value.qualifications),latestSavedBatch:structuredClone(saved),report:running.report,why:outcome.why};
}

/** One click sends one POST. All continued activity is exact-attempt read-only
 * observation, paused on hidden pages. An ambiguous POST is never resent.
 * @param {{changed:(state:any)=>void,request?:(url:string,options?:RequestInit)=>Promise<any>,visible?:()=>boolean,listen?:(callback:()=>void)=>()=>void,interval?:number}} options */
export function createIndexStopLaunch({changed,request=ask,visible=()=>typeof document==='undefined'||!document.hidden,listen=callback=>{if(typeof document==='undefined')return ()=>{};document.addEventListener('visibilitychange',callback);return()=>document.removeEventListener('visibilitychange',callback);},interval=2000}){
 const reads=createPageRequests();let disposed=false,postAbort=/** @type {AbortController|null} */(null),epoch=0,failures=0;
 let state=/** @type {any} */({phase:'idle',attempt:null,plan:null,searchIdentity:null,completedBatches:null,exhausted:null,qualifications:[],report:null,why:''});
 const publish=()=>{if(!disposed)changed(freeze(structuredClone(state)));};
 /** @param {unknown} why */const unknown=why=>{state={...state,phase:'unknown',why:why instanceof Error?why.message:String(why)};publish();};
 /** @param {import('./page-requests.js').ReadTicket} ticket */
 async function observe(ticket){try{const response=await request(`/backtest/run.json?attempt=${encodeURIComponent(state.attempt)}`,{cache:'no-store',signal:ticket.signal});if(!ticket.current())return;if(!response.ok)throw new Error(`This exact attempt could not be read (HTTP ${response.status}); its launch will not be resent.`);const body=await response.json();if(!ticket.current())return;state=indexStopLaunchObservation(body?.running,state.plan,state.attempt,state);failures=0;publish();}catch(why){if(ticket.current()){failures=Math.min(4,failures+1);unknown(why);}}finally{if(ticket.current()&&visible()&&['running','unknown'].includes(state.phase))reads.schedule(observe,Math.min(30000,interval*2**failures));}}
 const recheck=()=>{if(disposed||!uint(state.attempt)||postAbort)return;reads.cancel();if(visible())return reads.run(observe);};
 const unlisten=listen(()=>{reads.cancel();if(visible()&&['running','unknown'].includes(state.phase))void recheck();});
 return {
  /** @param {any} plan */async start(plan){
   if(disposed||!plans.has(plan))throw new Error('A validated current plan is required before clicking Run sweep.');
   if(postAbort||['starting','running','unknown'].includes(state.phase))throw new Error('The previous request is still active or unconfirmed; no duplicate launch is allowed.');
   reads.cancel();failures=0;const ticket=++epoch,abort=new AbortController();postAbort=abort;state={phase:'starting',attempt:null,plan,searchIdentity:null,completedBatches:null,exhausted:null,qualifications:[],report:null,why:''};publish();
   try{const response=await request('/engine/command',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(plan),signal:abort.signal});if(disposed||ticket!==epoch)return;const body=await response.json();if(disposed||ticket!==epoch)return;
    if(body?.accepted===false&&text(body.refusal)&&response.status!==202&&body.attempt==null&&body.attempt_key==null&&body.started!==true){state={...state,phase:'refused',why:body.refusal};publish();return;}
    if(body?.accepted===true&&uint(body.attempt))state={...state,attempt:body.attempt};
    if(!response.ok||response.status!==202||body?.accepted!==true||body.refusal!==null||state.attempt===null||body.attempt_key!==undefined&&body.attempt_key!==state.attempt||body.started===false)throw new Error('The launch response is unconfirmed. It may have started; no duplicate request will be sent.');
    state={...state,phase:'running'};publish();
   }catch(why){if(!disposed&&ticket===epoch)unknown(why);}finally{if(postAbort===abort)postAbort=null;}
   if(!disposed&&ticket===epoch&&state.phase==='running')await recheck();
  },
  recheck,
  /** Restore observation only; current form edits cannot replace a saved request.
   * @param {any} running */async restore(running){if(disposed||postAbort)return;reads.cancel();try{const attempt=liveAttemptKey(running),plan=recordedPlan(running);if(typeof attempt!=='string'||!uint(attempt)||state.attempt!==null&&state.attempt!==attempt||state.plan!==null&&!same(state.plan,plan)||state.plan!==null&&state.attempt===null&&state.phase==='unknown')throw new Error('The restored attempt changed or cannot establish the original request.');state=indexStopLaunchObservation(running,plan,attempt,state.attempt===null?null:state);publish();if(state.phase==='running')await recheck();}catch(why){if(!disposed)unknown(why);}},
  stop(){epoch++;postAbort?.abort();postAbort=null;reads.cancel();if(['starting','running','unknown'].includes(state.phase))unknown('Observation stopped; the server may continue. Rechecking cannot resend the launch.');},
  dispose(){disposed=true;epoch++;postAbort?.abort();reads.dispose();unlisten();}
 };
}

/** @param {string} identity @param {string} completion */
export function indexStopQualificationHref(identity,completion){if(!hex(identity)||!hex(completion))throw new Error('A saved comparison requires its exact identity and completion.');return `/backtest?${new URLSearchParams({index_stop_qualification:identity,completion})}#index-stop-qualification`;}

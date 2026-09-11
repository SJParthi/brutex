import {ask} from './ask.js';
import {detailRefusal} from './detail-refusal.js';
import {createPageRequests} from './page-requests.js';
import {CAMPAIGN_RUNGS} from './boolean-campaign.js';
import {validateSetting} from './index-stop-results.js';
import {validateNativeAdmissionRow} from './boolean-evidence.js';
import {validateIndexConsistency} from './index-consistency.js';

const U64=(1n<<64n)-1n,I64=(1n<<63n)-1n;
/** @param {any} v */const object=v=>v!==null&&typeof v==='object'&&!Array.isArray(v);
/** @param {any} v @param {string[]} names */const keys=(v,names)=>object(v)&&Object.keys(v).length===names.length&&names.every(k=>Object.hasOwn(v,k));
/** @param {any} v */const hex=v=>typeof v==='string'&&/^[a-f0-9]{64}$/.test(v)&&v!=='0'.repeat(64);
/** @param {any} v */const uint=v=>typeof v==='string'&&v.length<=20&&/^(0|[1-9]\d*)$/.test(v)&&BigInt(v)<=U64;
/** @param {any} v */const sint=v=>typeof v==='string'&&v.length<=20&&/^(0|-?[1-9]\d*)$/.test(v)&&BigInt(v)>=-I64-1n&&BigInt(v)<=I64;
/** @param {any} v */const link=v=>keys(v,['identity','completion'])&&hex(v.identity)&&hex(v.completion);
/** @param {any} v */const checkpoint=v=>keys(v,['sequence','completion'])&&uint(v.sequence)&&v.sequence!=='0'&&hex(v.completion);
/** @param {any} a @param {any} b */const samePin=(a,b)=>a.sequence===b.sequence&&a.completion===b.completion;
/** @param {any} input */
function selection(input){
 const {identity,timeframe,checkpoint:pin=null,filter='qualified',offset='0',limit=16}=input??{};
 if(!hex(identity)||!CAMPAIGN_RUNGS.includes(timeframe)||pin!==null&&!checkpoint(pin)||!['qualified','all'].includes(filter)||!uint(offset)||!Number.isInteger(limit)||limit<1||limit>256||offset!=='0'&&pin===null)throw new Error('Select one saved search, intraday timeframe and exact checkpoint for further pages.');
 return {identity,timeframe,checkpoint:pin,filter,offset,limit};
}
/** @param {any} scope @param {string} allTotal */
function validateScope(scope,allTotal){
 const counters=['completed_batches','completed_programs','completed_work','included_families','interrupted_checkpoints'],dates=['training_first_day','training_last_day','later_first_day','later_last_day'];
 if(!keys(scope,[...counters,...dates,'first_batch','last_batch','pending_next_batch','exhausted','writer_observed_at_admission','instrument','feed'])||!counters.every(k=>uint(scope[k]))||!dates.every(k=>sint(scope[k]))||!['pending_next_batch','exhausted','writer_observed_at_admission'].every(k=>typeof scope[k]==='boolean')||scope.pending_next_batch&&scope.exhausted||!['NSE-NIFTY','NSE-BANKNIFTY'].includes(scope.instrument)||typeof scope.feed!=='string'||scope.feed.length===0||scope.feed.length>256||!/^[a-z][a-z0-9_-]*$/.test(scope.feed)||BigInt(scope.training_first_day)>BigInt(scope.training_last_day)||BigInt(scope.training_last_day)>=BigInt(scope.later_first_day)||BigInt(scope.later_first_day)>BigInt(scope.later_last_day))throw new Error('The cumulative result scope has invalid dates, status or source identity.');
 const families=BigInt(scope.included_families),batches=BigInt(scope.completed_batches),programs=BigInt(scope.completed_programs);
 if(programs*2n!==BigInt(allTotal)||families>batches||families>programs||(families===0n)!==(programs===0n)||families===0n&&(scope.first_batch!==null||scope.last_batch!==null)||families>0n&&(!uint(scope.first_batch)||!uint(scope.last_batch)||BigInt(scope.first_batch)>BigInt(scope.last_batch)||BigInt(scope.last_batch)>=batches||BigInt(scope.last_batch)-BigInt(scope.first_batch)+1n<families))throw new Error('The complete acknowledged program and batch counts do not reconcile.');
}
/** @param {any} row @param {any} body @param {bigint} rank */
function validateRow(row,body,rank){
 if(!keys(row,['index','rank','global_rank','batch','qualification_rank','qualification_total','qualification','original_source','later_source','original','later','institutional','index_consistency'])||!['index','rank','global_rank','batch','qualification_rank','qualification_total'].every(k=>uint(row[k]))||row.rank!==String(rank)||!link(row.qualification)||!link(row.original_source)||!link(row.later_source)||row.original_source.identity===row.later_source.identity||BigInt(row.qualification_total)===0n||BigInt(row.qualification_total)%2n!==0n||BigInt(row.qualification_total)>BigInt(body.all_total)||BigInt(row.index)>=BigInt(row.qualification_total)||BigInt(row.qualification_rank)<1n||BigInt(row.qualification_rank)>BigInt(row.qualification_total)||BigInt(row.global_rank)<rank||BigInt(row.global_rank)>BigInt(body.all_total)||body.filter==='all'&&row.global_rank!==row.rank||BigInt(row.batch)<BigInt(body.scope.first_batch)||BigInt(row.batch)>BigInt(body.scope.last_batch))throw new Error('A cumulative rank or exact batch/setting link is missing or relabelled.');
 validateSetting(row.original);validateSetting(row.later);
 if(row.original.index!==row.index||row.later.index!==row.index||!['program_index','expression','instrument','direction','timeframe'].every(k=>row.original[k]===row.later[k])||row.original.instrument!==body.scope.instrument||row.original.timeframe!==body.timeframe||row.original.first_day!==body.scope.training_first_day||row.original.last_day!==body.scope.training_last_day||row.later.first_day!==body.scope.later_first_day||row.later.last_day!==body.scope.later_last_day||row.original.run_id===row.later.run_id||!validateNativeAdmissionRow(row.institutional)||row.institutional.index!==row.index||row.institutional.source_index!==row.index||row.institutional.identity!==row.later.run_id)throw new Error('A ranked setting changed its original/later source, direction or institutional evidence.');
 validateIndexConsistency(row.index_consistency,{qualification:row.qualification,setting_index:row.index,instrument:row.original.instrument,institutional:row.institutional.status});
 const daily=row.index_consistency;
 if(!daily||['not_assessed','not_applicable'].includes(daily.state)||!['training','later'].every(period=>{const source=period==='training'?row.original:row.later,p=daily.periods?.[period];return p&&p.first_day===source.first_day&&p.last_day===source.last_day;})||body.filter==='qualified'&&daily.combined_qualifies!==true)throw new Error('The required index assessment is absent, changed, or does not pass the selected filter.');
}
/** @param {any} input @param {(url:string)=>Promise<any>} [request] */
export async function fetchIndexStopRanking(input,request=ask){
 const a=selection(input),query=new URLSearchParams({identity:a.identity,timeframe:a.timeframe,filter:a.filter,offset:a.offset,limit:String(a.limit)});
 if(a.checkpoint){query.set('sequence',a.checkpoint.sequence);query.set('completion',a.checkpoint.completion);}
 const response=await request('/index-stop-ranking.json?'+query);
 if(!response?.ok)throw new Error(await detailRefusal(response,'The cumulative saved comparison is unavailable.'));
 const body=await response.json(),summary=['institutional_admitted','rejected','refused','unmeasured','combined_qualified'];
 if(!keys(body,['schema_version','status','model','identity','timeframe','rung','checkpoint','scope','ranking','filter','total','all_total','offset','limit','summary','read_work','rows','refusal'])||body.schema_version!==1||body.status!=='saved'||body.model!=='index-stop-ranking'||body.identity!==a.identity||body.timeframe!==a.timeframe||body.rung!==String(CAMPAIGN_RUNGS.indexOf(a.timeframe))||!checkpoint(body.checkpoint)||a.checkpoint!==null&&!samePin(body.checkpoint,a.checkpoint)||body.ranking!=='later_pessimistic_descending_then_batch_then_setting'||body.filter!==a.filter||body.offset!==a.offset||body.limit!==a.limit||!uint(body.total)||!uint(body.all_total)||!keys(body.summary,summary)||!summary.every(k=>uint(body.summary[k]))||!Array.isArray(body.rows)||body.refusal!==null)throw new Error('The cumulative response changed its search, timeframe, checkpoint or bounded page.');
 validateScope(body.scope,body.all_total);
 const work=['retained_serialized_bytes','charged_grammar_nodes','charged_required_bootstrap_work','charged_required_split_work'];
 if(!keys(body.read_work,[...work,'cold','warm'])||!work.every(k=>uint(body.read_work[k]))||body.read_work.cold!=='complete_acknowledged_prefix_then_global_sort'||body.read_work.warm!=='ancestor_generation_checks_then_bounded_indexed_page')throw new Error('The cumulative read work is not explicitly bounded.');
 const start=BigInt(a.offset),total=BigInt(body.total),count=total-start<BigInt(a.limit)?total-start:BigInt(a.limit);
 if(start>total||BigInt(body.rows.length)!==count||summary.slice(0,4).reduce((n,k)=>n+BigInt(body.summary[k]),0n)!==BigInt(body.all_total)||BigInt(body.summary.combined_qualified)>BigInt(body.summary.institutional_admitted)||body.total!==(a.filter==='qualified'?body.summary.combined_qualified:body.all_total))throw new Error('Cumulative verdict totals, filter or page extent do not reconcile.');
 let prior=/** @type {any} */(null);const seen=new Set(),families=new Map(),observed={institutional_admitted:0n,rejected:0n,refused:0n,unmeasured:0n,combined_qualified:0n};
 for(const [index,row] of body.rows.entries()){
  validateRow(row,body,start+BigInt(index)+1n);
  const key=row.batch+':'+row.index;if(seen.has(key))throw new Error('A cumulative setting was repeated.');seen.add(key);
  const familyKey=JSON.stringify([row.qualification,row.original_source,row.later_source,row.qualification_total]);if(families.has(row.batch)&&families.get(row.batch)!==familyKey)throw new Error('One saved batch changed its exact qualification or source pins.');families.set(row.batch,familyKey);
  const score=BigInt(row.later.metrics.pessimistic_paisa);
  if(prior&&(BigInt(row.global_rank)<=BigInt(prior.global_rank)||score>BigInt(prior.later.metrics.pessimistic_paisa)||score===BigInt(prior.later.metrics.pessimistic_paisa)&&(BigInt(row.batch)<BigInt(prior.batch)||row.batch===prior.batch&&BigInt(row.index)<=BigInt(prior.index))))throw new Error('The cumulative later-return ordering or tie-break changed.');prior=row;
  observed[/** @type {keyof typeof observed} */(row.institutional.status==='admitted'?'institutional_admitted':row.institutional.status)]++;
  if(row.index_consistency.combined_qualifies)observed.combined_qualified++;
 }
 if(summary.some(k=>observed[/** @type {keyof typeof observed} */(k)]>BigInt(body.summary[k])||a.filter==='all'&&start===0n&&count===total&&observed[/** @type {keyof typeof observed} */(k)]!==BigInt(body.summary[k])))throw new Error('Visible outcomes contradict the cumulative summary.');
 return {...body,next:start+count<total?String(start+count):null};
}
/** Translate a global rank back to its exact local saved page and canonical row.
 * @param {any} row */
export function indexStopRankingDetail(row){
 if(!link(row?.qualification)||!uint(row.index)||!uint(row.qualification_rank)||row.qualification_rank==='0'||!uint(row.qualification_total)||BigInt(row.index)>=BigInt(row.qualification_total)||BigInt(row.qualification_rank)>BigInt(row.qualification_total))throw new Error('Exact ranked qualification detail is unavailable.');
 return {identity:row.qualification.identity,completion:row.qualification.completion,offset:String((BigInt(row.qualification_rank)-1n)/16n*16n),setting:row.index};
}
/** One active read and one newest queued selection; old scopes stay labelled
 * until a replacement is authenticated. No request here starts research.
 * @param {(state:any)=>void} changed @param {(url:string,options?:RequestInit)=>Promise<any>} [request] */
export function createIndexStopRankingReader(changed,request=ask){
 const reads=createPageRequests();let disposed=false,retained=/** @type {any} */(null);
 return {
  /** @param {any} input */async open(input){
   if(disposed)return;reads.cancel();let a;
   try{a=selection(input);}catch(why){changed({phase:'failed',body:retained,why:String(why),selection:null});return;}
   if(retained&&retained.identity!==a.identity)retained=null;
   changed({phase:'loading',body:retained,why:'',selection:a});
   await reads.run(async ticket=>{try{
    const body=await fetchIndexStopRanking(a,url=>request(url,{cache:'no-store',signal:ticket.signal}));
    if(ticket.current()){
     if(retained&&retained.identity===body.identity&&a.checkpoint===null){
      const counters=['completed_batches','completed_programs','completed_work'],same=body.checkpoint.sequence===retained.checkpoint.sequence;
      if(BigInt(body.checkpoint.sequence)<BigInt(retained.checkpoint.sequence)||same&&(body.checkpoint.completion!==retained.checkpoint.completion||[...counters,'pending_next_batch','exhausted'].some(k=>body.scope[k]!==retained.scope[k]))||counters.some(k=>BigInt(body.scope[k])<BigInt(retained.scope[k]))||retained.scope.exhausted&&!same)throw new Error('Latest checkpoint moved backwards or changed its immutable pin or acknowledged counters.');
     }
     retained=body;changed({phase:'ready',body,why:'',selection:a});
    }
   }catch(why){if(ticket.current())changed({phase:'failed',body:retained,why:String(why),selection:a});}});
  },
  close(){reads.cancel();retained=null;if(!disposed)changed({phase:'idle',body:null,why:'',selection:null});},
  dispose(){disposed=true;reads.dispose();retained=null;}
 };
}

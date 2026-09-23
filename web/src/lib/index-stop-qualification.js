import {ask} from './ask.js';
import {detailRefusal} from './detail-refusal.js';
import {createPageRequests} from './page-requests.js';
import {validateSetting} from './index-stop-results.js';
import {validateNativeAdmissionRow} from './boolean-evidence.js';
import {validateNativeResearchPolicy} from './boolean-launch.js';
import {validateIndexConsistency} from './index-consistency.js';
import {CAMPAIGN_RUNGS} from './boolean-campaign.js';
const U64=(1n<<64n)-1n,I64=(1n<<63n)-1n,MODEL='index-stop-qualification';
/** @param {any} v */const object=v=>v!==null&&typeof v==='object'&&!Array.isArray(v);
/** @param {any} v @param {string[]} names */const keys=(v,names)=>object(v)&&Object.keys(v).length===names.length&&names.every(k=>Object.hasOwn(v,k));
/** @param {any} v */const hex=v=>typeof v==='string'&&v.length===64&&/^[a-f0-9]{64}$/.test(v)&&v!=='0'.repeat(64);
/** @param {any} v */const uint=v=>typeof v==='string'&&v.length<=20&&/^(0|[1-9]\d*)$/.test(v)&&BigInt(v)<=U64;
/** @param {any} v */const sint=v=>typeof v==='string'&&v.length<=20&&/^(0|-?[1-9]\d*)$/.test(v)&&BigInt(v)>=-I64-1n&&BigInt(v)<=I64;
/** @param {any} v */const link=v=>keys(v,['identity','completion'])&&hex(v.identity)&&hex(v.completion);
/** @param {any} a @param {any} b */const sameLink=(a,b)=>link(a)&&link(b)&&a.identity===b.identity&&a.completion===b.completion;
/** @param {any} body @param {any} row */
export const indexStopQualificationContext=(body,row)=>({qualification:{identity:body.identity,completion:body.completion},setting_index:row.index,instrument:row.original.instrument,institutional:row.institutional.status});
/** Open a cumulative-ranking link only on its exact pinned qualification page.
 * The expected setting is an address, not an array position or a new rank.
 * @param {any} body @param {{identity:string,completion:string,offset:string,setting:string}} expected */
export function indexStopQualificationInitialRow(body,expected){
 if(!hex(expected?.identity)||!hex(expected?.completion)||!uint(expected?.offset)||!uint(expected?.setting)||body?.identity!==expected.identity||body?.completion!==expected.completion||body?.offset!==expected.offset||!Array.isArray(body.rows))throw new Error('The requested setting does not match this exact saved comparison page.');
 const rows=body.rows.filter((/** @type {any} */ row)=>row.index===expected.setting);
 if(rows.length!==1)throw new Error('The requested setting is absent or repeated on its pinned comparison page. No other setting was selected.');
 return rows[0];
}
/** @param {any} select */
function selection(select){const {identity,completion=null,offset='0',limit=16}=select??{};if(!hex(identity)||completion!==null&&!hex(completion)||!uint(offset)||!Number.isInteger(limit)||limit<1||limit>256||offset!=='0'&&completion===null)throw new Error('Select an exact saved qualification and a pinned bounded page.');return {identity,completion,offset,limit};}
/** @param {any} select @param {(url:string)=>Promise<any>} [request] */
export async function fetchIndexStopQualification(select,request=ask){
 const a=selection(select),query=new URLSearchParams({identity:a.identity,kind:'settings',offset:a.offset,limit:String(a.limit)});if(a.completion)query.set('completion',a.completion);
 const response=await request('/index-stop-qualification.json?'+query);if(!response?.ok)throw new Error(await detailRefusal(response,'The saved single-stop comparison is unavailable.'));
 const body=await response.json(),summaryFields=['institutional_admitted','rejected','refused','unmeasured','combined_qualified'];
 if(!keys(body,['schema_version','status','model','kind','identity','completion','search_identity','batch','rung','original','later','total','offset','limit','summary','ranking','policy','rows','refusal'])||body.schema_version!==1||body.status!=='saved'||body.model!==MODEL||body.kind!=='settings'||body.identity!==a.identity||!hex(body.completion)||a.completion!==null&&a.completion!==body.completion||!hex(body.search_identity)||!uint(body.batch)||!uint(body.rung)||BigInt(body.rung)>=BigInt(CAMPAIGN_RUNGS.length)||!sameLink(body.original,body.original)||!sameLink(body.later,body.later)||body.original.identity===body.later.identity||body.offset!==a.offset||body.limit!==a.limit||!uint(body.total)||BigInt(body.total)===0n||BigInt(body.total)%2n!==0n||!keys(body.summary,summaryFields)||!summaryFields.every(k=>uint(body.summary[k]))||body.ranking!=='later_pessimistic_descending'||body.refusal!==null||!Array.isArray(body.rows)||!keys(body.policy,['digest','values']))throw new Error('The comparison does not match its exact qualification, sources and bounded page.');
 validateNativeResearchPolicy({...body.policy,ready:true,refusal:null});
 const start=BigInt(a.offset),total=BigInt(body.total),available=total-start,count=available<BigInt(a.limit)?available:BigInt(a.limit);
 if(start>total||BigInt(body.rows.length)!==count||summaryFields.slice(0,4).reduce((n,k)=>n+BigInt(body.summary[k]),0n)!==total||BigInt(body.summary.combined_qualified)>BigInt(body.summary.institutional_admitted))throw new Error('The qualification counts or page extent do not reconcile.');
 const seen=new Set(),observed={institutional_admitted:0n,rejected:0n,refused:0n,unmeasured:0n,combined_qualified:0n};let previous=/** @type {any} */(null);
 for(const [n,row] of body.rows.entries()){
  if(!keys(row,['index','rank','original','later','institutional','index_consistency'])||!uint(row.index)||BigInt(row.index)>=total||seen.has(row.index)||row.rank!==String(start+BigInt(n)+1n))throw new Error('A ranked setting is missing, duplicated or relabelled.');seen.add(row.index);
  validateSetting(row.original);validateSetting(row.later);
  if(row.original.index!==row.index||row.later.index!==row.index||!['program_index','expression','instrument','direction','timeframe'].every(k=>row.original[k]===row.later[k])||row.original.timeframe!==CAMPAIGN_RUNGS[Number(body.rung)]||BigInt(row.original.last_day)>=BigInt(row.later.first_day)||row.original.run_id===row.later.run_id||!keys(row.institutional,['index','identity','source_index','status','failed','unmeasured','refused','checks','values'])||!validateNativeAdmissionRow(row.institutional)||row.institutional.index!==row.index||row.institutional.identity!==row.later.run_id)throw new Error('Original, later or institutional evidence belongs to a different setting.');
  validateIndexConsistency(row.index_consistency,indexStopQualificationContext(body,row));
  if(!row.index_consistency||['not_assessed','not_applicable'].includes(row.index_consistency.state)||!['training','later'].every(period=>{const source=period==='training'?row.original:row.later,p=row.index_consistency.periods?.[period];return p&&p.first_day===source.first_day&&p.last_day===source.last_day;}))throw new Error('The required index assessment is absent or uses different training/later periods.');
  const score=BigInt(row.later.metrics.pessimistic_paisa);if(previous&&(score>BigInt(previous.later.metrics.pessimistic_paisa)||score===BigInt(previous.later.metrics.pessimistic_paisa)&&BigInt(row.index)<=BigInt(previous.index)))throw new Error('Saved later-return ordering changed.');previous=row;
  observed[/** @type {keyof typeof observed} */(row.institutional.status==='admitted'?'institutional_admitted':row.institutional.status)]++;
  if(row.index_consistency.combined_qualifies)observed.combined_qualified++;
 }
 if(summaryFields.some(k=>observed[/** @type {keyof typeof observed} */(k)]>BigInt(body.summary[k])||start===0n&&count===total&&observed[/** @type {keyof typeof observed} */(k)]!==BigInt(body.summary[k])))throw new Error('Visible verdicts contradict the saved comparison summary.');
 return {...body,next:start+count<total?String(start+count):null};
}
/** @param {(state:any)=>void} changed @param {(url:string,options?:RequestInit)=>Promise<any>} [request] */
export function createIndexStopQualificationReader(changed,request=ask){const reads=createPageRequests();let disposed=false;return {
 /** @param {any} input */async open(input){if(disposed)return;reads.cancel();let a;try{a=selection(input);}catch(why){changed({phase:'failed',body:null,why:String(why),selection:null});return;}changed({phase:'loading',body:null,why:'',selection:a});await reads.run(async ticket=>{try{const body=await fetchIndexStopQualification(a,url=>request(url,{cache:'no-store',signal:ticket.signal}));if(ticket.current())changed({phase:'ready',body,why:'',selection:a});}catch(why){if(ticket.current())changed({phase:'failed',body:null,why:String(why),selection:a});}});},
 close(){reads.cancel();if(!disposed)changed({phase:'idle',body:null,why:'',selection:null});},dispose(){disposed=true;reads.dispose();}
};}
/** @param {any} body @param {any} selected @param {string} [offset] @param {(url:string)=>Promise<any>} [request] */
export async function fetchIndexStopFolds(body,selected,offset='0',request=ask){
 if(!hex(body?.identity)||!hex(body?.completion)||!uint(selected?.index)||!uint(offset)||!link(body.original)||!link(body.later))throw new Error('Fold evidence requires an exact saved setting and source links.');
 const query=new URLSearchParams({identity:body.identity,completion:body.completion,kind:'folds',setting:selected.index,offset,limit:'16'}),response=await request('/index-stop-qualification.json?'+query);
 if(!response?.ok)throw new Error(await detailRefusal(response,'The saved fold page is unavailable.'));const p=await response.json();
 if(!keys(p,['schema_version','status','model','kind','identity','completion','original','later','setting','total','offset','limit','rows','refusal'])||p.schema_version!==1||p.status!=='saved'||p.model!==MODEL||p.kind!=='folds'||p.identity!==body.identity||p.completion!==body.completion||p.setting!==selected.index||!sameLink(p.original,body.original)||!sameLink(p.later,body.later)||p.offset!==offset||p.limit!==16||!uint(p.total)||!Array.isArray(p.rows)||p.refusal!==null)throw new Error('Fold evidence changed its exact setting or sources.');
 const start=BigInt(offset),total=BigInt(p.total),count=total-start<16n?total-start:16n;if(start>total||BigInt(p.rows.length)!==count)throw new Error('Saved fold page is missing or repeated.');let prior=/** @type {any} */(null);
 for(const [n,row] of p.rows.entries()){if(!keys(row,['index','first_day','last_day','observed_days','trades','wins','pessimistic_paisa','refused','decided'])||row.index!==String(start+BigInt(n))||!['index','observed_days','trades','wins','refused'].every(k=>uint(row[k]))||!['first_day','last_day','pessimistic_paisa'].every(k=>sint(row[k]))||typeof row.decided!=='boolean'||BigInt(row.wins)>BigInt(row.trades)||BigInt(row.first_day)>BigInt(row.last_day)||BigInt(row.first_day)<BigInt(selected.later.first_day)||BigInt(row.last_day)>BigInt(selected.later.last_day)||prior&&BigInt(row.first_day)<=BigInt(prior.last_day)||row.trades==='0'&&row.pessimistic_paisa!=='0')throw new Error('A saved fold has inconsistent dates, counts or returns.');prior=row;}
 return {...p,next:start+count<total?String(start+count):null};
}

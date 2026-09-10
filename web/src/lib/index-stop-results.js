import {ask} from './ask.js';
import {detailRefusal} from './detail-refusal.js';
import {createPageRequests} from './page-requests.js';
import {CAMPAIGN_RUNGS} from './boolean-campaign.js';
const U64=(1n<<64n)-1n,I64=(1n<<63n)-1n,MINUTE=60000000n,DAY=86400000000n,IST=19800000000n;
/** @param {bigint} stamp */ const dayOf=stamp=>{const value=stamp+IST;return value>=0n?value/DAY:(value-DAY+1n)/DAY;};
/** @param {bigint} stamp */ const minuteOf=stamp=>(stamp+IST-dayOf(stamp)*DAY)/MINUTE;
/** @param {any} selected */ const signalLength=selected=>BigInt(selected.timeframe.slice(0,-3))*MINUTE;
/** @param {any} v */ const object=v=>v!==null&&typeof v==='object'&&!Array.isArray(v);
/** @param {any} v */ const hex=v=>typeof v==='string'&&v.length===64&&/^[0-9a-f]{64}$/.test(v)&&v!=='0'.repeat(64);
/** @param {any} v */ const uint=v=>typeof v==='string'&&v.length<=20&&v.trim()===v&&/^(0|[1-9]\d*)$/.test(v)&&BigInt(v)<=U64;
/** @param {any} v */ const sint=v=>typeof v==='string'&&v.length<=20&&v.trim()===v&&/^(0|-?[1-9]\d*)$/.test(v)&&BigInt(v)>=-I64-1n&&BigInt(v)<=I64;
/** @param {any} v @param {string[]} fields */ const keys=(v,fields)=>object(v)&&Object.keys(v).length===fields.length&&fields.every(key=>Object.hasOwn(v,key));
/** @param {any} v @param {string[]} fields */ const sum=(v,fields)=>fields.reduce((n,key)=>n+BigInt(v[key]),0n);
/** @param {any} a @param {any} b @returns {boolean} */
function same(a,b){if(a===b)return true;if(Array.isArray(a)&&Array.isArray(b))return a.length===b.length&&a.every((v,n)=>same(v,b[n]));return object(a)&&object(b)&&Object.keys(a).length===Object.keys(b).length&&Object.keys(a).every(k=>Object.hasOwn(b,k)&&same(a[k],b[k]));}
const METRICS=['trades','wins','optimistic_paisa','pessimistic_paisa','drawdown_paisa','worst_trade_paisa','adverse_paisa','favourable_paisa','holding_minutes','unreachable','too_late','gap_invalid','while_open','entry_refused','path_refused','closing_refused','stopped','forced','stop_gaps'];
const MONEY=['optimistic_paisa','pessimistic_paisa','drawdown_paisa','worst_trade_paisa','adverse_paisa','favourable_paisa'];
const REASONS=['priced','immediate_entry_minute_missing','entry_at_or_after_1510','entry_open_at_or_beyond_stop','position_still_occupied','entry_minute_refused_or_duplicated','held_path_missing_refused_or_duplicated','unique_accepted_1509_close_missing'];
const REASON_LABELS=/** @type {Record<string,string>} */ ({priced:'Trade recorded',immediate_entry_minute_missing:'Next entry candle missing',entry_at_or_after_1510:'Entry at or after 15:10',entry_open_at_or_beyond_stop:'Opening price at or beyond the stop',position_still_occupied:'Earlier trade or refused path still occupies this minute',entry_minute_refused_or_duplicated:'Entry candle failed data checks',held_path_missing_refused_or_duplicated:'Missing or invalid candle before exit',unique_accepted_1509_close_missing:'Verified 15:10 close unavailable',signal_candle_stop:'Fixed candle stop',forced_1510_close:'Forced 15:10 close'});
/** @param {string} reason */ export const indexStopReasonLabel=reason=>REASON_LABELS[reason]??'Unrecognized saved reason';
/** Exact index points, not account rupees or a return on assumed capital.
 * @param {any} value */
export function indexStopPoints(value){if(!sint(value))return 'Unavailable';const n=BigInt(value),m=n<0n?-n:n;return `${n<0n?'−':''}${m/100n}.${String(m%100n).padStart(2,'0')} pts`;}
/** @param {any} value */
export function indexStopSelection(value){
 const {identity,completion=null,kind='settings',setting=null,offset='0',limit=16,selected=null}=value??{};
 if(!hex(identity)||completion!==null&&!hex(completion)||!['settings','trades','events','days'].includes(kind)||!uint(offset)||!Number.isInteger(limit)||limit<1||limit>256||
   (kind==='settings'?setting!==null||selected!==null:!uint(setting)||!object(selected)||selected.index!==setting)||
   (kind!=='settings'||offset!=='0')&&completion===null)throw new Error('Choose an exact saved single-stop identity, setting and bounded page.');
 if(selected)validateSetting(selected);
 return {identity,completion,kind,setting,offset,limit,selected};
}
/** @param {any} p */
function validatePolicy(p){
 const fixed={model:'completed_signal_candle_stop',entry:'immediate_next_1min_open',long_stop:'completed_signal_candle_low',short_stop:'completed_signal_candle_high',forced_exit_ist:'15:10',target:false,trail:false,horizon:false,costs_included:false,institutional_admission:'not_assessed_in_this_view'};
 if(!keys(p,['digest',...Object.keys(fixed)])||!hex(p.digest)||!Object.entries(fixed).every(([k,v])=>p[k]===v))throw new Error('The saved execution rule differs from the single candle stop and forced15:10 model.');
}
/** @param {any} m */
function validateMetrics(m){
 if(!keys(m,METRICS)||!METRICS.every(k=>(MONEY.includes(k)?sint:uint)(m[k]))||BigInt(m.wins)>BigInt(m.trades)||sum(m,['stopped','forced'])!==BigInt(m.trades)||BigInt(m.stop_gaps)>BigInt(m.stopped)||BigInt(m.holding_minutes)<BigInt(m.trades)||BigInt(m.pessimistic_paisa)>BigInt(m.optimistic_paisa)||['drawdown_paisa','adverse_paisa','favourable_paisa'].some(k=>BigInt(m[k])<0n)||m.trades==='0'&&MONEY.some(k=>m[k]!=='0'))throw new Error('Single-stop saved counts or gross fill totals do not reconcile.');
}
/** @param {any} s */
export function validateSetting(s){
 if(!keys(s,['index','program_index','run_id','source_id','evaluation_digest','instrument','direction','timeframe','first_day','last_day','expression','truth','metrics','events_count','trades_count','days_count','feed'])||!['index','program_index','events_count','trades_count','days_count'].every(k=>uint(s[k]))||!['run_id','source_id','evaluation_digest'].every(k=>hex(s[k]))||!['NSE-NIFTY','NSE-BANKNIFTY'].includes(s.instrument)||!CAMPAIGN_RUNGS.includes(s.timeframe)||!sint(s.first_day)||!sint(s.last_day)||BigInt(s.first_day)>BigInt(s.last_day)||s.program_index!==String(BigInt(s.index)/2n)||s.direction!==(BigInt(s.index)%2n===0n?'long':'short')||typeof s.expression!=='string'||s.expression.length===0||s.expression.length>16384||s.feed!==null)throw new Error('The single-stop setting lost its program, direction or source identity.');
 if(!keys(s.truth,['evaluated','hits','misses','unknown'])||!Object.values(s.truth).every(uint)||sum(s.truth,['hits','misses','unknown'])!==BigInt(s.truth.evaluated)||s.events_count!==s.truth.hits||s.trades_count!==s.metrics?.trades)throw new Error('Unknown or refused single-stop signals cannot be counted as trades.');
 validateMetrics(s.metrics);
 if(sum(s.metrics,['trades','unreachable','too_late','gap_invalid','while_open','entry_refused','path_refused','closing_refused'])!==BigInt(s.events_count))throw new Error('Single-stop signal dispositions do not cover every definite signal.');
 return s;
}
/** @param {any} r @param {any} selected */
function validateTrade(r,selected){
 const counts=['index','signal_bar','entry_bar','exit_bar','holding_minutes'],money=['stop_paisa','entry_paisa','optimistic_exit_paisa','pessimistic_exit_paisa','optimistic_paisa','pessimistic_paisa','adverse_paisa','favourable_paisa'],time=['signal_micros','signal_close_micros','entry_micros','exit_bar_micros','exit_from_micros','exit_until_micros'];
 if(!keys(r,[...counts,...money,...time,'exit_reason','gapped'])||!counts.every(k=>uint(r[k]))||![...money,...time].every(k=>sint(r[k]))||typeof r.gapped!=='boolean'||!['signal_candle_stop','forced_1510_close'].includes(r.exit_reason))throw new Error('Single-stop trade fields are incomplete or malformed.');
 const n=Object.fromEntries([...counts,...money,...time].map(k=>[k,BigInt(r[k])])),long=selected.direction==='long';
 if(n.signal_micros>=n.entry_micros||n.signal_close_micros!==n.entry_micros||n.entry_micros>n.exit_bar_micros||n.entry_bar>n.exit_bar||n.entry_micros%MINUTE!==0n||n.exit_bar_micros%MINUTE!==0n||n.exit_bar-n.entry_bar+1n!==n.holding_minutes||(n.exit_bar_micros-n.entry_micros)/MINUTE+1n!==n.holding_minutes||money.slice(0,4).some(k=>n[k]<=0n)||n.adverse_paisa<0n||n.favourable_paisa<0n||n.pessimistic_paisa>n.optimistic_paisa||n.entry_paisa===n.stop_paisa||long!==(n.entry_paisa>n.stop_paisa)||n.optimistic_paisa!==(long?n.optimistic_exit_paisa-n.entry_paisa:n.entry_paisa-n.optimistic_exit_paisa)||n.pessimistic_paisa!==(long?n.pessimistic_exit_paisa-n.entry_paisa:n.entry_paisa-n.pessimistic_exit_paisa))throw new Error('Single-stop trade order, direction or exact gross returns changed.');
 const entryDay=dayOf(n.entry_micros),exitDay=dayOf(n.exit_bar_micros),end=n.exit_bar_micros+MINUTE;
 if(entryDay!==exitDay||entryDay<BigInt(selected.first_day)||entryDay>BigInt(selected.last_day)||minuteOf(end)>910n||n.signal_close_micros-n.signal_micros!==signalLength(selected))throw new Error('A single-stop trade crosses its saved intraday or signal boundary.');
 if(r.exit_reason==='forced_1510_close'?(n.exit_from_micros!==end||n.exit_until_micros!==end||minuteOf(end)!==910n||n.optimistic_exit_paisa!==n.pessimistic_exit_paisa||r.gapped):(n.exit_from_micros!==n.exit_bar_micros||n.exit_until_micros!==end||n.optimistic_paisa>=0n))throw new Error('The single-stop exit time is not its recorded bounded stop minute or exact15:10 close.');
}
/** @param {any} r @param {any} selected */
function validateEvent(r,selected){
 if(!keys(r,['index','signal_bar','signal_micros','signal_close_micros','stop_paisa','reason','entry_bar','entry_micros','occupied_through_micros','trade_index'])||!uint(r.index)||!uint(r.signal_bar)||!['signal_micros','signal_close_micros','stop_paisa'].every(k=>sint(r[k]))||!REASONS.includes(r.reason)||!(r.entry_bar===null||uint(r.entry_bar))||!(r.trade_index===null||uint(r.trade_index))||!['entry_micros','occupied_through_micros'].every(k=>r[k]===null||sint(r[k]))||(r.entry_bar===null)!==(r.entry_micros===null)||BigInt(r.signal_micros)>=BigInt(r.signal_close_micros)||BigInt(r.stop_paisa)<=0n||(r.reason==='priced')!==(r.trade_index!==null)||r.entry_micros!==null&&r.entry_micros!==r.signal_close_micros||r.trade_index!==null&&(r.entry_bar===null||r.occupied_through_micros===null))throw new Error('Single-stop signal event or its exact trade link changed.');
 const day=dayOf(BigInt(r.signal_micros));
 if((r.reason==='immediate_entry_minute_missing')!==(r.entry_bar===null)||BigInt(r.signal_close_micros)-BigInt(r.signal_micros)!==signalLength(selected)||day<BigInt(selected.first_day)||day>BigInt(selected.last_day)||r.trade_index!==null&&BigInt(r.trade_index)>=BigInt(selected.trades_count)||r.occupied_through_micros!==null&&BigInt(r.occupied_through_micros)<BigInt(r.signal_close_micros)||['immediate_entry_minute_missing','entry_at_or_after_1510','entry_open_at_or_beyond_stop'].includes(r.reason)&&r.occupied_through_micros!==null)throw new Error('The saved signal changed its source clock, refusal ownership or trade address.');
}
/** @param {any} r @param {any} selected */
function validateDay(r,selected){
 const counts=['index','offered_bars','accepted_bars','unavailable_minutes','trades','wins','refused','signals'];
 if(!keys(r,[...counts,'day','close_verified','optimistic_paisa','pessimistic_paisa'])||!counts.every(k=>uint(r[k]))||!['day','optimistic_paisa','pessimistic_paisa'].every(k=>sint(r[k]))||typeof r.close_verified!=='boolean'||r.offered_bars==='0'||BigInt(r.accepted_bars)>BigInt(r.offered_bars)||BigInt(r.wins)>BigInt(r.trades)||sum(r,['trades','refused'])>BigInt(r.signals)||BigInt(r.pessimistic_paisa)>BigInt(r.optimistic_paisa)||r.trades==='0'&&(r.optimistic_paisa!=='0'||r.pessimistic_paisa!=='0')||BigInt(r.day)<BigInt(selected.first_day)||BigInt(r.day)>BigInt(selected.last_day))throw new Error('Single-stop day counts, gross returns or source range changed.');
}
/** @param {any} selection @param {(url:string)=>Promise<any>} [request] */
export async function fetchIndexStop(selection,request=ask){
 const asked=indexStopSelection(selection),query=new URLSearchParams({identity:asked.identity,kind:asked.kind,offset:asked.offset,limit:String(asked.limit)});if(asked.completion)query.set('completion',asked.completion);if(asked.setting!==null)query.set('setting',asked.setting);
 const response=await request('/index-stop.json?'+query);if(!response?.ok)throw new Error(await detailRefusal(response,'The saved single-stop page is unavailable.'));
 const body=await response.json();
 if(!keys(body,['schema_version','status','model','authority','identity','completion','kind','setting','selected','offset','limit','total','next','rows','refusal','admitted_bytes','observation_byte_limit','policy'])||body.schema_version!==1||body.status!=='saved'||body.model!=='index-stop'||body.authority!=='authenticated-saved-observations'||body.refusal!==null||body.identity!==asked.identity||!hex(body.completion)||asked.completion!==null&&body.completion!==asked.completion||!(/** @type {const} */ (['kind','setting','offset','limit'])).every(k=>body[k]===asked[k])||!uint(body.total)||!uint(body.admitted_bytes)||!uint(body.observation_byte_limit)||BigInt(body.admitted_bytes)>BigInt(body.observation_byte_limit)||!Array.isArray(body.rows))throw new Error('The saved single-stop page does not match its exact receipt and bounded selection.');
 validatePolicy(body.policy);
 const start=BigInt(asked.offset),total=BigInt(body.total),available=total-start,count=available<BigInt(asked.limit)?available:BigInt(asked.limit);
 if(start>total||BigInt(body.rows.length)!==count||body.next!==(start+count<total?String(start+count):null))throw new Error('The single-stop page is truncated, repeated or outside the saved extent.');
 if(asked.kind==='settings') {if(body.selected!==null||total===0n||total%2n!==0n)throw new Error('Saved settings require complete long and short pairs.');}
 else {validateSetting(body.selected);if(!same(body.selected,asked.selected)||body.total!==asked.selected[`${asked.kind==='trades'?'trades':asked.kind==='events'?'events':'days'}_count`])throw new Error('The selected single-stop setting changed its original evidence.');}
 let previous=/** @type {any} */ (null);
 for(const [index,row] of body.rows.entries()) {
  if(row?.index!==String(start+BigInt(index)))throw new Error('Single-stop row order changed.');
  if(asked.kind==='settings') {validateSetting(row);if(previous&&BigInt(row.index)%2n===1n&&(!['program_index','expression','source_id','instrument','timeframe','first_day','last_day','days_count','events_count'].every(k=>row[k]===previous[k])||!same(row.truth,previous.truth)||row.run_id===previous.run_id))throw new Error('Paired long/short settings use different source inputs or expressions.');}
  else if(asked.kind==='trades') {validateTrade(row,body.selected);if(previous&&BigInt(row.entry_micros)<=BigInt(previous.exit_bar_micros))throw new Error('Saved single-stop trades overlap.');}
  else if(asked.kind==='events') {validateEvent(row,body.selected);if(previous&&(BigInt(row.signal_bar)<=BigInt(previous.signal_bar)||BigInt(row.signal_micros)<=BigInt(previous.signal_micros)))throw new Error('Saved signal events repeat or change order.');}
  else {validateDay(row,body.selected);if(previous&&BigInt(row.day)<=BigInt(previous.day))throw new Error('Saved observed days repeat or change order.');}
  previous=row;
 }
 return body;
}
/** @param {(value:any)=>void} publish @param {(url:string,options?:RequestInit)=>Promise<any>} [request] */
export function createIndexStopReader(publish,request=ask){const reads=createPageRequests();let disposed=false;return {
 /** @param {any} selection */async open(selection){if(disposed)return;reads.cancel();let asked;try{asked=structuredClone(indexStopSelection(selection));}catch(why){publish({phase:'failed',body:null,why:String(why),selection:null});return;}
 publish({phase:'loading',body:null,why:'',selection:asked});await reads.run(async ticket=>{try{const body=await fetchIndexStop(asked,url=>request(url,{cache:'no-store',signal:ticket.signal}));if(ticket.current())publish({phase:'ready',body,why:'',selection:asked});}catch(why){if(ticket.current())publish({phase:'failed',body:null,why:String(why),selection:asked});}});},
 close(){reads.cancel();if(!disposed)publish({phase:'idle',body:null,why:'',selection:null});},dispose(){disposed=true;reads.dispose();}
};}

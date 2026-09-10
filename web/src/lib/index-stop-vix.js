import {ask} from './ask.js';
import {detailRefusal} from './detail-refusal.js';
import {createPageRequests} from './page-requests.js';
import {validateSetting} from './index-stop-results.js';

const U64=(1n<<64n)-1n,I64=(1n<<63n)-1n,MINUTE=60000000n,IST=19800000000n;
/** @param {any} v */const object=v=>v!==null&&typeof v==='object'&&!Array.isArray(v);
/** @param {any} v */const hex=v=>typeof v==='string'&&/^[0-9a-f]{64}$/.test(v)&&v!=='0'.repeat(64);
/** @param {any} v */const uint=v=>typeof v==='string'&&v.length<=20&&/^(0|[1-9]\d*)$/.test(v)&&BigInt(v)<=U64;
/** @param {any} v */const sint=v=>typeof v==='string'&&v.length<=20&&/^(0|-?[1-9]\d*)$/.test(v)&&BigInt(v)>=-I64-1n&&BigInt(v)<=I64;
/** @param {any} v @param {string[]} fields */const keys=(v,fields)=>object(v)&&Object.keys(v).length===fields.length&&fields.every(key=>Object.hasOwn(v,key));
/** @param {any} a @param {any} b @returns {boolean} */const same=(a,b)=>a===b||object(a)&&object(b)&&Object.keys(a).length===Object.keys(b).length&&Object.keys(a).every(key=>same(a[key],b[key]));
const FIELDS=['schema_version','status','model','provenance_status','catalog_identity','catalog_completion','reference','selected','feed','reference_symbol','reference_timeframe','reference_only','entry_basis','exit_basis','policy','summary','total','offset','limit','next','rows','admitted_bytes','observation_byte_limit'];
const SUMMARY=['settings','trades','exact_stamps','absent_stamps','unavailable_stamps'];
const TIMES=['entry_micros','exit_bar_micros','exit_from_micros','exit_until_micros'];
const ROW=['trade_index','run_id','original_trade_digest',...TIMES,'month','entry','exit','original_trade'];
const MONTH=['index','year','month','records','snapshot_digest','unavailable_code','unavailable_reason'];
const CANDLE=['micros','open_paisa','high_paisa','low_paisa','close_paisa','volume','open_interest'];

/** One selected saved trade, not a request to find today's VIX.
 * @param {any} value */
export function indexStopVixSelection(value){
 const {identity,completion,selected,trade}=value??{};
 if(!hex(identity)||!hex(completion)||!object(selected)||!object(trade))throw new Error('Select an exact saved result, completion and trade to inspect its original VIX reference.');
 validateSetting(selected);
 if(!uint(trade.index)||BigInt(trade.index)>=BigInt(selected.trades_count)||!TIMES.every(k=>sint(trade[k]))||BigInt(trade.entry_micros)%MINUTE!==0n||BigInt(trade.exit_bar_micros)%MINUTE!==0n||BigInt(trade.exit_bar_micros)<BigInt(trade.entry_micros)||!['signal_candle_stop','forced_1510_close'].includes(trade.exit_reason))throw new Error('The saved trade does not identify an original entry and exit minute.');
 const entry=BigInt(trade.entry_micros),exit=BigInt(trade.exit_bar_micros),from=BigInt(trade.exit_from_micros),until=BigInt(trade.exit_until_micros);
 if((entry+IST)/86400000000n!==(exit+IST)/86400000000n||trade.exit_reason==='signal_candle_stop'&&(from!==exit||until!==exit+MINUTE)||trade.exit_reason==='forced_1510_close'&&(from!==exit+MINUTE||until!==from||(from+IST)%86400000000n!==910n*MINUTE))throw new Error('The saved trade exit interval is inconsistent; an exact stop time will not be inferred.');
 return {identity,completion,selected,trade,offset:trade.index,limit:1};
}

/** @param {any} candle @param {string} micros */
function checkCandle(candle,micros){
 if(!keys(candle,CANDLE)||candle.micros!==micros||!sint(candle.micros)||BigInt(candle.micros)%MINUTE!==0n||!['open_paisa','high_paisa','low_paisa','close_paisa','volume'].every(k=>uint(candle[k])&&BigInt(candle[k])<=I64)||!sint(candle.open_interest))throw new Error('An original VIX OHLCV row is malformed or belongs to another minute.');
 const open=BigInt(candle.open_paisa),high=BigInt(candle.high_paisa),low=BigInt(candle.low_paisa),close=BigInt(candle.close_paisa),oi=BigInt(candle.open_interest);
 if(high<open||high<low||high<close||low>open||low>close||oi<0n&&oi!==-I64-1n)throw new Error('The original VIX candle has inconsistent prices or counts.');
}

/** @param {any} month @param {string} micros */
function checkMonth(month,micros){
 if(!keys(month,MONTH)||!uint(month.index)||!Number.isInteger(month.year)||month.year<1||month.year>65535||!Number.isInteger(month.month)||month.month<1||month.month>12)throw new Error('The original VIX month provenance is malformed.');
 const at=new Date(Number((BigInt(micros)+IST)/1000n));
 if(!Number.isFinite(at.getTime())||at.getUTCFullYear()!==month.year||at.getUTCMonth()+1!==month.month)throw new Error('The saved VIX month does not contain this trade minute.');
 const unavailable=month.unavailable_code!==null;
 if(unavailable){
  if(month.unavailable_code!=='reference_month_refused'||month.records!==null||month.snapshot_digest!==null||typeof month.unavailable_reason!=='string'||month.unavailable_reason.trim().length===0||new TextEncoder().encode(month.unavailable_reason).length>8192)throw new Error('Unavailable VIX evidence lost its original refusal or gained invented measurements.');
 }else if(!uint(month.records)||BigInt(month.records)>44640n||!hex(month.snapshot_digest)||month.unavailable_reason!==null)throw new Error('The validated VIX month is missing its original snapshot receipt.');
 return unavailable;
}

/** @param {any} value @param {string} micros @param {boolean} unavailable @param {any} month */
function checkStamp(value,micros,unavailable,month){
 if(!keys(value,['state','candle'])||!['exact','absent','unavailable'].includes(value.state)||(value.state==='unavailable')!==unavailable)throw new Error('VIX absence and unavailable reference months cannot be substituted for one another.');
 if(value.state==='exact'){
  if(month.records==='0')throw new Error('An empty original VIX month cannot contain an exact candle.');
  checkCandle(value.candle,micros);
 }else if(value.candle!==null)throw new Error('An absent or unavailable VIX observation must not contain a zero or replacement candle.');
}

/** @param {any} selection @param {(url:string)=>Promise<any>} [request] */
export async function fetchIndexStopVix(selection,request=ask){
 const asked=structuredClone(indexStopVixSelection(selection)),query=new URLSearchParams({identity:asked.identity,pin:asked.completion,setting:asked.selected.index,offset:asked.offset,limit:String(asked.limit)});
 const response=await request('/index-stop-vix.json?'+query);
 if(!response?.ok)throw new Error(await detailRefusal(response,'The saved VIX companion is unavailable. Original trade and candle evidence remains separate.'));
 const body=await response.json();
 if(!keys(body,FIELDS)||body.schema_version!==1||body.status!=='saved'||body.model!=='index-stop-vix'||body.provenance_status!=='saved_reference_snapshot'||body.catalog_identity!==asked.identity||body.catalog_completion!==asked.completion||!same(body.selected,asked.selected)||!keys(body.reference,['identity','publication_id','completion'])||!Object.values(body.reference).every(hex)||typeof body.feed!=='string'||! /^[a-z][a-z0-9_-]{0,63}$/.test(body.feed)||body.reference_symbol!=='NSE-INDIAVIX'||body.reference_timeframe!=='1min'||body.reference_only!==true||body.entry_basis!=='entry_minute'||body.exit_basis!=='exit_minute_candle'||typeof body.policy!=='string'||body.policy.length===0||body.policy.length>4096||!keys(body.summary,SUMMARY)||!SUMMARY.every(k=>uint(body.summary[k]))||!uint(body.total)||body.total!==asked.selected.trades_count||body.offset!==asked.offset||body.limit!==1||!Array.isArray(body.rows)||body.rows.length!==1||!uint(body.admitted_bytes)||!uint(body.observation_byte_limit)||body.admitted_bytes==='0'||BigInt(body.admitted_bytes)>BigInt(body.observation_byte_limit))throw new Error('Saved VIX evidence changed its original catalog, trade selection, reference policy or bounds.');
 const next=BigInt(asked.offset)+1n,expectedNext=next<BigInt(body.total)?String(next):null;
 if(body.next!==expectedNext||BigInt(body.summary.settings)<=BigInt(asked.selected.index)||BigInt(body.summary.trades)<BigInt(body.total)||BigInt(body.summary.exact_stamps)+BigInt(body.summary.absent_stamps)+BigInt(body.summary.unavailable_stamps)!==2n*BigInt(body.summary.trades))throw new Error('Saved VIX counts or continuation do not match the admitted original catalog.');
 const row=body.rows[0];
 if(!keys(row,ROW)||row.trade_index!==asked.trade.index||row.run_id!==asked.selected.run_id||!hex(row.original_trade_digest)||!same(row.original_trade,asked.trade)||!TIMES.every(k=>row[k]===asked.trade[k]))throw new Error('This VIX annotation belongs to a different saved trade or exit interval.');
 const unavailable=checkMonth(row.month,row.entry_micros);checkMonth(row.month,row.exit_bar_micros);
 checkStamp(row.entry,row.entry_micros,unavailable,row.month);checkStamp(row.exit,row.exit_bar_micros,unavailable,row.month);
 if(row.entry_micros===row.exit_bar_micros&&!same(row.entry,row.exit))throw new Error('The same original VIX minute cannot have different entry and exit observations.');
 for(const state of ['exact','absent','unavailable'])if(BigInt([row.entry,row.exit].filter(stamp=>stamp.state===state).length)>BigInt(body.summary[`${state}_stamps`]))throw new Error('The selected VIX stamps exceed their saved catalog counts.');
 return body;
}

/** Human labels keep an exact zero, an absent minute and an unreadable month apart.
 * @param {any} body */
export function indexStopVixBoundaries(body){
 const row=body.rows[0];
 return [['Entry minute',row.entry_micros,row.entry],['Exit minute',row.exit_bar_micros,row.exit]].map(([label,micros,stamp])=>({label,micros,...stamp,
  availability:stamp.state==='exact'?'Exact saved candle':stamp.state==='absent'?'Absent at this minute':'Original month unavailable',
  reason:stamp.state==='unavailable'?row.month.unavailable_reason:stamp.state==='absent'?'The validated original month has no candle at this exact minute.':null}));
}

/** One read in flight; selection changes revoke stale publication even if abort is ignored.
 * @param {(state:any)=>void} publish @param {(url:string,options?:RequestInit)=>Promise<any>} [request] */
export function createIndexStopVixReader(publish,request=ask){
 const reads=createPageRequests();let disposed=false;
 return {
  /** @param {any} value */async open(value){if(disposed)return;reads.cancel();let asked;try{asked=structuredClone(indexStopVixSelection(value));}catch(why){publish({phase:'failed',body:null,selection:null,why:String(why)});return;}
   publish({phase:'loading',body:null,selection:asked,why:''});await reads.run(async ticket=>{try{const body=await fetchIndexStopVix(asked,url=>request(url,{method:'GET',cache:'no-store',signal:ticket.signal}));if(ticket.current())publish({phase:'ready',body,selection:asked,why:''});}catch(why){if(ticket.current())publish({phase:'failed',body:null,selection:asked,why:String(why)});}});
  },
  close(){reads.cancel();if(!disposed)publish({phase:'idle',body:null,selection:null,why:''});},
  dispose(){disposed=true;reads.dispose();}
 };
}

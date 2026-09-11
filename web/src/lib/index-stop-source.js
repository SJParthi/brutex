import {ask} from './ask.js';
import {detailRefusal} from './detail-refusal.js';
import {createPageRequests} from './page-requests.js';
import {validateSetting} from './index-stop-results.js';
import {indexStopChartData} from './index-stop-chart.js';

const U64=(1n<<64n)-1n,I64=(1n<<63n)-1n;
/** @param {any} v */const object=v=>v!==null&&typeof v==='object'&&!Array.isArray(v);
/** @param {any} v */const hex=v=>typeof v==='string'&&/^[0-9a-f]{64}$/.test(v)&&v!=='0'.repeat(64);
/** @param {any} v */const uint=v=>typeof v==='string'&&v.length<=20&&/^(0|[1-9]\d*)$/.test(v)&&BigInt(v)<=U64;
/** @param {any} v */const sint=v=>typeof v==='string'&&v.length<=20&&/^(0|-?[1-9]\d*)$/.test(v)&&BigInt(v)>=-I64-1n&&BigInt(v)<=I64;
/** @param {any} v @param {string[]} fields */const keys=(v,fields)=>object(v)&&Object.keys(v).length===fields.length&&fields.every(key=>Object.hasOwn(v,key));
/** @param {any} a @param {any} b @returns {boolean} */const same=(a,b)=>a===b||object(a)&&object(b)&&Object.keys(a).length===Object.keys(b).length&&Object.keys(a).every(key=>same(a[key],b[key]));
const FIELDS=['schema_version','model','provenance_status','kind','catalog_identity','catalog_completion','setting','source_id','run_id','source_context_identity','source_context_completion','feed','instrument','timeframe','expression','original_build_commit','vocabulary_version','condition_names','direction','source_first_day','source_last_day','measurement_first_day','measurement_last_day','trade_index','before','after','first_bar','total_bars','candles','trade','admitted_bytes','observation_byte_limit'];

/** @param {any} value */
export function indexStopSourceSelection(value){
 const {identity,completion,selected,trade=null,before='0',after='0'}=value??{};
 if(!hex(identity)||!hex(completion)||!object(selected)||!uint(before)||!uint(after)||BigInt(before)+BigInt(after)>4095n)throw new Error('Original-source inspection needs an exact saved result, completion and bounded candle context.');
 validateSetting(selected);
 if(trade!==null&&(!object(trade)||!uint(trade.index)||BigInt(trade.index)>=BigInt(selected.trades_count)||!uint(trade.entry_bar)||!uint(trade.exit_bar)||BigInt(trade.exit_bar)<BigInt(trade.entry_bar)))throw new Error('Select one existing saved trade before opening its chart.');
 if(trade===null&&(before!=='0'||after!=='0'))throw new Error('Candle context requires a saved trade.');
 if(trade!==null&&(BigInt(before)>BigInt(trade.entry_bar)||BigInt(trade.exit_bar)-BigInt(trade.entry_bar)+1n+BigInt(before)+BigInt(after)>4096n))throw new Error('The requested candle page exceeds its source start or display limit.');
 return {identity,completion,selected,trade,before,after,kind:trade===null?'source':'candles'};
}

/** Render original names from the authenticated companion only. Tokenization
 * avoids accidentally rewriting numbers or operators contained in a name.
 * @param {string} expression @param {{bit:number,name:string}[]} names */
export function indexStopNamedRule(expression,names){
 const byBit=new Map();let previous=-1;
 if(!Array.isArray(names)||names.length===0||names.length>4096)throw new Error('The original condition-name table is unavailable.');
 for(const row of names){if(!keys(row,['bit','name'])||!Number.isInteger(row.bit)||row.bit<0||row.bit>0xffffffff||row.bit<=previous||typeof row.name!=='string'||!row.name.trim()||row.name.length>256||/[\u0000-\u001f\u007f]/.test(row.name))throw new Error('The original condition-name table is malformed.');previous=row.bit;byBit.set(row.bit,row.name);}
 if(typeof expression!=='string'||expression.length===0||expression.length>16384)throw new Error('The original expression is unavailable.');
 const tokens=expression.match(/\d+|[!&|()]|\s+/g)??[];
 if(tokens.join('')!==expression)throw new Error('The saved expression contains an unknown token.');
 let operand=true,depth=0;
 for(const token of tokens){
  if(/^\s+$/.test(token))continue;
  if(/^\d+$/.test(token)){if(!operand)throw new Error('The saved expression is malformed.');operand=false;}
  else if(token==='!'||token==='('){if(!operand)throw new Error('The saved expression is malformed.');if(token==='(')depth++;}
  else if(token===')'){if(operand||depth===0)throw new Error('The saved expression is malformed.');depth--;}
  else {if(operand)throw new Error('The saved expression is malformed.');operand=true;}
 }
 if(operand||depth!==0)throw new Error('The saved expression is malformed.');
 return tokens.map(token=>{if(/^\d+$/.test(token)){const bit=Number(token),name=byBit.get(bit);if(!Number.isSafeInteger(bit)||!name)throw new Error('A saved condition has no original name. Current names will not be substituted.');return `[${name}]`;}return token==='&'?' AND ':token==='|'?' OR ':token==='!'?'NOT ':token;}).join('');
}

/** @param {any} selection @param {(url:string)=>Promise<any>} [request] */
export async function fetchIndexStopSource(selection,request=ask){
 const asked=indexStopSourceSelection(selection),query=new URLSearchParams({identity:asked.identity,pin:asked.completion,setting:asked.selected.index,kind:asked.kind});
 if(asked.trade!==null){query.set('trade',asked.trade.index);query.set('before',asked.before);query.set('after',asked.after);}
 const response=await request('/index-stop-candles.json?'+query);
 if(!response?.ok)throw new Error(await detailRefusal(response,'The original source context is unavailable. Saved trade tables remain available.'));
 const body=await response.json(),selected=asked.selected;
 if(!keys(body,FIELDS)||body.schema_version!==1||body.model!=='index-stop-candles'||body.provenance_status!=='verified_original_source'||body.kind!==asked.kind||body.catalog_identity!==asked.identity||body.catalog_completion!==asked.completion||body.setting!==selected.index||body.source_id!==selected.source_id||body.run_id!==selected.run_id||body.instrument!==selected.instrument||body.timeframe!==selected.timeframe||body.direction!==selected.direction||body.expression!==selected.expression||!hex(body.source_context_identity)||!hex(body.source_context_completion)||typeof body.feed!=='string'||! /^[a-z][a-z0-9_-]{0,63}$/.test(body.feed)||typeof body.original_build_commit!=='string'||! /^[0-9a-f]{40}$/.test(body.original_build_commit)||body.original_build_commit==='0'.repeat(40)||!Number.isInteger(body.vocabulary_version)||body.vocabulary_version<1||body.vocabulary_version>0xffffffff||!['source_first_day','source_last_day','measurement_first_day','measurement_last_day'].every(k=>sint(body[k]))||body.measurement_first_day!==selected.first_day||body.measurement_last_day!==selected.last_day||BigInt(body.source_first_day)>BigInt(body.measurement_first_day)||BigInt(body.source_last_day)<BigInt(body.measurement_last_day)||!uint(body.total_bars)||body.total_bars==='0'||!uint(body.admitted_bytes)||!uint(body.observation_byte_limit)||BigInt(body.admitted_bytes)>BigInt(body.observation_byte_limit)||!Array.isArray(body.candles))throw new Error('Original-source evidence changed its saved identity, metadata or bounds.');
 const namedRule=indexStopNamedRule(body.expression,body.condition_names);
 if(asked.kind==='source'){
  if(body.trade!==null||body.trade_index!==null||body.before!==null||body.after!==null||body.first_bar!==null||body.candles.length!==0)throw new Error('The metadata response contains unrequested trade or candle evidence.');
 }else{
  const count=BigInt(asked.trade.exit_bar)-BigInt(asked.trade.entry_bar)+1n+BigInt(asked.before)+BigInt(asked.after),first=BigInt(asked.trade.entry_bar)-BigInt(asked.before);
  if(body.trade_index!==asked.trade.index||body.before!==asked.before||body.after!==asked.after||!same(body.trade,asked.trade)||body.first_bar!==String(first)||BigInt(body.candles.length)!==count||first+count>BigInt(body.total_bars))throw new Error('The original candle window is incomplete or belongs to another saved trade.');
  for(const row of body.candles){if(!keys(row,['micros','open_paisa','high_paisa','low_paisa','close_paisa','volume','open_interest'])||!uint(row.volume)||BigInt(row.volume)>I64||!sint(row.open_interest))throw new Error('An original OHLCV row is malformed.');}
  const display=indexStopChartData(body.candles,body.trade,body.direction),entry=body.candles[Number(asked.before)],exit=body.candles[Number(count-BigInt(asked.after)-1n)];
  if(entry.micros!==body.trade.entry_micros||entry.open_paisa!==body.trade.entry_paisa||exit.micros!==body.trade.exit_bar_micros||display.markers.length!==2)throw new Error('The saved entry or exit does not match the returned original candles.');
  for(let n=Number(asked.before)+1;n<Number(count-BigInt(asked.after));n++)if(BigInt(body.candles[n].micros)-BigInt(body.candles[n-1].micros)!==60000000n)throw new Error('A held trade minute is missing from the authenticated window.');
 }
 return {...body,namedRule};
}

/** No request in this reader can start computation; stale responses are revoked.
 * @param {(state:any)=>void} publish @param {(url:string,options?:RequestInit)=>Promise<any>} [request] */
export function createIndexStopSourceReader(publish,request=ask){
 const reads=createPageRequests();let disposed=false;
 return {
  /** @param {any} value */async open(value){if(disposed)return;reads.cancel();let asked;try{asked=structuredClone(indexStopSourceSelection(value));}catch(why){publish({phase:'failed',body:null,selection:null,why:String(why)});return;}
   publish({phase:'loading',body:null,selection:asked,why:''});await reads.run(async ticket=>{try{const body=await fetchIndexStopSource(asked,url=>request(url,{method:'GET',cache:'no-store',signal:ticket.signal}));if(ticket.current())publish({phase:'ready',body,selection:asked,why:''});}catch(why){if(ticket.current())publish({phase:'failed',body:null,selection:asked,why:String(why)});}});
  },
  close(){reads.cancel();if(!disposed)publish({phase:'idle',body:null,selection:null,why:''});},
  dispose(){disposed=true;reads.dispose();}
 };
}

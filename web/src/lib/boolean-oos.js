import { detailRefusal } from './detail-refusal.js';
import { catalogCoordinate, gridSummary } from './boolean-catalog.js';
const U64=18446744073709551615n, I64=9223372036854775807n;
/** @param {any} v */
const uint=v=>typeof v==='string'&&/^(0|[1-9][0-9]*)$/.test(v)&&BigInt(v)<=U64;
/** @param {any} v */
const sint=v=>typeof v==='string'&&/^(0|-?[1-9][0-9]*)$/.test(v)&&BigInt(v)>=-I64-1n&&BigInt(v)<=I64;
/** @param {any} v */
const hex=v=>typeof v==='string'&&/^[0-9a-f]{64}$/.test(v);
/** @param {any} v */
const object=v=>v!==null&&typeof v==='object'&&!Array.isArray(v);
/** @param {any} v @param {string[]} keys @param {(value:any)=>boolean} valid */
const fields=(v,keys,valid)=>object(v)&&keys.every(key=>valid(v[key]));
/** @param {string} micros */
const day=micros=>{const shifted=BigInt(micros)+19800000000n;return (shifted<0n?shifted-86399999999n:shifted)/86400000000n;};
/** @param {string} micros */
const clock=micros=>((BigInt(micros)+19800000000n)%86400000000n+86400000000n)%86400000000n;
/** @param {any} selection */
export function laterSelection(selection){
 const {identity,completion=null,kind='coordinates',candidate=null,offset='0',limit=32}=selection;
 if(!hex(identity)||(completion!==null&&!hex(completion))||!['coordinates','trades','sessions'].includes(kind)||
  !uint(offset)||!Number.isInteger(limit)||limit<1||limit>256||((kind!=='coordinates')!==(candidate!==null))||
  (candidate!==null&&!uint(candidate))||((kind!=='coordinates'||offset!=='0')&&completion===null))throw new Error('Choose an exact later comparison and its pinned coordinate page.');
 return {identity,completion,kind,candidate,offset,limit};
}
/** @param {any} body @param {any} row */
function pair(body,row){
 if(!object(row)||!uint(row.index)||!catalogCoordinate({...body,session_count:body.training_session_count},row.training)||!catalogCoordinate(body,row.later)||row.training.index!==row.index||row.later.index!==row.index)return false;
 return ['program_index','expression','side','ordinal'].every(key=>row.training[key]===row.later[key])&&row.training.run!==row.later.run&&
  ['stop','target','tsl','ttp'].every(key=>JSON.stringify(row.training.cell[key])===JSON.stringify(row.later.cell[key]))&&JSON.stringify(row.training.levels)===JSON.stringify(row.later.levels);
}
/** @param {any} selection @param {(url:string)=>Promise<any>} request */
export async function fetchBooleanLater(selection,request){
 const asked=laterSelection(selection);const query=new URLSearchParams({identity:asked.identity,kind:asked.kind,offset:asked.offset,limit:String(asked.limit)});
 if(asked.completion!==null)query.set('completion',asked.completion);if(asked.candidate!==null)query.set('candidate',asked.candidate);
 const response=await request('/boolean-oos.json?'+query);
 if(!response?.ok)throw new Error(await detailRefusal(response,'Later comparison unavailable (HTTP '+String(response?.status)+').'));
 const body=await response.json();
 if(!object(body)||body.schema_version!==1||body.status!=='saved'||body.authority!=='authenticated-later-comparison-observation'||body.identity!==asked.identity||!hex(body.completion)||(asked.completion!==null&&body.completion!==asked.completion)||
  !fields(body.parent,['identity','completion'],hex)||!fields(body,['cohort','membership_digest'],hex)||typeof body.instrument!=='string'||!body.instrument||typeof body.cash!=='boolean'||typeof body.scope!=='string'||body.refusal!==null||
  !fields(body,['program_count','coordinate_count','training_session_count','session_count','admitted_bytes','total'],uint)||['program_count','coordinate_count','training_session_count','session_count'].some(key=>body[key]==='0')||
  !Array.isArray(body.grids)||body.grids.length!==2||!body.grids.every(gridSummary)||body.grids[0].side!=='long'||body.grids[1].side!=='short'||
  !fields(body.later,['source','execution'],hex)||!fields(body.later,['first_micros','last_micros'],sint)||!uint(body.later.bars)||body.later.bars==='0'||
  !['from','to'].every(key=>typeof body.later[key]==='string'&&/^\d{4}-(0[1-9]|1[0-2])$/.test(body.later[key]))||body.later.from>body.later.to||
  !['kind','candidate','offset','limit'].every(key=>body[key]===asked[/** @type {keyof typeof asked} */(key)])||body.page_complete!==true||!Array.isArray(body.rows))throw new Error('Later comparison identity, original link or bounded page contract changed.');
 const first=BigInt(body.later.first_micros),last=BigInt(body.later.last_micros);
 if(first>last||first%60000000n!==0n||last%60000000n!==0n||body.grids.some((/** @type {any} */ grid)=>day(body.later.first_micros)<=day(grid.last_micros))||
  BigInt(body.coordinate_count)!==BigInt(body.program_count)*(BigInt(body.grids[0].cells)+BigInt(body.grids[1].cells)))throw new Error('Later dates overlap training or the complete coordinate populations differ.');
 if(asked.candidate===null?body.selected!==null:!pair(body,body.selected)||body.selected.index!==asked.candidate)throw new Error('Later detail belongs to another original coordinate.');
 const start=BigInt(asked.offset),total=BigInt(body.total),count=total-start<BigInt(asked.limit)?total-start:BigInt(asked.limit),end=start+BigInt(body.rows.length);
 const expectedTotal=asked.kind==='coordinates'?body.coordinate_count:asked.kind==='sessions'?body.session_count:body.selected.later.cell.trades;
 if(start>total||String(total)!==expectedTotal||count!==BigInt(body.rows.length)||body.next!==(end<total?String(end):null))throw new Error('Later page skips, repeats, truncates or ends before its recorded total.');
 for(const [index,row] of body.rows.entries()){
  if(!object(row)||row.index!==String(start+BigInt(index)))throw new Error('Later row order differs.');
  let valid=false;
  if(asked.kind==='coordinates')valid=pair(body,row);
  if(asked.kind==='sessions')valid=fields(row,['day','return_paisa'],sint)&&fields(row,['trades','wins'],uint)&&BigInt(row.wins)<=BigInt(row.trades)&&BigInt(row.day)>=day(body.later.first_micros)&&BigInt(row.day)<=day(body.later.last_micros)&&(index===0||BigInt(body.rows[index-1].day)<BigInt(row.day));
  if(asked.kind==='trades')valid=fields(row,['signal_bar','entry_bar','exit_bar','adverse_ppm','favourable_ppm'],uint)&&fields(row,['best','worst','entry_micros','exit_micros','adverse_paisa','favourable_paisa'],sint)&&
   BigInt(row.entry_bar)<=BigInt(row.exit_bar)&&BigInt(row.exit_bar)<BigInt(body.later.bars)&&BigInt(row.entry_micros)>=first&&BigInt(row.exit_micros)<=last&&BigInt(row.entry_micros)<=BigInt(row.exit_micros)&&BigInt(row.worst)<=BigInt(row.best)&&
   BigInt(row.entry_micros)%60000000n===0n&&BigInt(row.exit_micros)%60000000n===0n&&day(row.entry_micros)===day(row.exit_micros)&&clock(row.exit_micros)<=54540000000n;
  if(!valid)throw new Error('Later facts or frozen original-coordinate linkage are invalid.');
 }
 return body;
}

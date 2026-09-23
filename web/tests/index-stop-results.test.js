import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {parse} from 'svelte/compiler';
import {fetchIndexStop,indexStopSelection,validateSetting,createIndexStopReader,indexStopReasonLabel,indexStopPoints} from '../src/lib/index-stop-results.js';

const hex=(/** @type {number} */ n)=>n.toString(16).padStart(2,'0').repeat(32);
const ID=hex(1),PIN=hex(2),MINUTE=60000000n,DAY=86400000000n,IST=19800000000n;
const reply=(/** @type {any} */ body)=>async()=>({ok:true,json:async()=>body});
const tick=()=>new Promise((/** @type {any} */ resolve)=>setTimeout(resolve,3));
/** @param {number} day @param {number} minute */const stamp=(day,minute)=>String(BigInt(day)*DAY+BigInt(minute)*MINUTE-IST);
/** Generated wire fixtures describe mechanics only, never market performance. */
function policy(){return {digest:hex(3),model:'completed_signal_candle_stop',entry:'immediate_next_1min_open',long_stop:'completed_signal_candle_low',short_stop:'completed_signal_candle_high',forced_exit_ist:'15:10',target:false,trail:false,horizon:false,costs_included:false,institutional_admission:'not_assessed_in_this_view'};}
/** @param {number} [index] */
function setting(index=0){return {index:String(index),program_index:String(Math.floor(index/2)),run_id:hex(index+4),source_id:hex(8),evaluation_digest:hex(index+9),instrument:'NSE-NIFTY',direction:index%2===0?'long':'short',timeframe:'1min',first_day:'0',last_day:'1',expression:'0 | !0',truth:{evaluated:'12',hits:'3',misses:'5',unknown:'4'},metrics:{trades:'1',wins:'0',optimistic_paisa:'-100',pessimistic_paisa:'-150',drawdown_paisa:'150',worst_trade_paisa:'-150',adverse_paisa:'150',favourable_paisa:'50',holding_minutes:'2',unreachable:'1',too_late:'0',gap_invalid:'0',while_open:'1',entry_refused:'0',path_refused:'0',closing_refused:'0',stopped:'1',forced:'0',stop_gaps:'0'},events_count:'3',trades_count:'1',days_count:'2',feed:null};}
/** @param {any} [selected] @param {number} [day] */
function trade(selected=setting(),day=0){const long=selected.direction==='long';return {index:'0',signal_bar:'0',entry_bar:'1',exit_bar:'2',holding_minutes:'2',signal_micros:stamp(day,555),signal_close_micros:stamp(day,556),entry_micros:stamp(day,556),exit_bar_micros:stamp(day,557),exit_from_micros:stamp(day,557),exit_until_micros:stamp(day,558),stop_paisa:long?'9900':'10100',entry_paisa:'10000',optimistic_exit_paisa:long?'9900':'10100',pessimistic_exit_paisa:long?'9850':'10150',optimistic_paisa:'-100',pessimistic_paisa:'-150',adverse_paisa:'150',favourable_paisa:'50',exit_reason:'signal_candle_stop',gapped:false};}
function events(){return [
 {index:'0',signal_bar:'0',signal_micros:stamp(0,555),signal_close_micros:stamp(0,556),stop_paisa:'9900',reason:'priced',entry_bar:'1',entry_micros:stamp(0,556),occupied_through_micros:stamp(0,557),trade_index:'0'},
 {index:'1',signal_bar:'1',signal_micros:stamp(0,556),signal_close_micros:stamp(0,557),stop_paisa:'9900',reason:'position_still_occupied',entry_bar:'2',entry_micros:stamp(0,557),occupied_through_micros:stamp(0,557),trade_index:null},
 {index:'2',signal_bar:'2',signal_micros:stamp(1,929),signal_close_micros:stamp(1,930),stop_paisa:'9900',reason:'immediate_entry_minute_missing',entry_bar:null,entry_micros:null,occupied_through_micros:null,trade_index:null}
];}
function days(){return [0,1].map(day=>({index:String(day),day:String(day),offered_bars:'375',accepted_bars:'375',unavailable_minutes:'0',close_verified:true,trades:day===0?'1':'0',wins:'0',refused:'0',signals:day===0?'2':'0',optimistic_paisa:day===0?'-100':'0',pessimistic_paisa:day===0?'-150':'0'}));}
/** @param {any} [select] @returns {any} */
function page(select={identity:ID}){
 const a=indexStopSelection(select),all=a.kind==='settings'?[setting(),setting(1)]:a.kind==='trades'?[trade(a.selected)]:a.kind==='events'?events():days(),offset=Number(a.offset),rows=all.slice(offset,offset+a.limit);
 return {schema_version:1,status:'saved',model:'index-stop',authority:'authenticated-saved-observations',identity:a.identity,completion:a.completion??PIN,kind:a.kind,setting:a.setting,selected:a.selected,offset:a.offset,limit:a.limit,total:String(all.length),next:offset+rows.length<all.length?String(offset+rows.length):null,rows,refusal:null,admitted_bytes:'12345',observation_byte_limit:'16000',policy:policy()};
}
/** @param {string} kind @param {any} [selected] */
const detail=(kind,selected=setting())=>({identity:ID,completion:PIN,kind,setting:selected.index,selected});
/** @returns {{promise:Promise<any>,resolve:(value:any)=>void,reject:(error:any)=>void}} */
function deferred(){let resolve=/** @type {(v:any)=>void} */(()=>{}),reject=/** @type {(v:any)=>void} */(()=>{});const promise=new Promise((yes,no)=>{resolve=yes;reject=no;});return {promise,resolve,reject};}

test('initial settings and pinned continuations preserve both directions, exact expressions and both fills',async()=>{
 const urls=/** @type {string[]} */([]),ask={identity:ID,limit:1};
 const first=await fetchIndexStop(ask,async url=>{urls.push(url);return {ok:true,json:async()=>page(ask)};});
 assert.equal(first.rows[0].direction,'long');assert.equal(first.next,'1');assert.equal(first.rows[0].expression,'0 | !0');
 const second={identity:ID,completion:first.completion,offset:first.next,limit:1};const later=await fetchIndexStop(second,async url=>{urls.push(url);return {ok:true,json:async()=>page(second)};});
 assert.equal(later.rows[0].direction,'short');assert.equal(later.next,null);assert.equal(later.total,'2');
 const url=new URL(urls[1],'http://fixture');assert.equal(url.pathname,'/index-stop.json');assert.equal(url.searchParams.get('completion'),PIN);assert.equal(url.searchParams.get('offset'),'1');assert.equal(url.searchParams.get('limit'),'1');
 for(const side of [setting(),setting(1)]){const a=detail('trades',side),got=await fetchIndexStop(a,reply(page(a)));assert.equal(got.rows[0].pessimistic_paisa,'-150');assert.equal(got.rows[0].optimistic_paisa,'-100');assert.equal(got.selected.direction,side.direction);assert.equal(got.selected.feed,null);}
});

test('definite unpriced signals, unknown truth and actually observed zero-trade days remain separate',async()=>{
 const a=detail('events'),got=await fetchIndexStop(a,reply(page(a)));assert.equal(got.rows.length,3);assert.equal(got.selected.truth.unknown,'4');assert.equal(got.rows[2].entry_bar,null);assert.equal(got.rows[2].trade_index,null);assert.equal(got.rows[1].reason,'position_still_occupied');
 const d=detail('days'),read=await fetchIndexStop(d,reply(page(d)));assert.equal(read.rows[1].trades,'0');assert.equal(read.rows[1].pessimistic_paisa,'0');assert.equal(read.rows[1].day,'1');
 const empty=setting();empty.truth={evaluated:'18446744073709551615',hits:'0',misses:'0',unknown:'18446744073709551615'};for(const key of Object.keys(empty.metrics))empty.metrics[/** @type {keyof typeof empty.metrics} */(key)]='0';empty.events_count='0';empty.trades_count='0';
 assert.equal(validateSetting(empty).truth.unknown,'18446744073709551615');const ask=detail('trades',empty),body=page(ask);body.total='0';body.rows=[];assert.equal((await fetchIndexStop(ask,reply(body))).rows.length,0);
});

test('human labels preserve data refusals, occupied intervals and the two actual exit rules',()=>{
 assert.equal(indexStopReasonLabel('priced'),'Trade recorded');assert.equal(indexStopReasonLabel('signal_candle_stop'),'Fixed candle stop');assert.equal(indexStopReasonLabel('forced_1510_close'),'Forced 15:10 close');
 assert.match(indexStopReasonLabel('entry_minute_refused_or_duplicated'),/failed data checks/);assert.match(indexStopReasonLabel('held_path_missing_refused_or_duplicated'),/Missing or invalid/);assert.match(indexStopReasonLabel('position_still_occupied'),/refused path/);assert.equal(indexStopReasonLabel('target'),'Unrecognized saved reason');
});

test('malformed or unpinned selectors issue zero requests',async()=>{
 let requests=0;for(const select of [{identity:'x'},{identity:ID,offset:'1'},{identity:ID,completion:PIN,limit:257},{identity:ID,completion:PIN,limit:0},{identity:ID,offset:'01'},{identity:ID,offset:'18446744073709551616'}, {...detail('trades'),completion:null},{...detail('days'),selected:null},{...detail('events'),setting:'1'},{identity:ID,kind:'grid'}])await assert.rejects(fetchIndexStop(select,async()=>{requests++;return {ok:true,json:async()=>null};}));
 assert.equal(requests,0);
});

test('wrong identity, receipt, policy, bounds, truncation or pair ownership cannot pass as saved evidence',async()=>{
 const changes=[(/** @type {any} */ b)=>b.identity=hex(99),(/** @type {any} */ b)=>b.completion=hex(99),(/** @type {any} */ b)=>b.policy.target=true,(/** @type {any} */ b)=>b.policy.trail=true,(/** @type {any} */ b)=>b.policy.costs_included=true,(/** @type {any} */ b)=>b.policy.institutional_admission='admitted',(/** @type {any} */ b)=>b.total='3',(/** @type {any} */ b)=>b.limit='16',(/** @type {any} */ b)=>b.admitted_bytes='16001',(/** @type {any} */ b)=>b.rows.pop(),(/** @type {any} */ b)=>b.rows[0].index='1',(/** @type {any} */ b)=>b.rows[1].expression='1',(/** @type {any} */ b)=>b.rows[1].run_id=b.rows[0].run_id,(/** @type {any} */ b)=>b.rows[1].truth.unknown='3',(/** @type {any} */ b)=>b.rows[0].metrics.trades='2',(/** @type {any} */ b)=>b.rows[0].feed='zerodha',(/** @type {any} */ b)=>b.rows[0].metrics.pessimistic_paisa='-9223372036854775809',(/** @type {any} */ b)=>b.refusal='corrupt',(/** @type {any} */ b)=>b.extra=true];
 for(const change of changes){const ask={identity:ID,completion:PIN},body=page(ask);change(body);await assert.rejects(fetchIndexStop(ask,reply(body)),String(change));}
 const d=detail('days'),body=page(d);body.selected={...body.selected,run_id:hex(99)};await assert.rejects(fetchIndexStop(d,reply(body)),/changed/);
});

test('intraday trade decoding refuses changed amounts, overlapping bars and fabricated exact stop ticks',async()=>{
 for(const change of [(/** @type {any} */ r)=>r.pessimistic_paisa='-151',(/** @type {any} */ r)=>r.optimistic_exit_paisa='10001',(/** @type {any} */ r)=>r.stop_paisa='10100',(/** @type {any} */ r)=>r.exit_until_micros=r.exit_from_micros,(/** @type {any} */ r)=>r.entry_bar='3',(/** @type {any} */ r)=>r.holding_minutes='1',(/** @type {any} */ r)=>r.signal_micros=stamp(0,554),(/** @type {any} */ r)=>r.entry_micros=stamp(0,557),(/** @type {any} */ r)=>r.exit_bar_micros=stamp(1,557),(/** @type {any} */ r)=>r.exit_reason='target']){const a=detail('trades'),body=page(a);change(body.rows[0]);await assert.rejects(fetchIndexStop(a,reply(body)),String(change));}
 const selected=setting();selected.trades_count='2';selected.events_count='4';selected.truth.hits='4';selected.truth.evaluated='13';selected.metrics.trades='2';selected.metrics.stopped='2';const a=detail('trades',selected),body=page(a);body.total='2';body.rows.push({...body.rows[0],index:'1'});await assert.rejects(fetchIndexStop(a,reply(body)),/overlap/);
});

test('forced exit is exactly15:10 and signed civil dates use floor division',async()=>{
 for(const day of [0,-1]){const selected=setting();selected.first_day=String(day);Object.assign(selected.metrics,{stopped:'0',forced:'1',wins:'1',holding_minutes:'354',optimistic_paisa:'100',pessimistic_paisa:'100',drawdown_paisa:'0',worst_trade_paisa:'100'});const a=detail('trades',selected),body=page(a),row=trade(selected,day);Object.assign(row,{exit_bar:'354',holding_minutes:'354',exit_bar_micros:stamp(day,909),exit_from_micros:stamp(day,910),exit_until_micros:stamp(day,910),exit_reason:'forced_1510_close',optimistic_exit_paisa:'10100',pessimistic_exit_paisa:'10100',optimistic_paisa:'100',pessimistic_paisa:'100'});body.rows=[row];assert.equal((await fetchIndexStop(a,reply(body))).rows[0].exit_until_micros,stamp(day,910));row.gapped=true;await assert.rejects(fetchIndexStop(a,reply(body)));row.gapped=false;row.exit_until_micros=stamp(day,911);await assert.rejects(fetchIndexStop(a,reply(body)));}
});

test('event trade links, no-entry reasons, holds and daily accounting cannot manufacture records',async()=>{
 for(const change of [(/** @type {any} */ r)=>r.trade_index='1',(/** @type {any} */ r)=>r.entry_bar=null,(/** @type {any} */ r)=>r.occupied_through_micros=stamp(0,555),(/** @type {any} */ r)=>r.reason='immediate_entry_minute_missing',(/** @type {any} */ r)=>r.signal_close_micros=stamp(0,558)]){const a=detail('events'),body=page(a);change(body.rows[0]);await assert.rejects(fetchIndexStop(a,reply(body)),String(change));}
 for(const change of [(/** @type {any} */ r)=>r.offered_bars='0',(/** @type {any} */ r)=>r.accepted_bars='376',(/** @type {any} */ r)=>r.trades='0',(/** @type {any} */ r)=>r.wins='2',(/** @type {any} */ r)=>r.signals='0',(/** @type {any} */ r)=>r.day='-1']){const a=detail('days'),body=page(a);change(body.rows[0]);await assert.rejects(fetchIndexStop(a,reply(body)),String(change));}
});

test('HTTP and JSON failures remain failures without automatic retry or invented empty pages',async()=>{
 let calls=0;await assert.rejects(fetchIndexStop({identity:ID},async()=>{calls++;return {ok:false,status:503,json:async()=>({schema_version:1,status:'refused',rows:[],refusal:'receipt replaced'})};}),/receipt replaced/);assert.equal(calls,1);
 await assert.rejects(fetchIndexStop({identity:ID},async()=>({ok:true,json:async()=>{throw new Error('invalid JSON');}})),/invalid JSON/);
});

test('rapid A to B to A keeps one active request and only the latest pending selection',async()=>{
 const states=/** @type {any[]} */([]),calls=/** @type {any[]} */([]),ctl=createIndexStopReader(s=>states.push(s),async(url,init)=>{const pending=deferred();calls.push({url,init,pending});return pending.promise;});
 const a={identity:ID},b={identity:hex(99)};const running=ctl.open(a);await tick();await ctl.open(b);await ctl.open(a);assert.equal(calls.length,1);assert.equal(calls[0].init.signal.aborted,true);assert.equal(states.at(-1).body,null);
 calls[0].pending.resolve({ok:true,json:async()=>page(a)});await running;await tick();assert.equal(calls.length,2);assert.ok(!states.some(s=>s.phase==='ready'));assert.match(calls[1].url,new RegExp(ID));calls[1].pending.resolve({ok:true,json:async()=>page(a)});await tick();assert.equal(states.at(-1).phase,'ready');
 const last=ctl.open(a);await tick();ctl.dispose();const count=states.length;calls[2].pending.reject(new Error('late'));await last;assert.equal(states.length,count);await ctl.open(b);assert.equal(calls.length,3);
});

test('closing during slow JSON decode revokes stale publication and invalid selection clears older evidence',async()=>{
 const states=/** @type {any[]} */([]),json=deferred(),ctl=createIndexStopReader(s=>states.push(s),async()=>({ok:true,json:()=>json.promise}));
 const running=ctl.open({identity:ID});await tick();ctl.close();const count=states.length;json.resolve(page());await running;assert.equal(states.length,count);assert.equal(states.at(-1).phase,'idle');await ctl.open({identity:'invalid'});assert.equal(states.at(-1).phase,'failed');assert.equal(states.at(-1).body,null);
});

test('the comparison component offers paged trade, signal and day evidence without implying ranking or approval',()=>{
 const source=readFileSync(new URL('../src/lib/IndexStopResults.svelte',import.meta.url),'utf8');assert.doesNotThrow(()=>parse(source));
 for(const text of ['Each program has a long and short evaluation','two readings of those same trades','These counts do not describe the full search population','There is no target, trailing stop or holding-horizon alternative','Actually observed days; missing dates are not created','no approval or ranking is inferred here','gross index points per underlying unit'])assert.ok(source.includes(text),text);
 assert.match(source,/onDestroy\(\(\)=>reader\.dispose\(\)\)/);assert.match(source,/completion:body\.completion/);assert.match(source,/\['trades','Trades'\],\['events','Every signal'\],\['days','Observed days'\]/);assert.match(source,/indexStopPoints\(trade\.pessimistic_paisa\)/);assert.match(source,/indexStopPoints\(trade\.optimistic_paisa\)/);assert.match(source,/\.scroll\{overflow:auto/);
});

test('index points keep exact signed integer precision without account currency or capital assumptions',()=>{
 assert.equal(indexStopPoints('-9223372036854775808'),'−92233720368547758.08 pts');assert.equal(indexStopPoints('9007199254740993'),'90071992547409.93 pts');assert.equal(indexStopPoints('0'),'0.00 pts');for(const bad of [null,0,'01','-0','9223372036854775808'])assert.equal(indexStopPoints(bad),'Unavailable');
});

// Generated protocol fixtures only. They do not describe market performance.
import test from 'node:test';
import assert from 'node:assert/strict';
import {fetchIndexStopSource,indexStopNamedRule,createIndexStopSourceReader} from '../src/lib/index-stop-source.js';
const hex=(/** @type {number} */ n)=>n.toString(16).padStart(2,'0').repeat(32),ID=hex(1),PIN=hex(2),start=20000n*86400000000n+555n*60000000n-19800000000n;
const metrics={trades:'1',wins:'0',optimistic_paisa:'-100',pessimistic_paisa:'-150',drawdown_paisa:'150',worst_trade_paisa:'-150',adverse_paisa:'150',favourable_paisa:'50',holding_minutes:'2',unreachable:'0',too_late:'0',gap_invalid:'0',while_open:'0',entry_refused:'0',path_refused:'0',closing_refused:'0',stopped:'1',forced:'0',stop_gaps:'0'};
const selected={index:'0',program_index:'0',run_id:hex(3),source_id:hex(4),evaluation_digest:hex(5),instrument:'NSE-NIFTY',direction:'long',timeframe:'1min',first_day:'20000',last_day:'20001',expression:'0 | !0',truth:{evaluated:'4',hits:'1',misses:'2',unknown:'1'},metrics,events_count:'1',trades_count:'1',days_count:'2',feed:null};
const trade={index:'0',signal_bar:'0',entry_bar:'1',exit_bar:'2',holding_minutes:'2',signal_micros:String(start),signal_close_micros:String(start+60000000n),entry_micros:String(start+60000000n),exit_bar_micros:String(start+120000000n),exit_from_micros:String(start+120000000n),exit_until_micros:String(start+180000000n),stop_paisa:'9900',entry_paisa:'10000',optimistic_exit_paisa:'9900',pessimistic_exit_paisa:'9850',optimistic_paisa:'-100',pessimistic_paisa:'-150',adverse_paisa:'150',favourable_paisa:'50',exit_reason:'signal_candle_stop',gapped:false};
const request=(chart=false)=>({identity:ID,completion:PIN,selected,...(chart?{trade,before:'1',after:'0'}:{})});
function body(chart=false){return {schema_version:1,model:'index-stop-candles',provenance_status:'verified_original_source',kind:chart?'candles':'source',catalog_identity:ID,catalog_completion:PIN,setting:'0',source_id:selected.source_id,run_id:selected.run_id,source_context_identity:hex(6),source_context_completion:hex(7),feed:'fixture-feed',instrument:selected.instrument,timeframe:selected.timeframe,expression:selected.expression,original_build_commit:'1'.repeat(40),vocabulary_version:3,condition_names:[{bit:0,name:'Original RSI(14) > 60'}],direction:'long',source_first_day:'19999',source_last_day:'20001',measurement_first_day:selected.first_day,measurement_last_day:selected.last_day,trade_index:chart?'0':null,before:chart?'1':null,after:chart?'0':null,first_bar:chart?'0':null,total_bars:'3',candles:chart?[0,1,2].map(n=>({micros:String(start+60000000n*BigInt(n)),open_paisa:'10000',high_paisa:'10100',low_paisa:n===2?'9850':'9950',close_paisa:'10000',volume:'0',open_interest:'-9223372036854775808'})):[],trade:chart?{...trade}:null,admitted_bytes:'65536',observation_byte_limit:'131072'};}
const reply=(/** @type {unknown} */ value)=>async()=>({ok:true,json:async()=>value});
const tick=()=>new Promise(resolve=>setTimeout(resolve,3));

test('original names and source metadata remain pinned independently of current names',async()=>{
 const urls=/** @type {string[]} */([]);
 const result=await fetchIndexStopSource(request(),async url=>{urls.push(url);return {ok:true,json:async()=>body()};});
 assert.equal(result.feed,'fixture-feed');assert.equal(result.original_build_commit,'1'.repeat(40));assert.match(result.namedRule,/Original RSI\(14\) > 60/);
 assert.match(urls[0],/^\/index-stop-candles\.json\?/);assert.match(urls[0],/kind=source/);assert.ok(!urls[0].includes('trade='));
 assert.deepEqual(result.candles,[]);assert.equal(result.trade,null);
 const chart=await fetchIndexStopSource(request(true),reply(body(true)));assert.equal(chart.candles.length,3);assert.deepEqual(chart.trade,trade);
});

test('names are replaced as tokens without rewriting digits or operators inside names',()=>{
 assert.equal(indexStopNamedRule('(0 & !1)',[{bit:0,name:'RSI 14 | 60'},{bit:1,name:'EMA 20'}]),'([RSI 14 | 60]  AND  NOT [EMA 20])');
 for(const names of [[],[{bit:1,name:'Other'}],[{bit:0,name:'x'},{bit:0,name:'y'}],[{bit:0,name:'\ninvalid'}]])assert.throws(()=>indexStopNamedRule('0',names));
 assert.throws(()=>indexStopNamedRule('0 + 0',[{bit:0,name:'Original'}]),/unknown token/);
 for(const malformed of ['0|','(0','0)','()','!','0 0','0(0)','0!0','|0'])assert.throws(()=>indexStopNamedRule(malformed,[{bit:0,name:'Original'}]),/malformed/);
});

test('foreign source, build shape, direction, names, range or unrequested candles refuse',async()=>{
 const changes=/** @type {Array<(b:any)=>void>} */([b=>b.catalog_completion=hex(99),b=>b.catalog_identity=hex(99),b=>b.source_id=hex(99),b=>b.run_id=hex(99),b=>b.setting='1',b=>b.direction='short',b=>b.timeframe='2min',b=>b.expression='1',b=>b.original_build_commit='current',b=>b.vocabulary_version=0,b=>b.condition_names=[],b=>b.measurement_first_day='19999',b=>b.source_first_day='20001',b=>b.admitted_bytes='131073',b=>b.candles=[{}],b=>b.trade={},b=>b.extra=true]);
 for(const change of changes){const value=body();change(value);await assert.rejects(fetchIndexStopSource(request(),reply(value)),String(change));}
});

test('a chart requires the complete exact saved trade window and entry price',async()=>{
 const changes=/** @type {Array<(b:any)=>void>} */([b=>b.candles.pop(),b=>b.candles[1].open_paisa='10001',b=>b.candles[1].micros=b.candles[0].micros,b=>b.candles[2].micros=String(start+180000000n),b=>b.trade.pessimistic_paisa='-151',b=>b.trade_index='1',b=>b.first_bar='1',b=>b.before='0',b=>b.total_bars='2',b=>b.candles[1].volume='-1']);
 for(const change of changes){const value=body(true);change(value);await assert.rejects(fetchIndexStopSource(request(true),reply(value)),String(change));}
});

test('unpinned and oversized requests issue no read; legacy refusal does not fetch current candles',async()=>{
 let calls=0;
 for(const value of [{...request(),completion:null},{...request(true),before:'2'},{...request(true),after:'4096'},{...request(),after:'1'},{...request(true),trade:{...trade,index:'1'}}])await assert.rejects(fetchIndexStopSource(value,async()=>{calls++;return {}; }));
 assert.equal(calls,0);
 await assert.rejects(fetchIndexStopSource(request(),async()=>{calls++;return {ok:false,status:503,json:async()=>({schema_version:1,status:'refused',rows:[],refusal:'original_context_unavailable'})};}),/original_context_unavailable/);
 assert.equal(calls,1);
});

test('native volume keeps the nonnegative signed-integer boundary',async()=>{
 const maximum=body(true);maximum.candles[0].volume=String((1n<<63n)-1n);
 const result=await fetchIndexStopSource(request(true),reply(maximum));
 assert.equal(result.candles[0].volume,maximum.candles[0].volume);
 const overflow=body(true);overflow.candles[0].volume=String(1n<<63n);
 await assert.rejects(fetchIndexStopSource(request(true),reply(overflow)),/OHLCV row is malformed/);
});

test('an aborted source selection cannot publish late candles or resend a launch',async()=>{
 const states=/** @type {any[]} */([]),calls=/** @type {any[]} */([]);let resolve=/** @type {(value:any)=>void} */(()=>{throw new Error('Pending fixture resolver was not installed');});
 const pending=new Promise(yes=>{resolve=yes;});
 const reader=createIndexStopSourceReader(state=>states.push(state),async(url,options)=>{calls.push({url,options});return pending;});
 const active=reader.open(request(true));await tick();reader.close();
 assert.equal(calls[0].options.method,'GET');assert.equal(calls[0].options.signal.aborted,true);
 const length=states.length;resolve({ok:true,json:async()=>body(true)});await active;
 assert.equal(states.length,length);assert.equal(states.at(-1).phase,'idle');reader.dispose();await reader.open(request());assert.equal(calls.length,1);
});

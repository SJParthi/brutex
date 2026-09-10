import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {parse} from 'svelte/compiler';
import {fetchIndexStopVix,indexStopVixBoundaries,indexStopVixSelection,createIndexStopVixReader} from '../src/lib/index-stop-vix.js';
import {vixHex,vixBody,vixSelection,vixTrade,vixCandle,VIX_ID,VIX_PIN} from './index-stop-vix-fixture.js';
const reply=(/** @type {unknown} */ body)=>async()=>({ok:true,json:async()=>body});
const tick=()=>new Promise(resolve=>setTimeout(resolve,5));

test('VIX inspection pins one original catalog and one selected saved trade',async()=>{
 const urls=/** @type {string[]} */([]),asked=vixSelection();
 const body=await fetchIndexStopVix(asked,async url=>{urls.push(url);return {ok:true,json:async()=>vixBody()};});
 assert.equal(urls.length,1);const url=new URL(urls[0],'http://fixture.invalid');
 assert.equal(url.pathname,'/index-stop-vix.json');assert.deepEqual(Object.fromEntries(url.searchParams),{identity:VIX_ID,pin:VIX_PIN,setting:'0',offset:'0',limit:'1'});
 assert.deepEqual(body.rows[0].original_trade,vixTrade);assert.equal(body.reference_only,true);
 assert.deepEqual(indexStopVixBoundaries(body).map(row=>[row.label,row.availability,row.micros]),[['Entry minute','Exact saved candle',vixTrade.entry_micros],['Exit minute','Exact saved candle',vixTrade.exit_bar_micros]]);
});

test('an exact zero candle stays measured while an absent minute stays null',async()=>{
 const body=vixBody();
 for(const key of /** @type {const} */(['open_paisa','high_paisa','low_paisa','close_paisa','volume']))body.rows[0].entry.candle[key]='0';
 Object.assign(body.rows[0].exit,{state:'absent',candle:null});body.summary.exact_stamps='1';body.summary.absent_stamps='1';
 const rows=indexStopVixBoundaries(await fetchIndexStopVix(vixSelection(),reply(body)));
 assert.equal(rows[0].availability,'Exact saved candle');assert.equal(rows[0].candle.close_paisa,'0');
 assert.equal(rows[1].availability,'Absent at this minute');assert.equal(rows[1].candle,null);assert.match(rows[1].reason,/no candle at this exact minute/);
});

test('a saved unavailable month retains its typed original refusal with no invented candles',async()=>{
 const body=vixBody();
 Object.assign(body.rows[0].month,{records:null,snapshot_digest:null,unavailable_code:'reference_month_refused',unavailable_reason:'Original fixture month could not be opened: checksum mismatch'});
 Object.assign(body.rows[0].entry,{state:'unavailable',candle:null});Object.assign(body.rows[0].exit,{state:'unavailable',candle:null});
 body.summary.exact_stamps='0';body.summary.unavailable_stamps='2';
 const rows=indexStopVixBoundaries(await fetchIndexStopVix(vixSelection(),reply(body)));
 assert.ok(rows.every(row=>row.availability==='Original month unavailable'&&row.candle===null));assert.match(rows[0].reason,/checksum mismatch/);
 for(const alter of /** @type {Array<(b:any)=>void>} */([b=>b.rows[0].entry.state='absent',b=>b.rows[0].month.records='0',b=>b.rows[0].month.snapshot_digest=vixHex(10),b=>b.rows[0].month.unavailable_reason='',b=>b.rows[0].month.unavailable_reason='x'.repeat(8193),b=>b.rows[0].month.unavailable_code='unknown'])){
  const bad=structuredClone(body);alter(bad);await assert.rejects(fetchIndexStopVix(vixSelection(),reply(bad)),String(alter));
 }
});

test('reference identity, native setting, trade bytes and exit intervals cannot drift',async()=>{
 const changes=/** @type {Array<(b:any)=>void>} */([b=>b.catalog_identity=vixHex(30),b=>b.catalog_completion=vixHex(30),b=>b.reference.completion=null,b=>b.reference.identity='0'.repeat(64),b=>b.reference.publication_id='bad',b=>b.selected.source_id=vixHex(30),b=>b.selected.evaluation_digest=vixHex(30),b=>b.selected.direction='short',b=>b.rows[0].run_id=vixHex(30),b=>b.rows[0].trade_index='1',b=>b.rows[0].original_trade_digest='bad',b=>b.rows[0].original_trade.pessimistic_paisa='-151',b=>b.rows[0].exit_from_micros=b.rows[0].exit_until_micros,b=>b.rows[0].entry_micros=b.rows[0].exit_bar_micros,b=>b.rows[0].extra=true]);
 for(const alter of changes){const bad=vixBody();alter(bad);await assert.rejects(fetchIndexStopVix(vixSelection(),reply(bad)),String(alter));}
});

test('only full original one-minute VIX reference evidence is accepted',async()=>{
 const changes=/** @type {Array<(b:any)=>void>} */([b=>b.reference_symbol='NSE-NIFTY',b=>b.reference_timeframe='5min',b=>b.reference_only=false,b=>b.entry_basis='instantaneous',b=>b.exit_basis='exact_stop_tick',b=>b.provenance_status='live',b=>b.model='backtest',b=>b.feed='',b=>b.policy='',b=>b.policy='x'.repeat(4097),b=>b.rows[0].entry.candle.micros=b.rows[0].exit_bar_micros,b=>b.rows[0].month.year++,b=>b.rows[0].month.month=13,b=>b.rows[0].month.records='44641',b=>b.rows[0].month.records='0',b=>b.rows[0].month.snapshot_digest=null,b=>b.rows[0].month.unavailable_reason='unbound reason',b=>b.rows[0].entry.state='absent',b=>b.rows[0].exit.state='unavailable',b=>b.rows[0].entry.candle.high_paisa='1400',b=>b.rows[0].entry.candle.low_paisa='1600',b=>b.rows[0].entry.candle.open_paisa='-1']);
 for(const alter of changes){const bad=vixBody();alter(bad);await assert.rejects(fetchIndexStopVix(vixSelection(),reply(bad)),String(alter));}
});

test('native signed count and open-interest sentinel boundaries remain exact',async()=>{
 const body=vixBody(),limit=String((1n<<63n)-1n);body.rows[0].entry.candle.volume=limit;body.rows[0].entry.candle.open_interest=limit;
 const value=await fetchIndexStopVix(vixSelection(),reply(body));assert.equal(value.rows[0].entry.candle.volume,limit);
 for(const [field,value] of /** @type {Array<['volume'|'open_interest',string]>} */([['volume','-1'],['volume',String(1n<<63n)],['open_interest','-1'],['open_interest',String(-(1n<<63n)-1n)]])){
  const bad=vixBody();bad.rows[0].entry.candle[field]=value;await assert.rejects(fetchIndexStopVix(vixSelection(),reply(bad)));
 }
});

test('page extent and global stamp counters cannot conceal missing or duplicated evidence',async()=>{
 const changes=/** @type {Array<(b:any)=>void>} */([b=>b.rows=[],b=>b.rows.push(structuredClone(b.rows[0])),b=>b.total='0',b=>b.offset='1',b=>b.limit=2,b=>b.next='1',b=>b.summary.settings='0',b=>b.summary.trades='0',b=>b.summary.exact_stamps='1',b=>{b.summary.exact_stamps='0';b.summary.unavailable_stamps='2';},b=>b.admitted_bytes='131073',b=>b.observation_byte_limit='0',b=>b.extra=true]);
 for(const alter of changes){const bad=vixBody();alter(bad);await assert.rejects(fetchIndexStopVix(vixSelection(),reply(bad)),String(alter));}
});

test('the forced 15:10 close is paired with its 15:09 candle, never a fabricated exact-stop candle',async()=>{
 const asked=vixSelection(),close=(20000n*86400000000n+910n*60000000n-19800000000n);
 Object.assign(asked.trade,{exit_bar_micros:String(close-60000000n),exit_from_micros:String(close),exit_until_micros:String(close),exit_reason:'forced_1510_close'});
 const body=vixBody();Object.assign(body.rows[0],{exit_bar_micros:asked.trade.exit_bar_micros,exit_from_micros:asked.trade.exit_from_micros,exit_until_micros:asked.trade.exit_until_micros,original_trade:structuredClone(asked.trade)});body.rows[0].exit.candle=vixCandle(asked.trade.exit_bar_micros);
 const returned=await fetchIndexStopVix(asked,reply(body));assert.equal(BigInt(returned.rows[0].exit.candle.micros)+60000000n,close);
 const bad=structuredClone(body);bad.rows[0].exit.candle.micros=String(close);await assert.rejects(fetchIndexStopVix(asked,reply(bad)),/another minute/);
 const malformed=structuredClone(asked);malformed.trade.exit_until_micros=String(close+1n);assert.throws(()=>indexStopVixSelection(malformed),/exit interval/);
});

test('same entry and exit minute cannot produce two different original reference candles',async()=>{
 const asked=vixSelection();Object.assign(asked.trade,{exit_bar_micros:asked.trade.entry_micros,exit_from_micros:asked.trade.entry_micros,exit_until_micros:String(BigInt(asked.trade.entry_micros)+60000000n)});
 const body=vixBody();Object.assign(body.rows[0],{exit_bar_micros:asked.trade.exit_bar_micros,exit_from_micros:asked.trade.exit_from_micros,exit_until_micros:asked.trade.exit_until_micros,original_trade:structuredClone(asked.trade)});body.rows[0].exit.candle=vixCandle(asked.trade.entry_micros);
 await fetchIndexStopVix(asked,reply(body));body.rows[0].exit.candle.close_paisa='1531';
 await assert.rejects(fetchIndexStopVix(asked,reply(body)),/same original VIX minute/);
});

test('invalid local selection, legacy absence and busy refusal never trigger a current-data fallback',async()=>{
 let calls=0;
 for(const value of [{...vixSelection(),completion:null},{...vixSelection(),trade:null},{...vixSelection(),identity:'bad'},{...vixSelection(),trade:{...vixTrade,index:'1'}},{...vixSelection(),trade:{...vixTrade,entry_micros:'00'}}])await assert.rejects(fetchIndexStopVix(value,async()=>{calls++;return {}; }));
 assert.equal(calls,0);
 for(const status of [404,429,503])await assert.rejects(fetchIndexStopVix(vixSelection(),async url=>{calls++;assert.match(url,/^\/index-stop-vix\.json\?/);return {ok:false,status,json:async()=>({schema_version:1,status:'refused',rows:[],refusal:'saved VIX reference companion unavailable; no current-data substitution'})};}),/no current-data substitution/);
 assert.equal(calls,3);
});

test('a copied selection cannot be retargeted while the original reference request is pending',async()=>{
 const asked=vixSelection();let settle=/** @type {(v:any)=>void} */(()=>{throw Error('no pending request');});
 const pending=new Promise(resolve=>{settle=resolve;});
 const result=fetchIndexStopVix(asked,()=>pending);asked.selected.source_id=vixHex(50);asked.trade.exit_from_micros='0';
 settle({ok:true,json:async()=>vixBody()});assert.equal((await result).selected.source_id,vixBody().selected.source_id);
});

test('rapid A to B to A selection keeps one read active and rejects every stale response',async()=>{
 const states=/** @type {any[]} */([]),calls=/** @type {any[]} */([]);let settle=/** @type {(v:any)=>void} */(()=>{throw Error('no pending request');});
 const pending=new Promise(resolve=>{settle=resolve;});
 const reader=createIndexStopVixReader(state=>states.push(state),async(url,options)=>{calls.push({url,options});return calls.length===1?pending:{ok:true,json:async()=>vixBody()};});
 const first=reader.open(vixSelection());await tick();
 const b=vixSelection();b.identity=vixHex(55);await reader.open(b);await reader.open(vixSelection());
 assert.equal(calls.length,1);assert.equal(calls[0].options.signal.aborted,true);
 settle({ok:true,json:async()=>vixBody()});await first;await tick();
 assert.equal(calls.length,2);assert.ok(calls.every(call=>call.options.method==='GET'&&call.options.cache==='no-store'&&new URL(call.url,'http://fixture.invalid').searchParams.get('identity')===VIX_ID));
 assert.equal(states.filter(state=>state.phase==='ready').length,1);assert.equal(states.at(-1).body.catalog_identity,VIX_ID);
 reader.dispose();await reader.open(vixSelection());assert.equal(calls.length,2);
});

test('leaving the panel aborts its read and a late transport result cannot republish it',async()=>{
 const states=/** @type {any[]} */([]);let settle=/** @type {(v:any)=>void} */(()=>{throw Error('no pending request');}),signal=/** @type {AbortSignal|undefined} */(undefined);
 const pending=new Promise(resolve=>{settle=resolve;});
 const reader=createIndexStopVixReader(value=>states.push(value),async(_url,options)=>{signal=/** @type {AbortSignal} */(options?.signal);return pending;});
 const active=reader.open(vixSelection());await tick();reader.close();const length=states.length;
 assert.equal(signal?.aborted,true);settle({ok:true,json:async()=>vixBody()});await active;assert.equal(states.length,length);assert.equal(states.at(-1).phase,'idle');reader.dispose();
});

test('the Backtest trade selection mounts independent original-candle and VIX panels',()=>{
 const result=readFileSync(new URL('../src/lib/IndexStopResults.svelte',import.meta.url),'utf8'),vix=readFileSync(new URL('../src/lib/IndexStopVix.svelte',import.meta.url),'utf8');
 assert.doesNotThrow(()=>parse(result));assert.doesNotThrow(()=>parse(vix));
 assert.match(result,/<IndexStopSource identity=\{body.identity\} completion=\{body.completion\} selected=\{row\} trade=\{chartTrade\}/);
 assert.match(result,/<IndexStopVix identity=\{body.identity\} completion=\{body.completion\} selected=\{row\} trade=\{chartTrade\}/);
 assert.match(result,/>Show candles and VIX<\/button>/);assert.match(vix,/if\(chosen&&savedTrade\)void reader.open/);
 assert.match(vix,/full candle becomes known after its minute completes/);assert.match(vix,/exact intraminute stop time and instantaneous VIX are unknown/);
 assert.doesNotMatch(vix,/engine\/command|method:\s*['"]POST|setInterval/);
});

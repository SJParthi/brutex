import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {parse} from 'svelte/compiler';
import {fetchIndexStopRanking,createIndexStopRankingReader,indexStopRankingDetail} from '../src/lib/index-stop-ranking.js';
import {page,SEARCH,PIN} from './index-stop-ranking-fixture.js';
import {deferred,tick} from './index-stop-fixture.js';
import {hex} from './index-consistency-fixture.js';
const reply=(/** @type {any} */ value)=>async()=>({ok:true,json:async()=>value});
const input=(filter='qualified')=>({identity:SEARCH,timeframe:'1min',filter});

test('global batch ranking and qualified-only view retain all outcome counts and both direction/fill evidence',async()=>{
 let url='';const filtered=await fetchIndexStopRanking(input(),async request=>{url=request;return {ok:true,json:async()=>page()};});
 assert.match(url,/^\/index-stop-ranking.json\?/);assert.equal(filtered.total,'2');assert.equal(filtered.all_total,'4');assert.equal(filtered.summary.refused,'1');assert.equal(filtered.rows[0].batch,'1');assert.equal(filtered.rows[1].batch,'0');assert.equal(filtered.rows[0].later.direction,'short');assert.equal(filtered.rows[0].later.metrics.optimistic_paisa,'1400');assert.equal(filtered.next,null);
 const all=await fetchIndexStopRanking(input('all'),reply(page('all')));assert.deepEqual(all.rows.map((/** @type {any} */ r)=>[r.batch,r.index]),[['1','1'],['0','0'],['0','1'],['1','0']]);assert.equal(all.rows[2].institutional.status,'rejected');assert.equal(all.rows[3].institutional.status,'refused');
});

test('continuation and filter changes stay on exact checkpoint rather than mixing a new pending batch',async()=>{
 const first=await fetchIndexStopRanking({...input('all'),limit:1},reply(page('all','0',1)));assert.equal(first.next,'1');
 let url='';const next=await fetchIndexStopRanking({...input('all'),offset:'1',limit:1,checkpoint:first.checkpoint},async request=>{url=request;return {ok:true,json:async()=>page('all','1',1)};});assert.equal(next.rows[0].batch,'0');assert.match(url,/sequence=5/);assert.match(url,new RegExp(`completion=${PIN}`));
 const pending=page();pending.checkpoint={sequence:'6',completion:hex(902)};pending.scope.pending_next_batch=true;const got=await fetchIndexStopRanking(input(),reply(pending));assert.equal(got.scope.completed_batches,'2');assert.equal(got.rows[0].batch,'1');assert.equal(got.all_total,'4');
 await assert.rejects(fetchIndexStopRanking({...input(),checkpoint:first.checkpoint},reply(pending)),/checkpoint/);
});

test('ranked drilldown translates local rank beyond page one into exact pinned canonical setting',()=>{
 const r=page().rows[0];r.index='480';r.qualification_rank='33';r.qualification_total='504';const detail=indexStopRankingDetail(r);assert.equal(detail.offset,'32');assert.equal(detail.setting,'480');assert.deepEqual({identity:detail.identity,completion:detail.completion},r.qualification);
 for(const mutate of [(/** @type {any} */ r)=>r.qualification_rank='0',(/** @type {any} */ r)=>r.qualification_rank='505',(/** @type {any} */ r)=>r.qualification.completion='0'.repeat(64),(/** @type {any} */ r)=>r.index='504']){const changed=structuredClone(r);mutate(changed);assert.throws(()=>indexStopRankingDetail(changed));}
});

test('foreign pins, altered scope, duplicate rows, invalid filters and page-local reordering cannot appear as cumulative results',async()=>{
 const mutations=[(/** @type {any} */ b)=>b.identity=hex(999),(/** @type {any} */ b)=>b.timeframe='2min',(/** @type {any} */ b)=>b.checkpoint.sequence='0',(/** @type {any} */ b)=>b.rows.pop(),(/** @type {any} */ b)=>b.scope.completed_programs='3',(/** @type {any} */ b)=>b.scope.completed_batches='1',(/** @type {any} */ b)=>b.scope.last_batch='2',(/** @type {any} */ b)=>b.scope.included_families='0',(/** @type {any} */ b)=>b.scope.training_last_day='12',(/** @type {any} */ b)=>b.scope.exhausted=b.scope.pending_next_batch=true,(/** @type {any} */ b)=>b.rows[0].qualification.completion=hex(555),(/** @type {any} */ b)=>b.rows[0].index_consistency.combined_qualifies=false,(/** @type {any} */ b)=>b.rows[1].original_source.completion=hex(555),(/** @type {any} */ b)=>b.rows[0].qualification_rank='0',(/** @type {any} */ b)=>b.rows[0].institutional.source_index='8',(/** @type {any} */ b)=>b.rows[0].later.first_day='12',(/** @type {any} */ b)=>b.rows[0].later.direction='long',(/** @type {any} */ b)=>b.summary.combined_qualified='3',(/** @type {any} */ b)=>b.all_total='2',(/** @type {any} */ b)=>b.total='5',(/** @type {any} */ b)=>b.read_work.cold='page_local_sort',(/** @type {any} */ b)=>b.rows[1].global_rank='1'];
 for(const mutate of mutations){const body=page('all');mutate(body);await assert.rejects(fetchIndexStopRanking(input('all'),reply(body)),String(mutate));}
 const swapped=page('all');[swapped.rows[1],swapped.rows[2]]=[swapped.rows[2],swapped.rows[1]];swapped.rows.forEach((/** @type {any} */ r,/** @type {number} */ index)=>r.rank=r.global_rank=String(index+1));await assert.rejects(fetchIndexStopRanking(input('all'),reply(swapped)),/ordering/);
 let calls=0;for(const select of [{identity:SEARCH,timeframe:'1day'},{...input(),offset:'1'},{...input(),checkpoint:{sequence:'1',completion:'bad'}},{...input(),limit:257},{...input(),filter:'winner'},{...input(),offset:'01'}])await assert.rejects(fetchIndexStopRanking(select,async()=>{calls++;return null;}));assert.equal(calls,0);
});

test('empty acknowledged work is visible without fabricated zero-progress or qualified evidence',async()=>{
 const empty=page();Object.assign(empty.scope,{completed_batches:'1',completed_programs:'0',completed_work:'12',included_families:'0',first_batch:null,last_batch:null});Object.assign(empty,{total:'0',all_total:'0',rows:[]});Object.keys(empty.summary).forEach(k=>empty.summary[k]='0');const got=await fetchIndexStopRanking(input(),reply(empty));assert.equal(got.scope.completed_work,'12');assert.equal(got.scope.completed_batches,'1');assert.equal(got.rows.length,0);
});

test('failed refresh keeps prior verified scope and never relabels it with a requested new batch count',async()=>{
 let next=/** @type {any} */({ok:true,json:async()=>page()}),states=/** @type {any[]} */([]);const reader=createIndexStopRankingReader(value=>states.push(value),async()=>next);
 await reader.open(input());const old=states.at(-1).body;next={ok:false,status:503,json:async()=>({schema_version:1,status:'refused',rows:[],refusal:'cumulative read budget exceeded'})};await reader.open(input());assert.equal(states.at(-1).phase,'failed');assert.equal(states.at(-1).body,old);assert.equal(states.at(-1).body.scope.completed_batches,'2');assert.match(states.at(-1).why,/budget/);
 const backward=page();backward.checkpoint.sequence='4';next={ok:true,json:async()=>backward};await reader.open(input());assert.equal(states.at(-1).phase,'failed');assert.equal(states.at(-1).body,old);assert.match(states.at(-1).why,/backwards/);
 reader.dispose();
});

test('rapid A to B to A, stale response after cancellation and unmount keep one active GET and newest ownership',async()=>{
 const calls=/** @type {any[]} */([]),states=/** @type {any[]} */([]),reader=createIndexStopRankingReader(s=>states.push(s),async(url,options)=>{const d=deferred();calls.push({url,options,d});return d.promise;});
 const first=reader.open(input());await tick();await reader.open({...input(),timeframe:'2min'});await reader.open(input());assert.equal(calls.length,1);assert.equal(calls[0].options.signal.aborted,true);calls[0].d.resolve({ok:true,json:async()=>page()});await first;await tick();assert.equal(calls.length,2);assert.ok(!states.some(s=>s.phase==='ready'));calls[1].d.resolve({ok:true,json:async()=>page()});await tick();assert.equal(states.at(-1).phase,'ready');
 const third=reader.open(input());await tick();reader.dispose();const count=states.length;calls[2].d.resolve({ok:true,json:async()=>page()});await third;assert.equal(states.length,count);assert.ok(calls.every(c=>c.options.method===undefined));
});

test('comparison component keeps explicit cumulative scope, qualified/all outcomes and exact details without any start action',()=>{
 const file=readFileSync(new URL('../src/lib/IndexStopRanking.svelte',import.meta.url),'utf8');parse(file);
 for(const copy of ['Best saved results across batches','Passed both checks','All outcomes, including failures','pending and is excluded','The declared search is not exhausted','strongest 0.1%','Previous results','Details and trades','initialOffset={selected.offset}','initialSetting={selected.setting}','initialCompletion={selected.completion}','previous verified table','Storage latency and total work are not O(1)'])assert.ok(file.includes(copy),copy);
 assert.doesNotMatch(file,/engine\/command|POST|Run sweep|setInterval|₹/);
 const helper=readFileSync(new URL('../src/lib/index-stop-ranking.js',import.meta.url),'utf8');assert.doesNotMatch(helper,/engine\/command|method:\s*['"]POST|setInterval/);
});

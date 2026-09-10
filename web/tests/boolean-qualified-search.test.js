import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {fetchQualifiedSearch,qualifiedSearchSelection} from '../src/lib/boolean-qualified-search.js';
import {createCampaignMonitor} from '../src/lib/campaign-monitor.js';
import {cash as cashAssessment, legacy as legacyAssessment, sync as syncAssessment} from './index-consistency-fixture.js';
const id='12'.repeat(32),pin='34'.repeat(32),child='56'.repeat(32),cpin='78'.repeat(32),rungs=['1min','2min','3min','5min','10min','15min','30min','60min'];
/** @returns {any} */
function overview(){return {projection_version:2,planned_campaign:child,planned_campaign_authority:'derived-plan-identity',schema_version:1,status:'saved',model:'qualified-search',kind:'overview',authority:'acknowledged-search-history',identity:id,pin,sequence:'9007199254740993',batch:'0',phase:'complete',state:'paused',reason:null,owner_active:false,completed_batches:'1',exhausted:false,grammar_work:'9007199254740993',programs:'2',node_only:false,alpha_ppm:'50000',admitted_bytes:'4096',observation_byte_limit:'67108864',replay_node_limit:'18446744073709551615',replay_nodes_charged:'9007199254740993',history_checked:true,child_bodies_checked:false,refusal:null,scope:'Recorded scope only',rows:rungs.map((rung,i)=>({rung,rung_index:String(i),qualification:{identity:child,completion:cpin},count:'1',counts:{admitted:'0',rejected:'0',unmeasured:'0',refused:'1'},projection_digest:id,allocation_digest:pin}))};}
/** @param {any} n */
function floating(n){const b=new DataView(new ArrayBuffer(8));b.setFloat64(0,n,true);return {bits:String(b.getBigUint64(0,true)),decimal:String(n)};}
/** Independent finite wire projection fixture, not generated market evidence. @returns {any} */
function detail(){
 const t={statistic:floating(1),probability:floating(.5),exact_probability:{numerator:'1',denominator:'2'},matched_or_exceeded:'1',draws:'2',strategies:'1',periods:'4'};
 const summary={scope:id,cohort:id,layout:id,families:'1',candidates:'1',periods:'4',segments:'2',periods_per_segment:'2',splits:'2',contributing_splits:'1',bottom_half_splits:'0',procedure:{draws:'2',seed:'0',block_length:'1'},bounds:Object.fromEntries(['families','candidates','observations','bootstrap_work','split_work','additional_working_bytes','saved_body_bytes'].map(k=>[k,'1000'])),test_families:{white:id,spa:id,romano:id},white:t,spa:structuredClone(t),romano_availability:'unavailable-constant-return-candidate'};
 const statistics={index:'0',identity:id,run:pin,source:{index:'0',coordinates:'1',identity:id,completion:pin,membership_digest:id,instrument:'NSE-RELIANCE',cash:true},coordinate:'0',program_index:'0',side:'long',ordinal:'0',execution_refusal_bits:'0',trades:'0',wins:'0',return_paisa:'-9007199254740993',wilson_lower:floating(0),period_digest:id,romano_availability:summary.romano_availability,romano:null};
 const comparison={index:'0',source_index:'0',identity:id,policy_digest:id,status:'refused',failed:'0',unmeasured:'0',refused:'1',checks:Array.from({length:44},(_,i)=>({index:String(i),name:'check_'+i,state:i===0?'refused':'passed'})),values:Array.from({length:44},(_,i)=>({name:'value_'+i,state:'measured',value:'0'}))};
 for(const [i,name,value] of /** @type {Array<[number,string,string]>} */ ([[0,'fwer_p_value_ppm','1000000'],[1,'romano_wolf_p_value_ppm','1000000'],[2,'white_reality_p_value_ppm','160000'],[3,'spa_p_value_ppm','1000000']]))Object.assign(comparison.values[i],{name,value});
 const source={schema_version:1,status:'saved',authority:'authenticated-saved-evidence-observation',model:'qualification',identity:child,completion:cpin,kind:'candidates',offset:'0',limit:16,total:'1',next:null,page_complete:true,refusal:null,scope:'Original qualification',admitted_bytes:'4096',summary,statistics_identity:id,statistics_completion:pin,policy:{digest:id,values:Array.from({length:39},(_,i)=>({name:'setting_'+i,value:'1'}))},qualification:{scope:id,unit:pin,original:{identity:id,completion:pin},fold_count:'2',procedure:{draws:'99',seed:'9007199254740993',block_length:'1'},allocation:{digest:id,batch:'0',batches:'1',rung:'0',rungs:'8',alpha_ppm:'50000',threshold:{numerator:'1',denominator:'160'},minimum_draws:'159',draw_resolution_met:false},later:[{index:'0',identity:id,completion:pin,coordinates:'1',sessions:'4',instrument:'NSE-RELIANCE'}]},rows:[{...structuredClone(comparison),statistics,qualification:{identity:id,family:'0',coordinate:'0',source:{identity:id,completion:pin},romano:{classification:'conservative-zero',probability:{numerator:'100',denominator:'100'},shared:null},folds:null}}]};
 ['max_fwer_p_value_ppm','max_spa_p_value_ppm','max_white_reality_p_value_ppm','max_romano_wolf_p_value_ppm'].forEach((name,i)=>Object.assign(source.policy.values[i],{name,value:'50000'}));
 return {...overview(),kind:'rung',authority:'authenticated-search-projection',rung:'0',completion:cpin,child_bodies_checked:true,source,search_policy:structuredClone(source.policy),summary:overview().rows[0],allocation:{digest:pin,batch:'0',rung:'0',rungs:'8',alpha_ppm:'50000',threshold:{numerator:'1',denominator:'320'},minimum_draws:'319',draw_resolution_met:false,rule:'alpha / (8 * (batch + 1) * (batch + 2))'},original_family_probabilities:{white:{numerator:'1',denominator:'100'},spa:{numerator:'100',denominator:'100'}},rows:[{comparison,probabilities:{romano:{numerator:'1',denominator:'1'},white:{numerator:'4',denominator:'25'},spa:{numerator:'1',denominator:'1'}}}]};
}
/** @param {any} body */
const reply=body=>async()=>({ok:true,json:async()=>body});
const selection={identity:id,pin,batch:'0',rung:'0',completion:cpin};
/** @returns {any} */
function tightenedDetail(){
 const b=detail();
 b.source.policy.values[3].value='25000';
 Object.assign(b.source.qualification.allocation,{alpha_ppm:'25000',threshold:{numerator:'1',denominator:'320'},minimum_draws:'319'});
 Object.assign(b.allocation,{alpha_ppm:'25000',threshold:{numerator:'1',denominator:'640'},minimum_draws:'639'});
 b.search_policy.digest=child;
 for(let i=0;i<4;i++)b.search_policy.values[i].value='25000';
 b.rows[0].comparison.policy_digest=child;
 return b;
}
test('overview keeps exact integers and explicitly separates all eight recorded outcomes',async()=>{
 let url='';const body=overview();const got=await fetchQualifiedSearch({identity:id},async path=>{url=path;return {ok:true,json:async()=>body};});
 assert.match(url,/^\/boolean-qualified-search\.json\?identity=/);assert.equal(got.grammar_work,'9007199254740993');assert.equal(got.rows.length,8);assert.equal(got.child_bodies_checked,false);
});
test('every selected subset preserves physical rung indices and excludes unselected history in versions 3 and 4',async()=>{
 for(const current of [false,true])for(let mask=1;mask<256;mask++){
  const body=overview();body.timeframes=rungs.filter((_,index)=>mask&(1<<index));
  body.projection_version=current?4:mask===255?2:3;
  body.rows=body.rows.filter((/** @type {any} */ row)=>body.timeframes.includes(row.rung));
  const got=await fetchQualifiedSearch({identity:id},reply(body));
  assert.deepEqual(got.timeframes,body.timeframes);
  assert.deepEqual(got.rows.map((/** @type {any} */ row)=>row.rung_index),body.timeframes.map((/** @type {string} */ label)=>String(rungs.indexOf(label))));
 }
});
test('subset history rejects missing scope, invalid order, widening and physical reindexing',async()=>{
 const selected=()=>{const body=overview();body.projection_version=3;body.timeframes=['3min','15min'];body.rows=[body.rows[2],body.rows[5]];return body;};
 for(const change of [
  (/** @type {any} */ b)=>delete b.timeframes,
  (/** @type {any} */ b)=>b.timeframes=[],
  (/** @type {any} */ b)=>b.timeframes=null,
  (/** @type {any} */ b)=>b.timeframes=['3min','3min'],
  (/** @type {any} */ b)=>b.timeframes=['15min','3min'],
  (/** @type {any} */ b)=>b.timeframes=['3min','1day'],
  (/** @type {any} */ b)=>b.rows.pop(),
  (/** @type {any} */ b)=>b.rows.reverse(),
  (/** @type {any} */ b)=>b.rows[0].rung_index='0',
  (/** @type {any} */ b)=>b.projection_version=2,
 ]){const body=selected();change(body);await assert.rejects(fetchQualifiedSearch({identity:id},reply(body)));}
 await assert.rejects(fetchQualifiedSearch({identity:id,timeframes:['1min']},reply(selected())),/selection changed/);
 const wrongVersion=overview();wrongVersion.projection_version=3;wrongVersion.timeframes=[...rungs];
 await assert.rejects(fetchQualifiedSearch({identity:id},reply(wrongVersion)),/version does not bind/);
});
test('subset detail retains eight reserved error units and refuses an excluded or changed selection',async()=>{
 const body=tightenedDetail();body.projection_version=3;body.timeframes=['1min','15min'];
 const exact={...selection,timeframes:['1min','15min']};
 const got=await fetchQualifiedSearch(exact,reply(body));
 assert.equal(got.allocation.rungs,'8');assert.equal(got.allocation.threshold.denominator,'640');
 await assert.rejects(fetchQualifiedSearch({...selection,timeframes:['1min']},reply(body)),/selection changed/);
 const excluded=structuredClone(body);excluded.timeframes=['15min'];
 await assert.rejects(fetchQualifiedSearch(selection,reply(excluded)),/outside this saved search/);
 let calls=0;
 await assert.rejects(fetchQualifiedSearch({...selection,timeframes:['15min']},async()=>{calls++;return {ok:true,json:async()=>body};}),/outside this saved search/);
 assert.equal(calls,0);
 const loosened=structuredClone(body);loosened.search_policy.values[0].value='50000';
 await assert.rejects(fetchQualifiedSearch(exact,reply(loosened)),/only tighten/);
});
test('reserved, refused, exhausted and node-only progress never acquire invented child success',async()=>{
 for(const phase of ['reserved','refused','complete']){
  const b=overview();b.rows.forEach((/** @type {any} */ r)=>Object.assign(r,{qualification:null,count:'0',counts:{admitted:'0',rejected:'0',unmeasured:'0',refused:'0'},projection_digest:null,allocation_digest:null}));b.phase=phase;b.completed_batches=phase==='complete'?'1':'0';b.node_only=phase==='complete';b.state=phase==='refused'?'refused':'paused';b.reason=phase==='refused'?'declared node allowance exhausted':null;
  await fetchQualifiedSearch({identity:id},reply(b));b.state='completed';await assert.rejects(fetchQualifiedSearch({identity:id},reply(b)));
 }
 const exhausted=overview();exhausted.state='completed';exhausted.exhausted=true;await fetchQualifiedSearch({identity:id},reply(exhausted));
 for(const change of [(/** @type {any} */ b)=>b.rows.pop(),(/** @type {any} */ b)=>b.rows[2].rung='1min',(/** @type {any} */ b)=>b.rows[0].counts.admitted='1',(/** @type {any} */ b)=>b.child_bodies_checked=true,(/** @type {any} */ b)=>b.replay_nodes_charged='18446744073709551616']){const b=overview();change(b);await assert.rejects(fetchQualifiedSearch({identity:id},reply(b)));}
});
test('exact pinned detail preserves the original policy, all44 checks and independent corrected fractions',async()=>{
 let url='';const b=detail();const got=await fetchQualifiedSearch(selection,async path=>{url=path;return {ok:true,json:async()=>b};});
 assert.match(url,/pin=3434/);assert.match(url,/completion=7878/);assert.equal(got.source.policy.values.length,39);assert.equal(got.rows[0].comparison.checks.length,44);assert.equal(got.source.rows[0].statistics.return_paisa,'-9007199254740993');assert.equal(got.rows[0].probabilities.white.numerator,'4');assert.equal(got.source.rows[0].qualification.folds,null);
});
test('unequal probability ceilings show the exact tighter policy while preserving original values',async()=>{
 const b=tightenedDetail(),before=structuredClone(b.source);
 const got=await fetchQualifiedSearch(selection,reply(b));
 assert.deepEqual(got.source,before);
 assert.equal(got.source.policy.values[0].value,'50000');
 assert.equal(got.search_policy.values[0].value,'25000');
 assert.equal(got.search_policy.values[1].value,'25000');
 assert.equal(got.rows[0].comparison.policy_digest,got.search_policy.digest);
 const view=readFileSync(new URL('../src/lib/BooleanQualifiedSearch.svelte',import.meta.url),'utf8');
 assert.match(view,/Original value/);assert.match(view,/Effective search value/);
});
test('missing, unrelated, loosened and incorrectly bound effective policies refuse',async()=>{
 for(const change of [
  (/** @type {any} */ b)=>delete b.search_policy,
  (/** @type {any} */ b)=>b.search_policy.values.pop(),
  (/** @type {any} */ b)=>b.search_policy.values[0].value='50000',
  (/** @type {any} */ b)=>b.search_policy.values[0].value='24999',
  (/** @type {any} */ b)=>b.search_policy.values[4].value='2',
  (/** @type {any} */ b)=>b.search_policy.values[0].name='missing_ceiling',
  (/** @type {any} */ b)=>b.search_policy.digest=id,
  (/** @type {any} */ b)=>b.rows[0].comparison.policy_digest=id,
  (/** @type {any} */ b)=>b.source.policy.values[0].value='1000001',
 ]){const b=tightenedDetail();change(b);await assert.rejects(fetchQualifiedSearch(selection,reply(b)));}
 const unchanged=detail();unchanged.search_policy.digest=child;unchanged.rows[0].comparison.policy_digest=child;
 await assert.rejects(fetchQualifiedSearch(selection,reply(unchanged)));
});
test('an admitted label cannot override the exact shared probability bound',async()=>{
 const b=tightenedDetail();
 for(const row of [b.source.rows[0],b.rows[0].comparison]){
  Object.assign(row,{status:'admitted',failed:'0',unmeasured:'0',refused:'0'});
  row.checks.forEach((/** @type {any} */ check)=>check.state='passed');
 }
 await assert.rejects(fetchQualifiedSearch(selection,reply(b)),/exceeds the exact shared probability allowance/);
});
test('historical version remains explicit and cannot silently become a current shared-limit result',async()=>{
 const b=tightenedDetail();b.projection_version=1;b.search_policy=structuredClone(b.source.policy);b.rows[0].comparison.policy_digest=id;
 const got=await fetchQualifiedSearch(selection,reply(b));
 assert.equal(got.projection_version,1);assert.deepEqual(got.search_policy,got.source.policy);
 const changed=structuredClone(b);changed.projection_version=2;
 await assert.rejects(fetchQualifiedSearch(selection,reply(changed)),/only tighten/);
 for(const version of [undefined,0,5,'2']){const invalid=overview();invalid.projection_version=version;await assert.rejects(fetchQualifiedSearch({identity:id},reply(invalid)),/arithmetic version/);}
 const view=readFileSync(new URL('../src/lib/BooleanQualifiedSearch.svelte',import.meta.url),'utf8');
 assert.match(view,/Historical calculation/);assert.match(view,/have not passed the newer shared-limit comparison/);
});
test('search alpha must match both original FWER/Romano ceilings and the finite qualification',async()=>{
 for(const [alpha,denominator,draws] of [['12500','1280','1279'],['50000','320','319']]){
  const b=tightenedDetail();Object.assign(b.allocation,{alpha_ppm:alpha,threshold:{numerator:'1',denominator},minimum_draws:draws});
  await assert.rejects(fetchQualifiedSearch(selection,reply(b)),/Search allowance differs/);
 }
 const foreignChild=tightenedDetail();Object.assign(foreignChild.source.qualification.allocation,{alpha_ppm:'50000',threshold:{numerator:'1',denominator:'160'},minimum_draws:'159'});
 await assert.rejects(fetchQualifiedSearch(selection,reply(foreignChild)),/Search allowance differs/);
 const invalidWriter=tightenedDetail();invalidWriter.source.policy.values[3].value='50000';invalidWriter.source.policy.values[1].value='25000';
 await assert.rejects(fetchQualifiedSearch(selection,reply(invalidWriter)),/Search allowance differs/);
});
test('wrong parent, child, allocation, probability, order and incomplete policy rows refuse',async()=>{
 for(const change of [(/** @type {any} */ b)=>b.pin=child,(/** @type {any} */ b)=>b.completion=id,(/** @type {any} */ b)=>b.batch='1',(/** @type {any} */ b)=>b.allocation.threshold.denominator='160',(/** @type {any} */ b)=>b.allocation.minimum_draws='159',(/** @type {any} */ b)=>b.allocation.draw_resolution_met=true,(/** @type {any} */ b)=>b.rows[0].probabilities.white.numerator='3',(/** @type {any} */ b)=>b.rows[0].comparison.identity=child,(/** @type {any} */ b)=>b.rows[0].comparison.checks.pop(),(/** @type {any} */ b)=>b.source.rows.pop(),(/** @type {any} */ b)=>b.source.policy.values.pop(),(/** @type {any} */ b)=>b.source.next='0']){const b=detail();change(b);await assert.rejects(fetchQualifiedSearch(selection,reply(b)));}
 for(const asked of [{identity:id,batch:'0',rung:'0'},{...selection,offset:'1',completion:null},{...selection,batch:0},{...selection,rung:'8'},{...selection,limit:257}])assert.throws(()=>qualifiedSearchSelection(asked));
 for(const status of [404,429,503])await assert.rejects(fetchQualifiedSearch({identity:id},async()=>({ok:false,status})),new RegExp(String(status)));
});
test('undeclared count fields cannot cancel inflated outcomes and completed batch totals are exact',async()=>{
 const extra=overview();extra.rows[0].counts.admitted='100';extra.rows[0].counts.undeclared='-100';
 await assert.rejects(fetchQualifiedSearch({identity:id},reply(extra)));
 const missing=overview();missing.completed_batches='0';await assert.rejects(fetchQualifiedSearch({identity:id},reply(missing)));
});
test('prospective campaign link preserves reserved status and accepts no malformed identity or invented authority',async()=>{
 const b=overview();b.phase='reserved';b.completed_batches='0';b.rows.forEach((/** @type {any} */ r)=>Object.assign(r,{qualification:null,count:'0',counts:{admitted:'0',rejected:'0',unmeasured:'0',refused:'0'},projection_digest:null,allocation_digest:null}));
 const got=await fetchQualifiedSearch({identity:id},reply(b));assert.equal(got.planned_campaign,child);assert.equal(got.phase,'reserved');assert.ok(got.rows.every((/** @type {any} */ r)=>r.qualification===null));
 b.planned_campaign=null;await fetchQualifiedSearch({identity:id},reply(b));
 for(const value of ['short','0'.repeat(64),'AB'.repeat(32),1]){b.planned_campaign=value;await assert.rejects(fetchQualifiedSearch({identity:id},reply(b)));}
 b.planned_campaign=child;b.planned_campaign_authority='completed';await assert.rejects(fetchQualifiedSearch({identity:id},reply(b)));
 const view=readFileSync(new URL('../src/lib/BooleanQualifiedSearch.svelte',import.meta.url),'utf8');assert.match(view,/View this batch’s saved timeframe progress/);assert.match(view,/link does not prove the batch started or completed/);
});
test('monitor serializes observation, follows new pins and stops on the saved pause',async()=>{
 const queue=/** @type {Array<()=>void>} */ ([]),states=/** @type {any[]} */ ([]);let calls=0;const initial=overview();initial.owner_active=true;initial.state='running';
 const monitor=createCampaignMonitor(async()=>{calls++;const b=structuredClone(initial);if(calls===2){b.sequence=String(BigInt(b.sequence)+1n);b.pin=child;b.owner_active=false;b.state='paused';}return fetchQualifiedSearch({identity:id},reply(b));},state=>states.push(state),{schedule:work=>{queue.push(work);return work;},cancel:()=>{}});
 monitor.start({identity:id});await new Promise(resolve=>setImmediate(resolve));assert.equal(calls,1);assert.equal(queue.length,1);const next=queue.shift();assert.ok(next);next();await new Promise(resolve=>setImmediate(resolve));assert.equal(calls,2);assert.equal(states.at(-1).watching,false);assert.equal(states.at(-1).body.pin,child);assert.equal(queue.length,0);monitor.stop();
});
test('dashboard retains explicit source pins and labels the correction separately from original qualification',()=>{
 const view=readFileSync(new URL('../src/lib/BooleanQualifiedSearch.svelte',import.meta.url),'utf8');assert.match(view,/All 44 search-wide policy checks/);assert.match(view,/All 39 saved common policy settings/);assert.match(view,/Original qualification:/);assert.match(view,/Grammar work only in this batch/);assert.match(view,/generation/);assert.match(view,/QualificationDetails body=\{s\}/);
 const page=readFileSync(new URL('../src/routes/backtest/+page.svelte',import.meta.url),'utf8');assert.match(page,/get\('boolean_qualified_search'\)/);
});

test('version 4 requires exact selected scope and matching original/corrected consistency receipts', async () => {
  for (const timeframes of [rungs, ['1min'], ['1min', '60min']]) {
    const b = tightenedDetail(); b.projection_version = 4; b.timeframes = [...timeframes];
    const c = cashAssessment(); c.qualification = { identity: b.source.identity, completion: b.source.completion }; c.setting_index = '0';
    syncAssessment(c, b.source.rows[0].status);
    b.source.rows[0].index_consistency = structuredClone(c);
    b.rows[0].comparison.index_consistency = structuredClone(c);
    const got = await fetchQualifiedSearch({ ...selection, timeframes }, reply(b));
    assert.equal(got.rows[0].comparison.index_consistency.state, 'not_applicable');
    assert.equal(got.rows[0].comparison.index_consistency.combined_qualifies, false);
    for (const change of [(/** @type {any} */ x) => delete x.timeframes, (/** @type {any} */ x) => delete x.rows[0].comparison.index_consistency, (/** @type {any} */ x) => x.rows[0].comparison.index_consistency.combined_qualifies = true,
      (/** @type {any} */ x) => x.rows[0].comparison.index_consistency.receipt.identity = id, (/** @type {any} */ x) => x.projection_version = 2]) {
      const invalid = structuredClone(b); change(invalid); await assert.rejects(fetchQualifiedSearch(selection, reply(invalid)), String(change));
    }
  }
  const absent = tightenedDetail(); absent.projection_version = 4; absent.timeframes = [...rungs];
  await assert.rejects(fetchQualifiedSearch(selection, reply(absent)), /required saved index-policy assessment/);
  const old = tightenedDetail(), legacy = legacyAssessment(); legacy.qualification = { identity: old.source.identity, completion: old.source.completion }; legacy.setting_index = '0';
  old.source.rows[0].index_consistency = structuredClone(legacy); old.rows[0].comparison.index_consistency = structuredClone(legacy);
  const got = await fetchQualifiedSearch(selection, reply(old)); assert.equal(got.rows[0].comparison.index_consistency.state, 'not_assessed');
});

import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {parse} from 'svelte/compiler';
import {validateIndexConsistencyPolicy,validateIndexConsistency,validateSearchIndexConsistency,indexConsistencyContext,indexConsistencyLabel,indexConsistencyReasons,indexCombinedLabel,fetchIndexConsistencyPage,createIndexConsistencyPages} from '../src/lib/index-consistency.js';
import {assessment,legacy,cash,context,policy,sync,reason,page,hex} from './index-consistency-fixture.js';
const reply=(/** @type {any} */ body) =>async()=>({ok:true,json:async()=>body});
const flush=()=>new Promise((/** @type {any} */ resolve) =>setTimeout(resolve,2));
/** @returns {{promise:Promise<any>,resolve:(value:any)=>void,reject:(error:any)=>void}} */
function deferred(){
 /** @type {(value:any)=>void} */ let resolve=()=>{throw new Error('not initialized');};
 /** @type {(error:any)=>void} */ let reject=()=>{throw new Error('not initialized');};
 const promise=new Promise((yes,no)=>{resolve=yes;reject=no;});return {promise,resolve,reject};
}

test('the native policy names only both indices and exact approved rules with all-period scope',()=>{
 assert.strictEqual(validateIndexConsistencyPolicy(policy()).minimum_winning_day_denominator,'5');
 for(const change of [(/** @type {any} */ v) =>v.instruments.push('NSE-RELIANCE'),(/** @type {any} */ v) =>v.instruments.reverse(),(/** @type {any} */ v) =>v.minimum_winning_day_numerator='4',(/** @type {any} */ v) =>v.minimum_winning_day_denominator=5,(/** @type {any} */ v) =>v.maximum_losing_day_streak='3',(/** @type {any} */ v) =>v.zero_days_reset_streak=true,(/** @type {any} */ v) =>v.costs_included=true,(/** @type {any} */ v) =>v.pnl_basis='net',(/** @type {any} */ v) =>delete v.evaluated_scope,(/** @type {any} */ v) =>v.evaluated_scope='later_only',(/** @type {any} */ v) =>v.policy_digest='0'.repeat(64),(/** @type {any} */ v) =>v.account_balance=100000]){const v=policy();change(v);assert.throws(()=>validateIndexConsistencyPolicy(v));}
});
test('saved results bind exact qualification and preserve training, later, full and independent institutional verdicts',()=>{
 const value=assessment();assert.strictEqual(validateIndexConsistency(value,context()),value);assert.equal(indexCombinedLabel(value),'Both checks passed');
 for(const institutional of ['rejected','refused','unmeasured']){const v=sync(assessment(),institutional);assert.equal(validateIndexConsistency(v,{...context(),institutional}).combined_qualifies,false);assert.equal(v.state,'passed');}
 for(const change of [(/** @type {any} */ c) =>c.qualification.identity=hex(99),(/** @type {any} */ c) =>c.qualification.completion=hex(99),(/** @type {any} */ c) =>c.setting_index='481',(/** @type {any} */ c) =>c.instrument='NSE-BANKNIFTY',(/** @type {any} */ c) =>c.institutional='passed']){const c=context();change(c);assert.throws(()=>validateIndexConsistency(value,c));}
 assert.deepEqual(indexConsistencyContext({identity:hex(1),completion:hex(2)},{index:'480',status:'rejected',statistics:{source:{instrument:'NSE-NIFTY'}}}),{...context(),institutional:'rejected'});
});
test('legacy admissions stay not assessed; cash exemption does not become an index pass',()=>{
 assert.equal(validateIndexConsistency(undefined,context()),null);assert.equal(validateIndexConsistency(legacy(),context()).state,'not_assessed');
 assert.match(indexConsistencyReasons(legacy())[0],/older result/);assert.match(indexCombinedLabel(legacy()),/Not assessed/);
 assert.throws(()=>validateIndexConsistency({...legacy(),combined_qualifies:true},context()));
 assert.throws(()=>validateIndexConsistency(null,context()));
 const v=cash();assert.equal(validateIndexConsistency(v,{...context(),instrument:'NSE-RELIANCE'}).state,'not_applicable');assert.match(indexCombinedLabel(v),/index rule does not apply/);
 v.evaluation.instrument='NSE-NIFTY';assert.throws(()=>validateIndexConsistency(v,context()),/cannot be shown as passed/);
});
test('training or later failure cannot be hidden by a passing full-span summary',()=>{
 for(const name of ['training','later']){const v=assessment();reason(v.periods[name],1,'failed');v.periods[name].summary.winning_days='2';v.periods[name].summary.losing_days='3';sync(v);assert.equal(validateIndexConsistency(v,context()).state,'failed');assert.equal(v.periods.full.state,'passed');assert.equal(v.combined_qualifies,false);assert.match(indexConsistencyReasons(v)[0],/60%/);v.state='passed';v.combined_qualifies=true;assert.throws(()=>validateIndexConsistency(v,context()));}
});
test('missing days dominate failure and unknown calendar dominates a measured failure',()=>{
 const v=assessment();reason(v.periods.training,8,'failed');reason(v.periods.later,128,'unmeasured');sync(v);assert.equal(validateIndexConsistency(v,context()).state,'unmeasured');
 reason(v.periods.full,64,'refused');v.periods.full.summary.observed_days='9';v.periods.full.summary.missing_days='1';v.periods.full.summary.losing_days='3';sync(v);
 assert.equal(validateIndexConsistency(v,context()).state,'refused');assert.equal(v.combined_qualifies,false);assert.ok(indexConsistencyReasons(v).some((/** @type {any} */ s) =>s.includes('expected trading day')));
});
test('flat and no-trade days are disjoint eligible days and never become wins',()=>{
 const v=assessment(),s=v.periods.training.summary;s.losing_days='0';s.longest_losing_streak='0';s.zero_days='1';s.no_trade_days='1';s.trades='8';sync(v);
 assert.equal(validateIndexConsistency(v,context()).periods.training.summary.eligible_days,'5');
 for(const change of [(/** @type {any} */ s) =>s.eligible_days='3',(/** @type {any} */ s) =>s.observed_days='4',(/** @type {any} */ s) =>s.winning_days='5',(/** @type {any} */ s) =>s.trades='3']){const x=structuredClone(v);change(x.periods.training.summary);assert.throws(()=>validateIndexConsistency(x,context()));}
});
test('exact counters, reason bits and arithmetic reject truncation or invented success',()=>{
 const mutations=[(/** @type {any} */ v) =>v.days_count='9',(/** @type {any} */ v) =>v.periods.training.days_count='4',(/** @type {any} */ v) =>v.periods.training.summary.passing_weeks='0',(/** @type {any} */ v) =>v.periods.training.summary.complete_weeks='0',(/** @type {any} */ v) =>v.periods.training.summary.longest_losing_streak='3',(/** @type {any} */ v) =>v.periods.training.summary.observed_days='9007199254740993',(/** @type {any} */ v) =>v.periods.training.summary.trades=10,(/** @type {any} */ v) =>v.periods.training.summary.trades='18446744073709551616',(/** @type {any} */ v) =>v.periods.training.summary.trades='10\n',(/** @type {any} */ v) =>v.periods.training.summary.pessimistic_paisa='9223372036854775808',(/** @type {any} */ v) =>v.periods.training.summary.pessimistic_paisa='-0',(/** @type {any} */ v) =>v.periods.training.reasons=['missing_expected_session'],(/** @type {any} */ v) =>v.periods.training.reasons_bits='16384',(/** @type {any} */ v) =>v.evaluation.summary.eligible_days='9',(/** @type {any} */ v) =>v.receipt.completion='0'.repeat(64),(/** @type {any} */ v) =>v.unrequested='x',(/** @type {any} */ v) =>delete v.periods.later];
 for(const change of mutations){const v=assessment();change(v);assert.throws(()=>validateIndexConsistency(v,context()),String(change));}
 const v=assessment();v.periods.training.summary.trades='18446744073709551615';v.periods.training.summary.pessimistic_paisa='-9223372036854775808';assert.equal(validateIndexConsistency(v,context()).periods.training.summary.trades,'18446744073709551615');
});
test('each native refusal reason stays refused and unmeasured reason cannot be promoted',()=>{
 for(let index=0;index<14;index++){const state=[6,8,9,10,11,12,13].includes(index)?'refused':[4,5,7].includes(index)?'unmeasured':'failed',v=assessment();reason(v.periods.training,1<<index,state);sync(v);assert.equal(validateIndexConsistency(v,context()).state,state);assert.equal(indexConsistencyReasons(v).length,1);assert.notEqual(indexConsistencyLabel(state),'Passed');}
});
test('search correction keeps identical daily evidence, changes only combined admission, and version4 requires the new receipt',()=>{
 const original=assessment(),comparison=sync(structuredClone(original),'rejected');
 assert.strictEqual(validateSearchIndexConsistency(original,comparison,{...context(),institutional:'rejected'},4),comparison);
 const reordered=Object.fromEntries(Object.entries(comparison).reverse());assert.strictEqual(validateSearchIndexConsistency(original,reordered,{...context(),institutional:'rejected'},4),reordered);
 for(const mutate of [(/** @type {any} */ v) =>v.receipt.identity=hex(99),(/** @type {any} */ v) =>v.policy_digest=hex(99),(/** @type {any} */ v) =>v.periods.training.summary.pessimistic_paisa='7',(/** @type {any} */ v) =>v.combined_qualifies=true]){const v=structuredClone(comparison);mutate(v);assert.throws(()=>validateSearchIndexConsistency(original,v,{...context(),institutional:'rejected'},4));}
 assert.equal(validateSearchIndexConsistency(undefined,undefined,context(),2),null);
 assert.equal(validateSearchIndexConsistency(legacy(),legacy(),context(),3).state,'not_assessed');
 for(const version of [1,2,3])assert.throws(()=>validateSearchIndexConsistency(original,original,context(),version));
 assert.throws(()=>validateSearchIndexConsistency(legacy(),legacy(),context(),4));assert.throws(()=>validateSearchIndexConsistency(original,undefined,context(),4));
});
test('day and week requests bind exact saved setting, receipt and selected period with bounded pages',async()=>{
 const v=assessment();for(const period of ['training','later','full'])for(const kind of ['index-days','index-weeks']){
  let path="";const result=await fetchIndexConsistencyPage(v,kind,'0',2,async (/** @type {any} */ url) =>{path=url;return {ok:true,json:async()=>page(v,kind,period,'0',2)};},period);
  const url=new URL(path,'http://fixture');assert.equal(url.pathname,'/boolean-qualification.json');assert.equal(url.searchParams.get('period'),period);assert.equal(url.searchParams.get('identity'),v.qualification.identity);assert.equal(url.searchParams.get('completion'),v.qualification.completion);assert.equal(url.searchParams.get('setting'),'480');assert.equal(url.searchParams.get('limit'),'2');assert.ok(result.rows.length<=2);assert.equal(result.next,kind==='index-days'?'2':null);
 }
});
test('page pin, period, total, ordering and row schema cannot be replaced or silently truncated',async()=>{
 const v=assessment();const changes=[(/** @type {any} */ b) =>b.receipt.completion=hex(99),(/** @type {any} */ b) =>b.identity=hex(99),(/** @type {any} */ b) =>b.setting='481',(/** @type {any} */ b) =>b.period='training',(/** @type {any} */ b) =>b.total='11',(/** @type {any} */ b) =>b.limit='16',(/** @type {any} */ b) =>b.offset='1',(/** @type {any} */ b) =>b.rows.pop(),(/** @type {any} */ b) =>b.rows[1].index='0',(/** @type {any} */ b) =>b.rows[1].day=b.rows[0].day,(/** @type {any} */ b) =>b.rows[0].trades='0',(/** @type {any} */ b) =>b.rows[0].pessimistic_paisa=400,(/** @type {any} */ b) =>b.rows[0].extra=1,(/** @type {any} */ b) =>b.refusal='missing'];
 for(const change of changes){const b=page(v,'index-days');change(b);await assert.rejects(fetchIndexConsistencyPage(v,'index-days','0',16,reply(b)),String(change));}
 for(const change of [(/** @type {any} */ b) =>b.rows[0].kind='short',(/** @type {any} */ b) =>b.rows[0].state='failed',(/** @type {any} */ b) =>b.rows[0].winning_days='4',(/** @type {any} */ b) =>b.rows[0].monday='5',(/** @type {any} */ b) =>b.rows[1].monday='18']){const b=page(v);change(b);await assert.rejects(fetchIndexConsistencyPage(v,'index-weeks','0',16,reply(b)));}
});
test('partial, holiday, unknown and missing weeks preserve separate labels and never appear passed',async()=>{
 const v=assessment();const cases=[{kind:'partial',state:'not_applicable',weekday_days:'4',eligible_days:'4',observed_days:'4',losing_days:'1'}, {kind:'short',state:'not_applicable',eligible_days:'4',observed_days:'4',losing_days:'1',closed_weekdays:'1'}, {kind:'unmeasured',state:'unmeasured',eligible_days:'4',observed_days:'4',losing_days:'1',unmeasured_days:'1'}, {kind:'complete',state:'refused',observed_days:'4',losing_days:'1',missing_days:'1'}, {kind:'complete',state:'failed',winning_days:'2',losing_days:'3'}];
 for(const replacement of cases){const b=page(v);Object.assign(b.rows[0],replacement);const got=await fetchIndexConsistencyPage(v,'index-weeks','0',16,reply(b));assert.equal(got.rows[0].state,replacement.state);b.rows[0].state='passed';await assert.rejects(fetchIndexConsistencyPage(v,'index-weeks','0',16,reply(b)),/relabelled/);}
});
test('invalid selections dispatch zero HTTP requests and read failures do not manufacture empty evidence',async()=>{
 let calls=0;const request=async()=>{calls++;return {ok:false,status:503,json:async()=>({schema_version:1,status:'refused',rows:[],refusal:'exact receipt unavailable'})};};
 for(const args of /** @type {Array<[any,string,string,number]>} */ ([[legacy(),'index-weeks','0',16],[assessment(),'unknown','0',16],[assessment(),'index-days','-1',16],[assessment(),'index-days','0',257]]))await assert.rejects(fetchIndexConsistencyPage(...args,request));
 assert.equal(calls,0);await assert.rejects(fetchIndexConsistencyPage(assessment(),'index-days','0',16,request),/exact receipt unavailable/);assert.equal(calls,1);
});
test('A to B to A churn keeps one active and one replaceable pending read, blocks stale publication and disposal',async()=>{
 const states=/** @type {any[]} */ ([]),requests=/** @type {any[]} */ ([]);const ctl=createIndexConsistencyPages((/** @type {any} */ state) =>states.push(state),async(url,init)=>{const pending=deferred();requests.push({url,init,pending});return pending.promise;});
 const a=assessment(),b=assessment();b.setting_index='481';
 const started=ctl.open(a);await flush();assert.equal(requests.length,1);
 await ctl.open(b,'index-days','0','later');await ctl.open(a,'index-days','0','training');
 assert.equal(requests.length,1);assert.equal(requests[0].init.signal.aborted,true);assert.equal(states.at(-1).body,null);
 requests[0].pending.resolve({ok:true,json:async()=>page(a)});await started;await flush();assert.equal(requests.length,2);assert.match(requests[1].url,/period=training/);assert.ok(!states.some((/** @type {any} */ s) =>s.phase==='ready'));
 requests[1].pending.resolve({ok:true,json:async()=>page(a,'index-days','training')});await flush();assert.equal(states.at(-1).phase,'ready');assert.equal(states.at(-1).period,'training');
 const last=ctl.open(a);await flush();ctl.dispose();const count=states.length;requests[2].pending.reject(new Error('late network error'));await last;assert.equal(states.length,count);await ctl.open(b);assert.equal(requests.length,3);
});
test('close during JSON decoding revokes ownership even when transport ignores abort',async()=>{
 const states=/** @type {any[]} */ ([]),json=deferred();const ctl=createIndexConsistencyPages((/** @type {any} */ s) =>states.push(s),async()=>({ok:true,json:()=>json.promise}));const reading=ctl.open(assessment());await flush();ctl.close();assert.equal(states.at(-1).phase,'idle');json.resolve(page(assessment()));await reading;assert.equal(states.at(-1).phase,'idle');ctl.dispose();
});
test('presentation keeps original verdict, all three periods and original trade drilldown without eager page reads',()=>{
 const source=readFileSync(new URL('../src/lib/IndexConsistency.svelte',import.meta.url),'utf8');const ast=parse(source);
 assert.match(source,/Existing institutional checks/);assert.match(source,/Combined result/);assert.match(source,/full-span pass alone is insufficient/);assert.match(source,/Flat and no-trade days are included/);assert.match(source,/Only a winning day resets/);assert.match(source,/Inspect saved weeks/);assert.match(source,/Inspect saved days/);assert.match(source,/original saved trades remain available/);
 const variable=(/** @type {any} */ name) =>ast.instance.content.body.filter((/** @type {any} */ n) =>n.type==='VariableDeclaration').flatMap((/** @type {any} */ n) =>n.declarations).find((/** @type {any} */ n) =>n.id.name===name);
 const day=variable('dayLabel').init;const classify=new Function(`return (${source.slice(day.start,day.end)});`)();
 assert.equal(classify({trades:'0',pessimistic_paisa:'0'}),'No trades');assert.equal(classify({trades:'2',pessimistic_paisa:'0'}),'Flat day');assert.equal(classify({trades:'2',pessimistic_paisa:'-9223372036854775808'}),'Losing day');assert.equal(classify({trades:'18446744073709551615',pessimistic_paisa:'9223372036854775807'}),'Winning day');
 const effect=ast.instance.content.body.find((/** @type {any} */ n) =>n.type==='ExpressionStatement'&&n.expression?.callee?.name==='$effect');assert.doesNotMatch(source.slice(effect.start,effect.end),/\.open\(/);assert.match(source,/onDestroy\(\(\) => pages\.dispose\(\)\)/);
 const tester=readFileSync(new URL('../src/lib/ResearchTester.svelte',import.meta.url),'utf8');assert.match(tester,/Daily \/ weekly rule/);assert.match(tester,/<IndexConsistency value=\{c\.consistency \?\? undefined\} context=\{c\.consistencyContext\}/);assert.match(tester,/List of trades/);
 const search=readFileSync(new URL('../src/lib/BooleanQualifiedSearch.svelte',import.meta.url),'utf8');assert.match(search,/consistency=\{c\.index_consistency\} institutionalStatus=\{c\.status\}/);
});

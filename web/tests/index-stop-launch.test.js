import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {parse} from 'svelte/compiler';
import {validateIndexStopMetadata,indexStopLaunchPlan,indexStopLaunchObservation,indexStopServerMonth,indexStopSplitProposal,indexStopContextMonth,indexStopRewardRiskCaption,createIndexStopLaunch,indexStopQualificationHref} from '../src/lib/index-stop-launch.js';
import {hex,RUNGS,ATTEMPT,metadata,researchPolicy,input,plan,running,saved,deferred,tick} from './index-stop-fixture.js';
const reply=(/** @type {any} */ body,status=200)=>({ok:status>=200&&status<300,status,json:async()=>body});

test('native metadata fixes both directions and fills, single stop, daily rule and configuration-only readiness',()=>{
 const value=validateIndexStopMetadata(metadata());assert.equal(value.ready,true);assert.match(value.readiness_scope,/configuration-only/);assert.deepEqual(value.execution_rules.readings,['pessimistic','optimistic']);assert.deepEqual(value.execution_rules.directions,['long','short']);assert.equal(value.policy.values.length,39);assert.equal(value.work_model.estimated_seconds,null);
 for(const mutate of [(/** @type {any} */ v)=>delete v.index_consistency_policy,(/** @type {any} */ v)=>v.execution_rules.exits.push('target'),(/** @type {any} */ v)=>v.execution_rules.directions.pop(),(/** @type {any} */ v)=>v.execution_rules.costs_included=true,(/** @type {any} */ v)=>v.work_model.estimated_seconds=1,(/** @type {any} */ v)=>v.policy.values.pop(),(/** @type {any} */ v)=>delete v.selected_timeframes_supported,(/** @type {any} */ v)=>v.configured.node_allowance=null,(/** @type {any} */ v)=>v.ready=false]){const v=metadata();mutate(v);assert.throws(()=>validateIndexStopMetadata(v));}
 const blocked=metadata();blocked.ready=false;blocked.configured={};blocked.refusal='Execution limits missing';assert.equal(validateIndexStopMetadata(blocked).ready,false);assert.throws(()=>indexStopLaunchPlan(input(),validateIndexStopMetadata(blocked)),/ready/);
});

test('each missing, replaced, duplicate or wrongly typed policy field blocks index launch before request dispatch',async()=>{
 const calls=/** @type {any[]} */([]),ctl=createIndexStopLaunch({changed:()=>{},listen:()=>()=>{},request:async(url,options)=>{calls.push({url,options});throw new Error('An invalid policy cannot reach dispatch');}});
 const required=researchPolicy().values;
 for(let index=0;index<required.length;index++)for(const defect of ['missing','unknown replacement','duplicate','wrong type']){
  const body=metadata(),name=required[index].name;
  if(defect==='missing')body.policy.values.splice(index,1);
  else if(defect==='unknown replacement')body.policy.values[index].name='unknown_policy_field';
  else if(defect==='duplicate')body.policy.values[index]=structuredClone(body.policy.values[(index+1)%required.length]);
  else body.policy.values[index].value=typeof required[index].value==='boolean'?'false':0;
  let branded=null;
  await assert.rejects(async()=>{branded=validateIndexStopMetadata(body);await ctl.start(indexStopLaunchPlan(input(),branded));},Error,`${name}: ${defect}`);
  assert.equal(branded,null,`${name}: ${defect} cannot be branded ready`);
 }
 assert.deepEqual(calls,[]);ctl.dispose();
 const override=metadata();override.policy.values.find((/** @type {any} */ field)=>field.name==='min_worst_reward_risk_ppm').value='2500000';
 const exact=validateIndexStopMetadata(override);assert.match(indexStopRewardRiskCaption(exact.policy),/at least 2\.5 times/);
 assert.equal(indexStopLaunchPlan(input(),exact).expected_policy_digest,exact.policy.digest);
});

test('the inherited reward/risk caption uses the exact effective threshold including overrides and u64 precision',()=>{
 for(const [ppm,ratio] of [['3000000','3'],['2500000','2.5'],['0','0'],['1','0.000001'],['18446744073709551615','18446744073709.551615']]){
  const policy=researchPolicy(),field=policy.values.find((/** @type {any} */ value)=>value.name==='min_worst_reward_risk_ppm');assert.ok(field);field.value=ppm;
  const before=structuredClone(policy),caption=indexStopRewardRiskCaption(policy);
  assert.ok(caption.includes(`at least ${ratio} times the largest losing trade`),caption);
  assert.match(caption,/smallest positive pessimistic trade in the later evaluation/);
  assert.match(caption,/separate from the winning-day rule and the signal-candle stop/);
  assert.match(caption,/Without an observed loss, this ratio stays unmeasured/);
  assert.deepEqual(policy,before);
 }
});

test('an unresolved or malformed effective ratio stays unavailable instead of assuming zero or three',()=>{
 const missing=researchPolicy();missing.values.find((/** @type {any} */ field)=>field.name==='min_worst_reward_risk_ppm').name='unrecognized_research_gate';
 const duplicate=researchPolicy();duplicate.values[0]={name:'min_worst_reward_risk_ppm',value:'3000000'};
 const malformed=['-1','01','1.5','18446744073709551616',3000000].map(value=>{const policy=researchPolicy();policy.values.find((/** @type {any} */ field)=>field.name==='min_worst_reward_risk_ppm').value=value;return policy;});
 for(const policy of [null,undefined,{},missing,duplicate,{ready:false,digest:null,values:[],refusal:'Policy is not configured'},...malformed]){
  const caption=indexStopRewardRiskCaption(policy);assert.match(caption,/effective reward\/risk threshold is unavailable/);assert.match(caption,/no default is assumed/);assert.doesNotMatch(caption,/at least|times|3:1/);
 }
});

test('the visible caption follows resolved policy changes without mutating a plan or submitting a request',async()=>{
 const calls=/** @type {any[]} */([]),ctl=createIndexStopLaunch({changed:()=>{},listen:()=>()=>{},request:async(url,options)=>{calls.push({url,options});throw new Error('Caption inspection cannot submit or observe a sweep');}});
 const source=readFileSync(new URL('../src/lib/IndexStopLaunch.svelte',import.meta.url),'utf8'),tree=parse(source);
 const declaration=tree.instance?.content.body.flatMap((/** @type {any} */ node)=>node.declarations??[]).find((/** @type {any} */ node)=>node.id?.name==='rewardRiskCaption');assert.ok(declaration);
 const expression=declaration.init.arguments[0],caption=new Function('config','indexStopRewardRiskCaption',`return (${source.slice(expression.start,expression.end)});`);
 const native=validateIndexStopMetadata(metadata()),p=indexStopLaunchPlan(input(),native),before=structuredClone(p);
 assert.match(caption({body:null},indexStopRewardRiskCaption),/unavailable/);
 assert.match(caption({body:native},indexStopRewardRiskCaption),/at least 0 times/);
 const override=metadata();override.policy.values.find((/** @type {any} */ field)=>field.name==='min_worst_reward_risk_ppm').value='2500000';
 assert.match(caption({body:validateIndexStopMetadata(override)},indexStopRewardRiskCaption),/at least 2\.5 times/);
 assert.match(caption({body:null},indexStopRewardRiskCaption),/unavailable/);
 const visible=source.indexOf('<p><b>Inherited institutional research gate.</b> {rewardRiskCaption}</p>'),advanced=source.indexOf('<details><summary>Advanced work limits and acceptance settings</summary>');
 assert.ok(visible>=0&&visible<advanced);assert.deepEqual(p,before);await tick();assert.deepEqual(calls,[]);ctl.dispose();
});

test('actual configuration A to B to A reads cannot restore stale readiness, and current form choices rebuild or refuse the exact plan',async()=>{
 const source=readFileSync(new URL('../src/lib/IndexStopLaunch.svelte',import.meta.url),'utf8'),tree=parse(source);
 const variable=(/** @type {string} */ name)=>{const node=tree.instance?.content.body.flatMap((/** @type {any} */ row)=>row.declarations??[]).find((/** @type {any} */ row)=>row.id?.name===name);assert.ok(node);return node;};
 const functions=['readConfiguration','start'].map(name=>{const node=tree.instance?.content.body.find((/** @type {any} */ row)=>row.type==='FunctionDeclaration'&&row.id?.name===name);assert.ok(node);return source.slice(node.start,node.end);}).join('\n');
 const expression=variable('prepared').init.arguments[0],fields=variable('fields').init;
 const mount=new Function('ask','validateIndexStopMetadata','indexStopServerMonth','indexStopLaunchPlan',`
  let active=true,feed='fixture-feed',symbols=['NIFTY'],rungs=['1min','5min'],from='2024-01',to='2024-12';
  let config={phase:'idle',body:null,why:''},serverMonth=null,settings={maxLossPoints:'20',batchPrograms:'2',nodeAllowance:'128',batchAllowance:'3'};
  let dates={from:'2024-01',to:'2024-08',laterFrom:'2024-09',laterTo:'2024-12'},generation=0,configAbort=null,busy=false,blockedReason='',localWhy='';
  const fields=${source.slice(fields.start,fields.end)},onTimeframes=()=>{},launches=[],controller={start:async plan=>{launches.push(plan);}};
  const prepare=()=>(${source.slice(expression.start,expression.end)})();
  const prepared={get plan(){return prepare().plan;}};
  ${functions}
  return {readConfiguration,start,launches,view:()=>({config,prepared:prepare(),settings,localWhy}),change:value=>{if(value.settings)Object.assign(settings,value.settings);if(value.symbols)symbols=value.symbols;if(value.rungs)rungs=value.rungs;if(value.feed)feed=value.feed;}};
 `);
 const pending=/** @type {any[]} */([]),app=mount(async(/** @type {string} */ url,/** @type {any} */ options)=>{const wait=deferred();pending.push({url,options,wait});return wait.promise;},validateIndexStopMetadata,indexStopServerMonth,indexStopLaunchPlan);
 const response=(/** @type {any} */ body)=>({...reply(body),headers:{get:()=> 'Wed, 09 Sep 2026 12:00:00 GMT'}});
 const first=app.readConfiguration();await app.start();assert.equal(app.view().prepared.plan,null);
 app.change({settings:{maxLossPoints:'21'}});const second=app.readConfiguration();
 app.change({settings:{maxLossPoints:'20'}});const third=app.readConfiguration();
 assert.equal(pending.length,3);assert.equal(pending[0].url,pending[2].url);assert.equal(pending[0].options.signal.aborted,true);assert.equal(pending[1].options.signal.aborted,true);
 const blocked=metadata();blocked.ready=false;blocked.refusal='The current grammar work limit is missing';blocked.configured.node_allowance=null;
 pending[2].wait.resolve(response(blocked));await third;await app.start();assert.equal(app.view().prepared.plan,null);
 const other=metadata();other.configured.max_loss_points='21';pending[1].wait.resolve(response(other));await second;
 pending[0].wait.resolve(response(metadata()));await first;
 assert.equal(app.view().config.body.ready,false);assert.equal(app.view().config.body.refusal,blocked.refusal);assert.deepEqual(app.launches,[]);
 const current=app.readConfiguration();pending[3].wait.resolve(response(metadata()));await current;
 const original=app.view().prepared.plan;assert.equal(original.index,'NSE-NIFTY');
 app.change({symbols:['BANKNIFTY'],rungs:['5min'],feed:'second-feed'});const changed=app.view().prepared.plan;
 assert.equal(changed.index,'NSE-BANKNIFTY');assert.equal(changed.feed,'second-feed');assert.deepEqual(changed.timeframes,['5min']);assert.deepEqual(original.timeframes,['1min','5min']);
 app.change({symbols:['NIFTY','BANKNIFTY']});await app.start();assert.equal(app.view().prepared.plan,null);
 app.change({symbols:['NIFTY'],rungs:[]});await app.start();assert.equal(app.view().prepared.plan,null);
 app.change({rungs:['1min'],settings:{maxLossPoints:'21'}});await app.start();assert.match(app.view().prepared.why,/Recheck/);
 assert.deepEqual(app.launches,[]);assert.ok(pending.every(row=>row.options.method===undefined));
 assert.match(source,/>Check configuration<\/button>/);assert.match(source,/worker must still admit the exact stored OHLCV/);
});

test('all255 exact nonempty timeframe selections preserve their native scope and exclude grid or foreign instruments',()=>{
 const native=validateIndexStopMetadata(metadata());
 for(let mask=1;mask<256;mask++){const timeframes=RUNGS.filter((_,index)=>mask&(1<<index));const p=indexStopLaunchPlan(input({timeframes}),native);assert.deepEqual(p.timeframes,timeframes);assert.equal(p.index,'NSE-NIFTY');assert.equal(p.bits,'all');assert.equal(p.max_loss_points,'20');assert.ok(!Object.keys(p).some(k=>/horizon|trail|target|stop_distance/.test(k)));}
 assert.equal(indexStopLaunchPlan(input({symbols:['NSE-BANKNIFTY']}),native).index,'NSE-BANKNIFTY');
 for(const extra of [{symbols:[]},{symbols:['NIFTY','BANKNIFTY']},{symbols:['NIFTY','RELIANCE']},{symbols:['RELIANCE']},{timeframes:[]},{timeframes:['5min','1min']},{timeframes:['1min','1min']},{timeframes:['1day']},{horizon:'10'},{from:'2024-08'},{laterFrom:'2024-12'},{laterFrom:'2024-08'},{nodeAllowance:'4097'},{maxLossPoints:'19'},{batchPrograms:'3'},{batchAllowance:'0'},{bits:'1,0'},{bits:'01'},{bits:'1,1'}])assert.throws(()=>indexStopLaunchPlan(input(extra),native),JSON.stringify(extra));
 assert.throws(()=>indexStopLaunchPlan(input(),structuredClone(native)),/configuration/);
});

test('chronological proposal uses server IST month, excludes incomplete months explicitly and never guesses clock clearance',()=>{
 assert.equal(indexStopServerMonth('Mon, 31 Aug 2026 20:00:00 GMT'),'2026-09');assert.equal(indexStopServerMonth('Tue, 08 Sep 2026 12:00:00 GMT'),'2026-09');
 for(const date of [null,'2026-09-08','Mon, 31 Aug 2026 25:00:00 GMT','Wed, 08 Sep 2026 12:00:00 GMT'])assert.equal(indexStopServerMonth(date),null);
 const p=indexStopSplitProposal('2024-01','2026-09','2026-09');assert.equal(p.from,'2024-01');assert.equal(p.to,'2026-01');assert.equal(p.laterFrom,'2026-02');assert.equal(p.laterTo,'2026-08');assert.equal(p.completeMonths,32);assert.equal(p.excludedCurrentOrFuture,1);assert.equal(p.selectedTo,'2026-09');
 for(const [a,b,current] of [['2024-01','2024-04','2026-09'],['2025-01','2024-12','2026-09'],['2024-01','2024-12',null]])assert.throws(()=>indexStopSplitProposal(/** @type {string} */(a),/** @type {string} */(b),current));
});

test('the required earlier context month crosses years without changing the selected training span or certifying source availability',()=>{
 assert.equal(indexStopContextMonth('2019-12'),'2019-11');
 assert.equal(indexStopContextMonth('2020-01'),'2019-12');
 assert.equal(indexStopContextMonth('2024-03'),'2024-02');
 const proposal=indexStopSplitProposal('2019-12','2026-09','2026-09');
 assert.equal(proposal.from,'2019-12');assert.equal(indexStopContextMonth(proposal.from),'2019-11');
 for(const invalid of ['','2024-13','2024-00','0001-01','2024-1',' 2024-01'])assert.throws(()=>indexStopContextMonth(invalid));
 const source=readFileSync(new URL('../src/lib/IndexStopLaunch.svelte',import.meta.url),'utf8');
 assert.match(source,/indexStopContextMonth\(dates\.from\)/);
 assert.match(source,/requirement, not confirmation that those files are present/);
 assert.match(source,/Missing context refuses the run; the selected history stays unchanged/);
});

test('acknowledged progress retains the exact older batch while a new reservation or empty final batch is observed',()=>{
 const p=plan(),prior=saved(p),complete=running(p,{completed_batches:'1',completed_programs:'2',completed_work:'128',current_batch:null,qualifications:prior.qualifications.map((/** @type {any} */ row)=>({...row,stage:'saved'})),latest_saved_batch:prior});
 const first=indexStopLaunchObservation(complete,p,ATTEMPT);assert.equal(first.latestSavedBatch.batch,'0');
 const pending=running(p,{completed_batches:'1',completed_programs:'2',completed_work:'128',current_batch:'1',latest_saved_batch:prior});const second=indexStopLaunchObservation(pending,p,ATTEMPT,first);assert.equal(second.qualifications[0].identity,null);assert.deepEqual(second.latestSavedBatch,prior);
 const empty=running(p,{completed_batches:'2',completed_programs:'2',completed_work:'128',current_batch:null,exhausted:true,latest_saved_batch:prior,qualifications:p.timeframes.map((/** @type {string} */ timeframe)=>({timeframe,rung:RUNGS.indexOf(timeframe),identity:null,completion:null,stage:null}))});Object.assign(empty,{in_flight:false,status:'completed',report:'The declared work allowance finished',finished_micros:100});const final=indexStopLaunchObservation(empty,p,ATTEMPT,second);assert.equal(final.phase,'done');assert.deepEqual(final.latestSavedBatch,prior);
 for(const mutate of [(/** @type {any} */ v)=>v.latest_saved_batch=null,(/** @type {any} */ v)=>v.latest_saved_batch.batch='1',(/** @type {any} */ v)=>v.latest_saved_batch.qualifications[0].completion=hex(99),(/** @type {any} */ v)=>v.latest_saved_batch.qualifications.pop(),(/** @type {any} */ v)=>v.completed_programs='3',(/** @type {any} */ v)=>v.current_batch='2',(/** @type {any} */ v)=>v.qualifications[0].timeframe='2min',(/** @type {any} */ v)=>v.search_identity=hex(99)]){const changed=structuredClone(pending);mutate(changed.index_stop);assert.throws(()=>indexStopLaunchObservation(changed,p,ATTEMPT,first),String(mutate));}
});

test('source preparation cannot manufacture zero measured progress or unknown terminal outcomes',()=>{
 const p=plan(),v=running(p,{search_identity:null,completed_batches:null,completed_programs:null,completed_work:null,current_batch:null,exhausted:null,elapsed_micros:null});const observed=indexStopLaunchObservation(v,p,ATTEMPT);assert.equal(observed.completedPrograms,null);assert.equal(observed.qualifications[0].stage,'preparing');
 for(const mutate of [(/** @type {any} */ r)=>r.index_stop.completed_programs='0',(/** @type {any} */ r)=>r.index_stop.elapsed_micros='01',(/** @type {any} */ r)=>r.index_stop.request.timeframes=['1min'],(/** @type {any} */ r)=>r.attempt_key='7',(/** @type {any} */ r)=>r.report='done',(/** @type {any} */ r)=>r.index_stop.exhausted=true]){const changed=structuredClone(v);mutate(changed);assert.throws(()=>indexStopLaunchObservation(changed,p,ATTEMPT));}
});

test('one deliberate click produces one POST; exact-attempt reads pause hidden and duplicate clicks cannot relaunch',async()=>{
 const p=plan(),calls=/** @type {any[]} */([]),states=/** @type {any[]} */([]);let visible=true,visibility=()=>{};
 const ctl=createIndexStopLaunch({changed:s=>states.push(s),visible:()=>visible,listen:fn=>{visibility=fn;return()=>{};},interval:100000,request:async(url,options)=>{calls.push({url,options});return options?.method==='POST'?reply({accepted:true,attempt:ATTEMPT,refusal:null},202):reply({running:running(p)});}});
 await ctl.start(p);assert.equal(calls.filter(c=>c.options?.method==='POST').length,1);assert.match(calls[1].url,new RegExp(`attempt=${ATTEMPT}`));assert.equal(states.at(-1).phase,'running');await assert.rejects(ctl.start(p),/duplicate/);
 visible=false;visibility();const count=calls.length;await ctl.recheck();assert.equal(calls.length,count);visible=true;visibility();await tick();assert.equal(calls.length,count+1);ctl.dispose();visibility();await tick();assert.equal(calls.length,count+1);
});

test('rapid repeated clicks cannot submit a second plan while the first acknowledgement is still pending',async()=>{
 const p=plan(),calls=/** @type {any[]} */([]),states=/** @type {any[]} */([]),ack=deferred();
 const ctl=createIndexStopLaunch({changed:value=>states.push(value),listen:()=>()=>{},interval:100000,request:async(url,options)=>{calls.push({url,options});return options?.method==='POST'?ack.promise:reply({running:running(p)});}});
 const first=ctl.start(p);assert.equal(states.at(-1).phase,'starting');
 await assert.rejects(ctl.start(p),/duplicate/);await assert.rejects(ctl.start(plan({symbols:['BANKNIFTY']})),/duplicate/);
 assert.equal(calls.length,1);assert.deepEqual(JSON.parse(calls[0].options.body),p);
 ack.resolve(reply({accepted:true,attempt:ATTEMPT,refusal:null},202));await first;
 assert.equal(calls.filter(row=>row.options?.method==='POST').length,1);ctl.dispose();
});

test('ambiguous submission and late acknowledgement are never resent or rebound to unrelated work',async()=>{
 const p=plan(),calls=/** @type {any[]} */([]),states=/** @type {any[]} */([]),ack=deferred();const ctl=createIndexStopLaunch({changed:s=>states.push(s),listen:()=>()=>{},request:async(url,options)=>{calls.push({url,options});return ack.promise;}});
 const start=ctl.start(p);ctl.stop();ack.resolve(reply({accepted:true,attempt:ATTEMPT,refusal:null},202));await start;assert.equal(states.at(-1).phase,'unknown');assert.equal(states.at(-1).attempt,null);assert.equal(calls.length,1);await assert.rejects(ctl.start(p));await ctl.restore(running(p));assert.equal(states.at(-1).attempt,null);assert.equal(calls.length,1);ctl.dispose();
 const refused=/** @type {any[]} */([]),no=createIndexStopLaunch({changed:s=>refused.push(s),listen:()=>()=>{},request:async()=>reply({accepted:false,refusal:'Busy'},409)});await no.start(p);assert.equal(refused.at(-1).phase,'refused');no.dispose();
});

test('restored exact request is observation-only; stale slow JSON cannot replace a newer read',async()=>{
 const p=plan(),states=/** @type {any[]} */([]),requests=/** @type {any[]} */([]),json=deferred();let n=0;const ctl=createIndexStopLaunch({changed:s=>states.push(s),listen:()=>()=>{},interval:100000,request:async(url,options)=>{requests.push({url,options});return ++n===1?{ok:true,json:()=>json.promise}:reply({running:running(p)});}});
 const restored=ctl.restore(running(p));await tick();void ctl.recheck();json.resolve({running:running(p,{search_identity:hex(99)})});await restored;await tick();assert.equal(states.at(-1).searchIdentity,hex(10));assert.equal(requests.length,2);assert.ok(requests.every(r=>!r.options.method));ctl.dispose();
});

test('Backtest alone owns the Run sweep click and visibly distinguishes the proposal, fixed exits, measured counters and saved batch',()=>{
 const source=readFileSync(new URL('../src/lib/IndexStopLaunch.svelte',import.meta.url),'utf8');parse(source);const page=readFileSync(new URL('../src/routes/backtest/+page.svelte',import.meta.url),'utf8');
 assert.match(source,/onclick=\{start\}[^>]*>Run sweep<\/button>/);assert.doesNotMatch(source,/\$effect\([\s\S]{0,70}controller\.start/);assert.match(source,/splitEdited\)return/);assert.match(source,/Proposed chronological split/);assert.match(source,/not claimed to be an untouched final holdout/);assert.match(source,/completedPrograms\?\?'Unavailable'/);assert.match(source,/run\.latestSavedBatch\.batch/);assert.match(source,/configuration[\s\S]*must still admit/);
 assert.match(page,/if \(receiptBusy \|\| booleanBusy \|\| indexStopBusy \|\| indexWorkflow \|\| sweepMode !== 'ordinary'\) return/);assert.match(page,/onclick=\{startDescent\}\s*disabled=\{!runtimeInspection.value.canSweep \|\| indexWorkflow/);
 const link=new URL(indexStopQualificationHref(hex(1),hex(2)),'http://fixture');assert.equal(link.pathname,'/backtest');assert.equal(link.searchParams.get('index_stop_qualification'),hex(1));assert.equal(link.searchParams.get('completion'),hex(2));assert.throws(()=>indexStopQualificationHref(hex(1),'missing'));
 const legacy=readFileSync(new URL('../src/lib/BooleanLaunch.svelte',import.meta.url),'utf8');assert.match(legacy,/if \(!active \|\| busy \|\| blockedReason/);assert.match(legacy,/onclick=\{start\} disabled=\{!active/);assert.match(page,/initialBooleanRun && booleanBusy/);
});

test('acknowledged results automatically open one exact timeframe comparison and tab changes only select a saved pin',()=>{
 const source=readFileSync(new URL('../src/lib/IndexStopLaunch.svelte',import.meta.url),'utf8'),tree=parse(source);
 const declaration=tree.instance?.content.body.flatMap((/** @type {any} */ n)=>n.declarations??[]).find((/** @type {any} */ n)=>n.id?.name==='selectedComparison');assert.ok(declaration);const value=declaration.init.arguments[0];
 const select=new Function('run','savedTimeframe',`return (${source.slice(value.start,value.end)});`),p=plan(),latest=saved(p),run={latestSavedBatch:latest};
 assert.strictEqual(select(run,''),latest.qualifications[0]);assert.strictEqual(select(run,'5min'),latest.qualifications[1]);assert.strictEqual(select(run,'1day'),latest.qualifications[0]);assert.equal(select({},''),null);
 assert.equal((source.match(/<IndexStopQualification /g)??[]).length,1);assert.match(source,/<IndexStopQualification initialIdentity=\{selectedComparison.identity\} initialCompletion=\{selectedComparison.completion\} autoLoad=\{true\}/);
 const nav=/<nav class="controls" aria-label="Choose saved results timeframe">([\s\S]*?)<\/nav>/.exec(source);assert.ok(nav);assert.match(nav[1],/onclick=\{\(\)=>\{savedTimeframe=row.timeframe;\}\}/);assert.doesNotMatch(nav[1],/controller|start|POST|fetch|request/);
});

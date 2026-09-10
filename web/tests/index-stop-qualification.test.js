import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {parse} from 'svelte/compiler';
import {fetchIndexStopQualification,createIndexStopQualificationReader,fetchIndexStopFolds,indexStopQualificationInitialRow} from '../src/lib/index-stop-qualification.js';
import {fetchIndexConsistencyPage,createIndexConsistencyPages} from '../src/lib/index-consistency.js';
import {assessment,reason,sync,page as consistencyPage} from './index-consistency-fixture.js';
import {hex,researchPolicy,deferred,tick} from './index-stop-fixture.js';
const ID=hex(1),PIN=hex(2);
const reply=(/** @type {any} */ v)=>async()=>({ok:true,json:async()=>v});
test('a cumulative ranking link opens its exact setting beyond the first page without replacing it by rank',()=>{
 const chosen={index:'981',rank:'482'},body={identity:ID,completion:PIN,offset:'480',rows:[{index:'17',rank:'481'},chosen]},expected={identity:ID,completion:PIN,offset:'480',setting:'981'};
 assert.equal(indexStopQualificationInitialRow(body,expected),chosen);
 for(const wrong of [{...expected,completion:hex(99)},{...expected,identity:hex(99)},{...expected,offset:'0'},{...expected,setting:'1'},{...expected,setting:'0981'}])assert.throws(()=>indexStopQualificationInitialRow(body,wrong));
 assert.throws(()=>indexStopQualificationInitialRow({...body,rows:[chosen,chosen]},expected),/repeated/);
});
/** Generated transport and daily totals; these are not market performance. @returns {any} */
function setting(index=0,later=false){return {index:String(index),program_index:String(Math.floor(index/2)),run_id:hex(index+10+(later?30:0)),source_id:hex(later?7:6),evaluation_digest:hex(index+50+(later?30:0)),instrument:'NSE-NIFTY',direction:index%2===0?'long':'short',timeframe:'1min',first_day:later?'11':'4',last_day:later?'15':'8',expression:'0 | !0',truth:{evaluated:'20',hits:'10',misses:'5',unknown:'5'},metrics:{trades:'10',wins:'6',optimistic_paisa:'1200',pessimistic_paisa:'1000',drawdown_paisa:'200',worst_trade_paisa:'-100',adverse_paisa:'200',favourable_paisa:'2000',holding_minutes:'20',unreachable:'0',too_late:'0',gap_invalid:'0',while_open:'0',entry_refused:'0',path_refused:'0',closing_refused:'0',stopped:'0',forced:'10',stop_gaps:'0'},events_count:'10',trades_count:'10',days_count:'5',feed:null};}
/** @returns {any} */
function row(index=0){const original=setting(index),later=setting(index,true),daily=assessment();daily.qualification={identity:ID,completion:PIN};daily.setting_index=String(index);return {index:String(index),rank:String(index+1),original,later,institutional:{index:String(index),source_index:String(index),identity:later.run_id,status:'admitted',failed:'0',unmeasured:'0',refused:'0',checks:Array.from({length:44},(_,i)=>({index:String(i),name:`fixture_check_${i}`,state:'passed'})),values:Array.from({length:44},(_,i)=>({name:`fixture_value_${i}`,state:'measured',value:'0'}))},index_consistency:daily};}
/** @returns {any} */
function page(){const policy=researchPolicy();return {schema_version:1,status:'saved',model:'index-stop-qualification',kind:'settings',identity:ID,completion:PIN,search_identity:hex(3),batch:'0',rung:'0',original:{identity:hex(4),completion:hex(5)},later:{identity:hex(6),completion:hex(7)},total:'2',offset:'0',limit:16,summary:{institutional_admitted:'2',rejected:'0',refused:'0',unmeasured:'0',combined_qualified:'2'},ranking:'later_pessimistic_descending',policy:{digest:policy.digest,values:policy.values},rows:[row(),row(1)],refusal:null};}
/** @param {any} body @param {any} selected @returns {any} */
function folds(body,selected){return {schema_version:1,status:'saved',model:'index-stop-qualification',kind:'folds',identity:body.identity,completion:body.completion,original:body.original,later:body.later,setting:selected.index,total:'2',offset:'0',limit:16,rows:[{index:'0',first_day:'11',last_day:'12',observed_days:'2',trades:'4',wins:'4',pessimistic_paisa:'800',refused:'0',decided:true},{index:'1',first_day:'13',last_day:'15',observed_days:'3',trades:'6',wins:'2',pessimistic_paisa:'200',refused:'0',decided:true}],refusal:null};}

test('saved comparison retains both direction/fill readings, original/later receipts and separate acceptance results',async()=>{
 const body=page();let url='';const got=await fetchIndexStopQualification({identity:ID},async value=>{url=value;return {ok:true,json:async()=>body};});assert.match(url,/^\/index-stop-qualification.json\?/);assert.equal(got.rows[0].original.metrics.optimistic_paisa,'1200');assert.equal(got.rows[1].later.direction,'short');assert.equal(got.rows[1].index_consistency.combined_qualifies,true);assert.equal(got.summary.combined_qualified,'2');assert.equal(got.next,null);
 const first={...page(),limit:1,rows:[row()]};const a=await fetchIndexStopQualification({identity:ID,limit:1},reply(first));assert.equal(a.next,'1');const second={...page(),offset:'1',limit:1,rows:[row(1)]};assert.equal((await fetchIndexStopQualification({identity:ID,completion:PIN,offset:'1',limit:1},reply(second))).rows[0].index,'1');
});

test('a three-day losing streak spanning training and later fails combined even when both individual periods pass',async()=>{
 const body=page(),r=body.rows[0],v=r.index_consistency;v.periods.full.summary.longest_losing_streak='3';reason(v.periods.full,8,'failed');sync(v);body.summary.combined_qualified='1';const got=await fetchIndexStopQualification({identity:ID},reply(body));assert.equal(got.rows[0].institutional.status,'admitted');assert.equal(got.rows[0].index_consistency.periods.training.state,'passed');assert.equal(got.rows[0].index_consistency.periods.later.state,'passed');assert.equal(got.rows[0].index_consistency.state,'failed');assert.equal(got.rows[0].index_consistency.combined_qualifies,false);
 body.summary.combined_qualified='2';await assert.rejects(fetchIndexStopQualification({identity:ID},reply(body)),/summary/);
});

test('ranking is later pessimistic only with canonical identity retained, not optimistic or new row numbering',async()=>{
 const body=page(),high=body.rows[1];high.later.metrics.pessimistic_paisa='1100';high.index_consistency.periods.later.summary.pessimistic_paisa='1100';high.index_consistency.periods.full.summary.pessimistic_paisa='2100';sync(high.index_consistency);body.rows.reverse();body.rows.forEach((/** @type {any} */ r,/** @type {number} */ i)=>r.rank=String(i+1));const got=await fetchIndexStopQualification({identity:ID},reply(body));assert.equal(got.rows[0].index,'1');assert.equal(got.rows[0].rank,'1');assert.equal(got.rows[0].later.direction,'short');
 body.rows.reverse();body.rows.forEach((/** @type {any} */ r,/** @type {number} */ i)=>r.rank=String(i+1));await assert.rejects(fetchIndexStopQualification({identity:ID},reply(body)),/ordering/);
});

test('corrupt summaries, ranks, policy, periods, source binding and missing assessment cannot be accepted',async()=>{
 for(const mutate of [(/** @type {any} */ b)=>b.completion=hex(99),(/** @type {any} */ b)=>b.original=b.later,(/** @type {any} */ b)=>b.rung='1',(/** @type {any} */ b)=>b.rows.pop(),(/** @type {any} */ b)=>b.rows[0].rank='0',(/** @type {any} */ b)=>b.rows[1].index='0',(/** @type {any} */ b)=>b.rows[0].later.direction='short',(/** @type {any} */ b)=>b.rows[0].later.expression='1',(/** @type {any} */ b)=>b.rows[0].institutional.identity=hex(99),(/** @type {any} */ b)=>b.rows[0].institutional.source_index='1',(/** @type {any} */ b)=>b.rows[0].index_consistency.qualification.completion=hex(99),(/** @type {any} */ b)=>b.rows[0].index_consistency.setting_index='1',(/** @type {any} */ b)=>delete b.rows[0].index_consistency,(/** @type {any} */ b)=>b.rows[0].index_consistency.periods.training.first_day='3',(/** @type {any} */ b)=>b.summary.combined_qualified='3',(/** @type {any} */ b)=>b.policy.values.pop(),(/** @type {any} */ b)=>b.total='3',(/** @type {any} */ b)=>b.offset='1',(/** @type {any} */ b)=>b.limit='16',(/** @type {any} */ b)=>b.refusal='Not saved']){const body=page();mutate(body);await assert.rejects(fetchIndexStopQualification({identity:ID,completion:PIN},reply(body)),String(mutate));}
 let calls=0;for(const select of [{identity:'bad'},{identity:ID,offset:'1'},{identity:ID,limit:257},{identity:ID,offset:'01'},{identity:ID,completion:'0'.repeat(64)}])await assert.rejects(fetchIndexStopQualification(select,async()=>{calls++;return null;}));assert.equal(calls,0);
});

test('fold pages keep exact parent/source/setting and refuse overlap, wrong totals or foreign period dates',async()=>{
 const b=page(),r=b.rows[1];let url='';const got=await fetchIndexStopFolds(b,r,'0',async value=>{url=value;return {ok:true,json:async()=>folds(b,r)};});assert.equal(got.rows[1].pessimistic_paisa,'200');const q=new URL(url,'http://fixture');assert.equal(q.searchParams.get('setting'),'1');assert.equal(q.searchParams.get('completion'),PIN);assert.equal(got.next,null);
 for(const mutate of [(/** @type {any} */ p)=>p.setting='0',(/** @type {any} */ p)=>p.original.completion=hex(99),(/** @type {any} */ p)=>p.rows[1].first_day='12',(/** @type {any} */ p)=>p.rows[1].last_day='16',(/** @type {any} */ p)=>p.rows[0].wins='5',(/** @type {any} */ p)=>p.rows[0].trades='0',(/** @type {any} */ p)=>p.rows.pop()]){const p=structuredClone(folds(b,r));mutate(p);await assert.rejects(fetchIndexStopFolds(b,r,'0',reply(p)));}
});

test('daily and weekly details use the new closed endpoint/model and exact receipt plus period selectors',async()=>{
 const value=page().rows[0].index_consistency;
 for(const period of ['training','later','full'])for(const kind of ['index-days','index-weeks']){const body=consistencyPage(value,kind,period);body.model='index-stop-qualification';let url='';const got=await fetchIndexConsistencyPage(value,kind,'0',16,async request=>{url=request;return {ok:true,json:async()=>body};},period,'index-stop-qualification');assert.equal(got.period,period);const parsed=new URL(url,'http://fixture');assert.equal(parsed.pathname,'/index-stop-qualification.json');assert.equal(parsed.searchParams.get('period'),period);assert.equal(parsed.searchParams.get('completion'),PIN);body.model='qualification';await assert.rejects(fetchIndexConsistencyPage(value,kind,'0',16,reply(body),period,'index-stop-qualification'));
 }
 await assert.rejects(fetchIndexConsistencyPage(value,'index-days','0',16,reply(null),'full','foreign-model'));
});

test('rapid comparison A to B to A cancels old reads and publishes only the newest exact source',async()=>{
 const states=/** @type {any[]} */([]),calls=/** @type {any[]} */([]),ctl=createIndexStopQualificationReader(s=>states.push(s),async(url,options)=>{const next=deferred();calls.push({url,options,next});return next.promise;});const a={identity:ID},b={identity:hex(99)};const first=ctl.open(a);await tick();await ctl.open(b);await ctl.open(a);assert.equal(calls.length,1);assert.equal(calls[0].options.signal.aborted,true);calls[0].next.resolve({ok:true,json:async()=>page()});await first;await tick();assert.equal(calls.length,2);assert.ok(!states.some(s=>s.phase==='ready'));calls[1].next.resolve({ok:true,json:async()=>page()});await tick();assert.equal(states.at(-1).phase,'ready');ctl.dispose();
 const slow=deferred(),other=/** @type {any[]} */([]),reader=createIndexStopQualificationReader(s=>other.push(s),async()=>({ok:true,json:()=>slow.promise}));const promise=reader.open(a);await tick();reader.close();const count=other.length;slow.resolve(page());await promise;assert.equal(other.length,count);reader.dispose();
});

test('day/week model switches revoke the old response instead of publishing another workflow’s evidence',async()=>{
 const value=page().rows[0].index_consistency,states=/** @type {any[]} */([]),slow=deferred();let model='index-stop-qualification';const ctl=createIndexConsistencyPages(s=>states.push(s),async()=>({ok:true,json:()=>slow.promise}),()=>model);const read=ctl.open(value,'index-days','0','full');await tick();model='qualification';ctl.close();const count=states.length;const body=consistencyPage(value,'index-days');body.model='index-stop-qualification';slow.resolve(body);await read;assert.equal(states.length,count);ctl.dispose();
});

test('comparison view exposes every requested human metric and the original/later exact selected trade',()=>{
 const source=readFileSync(new URL('../src/lib/IndexStopQualification.svelte',import.meta.url),'utf8');parse(source);for(const term of ['Winning days / eligible','Full weeks passed / failed','Longest losing-day streak','pessimistic / optimistic','Both passed','indexConsistencyReasons','indexStopPoints','Details and trades','Original training trades','Later trades','initialSelected={','model="index-stop-qualification"','strongest 0.1%'])assert.ok(source.includes(term),term);assert.doesNotMatch(source,/candidateMoney|₹|account profit/);
 const trades=readFileSync(new URL('../src/lib/IndexStopResults.svelte',import.meta.url),'utf8');assert.match(trades,/initialSelected\?\{kind:'trades',setting:initialSelected.index,selected:initialSelected\}/);assert.match(trades,/const saved=overview\?\?report.body/);
});

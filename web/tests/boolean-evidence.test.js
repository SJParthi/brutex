import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fetchBooleanEvidence, booleanEvidenceSelection } from '../src/lib/boolean-evidence.js';
import { evidenceComparison } from '../src/lib/sweep-evidence.js';
const identity='12'.repeat(32),completion='34'.repeat(32),catalog='56'.repeat(32);
/** @param {number} value */
function float(value){const view=new DataView(new ArrayBuffer(8));view.setFloat64(0,value,true);return {bits:String(view.getBigUint64(0,true)),decimal:Object.is(value,-0)?'-0':String(value)};}
/** @returns {any} */
function summary(){const test={statistic:float(2),probability:float(.5),exact_probability:{numerator:'1',denominator:'2'},matched_or_exceeded:'1',draws:'2',strategies:'2',periods:'4'};return {scope:identity,cohort:identity,layout:identity,families:'1',candidates:'2',periods:'4',segments:'2',periods_per_segment:'2',splits:'2',contributing_splits:'1',bottom_half_splits:'0',procedure:{draws:'2',seed:'9007199254740993',block_length:'1'},bounds:Object.fromEntries(['families','candidates','observations','bootstrap_work','split_work','additional_working_bytes','saved_body_bytes'].map(name=>[name,'1000'])),test_families:{white:identity,spa:identity,romano:identity},white:structuredClone(test),spa:structuredClone(test),romano_availability:'measured'};}
/** @returns {any} */
function source(){return {index:'0',coordinates:'2',identity:catalog,completion,instrument:'NSE-NIFTY',cash:false,membership_digest:identity};}
/** @param {string} index @returns {any} */
function row(index='0'){return {index,identity,run:completion,source:source(),coordinate:index,program_index:'0',side:index==='0'?'long':'short',ordinal:'0',execution_refusal_bits:'0',trades:'2',wins:'1',return_paisa:'-9007199254740993',wilson_lower:float(.5),period_digest:identity,romano_availability:'measured',romano:{strategy:index,stepdown_rank:index,statistic:float(-0),strict_exceedances:'1',initial:{numerator:'1',denominator:'2'},adjusted:{numerator:'1',denominator:'2'}}};}
/** @returns {any} */
function page(){return {schema_version:1,status:'saved',authority:'authenticated-saved-evidence-observation',model:'statistics',identity,completion,kind:'candidates',offset:'0',limit:16,total:'2',next:null,page_complete:true,rows:[row(),row('1')],refusal:null,scope:'Saved finite research evidence only',summary:summary(),admitted_bytes:'9007199254740993'};}
/** @returns {any} */
function admission(){const body=page();body.model='admission';body.statistics_identity=catalog;body.statistics_completion=completion;body.policy={digest:identity,values:Array.from({length:39},(_,i)=>({name:'policy_'+i,value:i<37?String(i):false}))};body.rows=body.rows.map((/** @type {any} */ statistics)=>({index:statistics.index,source_index:statistics.index,identity:statistics.identity,status:'refused',failed:'1',unmeasured:'2',refused:'4',statistics,checks:Array.from({length:44},(_,i)=>({index:String(i),name:'check_'+i,state:i===0?'failed':i===1?'unmeasured':i===2?'refused':'passed'})),values:Array.from({length:44},(_,i)=>({name:'value_'+i,state:i===0?'measured':i===1?'unmeasured':i===2?'refused':'complete',value:i===0?'18446744073709551615':null}))}));return body;}
const response=(/** @type {any} */ body)=>async()=>({ok:true,json:async()=>body});
test('statistics preserve exact integers, float bits and source completion links',async()=>{
 const body=page();let url='';const got=await fetchBooleanEvidence({identity,completion},async value=>{url=value;return {ok:true,json:async()=>body};});
 assert.match(url,/^\/boolean-statistics\.json\?/);assert.match(url,/completion=3434/);assert.equal(got.summary.procedure.seed,'9007199254740993');assert.equal(got.rows[0].return_paisa,'-9007199254740993');assert.equal(got.rows[0].source.identity,catalog);assert.equal(got.rows[0].source.completion,completion);assert.equal(got.rows[0].romano.statistic.bits,'9223372036854775808');
});
test('all saved admission states and false policy flags survive without zero defaults',async()=>{
 const body=admission();const got=await fetchBooleanEvidence({model:'admission',identity,completion},response(body));
 assert.equal(got.rows[0].status,'refused');assert.deepEqual(got.rows[0].checks.slice(0,4).map((/** @type {any} */ v)=>v.state),['failed','unmeasured','refused','passed']);assert.equal(got.rows[0].values[0].value,'18446744073709551615');assert.equal(got.rows[0].values[1].value,null);assert.equal(got.rows[0].checks.length,44);assert.equal(got.rows[0].values.length,44);assert.equal(got.policy.values.length,39);assert.equal(got.policy.values[38].value,false);
 for(const [status,failed,unmeasured,refused] of [['admitted','0','0','0'],['rejected','1','0','0'],['unmeasured','1','2','0']]){const next=admission();for(const r of next.rows){Object.assign(r,{status,failed,unmeasured,refused});r.checks.forEach((/** @type {any} */ check,/** @type {number} */ i)=>{check.state=i===0&&failed==='1'?'failed':i===1&&unmeasured==='2'?'unmeasured':'passed';});}assert.equal((await fetchBooleanEvidence({model:'admission',identity},response(next))).rows[0].status,status);}
});
test('changed pins, missing rows, wrong models, false floats and parent mismatches refuse',async()=>{
 for(const change of [(/** @type {any} */ b)=>b.status='missing',(/** @type {any} */ b)=>b.completion=identity,(/** @type {any} */ b)=>b.model='admission',(/** @type {any} */ b)=>b.rows.pop(),(/** @type {any} */ b)=>b.next='0',(/** @type {any} */ b)=>b.total='3',(/** @type {any} */ b)=>b.summary.periods='5',(/** @type {any} */ b)=>b.rows[0].wilson_lower.decimal='0.4',(/** @type {any} */ b)=>b.rows[0].romano.statistic.decimal='0',(/** @type {any} */ b)=>b.rows[0].source.index='1',(/** @type {any} */ b)=>b.rows[0].coordinate='2',(/** @type {any} */ b)=>b.rows[0].romano.adjusted.denominator='0']){const body=page();change(body);await assert.rejects(fetchBooleanEvidence({identity,completion},response(body)));}
 for(const status of [429,503])await assert.rejects(fetchBooleanEvidence({identity},async()=>({ok:false,status})),new RegExp(String(status)));
});
test('missing and overlapping check partitions cannot appear as admitted or complete',async()=>{
 for(const change of [(/** @type {any} */ b)=>b.rows[0].status='admitted',(/** @type {any} */ b)=>b.rows[0].failed='3',(/** @type {any} */ b)=>b.rows[0].checks.pop(),(/** @type {any} */ b)=>b.rows[0].checks[43].state='failed',(/** @type {any} */ b)=>b.rows[0].values.pop(),(/** @type {any} */ b)=>b.rows[0].values[1].value='0',(/** @type {any} */ b)=>b.rows[0].statistics.identity=completion,(/** @type {any} */ b)=>b.rows[0].source_index='1',(/** @type {any} */ b)=>b.policy.values.pop(),(/** @type {any} */ b)=>b.policy.values[1].name=b.policy.values[0].name]){const body=admission();change(body);await assert.rejects(fetchBooleanEvidence({model:'admission',identity,completion},response(body)));}
});
test('unavailable Romano-Wolf remains explicit and nonfinite bits never become numeric zero',async()=>{
 for(const availability of ['unavailable-constant-return-candidate','unavailable-numerical-refusal']){const body=page();body.summary.romano_availability=availability;for(const r of body.rows){r.romano_availability=availability;r.romano=null;}assert.equal((await fetchBooleanEvidence({identity},response(body))).rows[0].romano,null);body.rows[0].romano={};await assert.rejects(fetchBooleanEvidence({identity},response(body)));}
 const body=page();body.summary.white.statistic={bits:'9218868437227405312',decimal:null};assert.equal((await fetchBooleanEvidence({identity},response(body))).summary.white.statistic.decimal,null);body.summary.white.statistic.decimal='0';await assert.rejects(fetchBooleanEvidence({identity},response(body)));
});
test('source and split pages require exact pins and reconcile complete segment partitions',async()=>{
 const body={...page(),kind:'sources',total:'1',rows:[source()]};assert.equal((await fetchBooleanEvidence({identity,completion,kind:'sources'},response(body))).rows[0].identity,catalog);
 const split={...page(),kind:'splits',rows:[{index:'0',train_mask:'1',test_mask:'2',bottom_half:false,rankable:true,scores_digest:identity},{index:'1',train_mask:'2',test_mask:'1',bottom_half:false,rankable:false,scores_digest:identity}]};assert.equal((await fetchBooleanEvidence({identity,completion,kind:'splits'},response(split))).rows.length,2);split.rows[1].bottom_half=true;await assert.rejects(fetchBooleanEvidence({identity,completion,kind:'splits'},response(split)));split.rows[1].bottom_half=false;split.rows[1].test_mask='2';await assert.rejects(fetchBooleanEvidence({identity,completion,kind:'splits'},response(split)));
 for(const selection of [{identity,kind:'sources'},{identity,offset:'1'},{identity,completion,offset:'01'},{identity,completion,offset:1},{identity,completion,limit:257},{identity,completion,model:'admission',kind:'splits'}])assert.throws(()=>booleanEvidenceSelection(selection));
});
test('later candidate pages cannot skip duplicate or lose the original completion',async()=>{
 const body={...page(),limit:1,next:'1',rows:[row()]};assert.equal((await fetchBooleanEvidence({identity,limit:1},response(body))).next,'1');const last={...body,offset:'1',next:null,rows:[row('1')]};assert.equal((await fetchBooleanEvidence({identity,completion,offset:'1',limit:1},response(last))).rows[0].index,'1');last.rows[0].index='0';await assert.rejects(fetchBooleanEvidence({identity,completion,offset:'1',limit:1},response(last)));
});
test('view offers every recorded reason and exact catalog navigation without authority claims',()=>{
 const component=readFileSync(new URL('../src/lib/BooleanEvidence.svelte',import.meta.url),'utf8');assert.match(component,/All 44 recorded checks/);assert.match(component,/All 44 evidence values/);assert.match(component,/All 39 saved policy settings/);assert.match(component,/initialCompletion=\{catalog.completion\}/);assert.match(component,/initialCandidate=\{catalog.candidate\}/);assert.match(component,/grants no Selection V6 authority/);assert.match(component,/completion:body.statistics_completion/);
 const parent=readFileSync(new URL('../src/lib/SweepEvidence.svelte',import.meta.url),'utf8');assert.match(parent,/\['boolean-statistics','boolean-admission','boolean-qualification'\]\.includes\(e.operation\)/);
 for(const operation of ['boolean-statistics','boolean-admission']){const comparison=evidenceComparison({status:'saved',rows:[],evidence:{operation,completion:'completed'}});assert.equal(comparison.find(row=>row.area==='Exact trade replay')?.status,'Not verified in this summary');}
});
test('linked evidence view carries the supplied completion into automatic loading',async()=>{
 const component=readFileSync(new URL('../src/lib/BooleanEvidence.svelte',import.meta.url),'utf8');
 assert.match(component,/const completion=initialCompletion/);
 assert.match(component,/load\(\{model:initialModel,identity:initialIdentity,completion\}\)/);
 const body=page();
 assert.equal((await fetchBooleanEvidence({model:'statistics',identity,completion},response(body))).completion,completion);
 body.completion=catalog;
 await assert.rejects(fetchBooleanEvidence({model:'statistics',identity,completion},response(body)));
});

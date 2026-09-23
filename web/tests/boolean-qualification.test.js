import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {validateQualification} from '../src/lib/boolean-qualification.js';
import {fetchBooleanEvidence,booleanEvidenceSelection} from '../src/lib/boolean-evidence.js';
import {evidenceComparison} from '../src/lib/sweep-evidence.js';
const id='12'.repeat(32),pin='34'.repeat(32),later='56'.repeat(32),parent='78'.repeat(32);
/** @param {number} n */
function floating(n){const b=new DataView(new ArrayBuffer(8));b.setFloat64(0,n,true);return {bits:String(b.getBigUint64(0,true)),decimal:String(n)};}
/** @returns {any} */
function fixture(){
 const test={statistic:floating(1),probability:floating(.5),exact_probability:{numerator:'1',denominator:'2'},matched_or_exceeded:'1',draws:'2',strategies:'1',periods:'4'};
 const summary={scope:id,cohort:id,layout:id,families:'1',candidates:'1',periods:'4',segments:'2',periods_per_segment:'2',splits:'2',contributing_splits:'1',bottom_half_splits:'0',procedure:{draws:'2',seed:'0',block_length:'1'},bounds:Object.fromEntries(['families','candidates','observations','bootstrap_work','split_work','additional_working_bytes','saved_body_bytes'].map(name=>[name,'1000'])),test_families:{white:id,spa:id,romano:id},white:test,spa:structuredClone(test),romano_availability:'unavailable-constant-return-candidate'};
 const statistics={index:'0',identity:id,run:pin,source:{index:'0',coordinates:'1',identity:parent,completion:pin,membership_digest:id,instrument:'NSE-RELIANCE',cash:true},coordinate:'0',program_index:'0',side:'long',ordinal:'0',execution_refusal_bits:'0',trades:'0',wins:'0',return_paisa:'0',wilson_lower:floating(0),period_digest:id,romano_availability:summary.romano_availability,romano:null};
 const folds={plan:id,selected:id,anchor:id,later,training_run:pin,later_run:later,resolution:id,ordinal:'0',training_last_day:'100',first_day:'101',last_day:'104',execution_refusal_bits:'0',decided:'2',profitable:'1',return_paisa:'-9007199254740993',rows:[{index:'0',first_day:'101',last_day:'102',sessions:'2',trades:'1',wins:'1',return_paisa:'1'},{index:'1',first_day:'103',last_day:'104',sessions:'2',trades:'1',wins:'0',return_paisa:'-9007199254740994'}]};
 return {schema_version:1,status:'saved',authority:'authenticated-saved-evidence-observation',model:'qualification',identity:id,completion:pin,kind:'candidates',offset:'0',limit:16,total:'1',next:null,page_complete:true,refusal:null,scope:'Fixed training / later research only',admitted_bytes:'4096',summary,statistics_identity:parent,statistics_completion:pin,policy:{digest:id,values:Array.from({length:39},(_,i)=>({name:'setting_'+i,value:i<37?'1':false}))},qualification:{scope:id,unit:pin,original:{identity:parent,completion:pin},fold_count:'2',procedure:{draws:'99',seed:'9007199254740993',block_length:'1'},allocation:{digest:id,batch:'0',batches:'1',rung:'0',rungs:'8',alpha_ppm:'50000',threshold:{numerator:'1',denominator:'160'},minimum_draws:'159',draw_resolution_met:false},later:[{index:'0',identity:later,completion:pin,coordinates:'1',sessions:'4',instrument:'NSE-RELIANCE'}]},rows:[{index:'0',identity:id,source_index:'0',status:'refused',failed:'1',unmeasured:'2',refused:'4',statistics,checks:Array.from({length:44},(_,i)=>({index:String(i),name:'check_'+i,state:i===0?'failed':i===1?'unmeasured':i===2?'refused':'passed'})),values:Array.from({length:44},(_,i)=>({name:'value_'+i,state:'measured',value:i===0?'18446744073709551615':'0'})),qualification:{identity:later,family:'0',coordinate:'0',source:{identity:later,completion:pin},romano:{classification:'conservative-zero',probability:{numerator:'100',denominator:'100'},shared:null},folds}}]};
}
/** @param {any} body */
const response=body=>async()=>({ok:true,json:async()=>body});
test('qualification route preserves all saved checks, later pins, unreduced probabilities and exact money',async()=>{
 const body=fixture();let path='';const result=await fetchBooleanEvidence({model:'qualification',identity:id,completion:pin},async url=>{path=url;return {ok:true,json:async()=>body};});
 assert.match(path,/^\/boolean-qualification\.json\?/);assert.match(path,/completion=3434/);assert.equal(result.rows[0].qualification.source.completion,pin);assert.equal(result.rows[0].qualification.folds.return_paisa,'-9007199254740993');assert.equal(result.qualification.procedure.seed,'9007199254740993');assert.equal(result.rows[0].qualification.romano.probability.denominator,'100');assert.equal(result.rows[0].checks.length,44);assert.equal(result.policy.values.length,39);
});
test('declared multiplicity and minimum draw resolution reconcile exactly, including zero alpha',()=>{
 const body=fixture();validateQualification(body);
 for(const change of [(/** @type {any} */ b)=>b.qualification.allocation.rungs='7',(/** @type {any} */ b)=>b.qualification.allocation.minimum_draws='158',(/** @type {any} */ b)=>b.qualification.allocation.draw_resolution_met=true,(/** @type {any} */ b)=>b.qualification.allocation.rung='8',(/** @type {any} */ b)=>b.qualification.allocation.threshold.denominator='0']){const b=fixture();change(b);assert.throws(()=>validateQualification(b));}
 const a=body.qualification.allocation;Object.assign(a,{alpha_ppm:'0',threshold:{numerator:'0',denominator:'1'},minimum_draws:null,draw_resolution_met:false});validateQualification(body);a.draw_resolution_met=true;assert.throws(()=>validateQualification(body));
});
test('null fold evidence stays unavailable and every present fold must conserve sessions, boundaries and signed returns',()=>{
 const body=fixture();body.rows[0].qualification.folds=null;validateQualification(body);assert.equal(body.rows[0].qualification.folds,null);
 for(const change of [(/** @type {any} */ f)=>f.rows.pop(),(/** @type {any} */ f)=>f.rows[1].first_day='102',(/** @type {any} */ f)=>f.rows[1].sessions='1',(/** @type {any} */ f)=>f.return_paisa='0',(/** @type {any} */ f)=>f.training_last_day='101',(/** @type {any} */ f)=>f.training_run=later,(/** @type {any} */ f)=>f.ordinal='1',(/** @type {any} */ f)=>f.execution_refusal_bits='64',(/** @type {any} */ f)=>f.profitable='2']){const b=fixture();change(b.rows[0].qualification.folds);assert.throws(()=>validateQualification(b));}
});
test('shared bootstrap position belongs to the nonconstant subfamily and zero never becomes positive',()=>{
 const body=fixture();body.summary.candidates='2';const row=body.rows[0];row.index='1';const r=row.qualification.romano;
 Object.assign(r,{classification:'measured-positive',probability:{numerator:'1',denominator:'100'},shared:{strategy:'0',stepdown_rank:'0',statistic:floating(2),strict_exceedances:'0',initial:{numerator:'1',denominator:'100'},adjusted:{numerator:'1',denominator:'100'}}});validateQualification(body);
 assert.notEqual(r.shared.strategy,row.index);r.shared.strategy='2';assert.throws(()=>validateQualification(body));r.shared.strategy='0';r.shared.statistic=floating(-1);assert.throws(()=>validateQualification(body));r.classification='conservative-nonpositive';r.probability={numerator:'100',denominator:'100'};validateQualification(body);r.probability.numerator='1';assert.throws(()=>validateQualification(body));
 const zero=fixture();zero.rows[0].qualification.romano.probability={numerator:'1',denominator:'1'};assert.throws(()=>validateQualification(zero));
});
test('foreign later links, incomplete pages, wrong pins and unavailable HTTP responses refuse visibly',async()=>{
 for(const change of [(/** @type {any} */ b)=>b.rows[0].qualification.source.completion=id,(/** @type {any} */ b)=>b.rows[0].qualification.coordinate='1',(/** @type {any} */ b)=>b.qualification.later.pop(),(/** @type {any} */ b)=>b.completion=id,(/** @type {any} */ b)=>b.rows.pop(),(/** @type {any} */ b)=>b.next='0']){const b=fixture();change(b);await assert.rejects(fetchBooleanEvidence({model:'qualification',identity:id,completion:pin},response(b)));}
 for(const status of [404,429,503])await assert.rejects(fetchBooleanEvidence({model:'qualification',identity:id},async()=>({ok:false,status})),new RegExp(String(status)));
 for(const selection of [{model:'qualification',identity:id,offset:'1'},{model:'qualification',identity:id,completion:pin,kind:'sources'},{model:'qualification',identity:id,limit:257}])assert.throws(()=>booleanEvidenceSelection(selection));
});
test('view separates training and later proof, exposes all folds and carries exact detail pins',()=>{
 const view=readFileSync(new URL('../src/lib/QualificationDetails.svelte',import.meta.url),'utf8');assert.match(view,/Fixed training settings, later observations/);assert.match(view,/Nonconstant-subfamily position/);assert.match(view,/initialCompletion=\{q.source.completion\}/);assert.match(view,/initialCandidate=\{q.coordinate\}/);assert.match(view,/Every fixed-training later fold/);assert.match(view,/Insufficient resolution for a rejection/);
 const page=readFileSync(new URL('../src/routes/backtest/+page.svelte',import.meta.url),'utf8');assert.match(page,/get\('boolean_qualification'\)/);
 const laterView=readFileSync(new URL('../src/lib/BooleanLater.svelte',import.meta.url),'utf8');assert.match(laterView,/completion:initialCompletion/);assert.match(laterView,/candidate:initialCandidate/);
 const comparison=evidenceComparison({status:'saved',rows:[],evidence:{operation:'boolean-qualification',completion:'completed'}});assert.equal(comparison.find(row=>row.area==='Exact trade replay')?.status,'Not verified in this summary');assert.equal(comparison.find(row=>row.area==='AND combination search')?.status,'Recorded fixed-training later qualification');
});

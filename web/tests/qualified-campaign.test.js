import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {CAMPAIGN_RUNGS} from '../src/lib/boolean-campaign.js';
import {fetchQualifiedCampaign} from '../src/lib/qualified-campaign.js';
const identity='12'.repeat(32),pin='34'.repeat(32);
/** @returns {any} */
function fixture(){return {schema_version:1,status:'saved',authority:'acknowledged-qualified-campaign-history',identity,pin,sequence:'17',descriptor_digest:identity,state:'completed',owner_active:false,history_records:'17',admitted_bytes:'160000',history_checked:true,child_completion_receipts_checked:false,child_bodies_checked:false,refusal:null,scope:'Recorded history only',rows:CAMPAIGN_RUNGS.map((rung,index)=>({rung,unit:String(index+1).padStart(64,'0'),state:'completed',reason:null,qualification:{identity:String(index+20).padStart(64,'0'),completion:pin}}))};}
/** @param {any} body */
const response=body=>async()=>({ok:true,json:async()=>body});
test('eight-slot overview preserves exact child pins without claiming child authentication',async()=>{
 const body=fixture();let url='';const got=await fetchQualifiedCampaign({identity,pin},async value=>{url=value;return {ok:true,json:async()=>body};});
 assert.match(url,/^\/boolean-qualified-campaign\.json\?/);assert.match(url,/pin=3434/);assert.equal(got.rows.length,8);assert.equal(got.rows[7].qualification.completion,pin);assert.equal(got.child_bodies_checked,false);assert.equal(got.child_completion_receipts_checked,false);
});
test('all canonical subsets reconcile only selected units without changing physical indices',async()=>{
 for(let mask=1;mask<256;mask++){
  const body=fixture();body.timeframes=CAMPAIGN_RUNGS.filter((_,index)=>mask&(1<<index));
  body.rows=body.rows.map((/** @type {any} */ row,/** @type {number} */ index)=>({...row,rung_index:String(index)})).filter((/** @type {any} */ row)=>body.timeframes.includes(row.rung));
  const got=await fetchQualifiedCampaign({identity,pin},response(body));
  assert.deepEqual(got.timeframes,body.timeframes);assert.equal(got.state,'completed');
  assert.deepEqual(got.rows.map((/** @type {any} */ row)=>row.rung_index),body.timeframes.map((/** @type {string} */ rung)=>String(CAMPAIGN_RUNGS.indexOf(rung))));
 }
});
test('selected campaign refuses omissions, malformed declarations and compacted physical indices',async()=>{
 const selected=()=>{const body=fixture();body.timeframes=['3min','15min'];body.rows=[{...body.rows[2],rung_index:'2'},{...body.rows[5],rung_index:'5'}];return body;};
 for(const change of [
  (/** @type {any} */ b)=>delete b.timeframes,
  (/** @type {any} */ b)=>b.timeframes=[],
  (/** @type {any} */ b)=>b.timeframes=['3min','3min'],
  (/** @type {any} */ b)=>b.timeframes=['15min','3min'],
  (/** @type {any} */ b)=>b.timeframes=['3min','1day'],
  (/** @type {any} */ b)=>b.rows[0].rung_index='0',
  (/** @type {any} */ b)=>delete b.rows[0].rung_index,
  (/** @type {any} */ b)=>b.rows.reverse(),
 ]){const body=selected();change(body);await assert.rejects(fetchQualifiedCampaign({identity},response(body)));}
 const paused=selected();paused.rows[1].state='waiting';paused.rows[1].qualification=null;paused.state='running';
 const got=await fetchQualifiedCampaign({identity},response(paused));assert.equal(got.state,'running');assert.equal(got.rows.length,2);
 paused.rows[0].state='waiting';paused.rows[0].qualification=null;paused.rows[1].state='completed';paused.rows[1].qualification={identity,pin:undefined,completion:pin};
 await assert.rejects(fetchQualifiedCampaign({identity},response(paused)),/skipped/);
});
test('waiting, started-without-owner and saved refusal remain distinct from completion',async()=>{
 for(const state of ['waiting','running','refused']){const body=fixture();for(const row of body.rows){row.state='waiting';row.qualification=null;}if(state!=='waiting'){body.rows[0].state=state;body.rows[0].reason=state==='refused'?'exact saved refusal':null;}body.state=state;assert.equal((await fetchQualifiedCampaign({identity},response(body))).state,state);}
 const body=fixture();body.rows[7].state='waiting';body.rows[7].qualification=null;body.state='running';assert.equal((await fetchQualifiedCampaign({identity},response(body))).rows[7].state,'waiting');
});
test('skipped units, duplicate scope, changed pins, missing links and false body assurance refuse',async()=>{
 for(const change of [(/** @type {any} */ b)=>b.rows.pop(),(/** @type {any} */ b)=>b.pin=identity,(/** @type {any} */ b)=>b.rows[1].unit=b.rows[0].unit,(/** @type {any} */ b)=>b.rows[0].qualification=null,(/** @type {any} */ b)=>b.rows[0].state='waiting',(/** @type {any} */ b)=>b.rows[0].reason='contradictory refusal',(/** @type {any} */ b)=>b.child_bodies_checked=true,(/** @type {any} */ b)=>b.history_records='18']){const body=fixture();change(body);await assert.rejects(fetchQualifiedCampaign({identity,pin},response(body)));}
 for(const status of [404,429,503])await assert.rejects(fetchQualifiedCampaign({identity},async()=>({ok:false,status})),new RegExp(String(status)));
});
test('overview reuses bounded monitor, cancels on unmount and keeps child detail pinned',()=>{
 const view=readFileSync(new URL('../src/lib/QualifiedCampaign.svelte',import.meta.url),'utf8');assert.match(view,/createCampaignMonitor/);assert.match(view,/return\(\)=>monitor.stop\(\)/);assert.match(view,/initialCompletion=\{detail.completion\}/);assert.match(view,/initialModel="qualification"/);assert.match(view,/Child links are recorded here/);assert.match(view,/not a heartbeat/);
 const page=readFileSync(new URL('../src/routes/backtest/+page.svelte',import.meta.url),'utf8');assert.match(page,/get\('boolean_qualified_campaign'\)/);
});

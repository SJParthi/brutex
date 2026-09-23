import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { CAMPAIGN_RUNGS, campaignState, campaignSelection, fetchBooleanCampaign } from '../src/lib/boolean-campaign.js';

const identity='12'.repeat(32), pin='34'.repeat(32), completion='56'.repeat(32);
/** @returns {any} */
function snapshot() {
  return {schema_version:1,status:'saved',authority:'acknowledged-campaign-snapshot',identity,pin,sequence:'9007199254740993',
    state:'paused',owner_active:false,child_completion_receipts_checked:true,child_bodies_checked:false,refusal:null,scope:'Saved finite catalog progress only',from:'2025-04',to:'2025-05',horizon_bars:'5',program_count:'2',program_digest:identity,descriptor_digest:pin,
    rows:CAMPAIGN_RUNGS.map((rung,index)=>({rung,state:index===0?'completed':index===1?'paused':'waiting',reason:index===1?'Invocation work allowance reached':null,
      expected_catalogs:[{instrument:'NSE-NIFTY',identity,completion:index===0?completion:null,cash:false,membership_digest:identity}],catalogs:index===0?[{instrument:'NSE-NIFTY',identity,completion}]:[],statistics:index===0?{identity,completion}:null,admission:index===0?{identity,completion}:null}))};
}
/** @param {any} body */
const reply=body=>async()=>({ok:true,json:async()=>body});

test('eight timeframe snapshot preserves exact integers and recorded completion pins',async()=>{
 const body=snapshot();let url='';
 const got=await fetchBooleanCampaign({identity,pin},async value=>{url=value;return {ok:true,json:async()=>body};});
 assert.equal(url,`/boolean-campaign.json?identity=${identity}&pin=${pin}`);
 assert.equal(got.sequence,'9007199254740993');assert.equal(got.rows[0].catalogs[0].completion,completion);
 assert.equal(got.child_bodies_checked,false);assert.equal(got.rows.length,8);
});
test('recorded starts distinguish an observed owner without claiming liveness or completion',()=>{
 assert.equal(campaignState('running',true),'Started; owner observed');
 assert.equal(campaignState('running',false),'Started; no owner observed');
 assert.deepEqual(['waiting','paused','refused','completed'].map(state=>campaignState(state,false)),['Waiting','Paused','Refused','Recorded complete']);
 assert.throws(()=>campaignState('idle',false));assert.throws(()=>campaignState('completed',/** @type {any} */ (null)));
});
test('unknown missing corrupt and replaced snapshots cannot become partial success',async()=>{
 for(const change of [(/** @type {any} */ b)=>b.pin=identity,(/** @type {any} */ b)=>b.identity=pin,(/** @type {any} */ b)=>b.rows.pop(),
  (/** @type {any} */ b)=>b.rows[1].rung='1min',(/** @type {any} */ b)=>b.sequence=9007199254740993,
  (/** @type {any} */ b)=>b.state='completed',(/** @type {any} */ b)=>b.rows[0].admission=null,
  (/** @type {any} */ b)=>b.rows[0].catalogs[0].completion=null,(/** @type {any} */ b)=>b.child_bodies_checked=true,
  (/** @type {any} */ b)=>b.child_completion_receipts_checked=false,(/** @type {any} */ b)=>b.rows[2].state='unknown',
  (/** @type {any} */ b)=>b.rows[0].expected_catalogs[0].identity=pin,(/** @type {any} */ b)=>b.rows[2].expected_catalogs=[],
  (/** @type {any} */ b)=>b.rows[2].expected_catalogs[0].instrument='NSE-BANKNIFTY',(/** @type {any} */ b)=>b.from='2025-06',
  (/** @type {any} */ b)=>b.program_count='0',(/** @type {any} */ b)=>b.horizon_bars=5]) {
   const body=snapshot();change(body);await assert.rejects(fetchBooleanCampaign({identity,pin},reply(body)));
 }
 for(const status of [400,429,503])await assert.rejects(fetchBooleanCampaign({identity},async()=>({ok:false,status})),new RegExp(String(status)));
});
test('fresh refresh is explicit and complete means every one of eight recorded outputs',async()=>{
 const body=snapshot();for(const row of body.rows){row.state='completed';row.reason=null;row.catalogs=[{instrument:'NSE-NIFTY',identity,completion}];row.expected_catalogs[0].completion=completion;row.statistics={identity,completion};row.admission={identity,completion};}body.state='completed';
 let url='';assert.equal((await fetchBooleanCampaign({identity},async value=>{url=value;return {ok:true,json:async()=>body};})).state,'completed');assert.ok(!url.includes('pin='));
 body.state='paused';assert.equal((await fetchBooleanCampaign({identity},reply(body))).state,'paused','finished children do not invent a parent terminal');
 for(const selection of [{identity:'short'},{identity,pin:''},{identity,pin:1},{identity:identity.toUpperCase()+'X'}])assert.throws(()=>campaignSelection(selection));
});
test('comparison labels receipt checks separately and opens exact pinned details',()=>{
 const component=readFileSync(new URL('../src/lib/BooleanCampaign.svelte',import.meta.url),'utf8');
 assert.match(component,/All eight declared intraday timeframes/);
 assert.match(component,/it does not open every child body/);
 assert.match(component,/An observed owner is not a progress heartbeat/);
 assert.match(component,/load\(\{identity:body.identity,pin:body.pin\}\)/);
 assert.equal((component.match(/initialCompletion=\{detail.completion\}/g)??[]).length,2);
 assert.match(component,/Finishing a finite catalog does not exhaust every Boolean expression/);
});

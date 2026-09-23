import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fetchBooleanLater, laterSelection } from '../src/lib/boolean-oos.js';
const identity='12'.repeat(32),completion='34'.repeat(32),parent='56'.repeat(32),pin='78'.repeat(32);
/** @returns {any} */
function grid(side='long'){return {side,resolution:pin,execution:pin,feed:pin,commit:pin,calendar:pin,cost_model:pin,bars:'100',cells:'1',horizon_bars:'5',stop_count:'1',target_count:'1',trail_count:'1',requested_stop_count:'1',requested_target_count:'1',requested_trail_count:'1',max_pairs:'100',max_cells:'100',max_levels_per_axis:'1',max_ambiguous_bars:'0',max_gap_fills:'0',first_micros:'1',last_micros:'60000001',ratio_min_hundredths:'100',ratio_max_hundredths:'10000',execution_seconds:'60',range_rounding:'floor',selector:'pessimistic-total',forced_stop:{kind:'disabled',ppm:null}};}
/** @returns {any} */
function coordinate(index='0'){return {index,identity:parent,run:pin,program_index:'0',ordinal:'0',side:index==='0'?'long':'short',expression:'(0 | !(1))',support_sessions:'1',execution_refusal_bits:'0',execution_refusals:[],truth:{evaluated:'9007199254740993',hits:'1',misses:'9007199254740992',unknown:'0'},cell:{trades:'1',wins:'1',pessimistic:'10',optimistic:'12',fill_cost:'0',stopped:'0',targeted:'0',trailed_stop:'0',trailed_profit:'0',timed_out:'1',ambiguous_bars:'0',gapped:'0',stop:null,target:null,tsl:null,ttp:null},levels:{stop_ppm:null,target_ppm:null,tsl_ppm:null,ttp_arm_ppm:null,ttp_trail_ppm:null}};}
/** @returns {any} */
function pair(index='0'){const training=coordinate(index),later=structuredClone(training);later.run=identity;later.cell.pessimistic='-9007199254740993';later.cell.optimistic='-9007199254740993';later.cell.wins='0';return {index,training,later};}
/** @returns {any} */
function page(){return {schema_version:1,status:'saved',authority:'authenticated-later-comparison-observation',identity,completion,parent:{identity:parent,completion:pin},cohort:parent,instrument:'NSE-NIFTY',cash:false,membership_digest:'00'.repeat(32),program_count:'1',coordinate_count:'2',training_session_count:'1',session_count:'1',grids:[grid(),grid('short')],later:{source:identity,execution:completion,first_micros:'99900000000',last_micros:'100440000000',bars:'10',from:'1970-01',to:'1970-01'},kind:'coordinates',candidate:null,offset:'0',limit:32,total:'2',next:null,page_complete:true,selected:null,rows:[pair(),pair('1')],admitted_bytes:'10000',refusal:null,scope:'Saved paired observations only'};}
/** @param {any} body */
const response=body=>async()=>({ok:true,json:async()=>body});
/** @returns {any} */
function trades(){return {...page(),kind:'trades',candidate:'0',selected:pair(),total:'1',rows:[{index:'0',signal_bar:'1',entry_bar:'1',exit_bar:'2',best:'-9007199254740993',worst:'-9007199254740993',entry_micros:'99960000000',exit_micros:'100020000000',adverse_ppm:'1',favourable_ppm:'0',adverse_paisa:'2',favourable_paisa:'0'}]};}
test('later comparison preserves exact paired populations frozen exits and original pins',async()=>{
 const body=page();let url='';const got=await fetchBooleanLater({identity,completion},async value=>{url=value;return {ok:true,json:async()=>body};});
 assert.match(url,/^\/boolean-oos.json\?/);assert.match(url,/completion=3434/);assert.deepEqual(got.parent,{identity:parent,completion:pin});
 assert.equal(got.rows[0].training.truth.evaluated,'9007199254740993');assert.equal(got.rows[0].later.cell.pessimistic,'-9007199254740993');assert.equal(got.rows.length,2);
 assert.equal((await fetchBooleanLater({identity,completion,kind:'trades',candidate:'0'},response(trades()))).rows[0].worst,'-9007199254740993');
});
test('changed pins original settings periods and partial pages refuse instead of replacing evidence',async()=>{
 for(const change of [(/** @type {any} */ b)=>b.identity=parent,(/** @type {any} */ b)=>b.completion=pin,(/** @type {any} */ b)=>b.parent.completion='',
  (/** @type {any} */ b)=>b.rows.pop(),(/** @type {any} */ b)=>b.next='0',(/** @type {any} */ b)=>b.rows[0].later.run=b.rows[0].training.run,
  (/** @type {any} */ b)=>b.rows[0].later.expression='0',(/** @type {any} */ b)=>{b.rows[0].later.cell.stop='0';b.rows[0].later.levels.stop_ppm='1000';},
  (/** @type {any} */ b)=>b.later.first_micros='60000000',(/** @type {any} */ b)=>b.later.first_micros='99900000001',
  (/** @type {any} */ b)=>b.coordinate_count='1',(/** @type {any} */ b)=>b.rows[0].training.truth.hits=1]){
   const body=page();change(body);await assert.rejects(fetchBooleanLater({identity,completion},response(body)));
 }
 for(const status of [429,503])await assert.rejects(fetchBooleanLater({identity},async()=>({ok:false,status})),new RegExp(String(status)));
});
test('later trades remain inside exact minute same-day execution bounds and before deadline',async()=>{
 const boundary=trades();boundary.later.last_micros='121200000000';boundary.rows[0].exit_micros='121140000000';
 assert.equal((await fetchBooleanLater({identity,completion,kind:'trades',candidate:'0'},response(boundary))).rows[0].exit_micros,'121140000000','15:09 bar closes at15:10');
 boundary.rows[0].exit_micros='121200000000';await assert.rejects(fetchBooleanLater({identity,completion,kind:'trades',candidate:'0'},response(boundary)),'15:10 bar closes too late');
 for(const change of [(/** @type {any} */ b)=>b.selected.index='1',(/** @type {any} */ b)=>b.rows[0].exit_bar='10',
  (/** @type {any} */ b)=>b.rows[0].entry_micros='99960000001',(/** @type {any} */ b)=>b.rows[0].exit_micros='100500000000',
  (/** @type {any} */ b)=>{b.later.last_micros='135000000000';b.rows[0].exit_micros='121260000000';}]){
    const body=trades();change(body);await assert.rejects(fetchBooleanLater({identity,completion,kind:'trades',candidate:'0'},response(body)));
 }
});
test('zero later trades retain their original comparison and every zero session',async()=>{
 const selected=pair();Object.assign(selected.later.cell,{trades:'0',wins:'0',timed_out:'0',pessimistic:'0',optimistic:'0'});selected.later.execution_refusal_bits='4';selected.later.execution_refusals=['No completed trades'];
 const body={...trades(),selected,total:'0',rows:[]};assert.equal((await fetchBooleanLater({identity,completion,kind:'trades',candidate:'0'},response(body))).rows.length,0);
 const sessions={...body,kind:'sessions',total:'1',rows:[{index:'0',day:'1',return_paisa:'0',trades:'0',wins:'0'}]};assert.equal((await fetchBooleanLater({identity,completion,kind:'sessions',candidate:'0'},response(sessions))).rows[0].trades,'0');
 sessions.rows[0].day='0';await assert.rejects(fetchBooleanLater({identity,completion,kind:'sessions',candidate:'0'},response(sessions)));
});
test('exact later continuation and selectors cannot skip or use a numeric unsafe coordinate',async()=>{
 const first={...page(),limit:1,next:'1',rows:[pair()]};assert.equal((await fetchBooleanLater({identity,limit:1},response(first))).next,'1');
 const last={...first,offset:'1',next:null,rows:[pair('1')]};assert.equal((await fetchBooleanLater({identity,completion,offset:'1',limit:1},response(last))).rows[0].index,'1');
 last.rows[0].index='0';await assert.rejects(fetchBooleanLater({identity,completion,offset:'1',limit:1},response(last)));
 for(const selection of [{identity,offset:'1'},{identity,kind:'trades',candidate:'0'},{identity,completion,candidate:1},{identity,completion,limit:257},{identity,completion,offset:'01'}])assert.throws(()=>laterSelection(selection));
});
test('later view states cost and source limits and opens the exact original receipt',()=>{
 const component=readFileSync(new URL('../src/lib/BooleanLater.svelte',import.meta.url),'utf8');
 assert.match(component,/initialCompletion=\{original.completion\}/);assert.match(component,/Every later session, including zero-trade days/);
 assert.match(component,/Returns exclude costs/);assert.match(component,/not current raw-market files/);assert.match(component,/Stored source is the projected execution-fill index/);
 assert.match(component,/does not establish statistical admission, full-campaign completion/);
 const campaign=readFileSync(new URL('../src/lib/BooleanCampaign.svelte',import.meta.url),'utf8');assert.match(campaign,/Not linked in this campaign snapshot/);
});

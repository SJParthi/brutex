import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fetchBooleanCatalog, catalogSelection, catalogDay } from '../src/lib/boolean-catalog.js';
import { evidenceComparison } from '../src/lib/sweep-evidence.js';
const identity='42'.repeat(32), completion='ab'.repeat(32);
/** @returns {any} */
function grid(side='long') {
 return {side,resolution:completion,execution:completion,feed:completion,commit:completion,calendar:completion,cost_model:completion,bars:'100',cells:'1',horizon_bars:'5',stop_count:'1',target_count:'1',trail_count:'1',requested_stop_count:'1',requested_target_count:'1',requested_trail_count:'1',max_pairs:'100',max_cells:'100',max_levels_per_axis:'1',max_ambiguous_bars:'0',max_gap_fills:'0',first_micros:'1',last_micros:'60000001',ratio_min_hundredths:'100',ratio_max_hundredths:'10000',execution_seconds:'60',range_rounding:'floor',selector:'pessimistic-total',forced_stop:{kind:'disabled',ppm:null}};
}
/** @returns {any} */
function coordinate(index='0',side='long',ordinal='0') {
 return {index,identity,run:completion,program_index:'0',ordinal,side,expression:'(0 | !(1))',support_sessions:'1',execution_refusal_bits:'0',execution_refusals:[],truth:{evaluated:'18446744073709551615',hits:'1',misses:'9007199254740993',unknown:'18437736874454810621'},cell:{trades:'1',wins:'1',pessimistic:'10',optimistic:'12',fill_cost:'2',stopped:'0',targeted:'0',trailed_stop:'0',trailed_profit:'0',timed_out:'1',ambiguous_bars:'0',gapped:'0',stop:null,target:null,tsl:null,ttp:null},levels:{stop_ppm:null,target_ppm:null,tsl_ppm:null,ttp_arm_ppm:null,ttp_trail_ppm:null}};
}
/** @returns {any} */
function page(kind='coordinates') {
 return {schema_version:1,status:'saved',authority:'authenticated-catalog-observation',identity,completion,cohort:identity,kind,candidate:null,side:null,axis:null,offset:'0',limit:32,total:kind==='coordinates'?'2':'1',next:null,page_complete:true,program_count:'1',coordinate_count:'2',session_count:'1',instrument:'NSE-NIFTY',cash:false,membership_digest:'00'.repeat(32),grids:[grid(),grid('short')],selected:null,rows:[coordinate(),coordinate('1','short')],refusal:null,scope:'Finite supplied catalog only.'};
}
const response=(/** @type {any} */ body)=>async()=>({ok:true,json:async()=>body});
test('catalog preserves exact integers and pins every detail request to its own completion',async()=>{
 const body=page();let url='';const actual=await fetchBooleanCatalog({identity},async value=>{url=value;return {ok:true,json:async()=>body};});
 assert.match(url,/^\/boolean-candidates\.json\?/);assert.equal(actual.rows[0].truth.misses,'9007199254740993');assert.equal(actual.rows[0].truth.evaluated,'18446744073709551615');
 const trades={...page('trades'),candidate:'0',selected:coordinate(),rows:[{index:'0',signal_bar:'9007199254740993',entry_bar:'9007199254740994',exit_bar:'9007199254740995',best:'12',worst:'10',entry_micros:'1',exit_micros:'60000001',adverse_ppm:'0',favourable_ppm:'2',adverse_paisa:'0',favourable_paisa:'2'}]};
 await fetchBooleanCatalog({identity,completion,kind:'trades',candidate:'0'},async value=>{url=value;return {ok:true,json:async()=>trades};});
 assert.match(url,/completion=abab/);assert.match(url,/candidate=0/);
});
test('missing malformed refused or incomplete pages never become successful empty evidence',async()=>{
 for(const change of [(/** @type {any} */ b)=>b.status='missing',(/** @type {any} */ b)=>b.authority='institutional-approved',(/** @type {any} */ b)=>b.completion=identity,(/** @type {any} */ b)=>b.identity=completion,(/** @type {any} */ b)=>b.rows=[],(/** @type {any} */ b)=>b.next='0',(/** @type {any} */ b)=>b.page_complete=false,(/** @type {any} */ b)=>b.rows[0].truth.hits=1,(/** @type {any} */ b)=>b.rows[0].truth.hits='2',(/** @type {any} */ b)=>b.rows[0].cell.trades='2',(/** @type {any} */ b)=>b.rows[0].levels.stop_ppm='1000',(/** @type {any} */ b)=>b.grids[1].side='long']) {
  const body=page();change(body);await assert.rejects(fetchBooleanCatalog({identity,completion},response(body)));
 }
 await assert.rejects(fetchBooleanCatalog({identity},async()=>({ok:false,status:503})),/503/);
 await assert.rejects(fetchBooleanCatalog({identity},async()=>({ok:false,status:429})),/429/);
});
test('zero trade setting still exposes its own zero-inclusive sessions without a winner substitute',async()=>{
 const selected=coordinate();selected.cell.trades='0';selected.cell.wins='0';selected.cell.timed_out='0';selected.cell.pessimistic='0';selected.cell.optimistic='0';selected.execution_refusal_bits='4';selected.execution_refusals=['No completed trades'];
 const empty={...page('trades'),candidate:'0',selected,total:'0',rows:[]};
 assert.equal((await fetchBooleanCatalog({identity,completion,kind:'trades',candidate:'0'},response(empty))).rows.length,0);
 const sessions={...page('sessions'),candidate:'0',selected,rows:[{index:'0',day:'-1',return_paisa:'0',trades:'0',wins:'0'}]};
 assert.equal((await fetchBooleanCatalog({identity,completion,kind:'sessions',candidate:'0'},response(sessions))).rows[0].trades,'0');
 sessions.selected.index='1';await assert.rejects(fetchBooleanCatalog({identity,completion,kind:'sessions',candidate:'0'},response(sessions)));
});
test('grid inputs and program pages use bounded exact selectors and reject wrong rows',async()=>{
 const body={...page('grid'),side:'short',axis:'requested-stop',rows:[{index:'0',numerator:'1',denominator:'2'}]};
 assert.equal((await fetchBooleanCatalog({identity,completion,kind:'grid',side:'short',axis:'requested-stop'},response(body))).rows[0].denominator,'2');
 body.rows[0].denominator='0';await assert.rejects(fetchBooleanCatalog({identity,completion,kind:'grid',side:'short',axis:'requested-stop'},response(body)));
 for(const selection of [{identity,offset:'1'},{identity,kind:'trades',candidate:'0'},{identity,completion,kind:'grid',side:'long'},{identity,completion,limit:257},{identity,completion,offset:1},{identity,completion,kind:'programs',candidate:'0'}])assert.throws(()=>catalogSelection(selection));
 const programs={...page('programs'),rows:[{index:'0',expression:'(0 | !(1))'}]};assert.equal((await fetchBooleanCatalog({identity,completion,kind:'programs'},response(programs))).rows[0].expression,'(0 | !(1))');
});
test('offset continuation is complete and its candidate identity cannot switch',async()=>{
 const body={...page(),coordinate_count:'33',total:'33',next:'32',rows:Array.from({length:32},(_,index)=>coordinate(String(index),'long',String(index)))};body.grids[0].cells='32';
 const first=await fetchBooleanCatalog({identity},response(body));assert.equal(first.next,'32');
 const last={...body,offset:'32',next:null,rows:[coordinate('32','short')]};assert.equal((await fetchBooleanCatalog({identity,completion,offset:'32'},response(last))).rows[0].index,'32');
 last.rows[0].index='31';await assert.rejects(fetchBooleanCatalog({identity,completion,offset:'32'},response(last)));
});
test('catalog navigation is limited to its recorded operation and keeps authority limitations visible',()=>{
 const evidence=readFileSync(new URL('../src/lib/SweepEvidence.svelte',import.meta.url),'utf8');
 assert.match(evidence,/\{#if e\.operation === 'boolean-candidates'\}/);assert.match(evidence,/<BooleanCatalog initialIdentity=\{identity\}/);
 const component=readFileSync(new URL('../src/lib/BooleanCatalog.svelte',import.meta.url),'utf8');
 assert.match(component,/not an exhaustive search/);assert.match(component,/separate receipts and linked catalog completions/);assert.match(component,/no winner trades are substituted/);
 assert.match(component,/Stored source \/ entry \/ exit indices/);assert.match(component,/Stored source is the projected execution-fill index/);assert.match(component,/not its source-series index/);assert.doesNotMatch(component,/Signal \/ entry \/ exit bars/);
 assert.match(catalogDay('-1'),/31.*Dec.*1969/);assert.equal(catalogDay('9223372036854775807'),'IST day 9223372036854775807');assert.equal(catalogDay('01'),'Unavailable');
 const comparison=evidenceComparison({status:'saved',rows:[],evidence:{operation:'boolean-candidates',completion:'completed'}});
 const trades=comparison.find(row=>row.area==='Exact trade replay');assert.equal(trades?.status,'Not verified in this summary');assert.match(trades?.detail??'',/Open the saved Boolean catalog/);
});

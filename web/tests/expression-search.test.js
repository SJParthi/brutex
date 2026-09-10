import test from 'node:test';
import assert from 'node:assert/strict';
import { fetchExpressionSearch } from '../src/lib/expression-search.js';
const identity='ab'.repeat(32), child='cd'.repeat(32), seal='ef'.repeat(32);
const snapshot={sequence:'9007199254740993',seal},next={sequence:'2',seal};
const response=(/** @type {any} */ body)=>async()=>({ok:true,json:async()=>body});
/** @returns {any} */
const page=()=>({schema_version:1,status:'observed',identity,snapshot,state:'writer-observed',writer_observed:true,exhausted:false,work:'18446744073709551615',candidates:'3',qualifying:'1',signal_rows:'30',interrupted_reservations:'1',limit:8,links:8,next,page_complete:true,authority:'saved-receipts-and-grammar-transitions',rows:[],refusal:null});
test('zero-candidate progress pages preserve exact counters and pinned continuation',async()=>{
 const first=await fetchExpressionSearch({identity},response(page()));assert.equal(first.work,'18446744073709551615');assert.deepEqual(first.next,next);
 let url='';const second={...page(),links:1,next:null};
 await fetchExpressionSearch({identity,snapshot,cursor:next},async(value)=>{url=value;return {ok:true,json:async()=>second};});
 assert.match(url,/snapshot=9007199254740993/);assert.match(url,/cursor=2/);
});
test('child identity and expression links remain exact while malformed pages refuse',async()=>{
 const body=page();body.rows=[{ordinal:'3',identity:child,attempt:'9007199254740994',expression:'(0 | !(1))',mask_words:['3','0','0','0','0','0'],signals:{evaluated:'10',hits:'2',misses:'3',unknown:'5'},capture_digest:seal}];
 const loaded=await fetchExpressionSearch({identity},response(body));assert.equal(loaded.rows[0].attempt,'9007199254740994');assert.equal(loaded.rows[0].capture_digest,seal);
 for(const change of [(/** @type {any} */ b)=>b.snapshot.seal=identity,(/** @type {any} */ b)=>b.rows[0].signals.hits='3',(/** @type {any} */ b)=>b.rows[0].ordinal='4',(/** @type {any} */ b)=>b.page_complete=false,(/** @type {any} */ b)=>b.next={...snapshot},(/** @type {any} */ b)=>b.state='completed',(/** @type {any} */ b)=>b.rows[0].mask_words[0]=3]){
 const broken=structuredClone(body);change(broken);await assert.rejects(fetchExpressionSearch({identity,snapshot},response(broken)));
 }
});
test('missing searches and exhausted cursors never become complete historical assurance',async()=>{
 const missing={schema_version:1,status:'missing',identity,why:'No saved search.',rows:[],refusal:null};assert.equal((await fetchExpressionSearch({identity},response(missing))).status,'missing');
 await assert.rejects(fetchExpressionSearch({identity,snapshot},response(missing)));
 const exhausted={...page(),state:'exhausted-observed',exhausted:true};assert.equal((await fetchExpressionSearch({identity},response(exhausted))).state,'exhausted-observed');
 await assert.rejects(fetchExpressionSearch({identity,cursor:next},response(page())));
 await assert.rejects(fetchExpressionSearch({identity},async()=>({ok:false,status:503})),/503/);
});

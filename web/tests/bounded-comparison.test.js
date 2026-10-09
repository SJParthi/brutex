import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {settleComparison} from '../src/lib/bounded-comparison.js';
import {codeOf,effects} from './page-code-fixture.js';
const flush=()=>new Promise(resolve=>setImmediate(resolve));

test('complete-frontier groups stay within two active reads and preserve independent refusals/order',async()=>{
 /** @type {(()=>void)[]} */ const releases=[];let active=0,peak=0;
 /** @type {number[]} */ const seen=[];
 const task=settleComparison([0,1,2,3,4],2,async item=>{active++;peak=Math.max(peak,active);seen.push(item);await /** @type {Promise<void>} */(new Promise(resolve=>{releases[item]=resolve;}));active--;if(item===1)throw new Error('exact group refusal');return item*10;},new AbortController().signal);
 assert.deepEqual(seen,[0,1]);releases[1]();await flush();assert.deepEqual(seen,[0,1,2]);releases[0]();await flush();releases[2]();await flush();releases[3]();releases[4]();
 const rows=await task;assert.equal(peak,2);assert.equal(rows[1].status,'rejected');assert.match(rows[1].reason.message,/exact group refusal/);assert.deepEqual(rows.filter(row=>row.status==='fulfilled').map(row=>row.value),[0,20,30,40]);
});

test('cancellation never starts queued groups or publishes a partial settled population',async()=>{
 const controller=new AbortController();
 /** @type {(()=>void)[]} */ const releases=[];
 /** @type {number[]} */ const seen=[];
 const task=settleComparison([0,1,2,3],2,async item=>{seen.push(item);await /** @type {Promise<void>} */(new Promise(resolve=>releases.push(resolve)));return item;},controller.signal);
 controller.abort(new Error('comparison closed'));const refused=assert.rejects(task,/comparison closed/);releases.forEach(resolve=>resolve());await refused;assert.deepEqual(seen,[0,1]);
});

test('invalid concurrency and already-cancelled work issue no requests',async()=>{
 let calls=0;const controller=new AbortController();controller.abort();
 for(const limit of [0,9,NaN])await assert.rejects(settleComparison([1],limit,async()=>{calls++;},controller.signal));
 await assert.rejects(settleComparison([1],2,async()=>{calls++;},controller.signal));assert.equal(calls,0);
});

test('actual comparison is opt-in, bounded, aborted on close/replacement, and still reads complete frontiers',()=>{
 // MATCHED AGAINST CODE, NOT TEXT (P19-05, D-2564). Every comment is blanked
 // first, so a pattern kept alive inside `/* … */` or `<!-- … -->` no longer
 // satisfies it.
 const page=codeOf(readFileSync(new URL('../src/routes/backtest/+page.svelte',import.meta.url),'utf8'));
 assert.match(page,/comparisonRequested = \$state\(false\)/);
 assert.match(page,/settleComparison\(selected, 2,/);assert.match(page,/fetchCompleteFrontier\(run.identity, \(url\) =>\s*ask_\(url, \{ signal: controller.signal \}\)/);
 assert.match(page,/Load saved prefix comparison/);
});

test('the board effect is EXECUTED: it fetches only when asked, and its cleanup aborts and invalidates (P19-05)',()=>{
 // The finding's mutation commented the cleanup out -- `/* return () => {…} */`
 // -- and the old text match still passed. Here the page's own effect callback
 // runs, so a cleanup that is not returned is a cleanup that is not there.
 const source=readFileSync(new URL('../src/routes/backtest/+page.svelte',import.meta.url),'utf8');
 const board=effects(source).filter(body=>body.includes('fetchBoard(rankableRuns)'));
 assert.equal(board.length,1,'exactly one effect owns the comparison board');
 const run=new Function('requested','controller',`
  let comparisonRequested=requested,boardSeq=0,boardAbort=controller,board=null;
  const rankableRuns=['row'],fetched=[];
  const fetchBoard=(rows)=>{fetched.push(rows);};
  const cleanup=(${board[0]})();
  return {cleanup,fetched,seq:()=>boardSeq,board:()=>board};`);
 for(const requested of [true,false]){
  const controller=new AbortController(),app=run(requested,controller);
  assert.deepEqual(app.fetched,requested?[['row']]:[],'fetched only when the operator asked');
  assert.equal(app.seq(),requested?0:1,'closing invalidates at once');
  assert.equal(controller.signal.aborted,!requested,'closing aborts at once');
  if(!requested)assert.deepEqual(app.board(),{phase:'idle',groups:[],why:''});
  assert.equal(typeof app.cleanup,'function','the effect returns its cleanup');
  app.cleanup();
  assert.equal(app.seq(),requested?1:2,'teardown/replacement invalidates every in-flight batch');
  assert.equal(controller.signal.aborted,true,'teardown/replacement aborts the in-flight reads');
 }
});

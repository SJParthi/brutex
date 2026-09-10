import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {settleComparison} from '../src/lib/bounded-comparison.js';
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
 const page=readFileSync(new URL('../src/routes/backtest/+page.svelte',import.meta.url),'utf8');
 assert.match(page,/comparisonRequested = \$state\(false\)/);assert.match(page,/if \(comparisonRequested\) void fetchBoard\(rankableRuns\)/);
 assert.match(page,/settleComparison\(selected, 2,/);assert.match(page,/fetchCompleteFrontier\(run.identity, \(url\) =>\s*ask_\(url, \{ signal: controller.signal \}\)/);
 assert.match(page,/return \(\) => \{ boardSeq \+= 1; boardAbort\?\.abort\(\); \}/);assert.match(page,/Load saved prefix comparison/);
});

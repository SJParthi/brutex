import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createPageRequests,watchVisible} from '../src/lib/page-requests.js';
const flush=()=>new Promise(resolve=>setImmediate(resolve));
function clock(){
 let id=0;
 /** @type {Map<number,{work:()=>void,delay:number}>} */ const timers=new Map();
 return {timers,
  /** @param {()=>void} work @param {number} delay */
  schedule(work,delay){timers.set(++id,{work,delay});return id;},
  /** @param {number} key */ cancel(key){timers.delete(key);},
  tick(){const next=timers.entries().next().value;if(next){timers.delete(next[0]);next[1].work();}}
 };
}
function deferred(){
 /** @type {()=>void} */ let resolve=()=>{throw new Error('deferred was not initialized');};
 const promise=/** @type {Promise<void>} */(new Promise(done=>{resolve=done;}));
 return {promise,resolve};
}

test('a response resolving after page disposal cannot publish or resurrect its status poll',async()=>{
 const timer=clock(),reads=createPageRequests(timer),reply=deferred();
 /** @type {string[]} */ const seen=[];
 /** @type {AbortSignal|undefined} */ let signal;
 const job=reads.run(async ticket=>{signal=ticket.signal;await reply.promise;if(!ticket.current())return;seen.push('done');reads.schedule(async()=>{seen.push('poll');},2000);});
 reads.dispose();assert.equal(signal?.aborted,true);reply.resolve();await job;timer.tick();assert.equal(seen.length,0);assert.equal(timer.timers.size,0);
 reads.schedule(async()=>{seen.push('resurrected');},0);await reads.run(async()=>{seen.push('late');});assert.deepEqual(seen,[]);
});

test('replacement waits for an abort-ignoring request to drain and discards its stale result',async()=>{
 const timer=clock(),reads=createPageRequests(timer),old=deferred(),fresh=deferred();let active=0,peak=0;
 /** @type {string[]} */ const seen=[];
 /** @param {ReturnType<typeof deferred>} reply @param {string} name @returns {import('../src/lib/page-requests.js').Work} */
 const work=(reply,name)=>async ticket=>{active++;peak=Math.max(peak,active);await reply.promise;active--;if(ticket.current())seen.push(name);};
 const prior=reads.run(work(old,'old'));reads.cancel();await reads.run(work(fresh,'fresh'));assert.equal(active,1);
 old.resolve();await prior;assert.equal(timer.timers.size,1);timer.tick();fresh.resolve();await flush();assert.equal(peak,1);assert.deepEqual(seen,['fresh']);reads.dispose();
});

test('poll delay starts after completion and repeated scheduling retains one timer',async()=>{
 const timer=clock(),reads=createPageRequests(timer),reply=deferred();let count=0;
 const job=reads.run(async()=>{count++;reads.schedule(async()=>{count++;},2000);reads.schedule(async()=>{count++;},3000);await reply.promise;});
 assert.equal(timer.timers.size,0);reply.resolve();await job;assert.equal(timer.timers.size,1);assert.equal([...timer.timers.values()][0].delay,3000);timer.tick();await flush();assert.equal(count,2);reads.dispose();
});

test('an already queued timer callback cannot restart a cancelled request generation',async()=>{
 const timer=clock(),reads=createPageRequests(timer);let calls=0;
 reads.schedule(async()=>{calls++;},2000);const queued=[...timer.timers.values()][0].work;
 reads.cancel();await reads.run(async()=>{calls++;});queued();await flush();
 assert.equal(calls,1);assert.equal(timer.timers.size,0);reads.dispose();
});

test('health watching is single-flight, pauses hidden, resumes once and cancels on teardown',async()=>{
 const timer=clock();let visible=true,unsubscribed=false;
 /** @type {{ticket:import('../src/lib/page-requests.js').ReadTicket,reply:ReturnType<typeof deferred>}[]} */ const requests=[];
 /** @type {number[]} */ const seen=[];
 /** @type {()=>void} */ let wake=()=>{throw new Error('listener not initialized');};
 const stop=watchVisible(async ticket=>{const reply=deferred();requests.push({ticket,reply});await reply.promise;if(ticket.current())seen.push(requests.length);},8000,{...timer,visible:()=>visible,listen(fn){wake=fn;return()=>{unsubscribed=true;};}});
 assert.equal(requests.length,1);for(let n=0;n<5;n++)timer.tick();assert.equal(requests.length,1);
 visible=false;wake();assert.equal(requests[0].ticket.signal.aborted,true);visible=true;wake();assert.equal(requests.length,1);
 requests[0].reply.resolve();await flush();timer.tick();assert.equal(requests.length,2);assert.deepEqual(seen,[]);
 requests[1].reply.resolve();await flush();assert.equal(timer.timers.size,1);visible=false;wake();assert.equal(timer.timers.size,0);
 visible=true;wake();assert.equal(requests.length,3);stop();requests[2].reply.resolve();await flush();assert.equal(unsubscribed,true);assert.equal(timer.timers.size,0);assert.deepEqual(seen,[2]);
});

test('the actual layout and legacy status stream use owned cancellation and guarded response publication',()=>{
 const layout=readFileSync(new URL('../src/routes/+layout.svelte',import.meta.url),'utf8');
 assert.match(layout,/watchVisible\(probe, PROBE_MS/);assert.doesNotMatch(layout,/setInterval\(probe/);assert.match(layout,/signal: ticket.signal/);
 assert.match(layout,/refreshProbe = watching.refresh/);assert.match(layout,/onclick=\{\(\) => refreshProbe\(\)\}/);
 const page=readFileSync(new URL('../src/routes/backtest/+page.svelte',import.meta.url),'utf8');
 const status=page.slice(page.indexOf('async function adoptRunning'),page.indexOf('async function startSweep'));
 assert.equal((status.match(/if \(!ticket.current\(\)\) return;/g)??[]).length,6);
 assert.doesNotMatch(page,/setTimeout\(pollSweep/);assert.match(page,/statusRequests.dispose\(\)/);
});

test('an absent or empty ordinary ledger does not claim that separately saved Boolean research is absent',()=>{
 const page=readFileSync(new URL('../src/routes/backtest/+page.svelte',import.meta.url),'utf8');
 assert.doesNotMatch(page,/Nothing has been swept yet|Every run that finishes appends/);
 assert.equal((page.match(/Ordinary sweep ledger has no rows/g)??[]).length,2);
 assert.equal((page.match(/href="#saved-boolean-research"/g)??[]).length,2);
 assert.match(page,/id="saved-boolean-research"/);
 assert.match(page,/Boolean research is saved separately; this response does not say/);
 assert.match(page,/does not mean there are no saved research results/);
});

test('the shared census and optional surface cannot publish for a replaced feed or unmounted page',()=>{
 const page=readFileSync(new URL('../src/routes/backtest/+page.svelte',import.meta.url),'utf8');
 const catalog=page.slice(page.indexOf('async function loadCatalog'),page.indexOf('const heldNow ='));
 assert.match(catalog,/const ticket = catalogGate.begin\(feed\)/);
 assert.match(catalog,/await readStoreCensus\(feed\);\s*if \(!catalogGate.admits\(ticket, activeFeed\)\) return/);
 assert.equal((catalog.match(/if \(!catalogGate.admits\(ticket, activeFeed\)\) return/g)??[]).length,5);
 assert.match(page,/return \(\) => catalogGate.invalidate\(\)/);
});

import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createCampaignMonitor } from '../src/lib/campaign-monitor.js';
const identity='12'.repeat(32),other='34'.repeat(32),pin='56'.repeat(32);
const flush=async()=>{for(let i=0;i<8;i++)await Promise.resolve();};
/** @param {string} state @param {string} [sequence] @returns {any} */
const snapshot=(state,sequence='1')=>({identity,pin,state,sequence});
function clock(){let next=0;const timers=new Map();return {timers,schedule:(/** @type {()=>void} */ work,/** @type {number} */ delay)=>{timers.set(++next,{work,delay});return next;},cancel:(/** @type {number} */ id)=>timers.delete(id),tick:()=>{assert.equal(timers.size,1);const [key,value]=[...timers.entries()][0];timers.delete(key);value.work();}};}
test('automatic campaign refresh follows newer snapshots sequentially and stops at every terminal or pause',async()=>{
 for(const terminal of ['paused','refused','completed']){
  const timer=clock(),seen=/** @type {any[]} */ ([]),calls=/** @type {any[]} */ ([]);let index=0;
  const monitor=createCampaignMonitor(async asked=>{calls.push(asked);return snapshot(index++===0?'running':terminal,String(index));},state=>seen.push(state),timer);
  monitor.start({identity,pin});await flush();assert.equal(seen.at(-1).watching,true);assert.equal(timer.timers.size,1);
  timer.tick();await flush();assert.equal(calls[0].pin,pin);assert.equal(calls[1].pin,null);assert.equal(seen.at(-1).body.state,terminal);assert.equal(seen.at(-1).watching,false);assert.equal(timer.timers.size,0);monitor.stop();
 }
});
test('identity changes abort and drain an old request before starting the new one; late completion cannot publish',async()=>{
 const timer=clock(),calls=/** @type {any[]} */ ([]),seen=/** @type {any[]} */ ([]),release=/** @type {((value:any)=>void)[]} */ ([]);
 const monitor=createCampaignMonitor((asked,signal)=>new Promise(resolve=>{calls.push({asked,signal});release.push(resolve);}),state=>seen.push(state),timer);
 monitor.start({identity});monitor.start({identity:other});assert.equal(calls.length,1);assert.equal(calls[0].signal.aborted,true);
 release[0](snapshot('completed'));await flush();assert.equal(calls.length,2);assert.equal(calls[1].asked.identity,other);assert.ok(!seen.some(state=>state.body));
 release[1]({...snapshot('completed'),identity:other});await flush();assert.equal(seen.at(-1).body.identity,other);
 monitor.stop();assert.equal(timer.timers.size,0);
});
test('unmount cancellation and invalid replacement selectors cannot resurrect an older campaign',async()=>{
 for(const invalid of [false,true]){
  const timer=clock(),seen=/** @type {any[]} */ ([]);let release=/** @type {(body:any)=>void} */ (()=>{});
  const monitor=createCampaignMonitor(()=>new Promise(resolve=>{release=resolve;}),state=>seen.push(state),timer);
  monitor.start({identity});if(invalid)assert.throws(()=>monitor.start({identity:'bad'}));else monitor.stop();
  release(snapshot('running'));await flush();assert.ok(!seen.some(state=>state.body));assert.equal(timer.timers.size,0);
 }
});
test('three consecutive refresh failures stop retries and retain last acknowledged snapshot explicitly',async()=>{
 const timer=clock(),seen=/** @type {any[]} */ ([]);let count=0;
 const monitor=createCampaignMonitor(async()=>{if(count++===0)return snapshot('running');throw new Error('busy');},state=>seen.push(state),timer);
 monitor.start({identity});await flush();
 for(let n=1;n<=3;n++){timer.tick();await flush();assert.equal(seen.at(-1).body.sequence,'1');assert.equal(seen.at(-1).why,'busy');assert.equal(seen.at(-1).failures,n);assert.equal(seen.at(-1).watching,n<3);}
 assert.equal(count,4);assert.equal(timer.timers.size,0);monitor.stop();
});
test('same-sequence replacement or history rollback never silently replaces an observed snapshot',async()=>{
 for(const replacement of [{...snapshot('running'),pin:other},snapshot('running','0')]){
  const timer=clock(),seen=/** @type {any[]} */ ([]);let count=0;
  const monitor=createCampaignMonitor(async()=>count++===0?snapshot('running'):replacement,state=>seen.push(state),timer);
  monitor.start({identity});await flush();timer.tick();await flush();assert.equal(seen.at(-1).body.pin,pin);assert.equal(seen.at(-1).body.sequence,'1');assert.match(seen.at(-1).why,/regressed or changed/);monitor.stop();
 }
});
test('URL-selected campaign watches automatically while automatic updates preserve exact open detail pins',()=>{
 const page=readFileSync(new URL('../src/routes/backtest/+page.svelte',import.meta.url),'utf8');assert.match(page,/searchParams.get\('boolean_campaign'\)/);assert.match(page,/autoLoad=\{routePage.url.searchParams.has\('boolean_campaign'\)\}/);
 const component=readFileSync(new URL('../src/lib/BooleanCampaign.svelte',import.meta.url),'utf8');assert.match(component,/state=>\{loaded=state;\}/);assert.match(component,/return \(\)=>monitor.stop\(\)/);assert.match(component,/initialCompletion=\{detail.completion\}/);
});

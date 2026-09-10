import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readDatabasePage } from '../src/lib/database-pages.js';
import { readFileSync } from 'node:fs';
import { parse } from 'svelte/compiler';

/** @param {string} month @param {number} rows @param {string} [instrument] */
const row=(month,rows,instrument='NSE-CASH-BAJAJ-AUTO')=>({key:`${instrument}:${month}`,instrument,month,timeframe:'1min',rows});
const signal=()=>new AbortController().signal;
/** @param {unknown[]} bars @param {number} [total] @param {Record<string,unknown>} [other] */
const response=(bars,total=10,other={})=>Response.json({bars,total,scanned:false,faults:null,months_read:1,months_missing:0,...other});

test('a deep time page seeks exactly its visible rows and never requests all-history extrema',async()=>{
  let calls=0;
  const out=await readDatabasePage('example',[row('2025-01',8000)],700000,8,699900,true,signal(),async(url,options)=>{
    calls++;assert.ok(options?.signal);
    const q=new URL(url,'http://local').searchParams;
    assert.equal(q.get('offset'),'100');assert.equal(q.get('limit'),'8');
    assert.equal(q.get('from'),'2025-01');assert.equal(q.get('to'),'2025-01');
    assert.equal(q.get('symbol'),'BAJAJ-AUTO');assert.equal(q.get('extremes'),'0');assert.equal(q.get('dir'),'desc');
    return response(Array.from({length:8},(_,i)=>({t:8000-i,chg:i,oichg:null})),8000);
  });
  assert.equal(calls,1);assert.equal(out[0].bars.length,8);assert.equal(out[0].paged,true);assert.equal(out[0].held,8000);
  assert.equal(out[0].bars[1].chg,1,'native predecessor changes remain on their original rows');
});

test('month boundaries, ascending order, zero-row files and exact contract tails retain the requested page',async()=>{
  /** @type {(string|null)[][]} */
  const requested=[];
  const files=[row('2025-01',10,'NSE-FNO-BANKNIFTY-2025-01-30-4800000-PE'),row('2025-02',0),row('2025-03',10)];
  const out=await readDatabasePage('example',files,8,5,0,false,signal(),async(url)=>{
    const q=new URL(url,'http://local').searchParams;requested.push([q.get('from'),q.get('offset'),q.get('limit'),q.get('contract'),q.get('dir')]);
    return response(Array.from({length:Number(q.get('limit'))},()=>({t:1})),10);
  });
  assert.deepEqual(requested,[['2025-01','8','2','2025-01-30-4800000-PE','asc'],['2025-03','0','3','','asc']]);
  assert.equal(out.reduce((n,file)=>n+file.bars.length,0),5);
});

test('bad offsets refuse before transport, malformed pages and missing months are visible failures',async()=>{
  let calls=0;const request=async()=>{calls++;return response([]);};
  for(const [first,limit,base] of [[-1,8,0],[0,0,0],[0,1001,0],[0,8,1],[Number.MAX_SAFE_INTEGER,8,0]]) {
    await assert.rejects(readDatabasePage('example',[row('2025-01',10)],first,limit,base,true,signal(),request),/database page/);
  }
  assert.equal(calls,0);
  for(const value of [response([],10),response([{},{}],10),response([{}],10,{scanned:true}),response([],0,{months_read:0,months_missing:1}),new Response('broken')]){
    const out=await readDatabasePage('example',[row('2025-01',10)],0,1,0,true,signal(),async()=>value);
    assert.equal(typeof out[0].error,'string');assert.deepEqual(out[0].bars,[]);
  }
});

test('cancellation stops queued month reads and does not return an incomplete success',async()=>{
  const controller=new AbortController();let calls=0;
  const files=Array.from({length:20},(_,i)=>row(`2025-${String(i+1).padStart(2,'0')}`,1));
  await assert.rejects(readDatabasePage('example',files,0,20,0,true,controller.signal,async()=>{
    calls++;controller.abort();return response([{}],1);
  }),/abort/i);
  assert.equal(calls,1);
});

test('the actual month cache replaces an aborted request and a late old result cannot replace or delete the newer page',async()=>{
  const source=readFileSync(new URL('../src/routes/db/+page.svelte',import.meta.url),'utf8');
  const ast=parse(source);
  const node=ast.instance?.content.body.find((/** @type {any} */ node)=>node.type==='FunctionDeclaration'&&node.id?.name==='readBarFile');
  assert.ok(node);
  const cache=new Map();
  /** @type {((value:any)=>void)[]} */
  const pending=[];
  const fetchBarFile=()=>new Promise(resolve=>pending.push(resolve));
  const read=new Function('barCache','fetchBarFile',source.slice(node.start,node.end)+'; return readBarFile;')(cache,fetchBarFile);
  const selected=row('2025-01',10), first=new AbortController(), second=new AbortController();
  const old=read('example',selected,first.signal);
  assert.equal(read('example',selected,first.signal),old);
  first.abort();
  const next=read('example',selected,second.signal);
  assert.notEqual(next,old);assert.equal(pending.length,2);
  pending[0]({error:'old cancellation'});await old;
  assert.equal(read('example',selected,second.signal),next,'old completion does not remove the newer in-flight request');
  const fresh={error:null,bars:[{t:2}]};pending[1](fresh);await next;
  assert.equal(await read('example',selected,second.signal),fresh);
});

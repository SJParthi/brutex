import { test } from 'node:test';
import assert from 'node:assert/strict';
import { prepareDatabase, instrumentMemo, createDatabasePreparation } from '../src/lib/database-preparation.js';
import { denominators, denomKey } from '../src/lib/completeness.js';
import { databasePage, generatedMonth } from './database-page-fixture.js';

const page = databasePage();
const tokens = (/** @type {any} */ row) => [row.instrument, row.sym, row.underlying, row.month];
const immediate = async () => {};

test('actual DB decoration preserves mixed-rung denominators, zero changes, contracts and unknowns across chunks', async () => {
  const rows = [
    {...generatedMonth(0), instrument:'NSE-INDEX-NIFTY', rows:20, timeframe:'1day'},
    {...generatedMonth(0), instrument:'NSE-INDEX-NIFTY', rows:7500},
    {...generatedMonth(0), instrument:'NSE-INDEX-BANKNIFTY', rows:7125},
    {...generatedMonth(0), instrument:'NSE-FNO-BANKNIFTY-2025-01-30-4800000-CE', rows:2,timeframe:'2min'},
    {...generatedMonth(0), instrument:'NSE-FNO-BANKNIFTY-2025-01-30-FUT',rows:3,timeframe:'2min'},
    {...generatedMonth(0), instrument:'NSE-CASH-BAJAJ-AUTO',rows:1,timeframe:'mystery'},
    {...generatedMonth(0), instrument:'bad', rows:0,first_ts:0,last_ts:0,chg_bps:null}
  ];
  const describe=instrumentMemo(page.instrumentDescription);
  const result=await prepareDatabase(rows,(r,d)=>page.decorateRow(r,d,describe(r.instrument)),{signal:new AbortController().signal,chunkSize:2,yieldControl:immediate,tokens});
  assert.deepEqual(result.observed,denominators(rows));
  assert.deepEqual(result.rows.map(r=>[r.short,r.sole,r.days,r.state]),[
    [0,true,20,'sole'],[0,false,20,'full'],[375,false,19,'gap'],
    [1,false,null,'gap'],[0,false,null,'full'],[0,true,null,'sole'],[7500,false,0,'gap']
  ]);
  assert.equal(result.rows[0].pct,null);assert.equal(result.rows[1].chg,0);
  assert.deepEqual([result.rows[3].underlying,result.rows[3].strike,result.rows[3].side,result.rows[3].expiry],['BANKNIFTY',4800000,'CE','2025-01-30']);
  assert.equal(result.rows[4].segRung,'futures');assert.equal(result.rows[5].underlying,'BAJAJ-AUTO');
  assert.equal(result.rows[6].firstAt,null);assert.equal(result.rows[6].lastAt,null);
  assert.deepEqual(rows.map(r=>r.rows),[20,7500,7125,2,3,1,0]);
});

test('chunked full-store prefix buckets preserve exact selected-universe and long-prefix answers',async()=>{
  const rows=[{...generatedMonth(0),instrument:'NSE-CASH-BAJAJ-AUTO'},
    {...generatedMonth(1),instrument:'nse-index-Nifty'},
    {...generatedMonth(2),instrument:'NSE-FNO-Nifty-2025-01-30-4800000-PE'}];
  const describe=instrumentMemo(page.instrumentDescription);
  const result=await prepareDatabase(rows,(r,d)=>page.decorateRow(r,d,describe(r.instrument)),{signal:new AbortController().signal,chunkSize:1,yieldControl:immediate,tokens});
  for(const typed of ['', 'N', 'NSE-', 'NIFTY', '2025-01', 'BAJAJ-AUTO', 'NO-MATCH']) {
    for(const universe of [null,new Set(['Nifty']),new Set(['BAJAJ-AUTO']),new Set()]) {
      const allowed=result.rows.filter(r=>universe===null||universe.has(r.underlying));
      const expected=allowed.filter(r=>typed===''||tokens(r).some(token=>token.toUpperCase().startsWith(typed)));
      assert.deepEqual(page.select(result.prefix,typed,allowed,universe),expected);
    }
  }
  assert.equal(result.prefix.get('N')?.length,3,'a row is included once even when multiple tokens share the prefix');
});

test('148222 generated month rows yield in bounded slices and parse each instrument once',async()=>{
  const rows=Array.from({length:148222},(_,i)=>generatedMonth(i));
  let calls=0,slice=0,maximum=0,yields=0,parses=0;
  const describe=instrumentMemo(name=>{parses++;return page.instrumentDescription(name);});
  const result=await prepareDatabase(rows,(r,d)=>{calls++;slice++;return page.decorateRow(r,d,describe(r.instrument));},{signal:new AbortController().signal,chunkSize:2048,tokens,
    yieldControl:async()=>{maximum=Math.max(maximum,slice);slice=0;yields++;}});
  assert.equal(result.rows.length,148222);assert.equal(calls,rows.length);assert.equal(parses,Math.ceil(rows.length/81));
  assert.equal(maximum,2048);assert.equal(yields,1+2*Math.ceil(rows.length/2048));
  assert.equal(result.prefix.get('NSE-')?.length,rows.length);
  assert.deepEqual(result.observed,denominators(rows));
  assert.equal(result.rows.at(-1).key,`${rows.at(-1)?.instrument}\0${rows.at(-1)?.month}\0${rows.at(-1)?.timeframe}`);
});

test('cancelled preparation never decorates or returns a partial census',async()=>{
  for(const stopAt of [0,1,3,4]) {
    const controller=new AbortController();let yields=0,calls=0;
    if(stopAt===0)controller.abort();
    const work=prepareDatabase([generatedMonth(0),generatedMonth(1),generatedMonth(2)],r=>{calls++;return r;},{signal:controller.signal,chunkSize:2,yieldControl:async()=>{if(++yields===stopAt)controller.abort();}});
    await assert.rejects(work,/abort/i);
    assert.equal(calls,stopAt===4?2:0);
  }
});

test('invalid bounds and a decoration failure refuse without an empty success',async()=>{
  for(const chunkSize of [0,8193,1.2,Infinity])await assert.rejects(prepareDatabase([],r=>r,{signal:new AbortController().signal,chunkSize,yieldControl:immediate}),/chunk/);
  await assert.rejects(prepareDatabase([generatedMonth(0)],()=>{throw new Error('exact decoration failure');},{signal:new AbortController().signal,yieldControl:immediate}),/exact decoration failure/);
});

test('initial DB pickers wait for complete held counts and preserve the reader choice',()=>{
  const emptySegments=[{key:'spot',why:'not held'},{key:'futures',why:'not held'},{key:'options',why:'not held'}];
  const heldSegments=[{key:'spot',why:'not held'},{key:'futures'},{key:'options',why:'not held'}];
  const emptyInstruments=[{key:'A',months:0},{key:'B',months:0}];
  const heldInstruments=[{key:'A',months:0},{key:'B',months:3}];
  assert.equal(page.seedSegment('',[],'B',emptySegments),'');
  assert.equal(page.seedInstrument('',[],emptyInstruments),'');
  assert.equal(page.seedSegment('',[{}],'',heldSegments),'');
  assert.equal(page.seedSegment('',[{}],'B',heldSegments),'futures');
  assert.equal(page.seedInstrument('',[{}],heldInstruments),'B');
  assert.equal(page.seedSegment('options',[{}],'B',heldSegments),'options');
  assert.equal(page.seedInstrument('A',[{}],heldInstruments),'A');
  assert.equal(page.seedTimeframe('','','', [['1min',1]]),'');
  assert.equal(page.seedTimeframe('','B','', [['1min',1]]),'');
  assert.equal(page.seedTimeframe('','B','futures', [['5min',1]]),'5min');
  assert.equal(page.seedTimeframe('30min','B','futures', [['5min',1]]),'30min');
});

test('the DB route publishes only a complete current raw snapshot and names its preparation and title',()=>{
  assert.match(page.source,/<svelte:head><title>Database · brutex<\/title><\/svelte:head>/);
  assert.match(page.source,/let prepared = \$state.raw/);
  assert.match(page.source,/const deco = \$derived\(prepared.input === rows/);
  assert.match(page.source,/if \(!controller.signal.aborted\) prepared = \{ input, body, why: null \}/);
  assert.match(page.source,/return \(\) => controller.abort\(\)/);
  assert.match(page.source,/Preparing the database view for/);
  assert.doesNotMatch(page.source,/denominators\(rows\)/);
  assert.match(page.source,/if \(view !== 'bars' \|\| !filter \|\| !kind \|\| !timeframe\) \{\s*return \{ all: \[\], read: \[\], held: 0 \}/);
  assert.equal(denomKey('2025-01','1min'),'2025-01\0'+'1min');
});

test('a complete DB preparation is reused on navigation and only a new immutable input rebuilds it',async()=>{
  const cache=createDatabasePreparation();
  const input=[generatedMonth(0),generatedMonth(1)];
  let calls=0;
  const decorate=(/** @type {ReturnType<typeof generatedMonth>} */ row)=>{calls++;return row;};
  const options={signal:new AbortController().signal,yieldControl:immediate};
  const first=await cache.read(input,decorate,options);
  assert.equal(await cache.read(input,decorate,options),first);
  assert.equal(calls,2);
  const changed=await cache.read([...input],decorate,options);
  assert.notEqual(changed,first);
  assert.equal(calls,4);
  const controller=new AbortController();controller.abort();
  await assert.rejects(cache.read(input,decorate,{...options,signal:controller.signal}),/abort/i);
});

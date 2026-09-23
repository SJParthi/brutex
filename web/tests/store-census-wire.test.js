import { test } from 'node:test';
import assert from 'node:assert/strict';
import { decodeCensus, CENSUS_ENCODING } from '../src/lib/store-census-wire.js';

const wire = () => ({schema:1,encoding:CENSUS_ENCODING,feed:'example',instruments:['NSE-CASH-BAJAJ-AUTO'],
  months:['2025-01'],timeframes:['1min'],reasons:['previous_not_stored'],
  rows:[[0,0,0,2,1000,2000,0,null,null,0]]});
const decode = (/** @type {unknown} */ value, signal = new AbortController().signal, yieldControl = async()=>{}) => decodeCensus(value,'example',signal,yieldControl);

test('compact census expands every field exactly, including zero, null, negative and hyphenated values', async()=>{
  assert.deepEqual(await decode(wire()),[{feed:'example',instrument:'NSE-CASH-BAJAJ-AUTO',month:'2025-01',timeframe:'1min',rows:2,
    first_ts:1000,last_ts:2000,chg_bps:0,chg_why:null,prev_chg_bps:null,prev_chg_why:'previous_not_stored'}]);
  const negative=wire();negative.rows[0][6]=-50;
  assert.equal((await decode(negative))[0].chg_bps,-50);
  const legacy=[{same:'legacy array'}];assert.equal(await decode(legacy),legacy);
});

test('malformed schema, feed, dictionaries, tuples, counts and ratio reasons refuse complete publication',async()=>{
  /** Malformed wire fixtures deliberately leave the valid production schema.
   * @type {((value:any)=>unknown)[]} */
  const mutations=[v=>v.schema=2,v=>v.encoding='later',v=>v.feed='other',v=>v.rows={},v=>v.months=[''],v=>v.timeframes=[1],
    v=>v.instruments=null,v=>v.rows[0].pop(),v=>v.rows[0][0]=-1,v=>v.rows[0][1]=1,v=>v.rows[0][2]=0.1,
    v=>v.rows[0][3]=-1,v=>v.rows[0][3]=Number.MAX_SAFE_INTEGER+1,v=>v.rows[0][4]=null,v=>v.rows[0][5]=Infinity,
    v=>v.rows[0][6]=null,v=>v.rows[0][7]=0,v=>v.rows[0][8]=0,v=>v.rows[0][9]=1];
  for(const mutate of mutations){const value=wire();mutate(value);await assert.rejects(decode(value),/valid complete/);}
  for(const value of [null,false,0,'',{}])await assert.rejects(decode(value),/valid complete/);
});

test('large inventory decoding yields in bounded slices and a cancelled generation never publishes partial rows',async()=>{
  const value=wire();value.rows=Array.from({length:5000},()=>[...value.rows[0]]);
  let yields=0;const complete=await decode(value,new AbortController().signal,async()=>{yields++;});
  assert.equal(complete.length,5000);assert.equal(yields,3);
  const controller=new AbortController();let steps=0;
  await assert.rejects(decode(value,controller.signal,async()=>{if(++steps===2)controller.abort();}),/abort/i);
  assert.equal(steps,2);
});

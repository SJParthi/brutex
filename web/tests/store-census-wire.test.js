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

/* WP-56 (DB-G84): the census as COLUMNS, which the census worker decodes into
   and the page rebuilds rows from. It must answer exactly what `decodeCensus`
   answers, and refuse exactly what it refuses. */
import { decodeCensusColumns, rowsOfColumns, columnBuffers } from '../src/lib/store-census-wire.js';

const many = () => ({schema:1,encoding:CENSUS_ENCODING,feed:'example',
  instruments:['NSE-INDEX-SAMPLE','NSE-FNO-SAMPLE-2020-01-30-1234500-CE'],months:['2020-01','2020-02'],timeframes:['1s','1day'],
  reasons:['previous_not_stored','corporate_action_unverified'],
  rows:[[0,0,0,2,1000,2000,0,null,null,0],[1,1,1,0,0,0,-75,null,125,null],[1,0,0,9,5,6,null,1,null,1],[0,1,1,1,-3,-2,2147483648,null,-1,null]]});

test('the columns rebuild every row decodeCensus builds, field for field', async()=>{
  const value=many();
  const objects=await decodeCensus(structuredClone(value),'example',new AbortController().signal,async()=>{});
  const cols=decodeCensusColumns(structuredClone(value),'example');
  assert.ok(cols);
  assert.equal(cols.n,4);
  const rows=rowsOfColumns(cols,0,cols.n,new Array(cols.n));
  assert.deepEqual(rows,objects);
  // in two slices, into the same array, the same answer
  const halves=new Array(cols.n);rowsOfColumns(cols,0,2,halves);rowsOfColumns(cols,2,4,halves);
  assert.deepEqual(halves,objects);
  // every numeric column moves as its own buffer
  assert.equal(new Set(columnBuffers(cols)).size,10);
  assert.equal(decodeCensusColumns([{legacy:true}],'example'),null,'a legacy array is already the rows');
});

test('the columns refuse exactly what decodeCensus refuses',async()=>{
  /** @type {((value:any)=>unknown)[]} */
  const mutations=[v=>v.schema=2,v=>v.encoding='later',v=>v.feed='other',v=>v.rows={},v=>v.months=[''],v=>v.timeframes=[1],
    v=>v.instruments=null,v=>v.rows[0].pop(),v=>v.rows[0][0]=-1,v=>v.rows[0][1]=2,v=>v.rows[0][2]=0.1,
    v=>v.rows[0][3]=-1,v=>v.rows[0][3]=Number.MAX_SAFE_INTEGER+1,v=>v.rows[0][4]=null,v=>v.rows[0][5]=Infinity,
    v=>v.rows[0][6]=null,v=>v.rows[0][7]=0,v=>v.rows[0][8]=0,v=>v.rows[0][9]=7];
  for(const mutate of mutations){
    const value=many();mutate(value);
    await assert.rejects(decode(structuredClone(value)),/valid complete/);
    assert.throws(()=>decodeCensusColumns(structuredClone(value),'example'),/valid complete/);
  }
  for(const value of [null,false,0,'',{}])assert.throws(()=>decodeCensusColumns(value,'example'),/valid complete/);
});

/* ======================================================================
   WP-56 (D-3327 item 3; DB-022, DB-060): THE CENSUS'S ORDER, AND A DELTA
   MERGED INTO IT. Every census below is invented for the fixture.
   ====================================================================== */
import { censusOrder, seriesOrder, rungOrder, columnsOrderFault, measureCensus, mergeCensus,
  noteCensusMeasure, censusMeasureOf } from '../src/lib/store-census-wire.js';

/** Runs the slices at once: the tests are about what is built, not when. */
const atOnce = async (/** @type {number} */ n, /** @type {(start: number, end: number) => void} */ work, /** @type {AbortSignal} */ signal) => {
  signal?.throwIfAborted();
  for (let start = 0; start < n; start += 3) work(start, Math.min(n, start + 3));
};

/** `2019-03 NSE-INDEX-NIFTY 1min` as a census row holding `rows` bars. */
const line = (/** @type {string} */ text, rows = 1) => {
  const [month, instrument, timeframe] = text.split(' ');
  return { feed: 'example', instrument, month, timeframe, rows, first_ts: 1, last_ts: 2,
    chg_bps: null, chg_why: 'previous_not_stored', prev_chg_bps: null, prev_chg_why: 'previous_not_stored' };
};
const names = (/** @type {any[]} */ rows) => rows.map((r) => `${r.month} ${r.instrument} ${r.timeframe}`);

/* THE SAME LIST `crates/api/src/store_route.rs` HOLDS
   (`the_whole_body_lists_newest_month_first_then_the_series_own_order`): the
   order the server writes a whole body in. The two tests pin both sides of
   the wire to one example. */
const SERVER_ORDER = [
  '2019-04 NSE-INDEX-NIFTY 1min',
  '2019-04 NSE-FNO-NIFTY-2019-04-25-1150000-CE 1s',
  '2019-03 NSE-INDEX-BANKNIFTY 1min',
  '2019-03 NSE-INDEX-NIFTY 1s',
  '2019-03 NSE-INDEX-NIFTY 1min',
  '2019-03 NSE-INDEX-NIFTY 1day',
  '2019-03 NSE-CASH-BAJAJ-AUTO 1s',
  '2019-03 NSE-CASH-RELIANCE 1s',
  '2019-03 BSE-INDEX-SENSEX 1day',
  '2019-03 NSE-FNO-BANKNIFTY-2019-03-28-2900000-CE 1s',
  '2019-03 NSE-FNO-BANKNIFTY-2019-04-25-1150000-CE 1s',
  '2019-03 NSE-FNO-NIFTY-2019-04-25-1150000-CE 1s',
  '2019-03 NSE-FNO-NIFTY-2019-04-25-1150000-PE 1s',
  '2019-03 NSE-FNO-NIFTY-2019-04-25-950000-CE 1s',
  '2019-03 NSE-FNO-NIFTY-2019-04-25-FUT 1s'
];

test('the census order is the server\'s: any shuffle of its list sorts back to it', () => {
  const rows = SERVER_ORDER.map((text) => line(text));
  const before = censusOrder();
  for (let i = 1; i < rows.length; i++) assert.ok(before(rows[i - 1], rows[i]) < 0, `${SERVER_ORDER[i - 1]} is before ${SERVER_ORDER[i]}`);
  // A fixed permutation (reversed, then every third row moved to the front).
  const shuffled = [...rows].reverse();
  const moved = [...shuffled.filter((_, i) => i % 3 === 0), ...shuffled.filter((_, i) => i % 3 !== 0)];
  assert.deepEqual(names(moved.sort(before)), SERVER_ORDER);
  assert.equal(before(rows[4], line(SERVER_ORDER[4])), 0, 'the same key is the same place');
});

test('a name or a rung the order does not know is refused, never guessed a place', () => {
  assert.deepEqual(seriesOrder('NSE-FNO-NIFTY-2019-04-25-1150000-CE'), { contract: '2019-04-25-1150000-CE', exchange: 0, segment: 2, symbol: 'NIFTY' });
  assert.deepEqual(seriesOrder('NSE-CASH-BAJAJ-AUTO'), { contract: null, exchange: 0, segment: 1, symbol: 'BAJAJ-AUTO' });
  assert.deepEqual(seriesOrder('NSE-FNO-BAJAJ-AUTO-2019-04-25-FUT'), { contract: '2019-04-25-FUT', exchange: 0, segment: 2, symbol: 'BAJAJ-AUTO' });
  for (const unknown of ['MCX-FNO-GOLD-2019-04-25-FUT', 'NSE-BONDS-GSEC', 'NSE-FNO-NIFTY', 'NSE-FNO-NIFTY-III', 'nifty', '']) {
    assert.equal(seriesOrder(unknown), null, unknown);
  }
  assert.equal(rungOrder('1s'), 1);
  assert.equal(rungOrder('15min'), 900);
  assert.equal(rungOrder('1day'), 86400);
  for (const unknown of ['', '1h', '0min', 'min', '1 min', '01s']) assert.ok(Number.isNaN(rungOrder(unknown)), unknown);
  const before = censusOrder();
  assert.throws(() => before(line('2019-03 NSE-FNO-NIFTY 1s'), line('2019-03 NSE-INDEX-NIFTY 1s')), /not a name this page can place/);
  assert.throws(() => before(line('2019-03 NSE-INDEX-NIFTY 1h'), line('2019-03 NSE-INDEX-NIFTY 1s')), /not a rung this page can place/);
});

/** A wire body of `lines`, in that order, each row holding `bars`. */
const wireOf = (/** @type {string[]} */ lines, bars = 2) => {
  /** @type {any} */
  const value = { schema: 1, encoding: CENSUS_ENCODING, feed: 'example', instruments: [], months: [], timeframes: [], reasons: ['previous_not_stored'], rows: [] };
  const at = (/** @type {string[]} */ list, /** @type {string} */ word) => (list.includes(word) ? list.indexOf(word) : list.push(word) - 1);
  for (const text of lines) {
    const [month, instrument, timeframe] = text.split(' ');
    value.rows.push([at(value.instruments, instrument), at(value.months, month), at(value.timeframes, timeframe), bars, 1, 2, null, 0, null, 0]);
  }
  return value;
};

test('a whole census is checked against the order as columns and as rows, and the first row out of it is named', async () => {
  const cols = decodeCensusColumns(wireOf(SERVER_ORDER), 'example');
  assert.ok(cols);
  assert.equal(columnsOrderFault(cols), null);
  const rows = SERVER_ORDER.map((text) => line(text, 2));
  assert.deepEqual(await measureCensus(rows, atOnce), { order: null, bars: 2 * SERVER_ORDER.length });
  assert.deepEqual(await measureCensus([], atOnce), { order: null, bars: 0 }, 'an empty census is in order and holds nothing');
  assert.deepEqual(await measureCensus([line(SERVER_ORDER[0], 7)], atOnce), { order: null, bars: 7 });

  // OUT OF ORDER: two rows swapped. DUPLICATE: a key twice is not "after" itself.
  const swapped = [...SERVER_ORDER];
  [swapped[5], swapped[6]] = [swapped[6], swapped[5]];
  const said = 'row 7 (2019-03 NSE-INDEX-NIFTY 1day) does not follow row 6 (2019-03 NSE-CASH-BAJAJ-AUTO 1s) in the order this page places a refreshed row by';
  assert.equal(columnsOrderFault(/** @type {any} */ (decodeCensusColumns(wireOf(swapped), 'example'))), said);
  assert.deepEqual(await measureCensus(swapped.map((text) => line(text)), atOnce), { order: said, bars: swapped.length },
    'the rows and the columns name the same row, and the bars are still counted');
  const twice = [SERVER_ORDER[0], SERVER_ORDER[0]];
  assert.match(String(columnsOrderFault(/** @type {any} */ (decodeCensusColumns(wireOf(twice), 'example')))), /^row 2 .* does not follow row 1/);
  assert.match(String((await measureCensus(twice.map((text) => line(text)), atOnce)).order), /^row 2 .* does not follow row 1/);
  // A NAME OR A RUNG NOBODY CAN ORDER is said by name, in both forms.
  const odd = ['2019-03 NSE-FNO-NIFTY-III 1s', '2019-03 NSE-INDEX-NIFTY 1s'];
  assert.equal(columnsOrderFault(/** @type {any} */ (decodeCensusColumns(wireOf(odd), 'example'))), '"NSE-FNO-NIFTY-III" is not a name this page can place in the census\'s order');
  assert.equal((await measureCensus(odd.map((text) => line(text)), atOnce)).order, '"NSE-FNO-NIFTY-III" is not a name this page can place in the census\'s order');
  assert.equal(columnsOrderFault(/** @type {any} */ (decodeCensusColumns(wireOf(['2019-03 NSE-INDEX-NIFTY 1h']), 'example'))), '"1h" is not a rung this page can place in the census\'s order');
  // What a page knows of an array rides beside the array, and only beside it.
  assert.equal(censusMeasureOf(rows), null);
  noteCensusMeasure(rows, { order: null, bars: 30 });
  assert.deepEqual(censusMeasureOf(rows), { order: null, bars: 30 });
  assert.equal(censusMeasureOf([...rows]), null, 'a copy is another array, unmeasured');
});

test('a delta merged into the census held is the whole census at the new generation, row for row', async () => {
  // HELD: the server's list without five of its rows. DELTA: those five (a
  // newest month, a row at the head of a month, one in its middle, its last
  // row, and the census's last), plus two held rows that changed in place.
  const absent = new Set([0, 2, 7, 13, 14]);
  const held = SERVER_ORDER.filter((_, i) => !absent.has(i)).map((text) => line(text, 2));
  const changed = new Set([4, 9]);
  const delta = SERVER_ORDER.map((text, i) => (absent.has(i) ? line(text, 5) : changed.has(i) ? line(text, 9) : null)).filter(Boolean);
  const whole = SERVER_ORDER.map((text, i) => line(text, absent.has(i) ? 5 : changed.has(i) ? 9 : 2));

  const merged = await mergeCensus(held, /** @type {any[]} */ (delta), atOnce);
  assert.deepEqual(merged.rows, whole, 'position for position');
  assert.equal(merged.added, 5);
  assert.equal(merged.replaced, 2);
  assert.equal(merged.bars, 5 * 5 + 2 * (9 - 2), 'the change in bars held');
  // The census held is not changed, and a row the delta did not name is the SAME object.
  assert.deepEqual(names(held), SERVER_ORDER.filter((_, i) => !absent.has(i)));
  assert.equal(merged.rows[1], held[0]);
  assert.equal(merged.rows[4], delta[2], 'a changed row is the delta\'s');
  assert.equal((await measureCensus(merged.rows, atOnce)).order, null, 'still in the census order');
});

test('merge extremes: nothing changed, nothing held, one row, every row, and a delta that is not in order', async () => {
  const all = SERVER_ORDER.map((text) => line(text, 2));
  // EMPTY DELTA: the same rows in a new array.
  const same = await mergeCensus(all, [], atOnce);
  assert.deepEqual(same, { rows: all, added: 0, replaced: 0, bars: 0 });
  // NOTHING HELD: the delta is the census.
  const first = await mergeCensus([], all, atOnce);
  assert.deepEqual(first.rows, all);
  assert.equal(first.added, all.length);
  assert.equal(first.bars, 2 * all.length);
  // ONE ROW HELD, ONE ROW ADDED BEFORE IT AND ONE AFTER.
  const around = await mergeCensus([all[5]], [all[0], all[14]], atOnce);
  assert.deepEqual(names(around.rows), [SERVER_ORDER[0], SERVER_ORDER[5], SERVER_ORDER[14]]);
  // EVERY ROW REPLACED: nothing added, and every row is the delta's.
  const again = SERVER_ORDER.map((text) => line(text, 3));
  const replaced = await mergeCensus(all, again, atOnce);
  assert.equal(replaced.added, 0);
  assert.equal(replaced.replaced, all.length);
  assert.equal(replaced.bars, all.length);
  assert.ok(replaced.rows.every((row, i) => row === again[i]));
  // A MONTH OLDER THAN ANY HELD goes last; one between two held months goes between.
  const months = await mergeCensus([line('2019-04 NSE-INDEX-NIFTY 1min'), line('2019-02 NSE-INDEX-NIFTY 1min')],
    [line('2019-03 NSE-INDEX-NIFTY 1min'), line('2019-01 NSE-INDEX-NIFTY 1min')], atOnce);
  assert.deepEqual(names(months.rows), ['2019-04 NSE-INDEX-NIFTY 1min', '2019-03 NSE-INDEX-NIFTY 1min', '2019-02 NSE-INDEX-NIFTY 1min', '2019-01 NSE-INDEX-NIFTY 1min']);
  // A DELTA OUT OF ORDER, OR NAMING A KEY TWICE, is refused: it is not what the server sends.
  await assert.rejects(mergeCensus(all, [all[3], all[1]], atOnce), /the delta's row 2 .* does not follow its row 1/);
  await assert.rejects(mergeCensus(all, [all[3], line(SERVER_ORDER[3], 8)], atOnce), /the delta's row 2 .* does not follow its row 1/);
  await assert.rejects(mergeCensus(all, [line('2019-03 NSE-FNO-NIFTY-III 1s')], atOnce), /not a name this page can place/);
  // CANCELLED: a newer read replaced this one.
  const controller = new AbortController();
  controller.abort(new Error('a newer read replaced this one'));
  await assert.rejects(mergeCensus(all, [line(SERVER_ORDER[0], 4)], atOnce, controller.signal), /newer read/);
});

test('a large merge places thousands of rows exactly: every third row new, every seventh changed', async () => {
  /** @type {string[]} */
  const order = [];
  for (let m = 12; m >= 1; m--) {
    const month = `2020-${String(m).padStart(2, '0')}`;
    for (let s = 0; s < 40; s++) order.push(`${month} NSE-CASH-S${String(s).padStart(3, '0')} 1min`);
    for (let k = 1; k <= 60; k++) order.push(`${month} NSE-FNO-SAMPLE-2020-12-31-${String(100000 + k * 5000)}-${k % 2 ? 'CE' : 'PE'} 1s`);
  }
  // The contract text sorts as text, so put the fixture in the order itself.
  const before = censusOrder();
  const sorted = order.map((text) => line(text, 2)).sort(before);
  const held = sorted.filter((_, i) => i % 3 !== 0);
  const delta = sorted.map((row, i) => (i % 3 === 0 ? { ...row, rows: 5 } : i % 7 === 0 ? { ...row, rows: 9 } : null)).filter(Boolean);
  const whole = sorted.map((row, i) => (i % 3 === 0 ? { ...row, rows: 5 } : i % 7 === 0 ? { ...row, rows: 9 } : row));
  const merged = await mergeCensus(held, /** @type {any[]} */ (delta), atOnce);
  assert.equal(merged.rows.length, 1200);
  assert.deepEqual(merged.rows, whole);
  assert.equal(merged.added, 400);
});

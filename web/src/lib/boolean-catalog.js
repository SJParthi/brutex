import { detailRefusal } from './detail-refusal.js';

const HEX = /^[0-9a-f]{64}$/;
const U64 = 18446744073709551615n, I64 = 9223372036854775807n;
/** @param {any} value */
const uint = value => typeof value === 'string' && /^(0|[1-9][0-9]*)$/.test(value) && BigInt(value) <= U64;
/** @param {any} value */
const sint = value => typeof value === 'string' && /^(0|-?[1-9][0-9]*)$/.test(value) && BigInt(value) >= -I64 - 1n && BigInt(value) <= I64;
/** @param {any} value */
const hex = value => typeof value === 'string' && HEX.test(value);
/** @param {any} value */
const object = value => value !== null && typeof value === 'object' && !Array.isArray(value);
/** @param {any} value @param {string[]} fields @param {(v:any)=>boolean} valid */
const fields = (value, fields, valid) => object(value) && fields.every(key => valid(value[key]));
const KINDS = ['coordinates', 'programs', 'trades', 'sessions', 'grid'];
const AXES = ['stop', 'target', 'trail', 'requested-stop', 'requested-target', 'requested-trail'];
/** Format an exact civil-day number without rounding an out-of-range value.
 * @param {string} day */
export function catalogDay(day) {
  if(!sint(day))return 'Unavailable';
  const millis=BigInt(day)*86400000n;
  if(millis < -8640000000000000n || millis > 8640000000000000n)return 'IST day '+day;
  return new Date(Number(millis)).toLocaleDateString('en-IN',{timeZone:'UTC',year:'numeric',month:'short',day:'2-digit'});
}
/** @param {any} selection */
export function catalogSelection(selection) {
  const {identity, completion=null, kind='coordinates', candidate=null, side=null, axis=null, offset='0', limit=32} = selection;
  if (!hex(identity) || (completion !== null && !hex(completion)) || !KINDS.includes(kind) || !uint(offset) ||
    !Number.isInteger(limit) || limit < 1 || limit > 256 ||
    (['trades','sessions'].includes(kind) !== (candidate !== null)) || (candidate !== null && !uint(candidate)) ||
    (kind === 'grid' ? !['long','short'].includes(side) || !AXES.includes(axis) : side !== null || axis !== null) ||
    ((kind !== 'coordinates' || offset !== '0') && completion === null)) throw new Error('Choose an exact catalog and its saved completion before opening detail pages.');
  return {identity,completion,kind,candidate,side,axis,offset,limit};
}
/** @param {any} row */
function coordinate(row) {
  if (!fields(row,['index','program_index','ordinal','support_sessions','execution_refusal_bits'],uint) ||
      !fields(row,['identity','run'],hex) || !['long','short'].includes(row.side) ||
      typeof row.expression !== 'string' || row.expression.length === 0 || row.expression.length > 131072 ||
      !fields(row.truth,['evaluated','hits','misses','unknown'],uint) ||
      BigInt(row.truth.evaluated) !== BigInt(row.truth.hits)+BigInt(row.truth.misses)+BigInt(row.truth.unknown) ||
      BigInt(row.execution_refusal_bits)>63n || !Array.isArray(row.execution_refusals) || row.execution_refusals.some((/** @type {any} */ value)=>typeof value!=='string' || !value) ||
      new Set(row.execution_refusals).size!==row.execution_refusals.length || BigInt(row.execution_refusal_bits).toString(2).replaceAll('0','').length!==row.execution_refusals.length) return false;
  const cell=row.cell;
  if (!fields(cell,['trades','wins','stopped','targeted','trailed_stop','trailed_profit','timed_out','ambiguous_bars','gapped'],uint) ||
      !fields(cell,['pessimistic','optimistic','fill_cost'],sint) || BigInt(cell.wins)>BigInt(cell.trades) ||
      BigInt(cell.pessimistic)>BigInt(cell.optimistic) || BigInt(cell.fill_cost)<0n ||
      !['stop','target','tsl'].every(key=>cell[key]===null || uint(cell[key])) ||
      !(cell.ttp===null || fields(cell.ttp,['arm','trail'],uint)) ||
      !fields(row.levels,['stop_ppm','target_ppm','tsl_ppm','ttp_arm_ppm','ttp_trail_ppm'],value=>value===null || (sint(value)&&BigInt(value)>0n))) return false;
  return BigInt(cell.trades) === ['stopped','targeted','trailed_stop','trailed_profit','timed_out'].reduce((sum,key)=>sum+BigInt(cell[key]),0n) &&
    ['stop','target','tsl'].every(key=>(cell[key]===null)===(row.levels[key+'_ppm']===null)) &&
    (cell.ttp===null)===(row.levels.ttp_arm_ppm===null) && (cell.ttp===null)===(row.levels.ttp_trail_ppm===null);
}
/** @param {any} grid */
export function gridSummary(grid) {
  return fields(grid,['resolution','execution','feed','commit','calendar','cost_model'],hex) && ['long','short'].includes(grid.side) &&
    fields(grid,['bars','cells','horizon_bars','stop_count','target_count','trail_count','requested_stop_count','requested_target_count','requested_trail_count','max_pairs','max_cells','max_levels_per_axis','max_ambiguous_bars','max_gap_fills'],uint) && BigInt(grid.horizon_bars)>0n &&
    fields(grid,['first_micros','last_micros','ratio_min_hundredths','ratio_max_hundredths'],sint) && BigInt(grid.first_micros)<=BigInt(grid.last_micros) &&
    grid.execution_seconds==='60' && ['floor','ceiling'].includes(grid.range_rounding) &&
    ['pessimistic-total','edge-then-pessimistic','guaranteed-floor'].includes(grid.selector) && object(grid.forced_stop) &&
    ['disabled','include-exact-observed','require-exact-observed'].includes(grid.forced_stop.kind) &&
    (grid.forced_stop.kind==='disabled' ? grid.forced_stop.ppm===null : sint(grid.forced_stop.ppm)&&BigInt(grid.forced_stop.ppm)>0n);
}
/** @param {any} body @param {any} row */
export function catalogCoordinate(body,row) {
  if(!coordinate(row))return false;
  const long=body.grids[0], short=body.grids[1], grid=row.side==='long'?long:short;
  const count=BigInt(long.cells)+BigInt(short.cells);
  const index=BigInt(row.program_index)*count+(row.side==='short'?BigInt(long.cells):0n)+BigInt(row.ordinal);
  return index===BigInt(row.index) && index<BigInt(body.coordinate_count) && BigInt(row.program_index)<BigInt(body.program_count) &&
    BigInt(row.ordinal)<BigInt(grid.cells) && BigInt(row.support_sessions)<=BigInt(body.session_count) &&
    (row.cell.stop===null||BigInt(row.cell.stop)<BigInt(grid.stop_count)) &&
    (row.cell.target===null||BigInt(row.cell.target)<BigInt(grid.target_count)) &&
    (row.cell.tsl===null||BigInt(row.cell.tsl)<BigInt(grid.trail_count)) &&
    (row.cell.ttp===null||BigInt(row.cell.ttp.arm)<BigInt(grid.target_count)&&BigInt(row.cell.ttp.trail)<BigInt(grid.trail_count));
}
/** @param {any} selection @param {(url:string)=>Promise<any>} request */
export async function fetchBooleanCatalog(selection,request) {
  const asked=catalogSelection(selection);
  const query=new URLSearchParams({identity:asked.identity,kind:asked.kind,offset:asked.offset,limit:String(asked.limit)});
  for(const key of ['completion','candidate','side','axis']) {
    const value=asked[/** @type {'completion'|'candidate'|'side'|'axis'} */ (key)];
    if(value!==null)query.set(key,value);
  }
  const response=await request('/boolean-candidates.json?'+query);
  if(!response?.ok)throw new Error(await detailRefusal(response,'Catalog detail request failed (HTTP '+String(response?.status)+').'));
  const body=await response.json();
  if(!object(body)||body.schema_version!==1||body.status!=='saved'||body.authority!=='authenticated-catalog-observation'||body.refusal!==null||
    body.identity!==asked.identity||!hex(body.completion)||(asked.completion!==null&&body.completion!==asked.completion)||
    !['kind','candidate','side','axis','offset','limit'].every(key=>body[key]===asked[/** @type {keyof typeof asked} */(key)])||
    body.page_complete!==true||!Array.isArray(body.rows)||!fields(body,['total','program_count','coordinate_count','session_count'],uint)||
    !hex(body.cohort)||!hex(body.membership_digest)||typeof body.instrument!=='string'||!body.instrument||typeof body.cash!=='boolean'||typeof body.scope!=='string'||
    !Array.isArray(body.grids)||body.grids.length!==2||!body.grids.every(gridSummary)||body.grids[0].side!=='long'||body.grids[1].side!=='short') throw new Error('Catalog identity, completion or page contract changed; no partial evidence accepted.');
  if(BigInt(body.program_count)===0n||BigInt(body.session_count)===0n||body.grids.some((/** @type {any} */ grid)=>BigInt(grid.cells)===0n||BigInt(grid.bars)===0n)||
    BigInt(body.coordinate_count)!==BigInt(body.program_count)*(BigInt(body.grids[0].cells)+BigInt(body.grids[1].cells)))throw new Error('Catalog program, side and grid populations do not reconcile.');
  const start=BigInt(asked.offset), total=BigInt(body.total), end=start+BigInt(body.rows.length);
  const count=total-start<BigInt(asked.limit)?total-start:BigInt(asked.limit);
  if(start>total||count!==BigInt(body.rows.length)||body.next!==(end<total?String(end):null))throw new Error('Catalog page skips, repeats, truncates or ends early.');
  if (asked.candidate===null ? body.selected!==null : !catalogCoordinate(body,body.selected)||body.selected.index!==asked.candidate) throw new Error('Catalog detail belongs to another candidate.');
  if(asked.kind==='coordinates'&&total!==BigInt(body.coordinate_count) || asked.kind==='programs'&&total!==BigInt(body.program_count) ||
    asked.kind==='sessions'&&total!==BigInt(body.session_count) || asked.kind==='trades'&&total!==BigInt(body.selected.cell.trades))throw new Error('Catalog page count differs from its sealed parent.');
  if(asked.kind==='grid') {
    const grid=body.grids.find((/** @type {any} */ grid)=>grid.side===asked.side);
    const field=asked.axis.replace('requested-','requested_')+'_count';
    if(total!==BigInt(grid[field]))throw new Error('Grid page count differs from the saved axis.');
  }
  for(const [index,row] of body.rows.entries()) {
    if(!object(row)||row.index!==String(start+BigInt(index)))throw new Error('Catalog row order is incomplete.');
    let valid=false;
    if(asked.kind==='coordinates')valid=catalogCoordinate(body,row);
    if(asked.kind==='programs')valid=typeof row.expression==='string'&&row.expression.length>0&&row.expression.length<=131072;
    if(asked.kind==='trades')valid=fields(row,['signal_bar','entry_bar','exit_bar','adverse_ppm','favourable_ppm'],uint)&&fields(row,['best','worst','entry_micros','exit_micros','adverse_paisa','favourable_paisa'],sint)&&BigInt(row.entry_bar)<=BigInt(row.exit_bar)&&BigInt(row.entry_micros)<=BigInt(row.exit_micros)&&BigInt(row.worst)<=BigInt(row.best);
    if(asked.kind==='sessions')valid=fields(row,['day','return_paisa'],sint)&&fields(row,['trades','wins'],uint)&&BigInt(row.wins)<=BigInt(row.trades)&&(index===0||BigInt(body.rows[index-1].day)<BigInt(row.day));
    if(asked.kind==='grid')valid=asked.axis.startsWith('requested-')?fields(row,['numerator','denominator'],uint)&&BigInt(row.denominator)>0n&&BigInt(row.numerator)<=BigInt(row.denominator):sint(row.ppm)&&BigInt(row.ppm)>0n;
    if(!valid)throw new Error('Catalog row values or available evidence are invalid.');
  }
  return body;
}

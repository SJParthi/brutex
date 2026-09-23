import { detailRefusal } from './detail-refusal.js';

export const CAMPAIGN_RUNGS = ['1min','2min','3min','5min','10min','15min','30min','60min'];
const STATES = ['waiting','running','paused','refused','completed'];
/** @param {any} value */
const hex = value => typeof value === 'string' && /^[0-9a-f]{64}$/.test(value);
/** @param {any} value */
const uint = value => typeof value === 'string' && /^(0|[1-9][0-9]*)$/.test(value) && BigInt(value) <= 18446744073709551615n;
/** @param {any} value */
const object = value => value !== null && typeof value === 'object' && !Array.isArray(value);

/** A recorded start is not a heartbeat or proof that a worker is progressing.
 * @param {string} state @param {boolean} ownerActive */
export function campaignState(state, ownerActive) {
  if (!STATES.includes(state) || typeof ownerActive !== 'boolean') throw new Error('Campaign state is unavailable.');
  if (state === 'running') return ownerActive ? 'Started; owner observed' : 'Started; no owner observed';
  return {waiting:'Waiting', paused:'Paused', refused:'Refused', completed:'Recorded complete'}[state];
}

/** @param {any} selection */
export function campaignSelection(selection) {
  const { identity, pin=null }=selection;
  if (!hex(identity) || (pin!==null && !hex(pin))) throw new Error('Use an exact campaign identity and snapshot pin.');
  return {identity,pin};
}

/** Only a saved, exact link can open another observer.
 * @param {any} value */
const link = value => object(value) && hex(value.identity) && hex(value.completion);
/** @param {any} value */
const month = value => typeof value==='string' && /^\d{4}-(0[1-9]|1[0-2])$/.test(value);

/** @param {any} selection @param {(url:string)=>Promise<any>} request */
export async function fetchBooleanCampaign(selection, request) {
  const asked=campaignSelection(selection);
  const query=new URLSearchParams({identity:asked.identity});
  if (asked.pin!==null) query.set('pin',asked.pin);
  const response=await request('/boolean-campaign.json?'+query);
  if (!response?.ok) throw new Error(await detailRefusal(response,'Campaign snapshot failed (HTTP '+String(response?.status)+').'));
  const body=await response.json();
  if (!object(body) || body.schema_version!==1 || body.status!=='saved' || body.authority!=='acknowledged-campaign-snapshot' ||
      body.identity!==asked.identity || !hex(body.pin) || (asked.pin!==null && body.pin!==asked.pin) || !uint(body.sequence) ||
      !STATES.includes(body.state) || typeof body.owner_active!=='boolean' || body.child_completion_receipts_checked!==true || body.child_bodies_checked!==false ||
      !month(body.from) || !month(body.to) || body.from>body.to || !uint(body.horizon_bars) || body.horizon_bars==='0' || !uint(body.program_count) || body.program_count==='0' ||
      !hex(body.program_digest) || !hex(body.descriptor_digest) ||
      body.refusal!==null || typeof body.scope!=='string' || !Array.isArray(body.rows) || body.rows.length!==CAMPAIGN_RUNGS.length) {
    throw new Error('Campaign identity, snapshot or bounded comparison contract changed; no partial status accepted.');
  }
  for (const [index,row] of body.rows.entries()) {
    if (!object(row) || row.rung!==CAMPAIGN_RUNGS[index] || !STATES.includes(row.state) ||
        !(row.reason===null || typeof row.reason==='string' && row.reason.length>0) ||
        !Array.isArray(row.expected_catalogs) || row.expected_catalogs.length===0 || !row.expected_catalogs.every((/** @type {any} */ item)=>object(item) && hex(item.identity) && (item.completion===null || hex(item.completion)) && typeof item.instrument==='string' && item.instrument.length>0 && typeof item.cash==='boolean' && hex(item.membership_digest)) ||
        new Set(row.expected_catalogs.map((/** @type {any} */ item)=>item.instrument)).size!==row.expected_catalogs.length ||
        !Array.isArray(row.catalogs) || !row.catalogs.every((/** @type {any} */ item)=>link(item) && typeof item.instrument==='string' && item.instrument.length>0) ||
        new Set(row.catalogs.map((/** @type {any} */ item)=>item.identity)).size!==row.catalogs.length ||
        !(row.statistics===null || link(row.statistics)) || !(row.admission===null || link(row.admission))) throw new Error('Campaign timeframe state or saved child links are invalid.');
    const saved=row.expected_catalogs.filter((/** @type {any} */ item)=>item.completion!==null);
    if (saved.length!==row.catalogs.length || saved.some((/** @type {any} */ item,/** @type {number} */ at)=>['instrument','identity','completion'].some(key=>item[key]!==row.catalogs[at][key]))) throw new Error('Campaign links differ from the expected source catalog identities.');
    if (index>0 && (row.expected_catalogs.length!==body.rows[0].expected_catalogs.length || row.expected_catalogs.some((/** @type {any} */ item,/** @type {number} */ at)=>['instrument','cash','membership_digest'].some(key=>item[key]!==body.rows[0].expected_catalogs[at][key])))) throw new Error('Campaign timeframes do not share the declared family scope.');
    if (row.state==='completed' && (row.catalogs.length!==row.expected_catalogs.length || !row.statistics || !row.admission)) throw new Error('A complete timeframe is missing its recorded outputs.');
  }
  if (body.state==='completed' && !body.rows.every((/** @type {any} */ row)=>row.state==='completed')) throw new Error('Campaign completion does not reconcile all eight timeframes.');
  return body;
}

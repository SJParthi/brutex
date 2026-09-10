import {CAMPAIGN_RUNGS,campaignSelection} from './boolean-campaign.js';
import {detailRefusal} from './detail-refusal.js';
import {savedTimeframes} from './saved-timeframes.js';
/** @param {any} v */
const hex=v=>typeof v==='string'&&/^[0-9a-f]{64}$/.test(v);
/** @param {any} v */
const uint=v=>typeof v==='string'&&v.length<=20&&/^(0|[1-9][0-9]*)$/.test(v)&&BigInt(v)<=18446744073709551615n;
/** @param {any} v */
const object=v=>v!==null&&typeof v==='object'&&!Array.isArray(v);
/** @param {any} selection @param {(url:string)=>Promise<any>} request */
export async function fetchQualifiedCampaign(selection,request){
 const asked=campaignSelection(selection),query=new URLSearchParams({identity:asked.identity});if(asked.pin!==null)query.set('pin',asked.pin);
 const response=await request('/boolean-qualified-campaign.json?'+query);
 if(!response?.ok)throw new Error(await detailRefusal(response,'Qualified campaign failed (HTTP '+String(response?.status)+').'));
 const body=await response.json();
 if(!object(body)||body.schema_version!==1||body.status!=='saved'||body.authority!=='acknowledged-qualified-campaign-history'||body.identity!==asked.identity||!hex(body.pin)||(asked.pin!==null&&body.pin!==asked.pin)||!hex(body.descriptor_digest)||!uint(body.sequence)||body.sequence==='0'||!uint(body.history_records)||body.history_records==='0'||BigInt(body.history_records)>BigInt(body.sequence)||!uint(body.admitted_bytes)||typeof body.owner_active!=='boolean'||body.history_checked!==true||body.child_completion_receipts_checked!==false||body.child_bodies_checked!==false||body.refusal!==null||typeof body.scope!=='string'||!Array.isArray(body.rows))throw new Error('Qualified campaign history or selected-slot contract differs.');
 const timeframes=savedTimeframes(body);
 if(body.rows.length!==timeframes.length)throw new Error('Qualified campaign must preserve every selected timeframe.');
 let pending=false;
 for(const [index,row] of body.rows.entries()){
   if(!object(row)||row.rung!==timeframes[index]||((Object.hasOwn(body,'timeframes')||Object.hasOwn(row,'rung_index'))&&row.rung_index!==String(CAMPAIGN_RUNGS.indexOf(timeframes[index])))||!hex(row.unit)||!['waiting','running','refused','completed'].includes(row.state)||!(row.reason===null||typeof row.reason==='string'&&row.reason.trim().length>0&&new TextEncoder().encode(row.reason).length<=1024))throw new Error('Qualified campaign slot is incomplete.');
   const saved=object(row.qualification)&&hex(row.qualification.identity)&&hex(row.qualification.completion);
   if((row.state==='completed')!==saved||(!saved&&row.qualification!==null)||(row.state==='refused')!==(row.reason!==null)||(pending&&row.state!=='waiting'))throw new Error('Qualified campaign skipped, retracted or invented a unit outcome.');
   pending||=row.state!=='completed';
 }
 if(new Set(body.rows.map((/** @type {any} */ r)=>r.unit)).size!==timeframes.length)throw new Error('Qualified campaign reuses a predeclared unit.');
 const expected=body.rows.every((/** @type {any} */ r)=>r.state==='completed')?'completed':body.rows.some((/** @type {any} */ r)=>r.state==='refused')?'refused':body.rows.some((/** @type {any} */ r)=>r.state!=='waiting')?'running':'waiting';
 if(body.state!==expected)throw new Error('Qualified campaign total does not reconcile every declared unit.');
 return {...body,timeframes};
}

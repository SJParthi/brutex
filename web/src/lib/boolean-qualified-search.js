import {CAMPAIGN_RUNGS,campaignSelection} from './boolean-campaign.js';
import {fetchBooleanEvidence} from './boolean-evidence.js';
import {detailRefusal} from './detail-refusal.js';
import {savedTimeframes,savedProjectionTimeframes} from './saved-timeframes.js';
import {indexConsistencyContext,validateSearchIndexConsistency} from './index-consistency.js';
const U64=(1n<<64n)-1n,U128=(1n<<128n)-1n;
/** @param {any} v @param {bigint} [max] */
const uint=(v,max=U64)=>typeof v==='string'&&v.length<=39&&/^(0|[1-9][0-9]*)$/.test(v)&&BigInt(v)<=max;
/** @param {any} v */
const hex=v=>typeof v==='string'&&/^[0-9a-f]{64}$/.test(v)&&v!=='0'.repeat(64);
/** @param {any} v */
const object=v=>v!==null&&typeof v==='object'&&!Array.isArray(v);
/** @param {any} v */
const fraction=v=>object(v)&&uint(v.numerator,U128)&&uint(v.denominator,U128)&&BigInt(v.denominator)>0n&&BigInt(v.numerator)<=BigInt(v.denominator);
const probabilityCeilings=new Set(['max_fwer_p_value_ppm','max_spa_p_value_ppm','max_white_reality_p_value_ppm','max_romano_wolf_p_value_ppm']);
/** @param {any} original @param {any} effective @param {string} alpha @param {number} version */
function searchPolicy(original,effective,alpha,version){
 if(!object(effective)||!hex(effective.digest)||!Array.isArray(effective.values)||effective.values.length!==39)throw new Error('Effective search policy is missing or incomplete.');
 let capped=0,changed=false;
 for(const [index,prior] of original.values.entries()){
  const next=effective.values[index];
  if(!object(next)||next.name!==prior.name)throw new Error('Effective search policy changed the saved setting names or order.');
  let expected=prior.value;
  if(probabilityCeilings.has(prior.name)){
   if(!uint(prior.value)||BigInt(prior.value)>1000000n)throw new Error('Original probability ceiling is invalid.');
   if(version>=2)expected=String(BigInt(prior.value)<BigInt(alpha)?BigInt(prior.value):BigInt(alpha));capped++;
  }
  if(next.value!==expected)throw new Error('Effective search policy must retain every setting and only tighten family probability ceilings.');
  changed||=next.value!==prior.value;
 }
 if(capped!==4||(effective.digest===original.digest)===changed)throw new Error('Effective search policy receipt does not match its unchanged or tightened values.');
}
/** @param {any} selection */
export function qualifiedSearchSelection(selection){
 const base=campaignSelection(selection),{batch=null,rung=null,completion=null,offset='0',limit=16}=selection;
 const timeframes=Object.hasOwn(selection,'timeframes')?savedTimeframes(selection):null;
 if(batch===null&&rung===null){if(completion!==null||offset!=='0')throw new Error('Overview cannot select a child page.');return {...base,batch:null,rung:null,completion:null,offset:'0',limit:16,timeframes};}
 if(!base.pin||!uint(batch)||!uint(rung)||BigInt(rung)>=8n||!uint(offset)||(completion!==null&&!hex(completion))||(offset!=='0'&&completion===null)||!Number.isInteger(limit)||limit<1||limit>256)throw new Error('Choose exact parent snapshot, batch, timeframe and child completion for continued pages.');
 if(timeframes!==null&&!timeframes.includes(CAMPAIGN_RUNGS[Number(rung)]))throw new Error('The requested timeframe is outside this saved search selection.');
 return {...base,batch,rung,completion,offset,limit,timeframes};
}
/** @param {any} row @param {number} index */
function summary(row,index){
 if(!object(row)||row.rung!==CAMPAIGN_RUNGS[index]||row.rung_index!==String(index)||!uint(row.count)||!object(row.counts)||Object.keys(row.counts).length!==4||!['admitted','rejected','unmeasured','refused'].every(k=>uint(row.counts[k]))||Object.values(row.counts).reduce((a,/** @type {any} */ n)=>a+BigInt(n),0n)!==BigInt(row.count))throw new Error('All declared search verdict counts must reconcile.');
 if(row.qualification===null){if(row.count!=='0'||row.projection_digest!==null||row.allocation_digest!==null)throw new Error('An unrecorded child cannot carry outcomes.');}
 else if(!object(row.qualification)||!hex(row.qualification.identity)||!hex(row.qualification.completion)||!hex(row.projection_digest)||!hex(row.allocation_digest))throw new Error('Search child receipt is incomplete.');
}
/** @param {any} body */
function overview(body){
 if(body.authority!=='acknowledged-search-history'||!['reserved','complete','refused'].includes(body.phase)||!['running','paused','refused','completed'].includes(body.state)||!uint(body.sequence)||body.sequence==='0'||!uint(body.completed_batches)||!uint(body.grammar_work)||!uint(body.programs)||!uint(body.alpha_ppm)||BigInt(body.alpha_ppm)>1000000n||typeof body.owner_active!=='boolean'||typeof body.exhausted!=='boolean'||typeof body.node_only!=='boolean'||body.history_checked!==true||body.child_bodies_checked!==false||!Array.isArray(body.rows)||body.rows.length!==body.timeframes.length)throw new Error('Search overview state or selected-timeframe history differs.');
 if(!(body.reason===null||typeof body.reason==='string'&&body.reason.length>0&&new TextEncoder().encode(body.reason).length<=1024)||(body.phase==='refused')!==(body.reason!==null)||body.exhausted&&body.phase!=='complete')throw new Error('Search refusal or exhaustion was replaced by success.');
 const state=body.phase==='refused'?'refused':body.exhausted?'completed':body.owner_active?'running':'paused';
 if(body.state!==state||BigInt(body.completed_batches)!==BigInt(body.batch)+(body.phase==='complete'?1n:0n))throw new Error('Search progress does not reconcile its acknowledged phase.');
 if(body.planned_campaign_authority!=='derived-plan-identity'||!(body.planned_campaign===null||hex(body.planned_campaign)))throw new Error('Prospective timeframe-progress identity is invalid.');
 body.rows.forEach((/** @type {any} */ row,/** @type {number} */ index)=>summary(row,CAMPAIGN_RUNGS.indexOf(body.timeframes[index])));
 const empty=body.rows.every((/** @type {any} */ r)=>r.qualification===null),full=body.rows.every((/** @type {any} */ r)=>r.qualification!==null);
 if(body.node_only!==(body.phase==='complete'&&empty)||(body.phase!=='complete'&&!empty)||(!empty&&!full))throw new Error('Node-only grammar progress or complete selected-timeframe comparison differs.');
}
/** @param {any} selection @param {(url:string)=>Promise<any>} request */
export async function fetchQualifiedSearch(selection,request){
 const asked=qualifiedSearchSelection(selection),query=new URLSearchParams({identity:asked.identity});if(asked.pin)query.set('pin',asked.pin);
 if(asked.batch!==null){query.set('batch',asked.batch);query.set('rung',asked.rung);query.set('offset',asked.offset);query.set('limit',String(asked.limit));if(asked.completion)query.set('completion',asked.completion);}
 const response=await request('/boolean-qualified-search.json?'+query);
 if(!response?.ok)throw new Error(await detailRefusal(response,'Qualified search failed (HTTP '+String(response?.status)+').'));
 const body=await response.json();
 if(![1,2,3,4].includes(body?.projection_version))throw new Error('Saved search arithmetic version is missing or unsupported.');
 if(!object(body)||body.schema_version!==1||body.status!=='saved'||body.model!=='qualified-search'||body.identity!==asked.identity||!hex(body.pin)||(asked.pin!==null&&body.pin!==asked.pin)||!uint(body.batch)||!uint(body.admitted_bytes)||!uint(body.observation_byte_limit)||!uint(body.replay_node_limit)||body.replay_node_limit==='0'||!uint(body.replay_nodes_charged)||BigInt(body.replay_nodes_charged)>BigInt(body.replay_node_limit)||BigInt(body.admitted_bytes)>BigInt(body.observation_byte_limit)||body.refusal!==null||typeof body.scope!=='string'||body.kind!==(asked.batch===null?'overview':'rung'))throw new Error('Exact search identity, snapshot or server-owned admission differs.');
 const normalized={...body,timeframes:savedProjectionTimeframes(body)};
 if(asked.timeframes!==null&&(asked.timeframes.length!==normalized.timeframes.length||asked.timeframes.some((rung,index)=>rung!==normalized.timeframes[index])))throw new Error('Saved search timeframe selection changed; no replacement scope is displayed.');
 if(asked.batch===null)overview(normalized);else await detail(normalized,asked);
 return normalized;
}
/** @param {any} body @param {any} asked */
async function detail(body,asked){
 if(!body.timeframes.includes(CAMPAIGN_RUNGS[Number(asked.rung)]))throw new Error('The requested timeframe is outside this saved search selection.');
 if(body.authority!=='authenticated-search-projection'||body.batch!==asked.batch||body.rung!==asked.rung||!hex(body.completion)||(asked.completion!==null&&asked.completion!==body.completion)||body.child_bodies_checked!==true||!object(body.source)||!Array.isArray(body.rows))throw new Error('Search detail belongs to another batch, timeframe or child.');
 summary(body.summary,Number(asked.rung));
 const source=await fetchBooleanEvidence({model:'qualification',identity:body.source.identity,completion:body.completion,offset:asked.offset,limit:asked.limit},async()=>({ok:true,json:async()=>body.source}));
 if(body.summary.qualification?.identity!==source.identity||body.summary.qualification?.completion!==source.completion||body.summary.count!==source.total||body.rows.length!==source.rows.length)throw new Error('Search projection lost its exact original qualification population.');
 const a=body.allocation,batch=BigInt(asked.batch),weight=8n*(batch+1n)*(batch+2n);
 if(!object(a)||a.batch!==asked.batch||a.rung!==asked.rung||a.rungs!=='8'||a.digest!==body.summary.allocation_digest||!uint(a.alpha_ppm)||BigInt(a.alpha_ppm)>1000000n||!fraction(a.threshold)||typeof a.draw_resolution_met!=='boolean'||typeof a.rule!=='string'||BigInt(a.threshold.numerator)*weight*1000000n!==BigInt(a.alpha_ppm)*BigInt(a.threshold.denominator))throw new Error('Search-wide error allocation differs from its declared exact batch slot.');
 const fwer=source.policy.values.find((/** @type {any} */ v)=>v.name==='max_fwer_p_value_ppm')?.value;
 const romano=source.policy.values.find((/** @type {any} */ v)=>v.name==='max_romano_wolf_p_value_ppm')?.value;
 if(!uint(fwer)||!uint(romano)||a.alpha_ppm!==String(BigInt(fwer)<BigInt(romano)?BigInt(fwer):BigInt(romano))||a.alpha_ppm!==source.qualification.allocation.alpha_ppm)throw new Error('Search allowance differs from the original FWER/Romano policy or saved qualification allocation.');
 searchPolicy(source.policy,body.search_policy,a.alpha_ppm,body.projection_version);
 const n=BigInt(a.threshold.numerator),d=BigInt(a.threshold.denominator),minimum=n===0n?null:(d+n-1n)/n-1n;
 if(a.minimum_draws!==(minimum===null?null:String(minimum))||a.draw_resolution_met!==(minimum!==null&&BigInt(source.qualification.procedure.draws)>=minimum))throw new Error('Search draw resolution differs from the saved later procedure.');
 if(!object(body.original_family_probabilities)||!fraction(body.original_family_probabilities.white)||!fraction(body.original_family_probabilities.spa))throw new Error('Original later family probabilities are unavailable.');
 const projected=structuredClone(source);
 projected.policy=body.search_policy;
 for(const [index,row] of body.rows.entries()){
  const original=source.rows[index];
  if(!object(row)||!object(row.comparison)||row.comparison.identity!==original.identity||row.comparison.index!==original.index||!object(row.probabilities)||!['romano','white','spa'].every(k=>fraction(row.probabilities[k])))throw new Error('Search comparison changed a retained coordinate or probability.');
  if(row.comparison.policy_digest!==body.search_policy.digest)throw new Error('Search row belongs to a different effective policy.');
  validateSearchIndexConsistency(original.index_consistency,row.comparison.index_consistency,indexConsistencyContext(source,original,row.comparison.status),body.projection_version);
  for(const key of ['romano','white','spa']){
   const raw=key==='romano'?original.qualification.romano.probability:body.original_family_probabilities[key],corrected=row.probabilities[key],scaled=BigInt(raw.numerator)*weight,den=BigInt(raw.denominator),num=scaled>den?den:scaled;
   if(BigInt(corrected.numerator)*den!==num*BigInt(corrected.denominator))throw new Error('Search correction does not match the original exact later probability.');
  }
  for(const [name,key] of [['fwer_p_value_ppm','romano'],['romano_wolf_p_value_ppm','romano'],['white_reality_p_value_ppm','white'],['spa_p_value_ppm','spa']]){
   const value=row.comparison.values?.find((/** @type {any} */ v)=>v.name===name),p=row.probabilities[key],n=BigInt(p.numerator)*1000000n,d=BigInt(p.denominator);
   if(value?.state!=='measured'||value.value!==String((n+d-1n)/d))throw new Error('Projected probability value differs from the conservative exact fraction.');
  }
  if(original.status!=='admitted'&&row.comparison.status==='admitted')throw new Error('Search comparison cannot promote a failed original qualification.');
  const guarded=body.projection_version>=2?Object.values(row.probabilities):[row.probabilities.romano];
  if(row.comparison.status==='admitted'&&guarded.some((/** @type {any} */ p)=>BigInt(p.numerator)*1000000n>BigInt(a.alpha_ppm)*BigInt(p.denominator)))throw new Error('Admitted search comparison exceeds the exact shared probability allowance.');
  projected.rows[index]={...original,...row.comparison};
 }
 await fetchBooleanEvidence({model:'qualification',identity:source.identity,completion:source.completion,offset:asked.offset,limit:asked.limit},async()=>({ok:true,json:async()=>projected}));
}

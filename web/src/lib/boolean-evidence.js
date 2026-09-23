import { detailRefusal } from './detail-refusal.js';
import { validateQualification } from './boolean-qualification.js';
const U64=18446744073709551615n, I64=9223372036854775807n;
/** @param {any} v */
const uint=v=>typeof v==='string'&&/^(0|[1-9][0-9]*)$/.test(v)&&BigInt(v)<=U64;
/** @param {any} v */
const integer=v=>typeof v==='string'&&/^(0|-?[1-9][0-9]*)$/.test(v)&&BigInt(v)>=-I64-1n&&BigInt(v)<=U64;
/** @param {any} v */
const hex=v=>typeof v==='string'&&/^[0-9a-f]{64}$/.test(v);
/** @param {any} v */
const object=v=>v!==null&&typeof v==='object'&&!Array.isArray(v);
/** @param {any} v @param {string[]} keys @param {(v:any)=>boolean} valid */
const fields=(v,keys,valid)=>object(v)&&keys.every(key=>valid(v[key]));
const RW=['measured','unavailable-constant-return-candidate','unavailable-numerical-refusal'];
/** Exact source-bit validation also distinguishes negative zero.
 * @param {any} v */
function floating(v) {
  if(!object(v)||!uint(v.bits))return false;
  const view=new DataView(new ArrayBuffer(8));view.setBigUint64(0,BigInt(v.bits),true);
  const value=view.getFloat64(0,true);
  return Number.isFinite(value)?typeof v.decimal==='string'&&v.decimal.length<=400&&/^-?(0|[1-9][0-9]*)(\.[0-9]+)?(e[+-]?[0-9]+)?$/i.test(v.decimal)&&Object.is(Number(v.decimal),value):v.decimal===null;
}
/** @param {any} v */
const fraction=v=>fields(v,['numerator','denominator'],uint)&&BigInt(v.denominator)>0n&&BigInt(v.numerator)<=BigInt(v.denominator);
/** @param {any} v */
function source(v) {
  return fields(v,['index','coordinates'],uint)&&fields(v,['identity','completion','membership_digest'],hex)&&typeof v.instrument==='string'&&v.instrument.length>0&&typeof v.cash==='boolean';
}
/** @param {any} v */
function familyTest(v) {
  return fields(v,['statistic','probability'],floating)&&fraction(v.exact_probability)&&fields(v,['matched_or_exceeded','draws','strategies','periods'],uint)&&BigInt(v.matched_or_exceeded)<=BigInt(v.draws);
}
/** @param {any} v */
function summary(v) {
  return fields(v,['scope','cohort','layout'],hex)&&fields(v,['families','candidates','periods','segments','periods_per_segment','splits','contributing_splits','bottom_half_splits'],uint)&&
    fields(v.procedure,['draws','seed','block_length'],uint)&&fields(v.bounds,['families','candidates','observations','bootstrap_work','split_work','additional_working_bytes','saved_body_bytes'],uint)&&
    fields(v.test_families,['white','spa','romano'],hex)&&familyTest(v.white)&&familyTest(v.spa)&&RW.includes(v.romano_availability)&&
    BigInt(v.families)>0n&&BigInt(v.candidates)>0n&&BigInt(v.periods)>0n&&BigInt(v.segments)>0n&&BigInt(v.periods_per_segment)*BigInt(v.segments)===BigInt(v.periods)&&
    BigInt(v.bottom_half_splits)<=BigInt(v.contributing_splits)&&BigInt(v.contributing_splits)<=BigInt(v.splits)&&
    [v.white,v.spa].every(test=>test.strategies===v.candidates&&test.periods===v.periods&&test.draws===v.procedure.draws);
}
/** @param {any} v @param {any} parent */
function statisticsRow(v,parent) {
  if(!fields(v,['index','coordinate','program_index','ordinal','execution_refusal_bits','trades','wins'],uint)||!fields(v,['identity','run','period_digest'],hex)||
    !source(v.source)||!['long','short'].includes(v.side)||!integer(v.return_paisa)||!floating(v.wilson_lower)||v.romano_availability!==parent.romano_availability||
    BigInt(v.source.index)>=BigInt(parent.families)||BigInt(v.coordinate)>=BigInt(v.source.coordinates)||BigInt(v.wins)>BigInt(v.trades)||BigInt(v.execution_refusal_bits)>63n)return false;
  if(v.romano_availability!=='measured')return v.romano===null;
  return fields(v.romano,['strategy','stepdown_rank','strict_exceedances'],uint)&&floating(v.romano.statistic)&&fraction(v.romano.initial)&&fraction(v.romano.adjusted)&&
    v.romano.strategy===v.index&&BigInt(v.romano.stepdown_rank)<BigInt(parent.candidates)&&BigInt(v.romano.strict_exceedances)<=BigInt(parent.procedure.draws);
}
/** Native 44-check verdict shared by both saved execution models.
 * @param {any} v */
export function validateNativeAdmissionRow(v) {
  if(!fields(v,['index','source_index','failed','unmeasured','refused'],uint)||!hex(v.identity)||v.source_index!==v.index||
    !['admitted','rejected','unmeasured','refused'].includes(v.status)||
    !Array.isArray(v.checks)||v.checks.length!==44||!Array.isArray(v.values)||v.values.length!==44)return false;
  const masks={failed:BigInt(v.failed),unmeasured:BigInt(v.unmeasured),refused:BigInt(v.refused)};
  const union=masks.failed|masks.unmeasured|masks.refused;
  if(union>=(1n<<44n)||(masks.failed&masks.unmeasured)!==0n||(masks.failed&masks.refused)!==0n||(masks.unmeasured&masks.refused)!==0n)return false;
  const status=masks.refused?'refused':masks.unmeasured?'unmeasured':masks.failed?'rejected':'admitted';
  if(v.status!==status||new Set(v.checks.map((/** @type {any} */ r)=>r.name)).size!==44||new Set(v.values.map((/** @type {any} */ r)=>r.name)).size!==44)return false;
  for(const [index,check] of v.checks.entries()) {
    const bit=1n<<BigInt(index);
    const state=masks.refused&bit?'refused':masks.unmeasured&bit?'unmeasured':masks.failed&bit?'failed':'passed';
    if(!object(check)||check.index!==String(index)||typeof check.name!=='string'||!check.name||check.state!==state)return false;
  }
  return v.values.every((/** @type {any} */ row)=>object(row)&&typeof row.name==='string'&&row.name.length>0&&
    ['measured','unmeasured','refused','complete','incomplete','rejected-null','did-not-reject'].includes(row.state)&&
    (row.state==='measured'?integer(row.value):row.value===null));
}
/** @param {any} v @param {any} parent */
function admissionRow(v,parent) {
  return validateNativeAdmissionRow(v)&&statisticsRow(v.statistics,parent)&&v.statistics.index===v.index&&v.statistics.identity===v.identity;
}
/** @param {any} selection */
export function booleanEvidenceSelection(selection) {
  const {model='statistics',identity,completion=null,kind='candidates',offset='0',limit=16}=selection;
  if(!['statistics','admission','qualification'].includes(model)||!hex(identity)||(completion!==null&&!hex(completion))||!uint(offset)||!Number.isInteger(limit)||limit<1||limit>256||
    !(model==='statistics'?['candidates','sources','splits']:['candidates']).includes(kind)||((offset!=='0'||kind!=='candidates')&&completion===null))throw new Error('Choose the exact evidence model, identity and completion for these pages.');
  return {model,identity,completion,kind,offset,limit};
}
/** @param {any} selection @param {(url:string)=>Promise<any>} request */
export async function fetchBooleanEvidence(selection,request) {
  const asked=booleanEvidenceSelection(selection);
  const query=new URLSearchParams({identity:asked.identity,kind:asked.kind,offset:asked.offset,limit:String(asked.limit)});
  if(asked.completion)query.set('completion',asked.completion);
  const path='/boolean-'+asked.model+'.json';
  const response=await request(path+'?'+query);
  if(!response?.ok)throw new Error(await detailRefusal(response,'Boolean '+asked.model+' detail failed (HTTP '+String(response?.status)+').'));
  const body=await response.json();
  if(!object(body)||body.schema_version!==1||body.status!=='saved'||body.authority!=='authenticated-saved-evidence-observation'||body.refusal!==null||body.model!==asked.model||body.identity!==asked.identity||
    !hex(body.completion)||(asked.completion!==null&&asked.completion!==body.completion)||body.kind!==asked.kind||body.offset!==asked.offset||body.limit!==asked.limit||body.page_complete!==true||
    !uint(body.total)||!uint(body.admitted_bytes)||!Array.isArray(body.rows)||!summary(body.summary)||typeof body.scope!=='string')throw new Error('Saved evidence identity, source chain or page contract differs.');
  const start=BigInt(asked.offset),total=BigInt(body.total),count=total-start<BigInt(asked.limit)?total-start:BigInt(asked.limit),end=start+BigInt(body.rows.length);
  if(start>total||count!==BigInt(body.rows.length)||body.next!==(end<total?String(end):null))throw new Error('Saved evidence page skips, truncates, repeats or ends early.');
  const expectedTotal=asked.kind==='sources'?body.summary.families:asked.kind==='splits'?body.summary.splits:body.summary.candidates;
  if(body.total!==expectedTotal)throw new Error('Saved evidence page count differs from the full parent population.');
  if(asked.model!=='statistics'&&(!hex(body.statistics_identity)||!hex(body.statistics_completion)||!object(body.policy)||!hex(body.policy.digest)||!Array.isArray(body.policy.values)||body.policy.values.length!==39||
    new Set(body.policy.values.map((/** @type {any} */ r)=>r.name)).size!==39||!body.policy.values.every((/** @type {any} */ r)=>object(r)&&typeof r.name==='string'&&r.name.length>0&&(typeof r.value==='boolean'||integer(r.value)))))throw new Error('Saved admission policy or statistics link is incomplete.');
  for(const [index,row] of body.rows.entries()) {
    if(!object(row)||row.index!==String(start+BigInt(index)))throw new Error('Saved row order changed.');
    let valid=false;
    if(asked.model!=='statistics')valid=admissionRow(row,body.summary);
    else if(asked.kind==='sources')valid=source(row);
    else if(asked.kind==='candidates')valid=statisticsRow(row,body.summary);
    else {
      valid=fields(row,['train_mask','test_mask'],uint)&&typeof row.bottom_half==='boolean'&&typeof row.rankable==='boolean'&&hex(row.scores_digest)&&(!row.bottom_half||row.rankable);
      if(valid) {const train=BigInt(row.train_mask),test=BigInt(row.test_mask),segments=BigInt(body.summary.segments);valid=segments<=63n&&(train&test)===0n&&(train|test)===(1n<<segments)-1n;}
    }
    if(!valid)throw new Error('Saved evidence values, partitions or source links do not reconcile.');
  }
  if(asked.model==='qualification')validateQualification(body);
  return body;
}

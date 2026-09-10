/** Additional fixed-training/later contracts after the common 44-check validator. */
import { validateIndexConsistency, indexConsistencyContext } from './index-consistency.js';
const U64=(1n<<64n)-1n, U128=(1n<<128n)-1n, I64=(1n<<63n)-1n;
/** @param {any} v @param {bigint} max */
const unsigned=(v,max=U64)=>typeof v==='string'&&v.length<=39&&/^(0|[1-9][0-9]*)$/.test(v)&&BigInt(v)<=max;
/** @param {any} v */
const signed=v=>typeof v==='string'&&v.length<=20&&/^(0|-?[1-9][0-9]*)$/.test(v)&&BigInt(v)>=-I64-1n&&BigInt(v)<=I64;
/** @param {any} v */
const hex=v=>typeof v==='string'&&/^[0-9a-f]{64}$/.test(v);
/** @param {any} v */
const object=v=>v!==null&&typeof v==='object'&&!Array.isArray(v);
/** @param {any} v @param {string[]} keys @param {(v:any)=>boolean} check */
const fields=(v,keys,check)=>object(v)&&keys.every(key=>check(v[key]));
/** @param {any} v */
const link=v=>fields(v,['identity','completion'],hex);
/** @param {any} p @param {bigint} max */
const fraction=(p,max=U64)=>object(p)&&unsigned(p.numerator,max)&&unsigned(p.denominator,max)&&BigInt(p.denominator)>0n&&BigInt(p.numerator)<=BigInt(p.denominator);
/** @param {any} v */
function floating(v){if(!object(v)||!unsigned(v.bits)||typeof v.decimal!=='string'||v.decimal.length>400||! /^-?(0|[1-9][0-9]*)(\.[0-9]+)?(e[+-]?[0-9]+)?$/i.test(v.decimal))return false;const data=new DataView(new ArrayBuffer(8));data.setBigUint64(0,BigInt(v.bits),true);const n=data.getFloat64(0,true);return Number.isFinite(n)&&Object.is(n,Number(v.decimal));}
/** @param {any} body */
export function validateQualification(body){
  const q=body.qualification,a=q?.allocation,p=q?.procedure;
  if(!fields(q,['scope','unit'],hex)||!link(q.original)||!unsigned(q.fold_count)||BigInt(q.fold_count)===0n||!fields(p,['draws','seed','block_length'],unsigned)||BigInt(p.block_length)===0n||
    !fields(a,['batch','batches','rung','rungs','alpha_ppm'],unsigned)||!hex(a.digest)||!fraction(a.threshold,U128)||
    BigInt(a.batches)===0n||BigInt(a.rungs)===0n||BigInt(a.batch)>=BigInt(a.batches)||BigInt(a.rung)>=BigInt(a.rungs)||BigInt(a.alpha_ppm)>1000000n||
    (a.minimum_draws!==null&&!unsigned(a.minimum_draws,U128))||typeof a.draw_resolution_met!=='boolean'||!Array.isArray(q.later)||String(q.later.length)!==body.summary.families)throw new Error('Qualification scope, allocation or procedure is incomplete.');
  const numerator=BigInt(a.threshold.numerator),denominator=BigInt(a.threshold.denominator);
  if(numerator*BigInt(a.batches)*BigInt(a.rungs)*1000000n!==BigInt(a.alpha_ppm)*denominator)throw new Error('Qualification threshold differs from its complete declared multiplicity.');
  const minimum=numerator===0n?null:String((denominator+numerator-1n)/numerator-1n);
  if(a.minimum_draws!==minimum||a.draw_resolution_met!==(minimum!==null&&BigInt(p.draws)>=BigInt(minimum)))throw new Error('Bootstrap draw resolution was misrepresented.');
  for(const [index,later] of q.later.entries())if(!link(later)||!fields(later,['index','coordinates','sessions'],unsigned)||later.index!==String(index)||!later.instrument)throw new Error('Qualification later family link is incomplete.');
  for(const row of body.rows){const detail=row.qualification,source=q.later[Number(detail?.family)];
    if(!fields(detail,['family','coordinate'],unsigned)||!hex(detail.identity)||!link(detail.source)||detail.family!==row.statistics.source.index||detail.coordinate!==row.statistics.coordinate||!source||source.identity!==detail.source.identity||source.completion!==detail.source.completion||BigInt(detail.coordinate)>=BigInt(source.coordinates))throw new Error('Qualification original/later coordinate linkage differs.');
    validateRomano(detail.romano,body.summary.candidates,p.draws);
    if(detail.folds!==null)validateFolds(detail.folds,q.fold_count,row.statistics,source.sessions);
    validateIndexConsistency(row.index_consistency,indexConsistencyContext(body,row));
  }
}
/** @param {any} r @param {string} count @param {string} draws */
function validateRomano(r,count,draws){
  if(!object(r)||!['measured-positive','conservative-zero','conservative-nonpositive'].includes(r.classification)||!fraction(r.probability))throw new Error('Qualification exact probability is invalid.');
  if(r.classification!=='measured-positive'&&(r.probability.numerator!==r.probability.denominator||BigInt(r.probability.denominator)!==BigInt(draws)+1n))throw new Error('Zero or nonpositive observations cannot acquire a favourable probability.');
  if(r.classification==='conservative-zero'){if(r.shared!==null)throw new Error('Zero-return evidence cannot invent shared bootstrap facts.');return;}
  const s=r.shared;
  if(!fields(s,['strategy','stepdown_rank','strict_exceedances'],unsigned)||BigInt(s.strategy)>=BigInt(count)||BigInt(s.stepdown_rank)>=BigInt(count)||!floating(s.statistic)||!fraction(s.initial)||!fraction(s.adjusted)||
    BigInt(s.strict_exceedances)>BigInt(draws)||BigInt(s.initial.numerator)!==BigInt(s.strict_exceedances)+1n||BigInt(s.initial.denominator)!==BigInt(draws)+1n||s.initial.denominator!==s.adjusted.denominator||BigInt(s.adjusted.numerator)<BigInt(s.initial.numerator)||
    (r.classification==='measured-positive'?(Number(s.statistic.decimal)<=0||r.probability.numerator!==s.adjusted.numerator||r.probability.denominator!==s.adjusted.denominator):Number(s.statistic.decimal)>0))throw new Error('Qualification shared bootstrap facts or classification differ.');
}
/** @param {any} f @param {string} count @param {any} original @param {string} sessions */
function validateFolds(f,count,original,sessions){
  if(!fields(f,['plan','selected','anchor','later','training_run','later_run','resolution'],hex)||!fields(f,['ordinal','execution_refusal_bits','decided','profitable'],unsigned)||!fields(f,['training_last_day','first_day','last_day','return_paisa'],signed)||!Array.isArray(f.rows)||String(f.rows.length)!==count||f.decided!==count||f.training_run!==original.run||f.ordinal!==original.ordinal||BigInt(f.execution_refusal_bits)>63n||BigInt(f.training_last_day)>=BigInt(f.first_day))throw new Error('Fixed-training fold identities, extent or clock boundary differ.');
  let next=BigInt(f.first_day),sum=0n,observed=0n,profitable=0n;
  for(const [index,row] of f.rows.entries()){
    if(!fields(row,['index','sessions','trades','wins'],unsigned)||!fields(row,['first_day','last_day','return_paisa'],signed)||row.index!==String(index)||BigInt(row.first_day)!==next||BigInt(row.last_day)<next||BigInt(row.wins)>BigInt(row.trades))throw new Error('A fixed-training fold is missing, overlapping or inconsistent.');
    next=BigInt(row.last_day)+1n;sum+=BigInt(row.return_paisa);observed+=BigInt(row.sessions);if(BigInt(row.return_paisa)>0n)profitable+=1n;
  }
  if(next!==BigInt(f.last_day)+1n||sum!==BigInt(f.return_paisa)||observed!==BigInt(sessions)||profitable!==BigInt(f.profitable))throw new Error('Fixed-training folds do not conserve all later sessions and returns.');
}

import test from 'node:test';
import assert from 'node:assert/strict';
import {validateBooleanWorkModel} from '../src/lib/boolean-work-model.js';
const descriptor=(conditions=384)=>({version:1,state:'lower_bound_only',scope:'full_live_alphabet_per_timeframe',live_conditions:conditions,max_instructions:1151,conjunction_program_lower_bound:String((1n<<BigInt(conditions))-1n),lower_bound_exceeds_cumulative_counter:conditions>64,timeframe_execution:'sequential',total_programs:null,eta_seconds:null,refusal:null});
test('all native mask widths retain exact mathematical lower bounds without totals or ETAs',()=>{
 for(let n=1;n<=384;n++){const d=descriptor(n);assert.strictEqual(validateBooleanWorkModel(d),d);assert.equal(d.lower_bound_exceeds_cumulative_counter,n>64);assert.equal(d.eta_seconds,null);}
 assert.equal(descriptor(64).conjunction_program_lower_bound,'18446744073709551615');assert.equal(descriptor().conjunction_program_lower_bound.length,116);
});
test('fake measured populations, estimated hours, impossible alphabets and rounded lower bounds are refused',()=>{
 for(const change of [(/** @type {any} */ d) =>d.total_programs='999',(/** @type {any} */ d) =>d.eta_seconds=3600,(/** @type {any} */ d) =>d.scope='custom_alphabet',(/** @type {any} */ d) =>d.live_conditions=0,(/** @type {any} */ d) =>d.live_conditions=385,(/** @type {any} */ d) =>d.live_conditions=1.5,(/** @type {any} */ d) =>d.max_instructions=766,(/** @type {any} */ d) =>d.conjunction_program_lower_bound=Number(d.conjunction_program_lower_bound),(/** @type {any} */ d) =>d.conjunction_program_lower_bound+='0',(/** @type {any} */ d) =>d.lower_bound_exceeds_cumulative_counter=false,(/** @type {any} */ d) =>d.timeframe_execution='parallel',(/** @type {any} */ d) =>delete d.refusal,(/** @type {any} */ d) =>d.extra=true]){const d=descriptor();change(d);assert.throws(()=>validateBooleanWorkModel(d),String(change));}
});
test('a native sizing refusal is visible and cannot carry invented successful metrics',()=>{
 const d={version:1,state:'refused',refusal:'live alphabet unavailable'};assert.strictEqual(validateBooleanWorkModel(d),d);
 for(const x of [null,{}, {...d,refusal:''},{...d,refusal:'x'.repeat(4097)},{...d,total_programs:'0'}])assert.throws(()=>validateBooleanWorkModel(x));
});

import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {nativePolicyFields,nativePolicyModule} from '../policy-guide/native-policy-schema.mjs';
import {NATIVE_POLICY_FIELDS,NATIVE_POLICY_NAMES} from '../src/lib/native-policy-schema.js';

const native=readFileSync(new URL('../../crates/runner/src/admission.rs',import.meta.url),'utf8');
test('the shipped browser schema exactly matches all native policy names and wire types without threshold defaults',()=>{
 assert.deepEqual(nativePolicyFields(native),NATIVE_POLICY_FIELDS);
 assert.equal(NATIVE_POLICY_NAMES.length,39);assert.equal(new Set(NATIVE_POLICY_NAMES).size,39);
 assert.equal(NATIVE_POLICY_FIELDS.min_worst_reward_risk_ppm,'u64');
 assert.equal(NATIVE_POLICY_FIELDS.min_pessimistic_profit_paisa,'i64');
 assert.equal(NATIVE_POLICY_FIELDS.require_white_reality_rejection,'bool');
 assert.equal(readFileSync(new URL('../src/lib/native-policy-schema.js',import.meta.url),'utf8'),nativePolicyModule(native));
 assert.ok(Object.isFrozen(NATIVE_POLICY_FIELDS));assert.ok(Object.isFrozen(NATIVE_POLICY_NAMES));
});

test('unsupported native types, duplicate names and a changed or missing V1 field cannot generate a permissive browser schema',()=>{
 const field='pub min_worst_reward_risk_ppm: u64,';assert.ok(native.includes(field));
 for(const replacement of ['pub min_worst_reward_risk_ppm: f64,','pub min_worst_reward_risk_ppm: u32,','pub min_worst_reward_risk_ppm: Option<u64>,',`${field}\n    ${field}`,'','pub renamed_reward_risk_ppm: u64,']){
  assert.throws(()=>nativePolicyFields(native.replace(field,replacement)),/Unsupported|Duplicate|39 required fields/,replacement);
 }
 assert.throws(()=>nativePolicyFields(native.replace('impl AdmissionFieldV1 {','impl MissingFieldTable {')),/boundaries/);
 assert.throws(()=>nativePolicyFields(native.replace('Self::MinWorstRewardRiskPpm => "min_worst_reward_risk_ppm"','Self::MinWorstRewardRiskPpm => "unrecognized_gate"')),/39 required fields/);
});

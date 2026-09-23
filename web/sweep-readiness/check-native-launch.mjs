// Frontend/native transport verification only. This never submits a request.
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import assert from 'node:assert/strict';
import { validateBooleanLaunchMetadata, booleanLaunchPlan } from '../src/lib/boolean-launch.js';

const artifact = process.argv[2];
if (!artifact || process.argv.length !== 3) throw new Error('Supply the retained native metadata test log.');
const log = await readFile(artifact, 'utf8');
const tagged = log.split('BRUTEX_BOOLEAN_LAUNCH_METADATA=');
assert.equal(tagged.length, 2, 'Require exactly one actual native metadata receipt.');
const serialized = tagged[1].split('\n')[0];
const metadata = validateBooleanLaunchMetadata(JSON.parse(serialized));
assert.equal(metadata.ready, true);
const plan = booleanLaunchPlan({
  feed: 'zerodha', symbols: ['NSE-NIFTY', 'NSE-RELIANCE'],
  from: '2025-01', to: '2025-02', laterFrom: '2025-03', laterTo: '2025-04',
  bits: 'all', horizonBars: '5', maxPoints: metadata.configured.max_points,
  batchPrograms: '1', nodeAllowance: '1', batchAllowance: '1'
}, metadata);
assert.equal(plan.expected_policy_digest, metadata.policy.digest);
assert.equal(metadata.policy.values.find(row => row.name === 'min_weakest_period_return_paisa').value, '-500');
assert.equal(metadata.policy.values.find(row => row.name === 'require_white_reality_rejection').value, true);
assert.equal(metadata.policy.values.find(row => row.name === 'max_losing_trades').value, '18446744073709551615');
console.log(JSON.stringify({
  schema_version: 1, check: 'Actual native launch metadata accepted by the shipped frontend', passed: true,
  metadata_sha256: createHash('sha256').update(serialized).digest('hex'),
  helper_sha256: createHash('sha256').update(await readFile(new URL('../src/lib/boolean-launch.js', import.meta.url))).digest('hex'),
  native_policy_digest: metadata.policy.digest, native_policy_values: metadata.policy.values.length,
  timeframes: metadata.timeframes, exact_policy_bound: true,
  signed_paisa_preserved: true, boolean_requirement_preserved: true, maximum_u64_preserved: true,
  scope: 'Transport fixture only; no historical computation and no request submitted'
}, null, 2));

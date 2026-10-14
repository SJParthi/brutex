// R4-01 (PERF-13 display): the projection line every plan preview renders,
// the contract R4-07, R4-23 and R4-29 serve against. Invented numbers only.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { bytesText, projectionLine } from '../src/lib/pull-plan.js';

const GiB = 1024 ** 3;

test('a_plan_projection_renders_one_line_and_a_refusal_by_name', () => {
  const ok = projectionLine({ projection: { bytes: 128 * GiB, free_after: 401 * GiB, floor: 75 * GiB, basis: 'extrapolation', refused: null } });
  assert.deepEqual(ok, { line: 'projected 128.0 GiB (extrapolation); free after 401.0 GiB; floor 75.0 GiB', refused: false, fault: false });
  // A refusal by name: a reason string, a coded object, or the floor itself.
  assert.equal(
    projectionLine({ projection: { bytes: 600 * GiB, free_after: 10 * GiB, floor: 75 * GiB, basis: 'extrapolation', refused: { code: 'below_disk_floor', why: 'free space after the plan is under the floor' } } })?.line,
    'projected 600.0 GiB (extrapolation); free after 10.0 GiB; floor 75.0 GiB; refused (below_disk_floor): free space after the plan is under the floor'
  );
  const floor = projectionLine({ projection: { bytes: 600 * GiB, free_after: 10 * GiB, floor: 75 * GiB, basis: 'extrapolation', refused: true } });
  assert.equal(floor?.refused, true);
  assert.match(floor?.line ?? '', /refused: free space after the plan \(10\.0 GiB\) would be below the floor \(75\.0 GiB\)$/);
  assert.match(projectionLine({ projection: { bytes: 1, free_after: 1, floor: 1, basis: 'extrapolation', refused: 'the drive is not mounted' } })?.line ?? '',
    /; refused: the drive is not mounted$/);
  // No field, no line.
  assert.equal(projectionLine({ months: [] }), null);
  assert.equal(projectionLine({ projection: null }), null);
  assert.equal(projectionLine(null), null);
  // The basis is the server's word, never relabelled; none is said.
  assert.match(projectionLine({ projection: { bytes: 0, free_after: 5, floor: 1, basis: 'measured' } })?.line ?? '', /^projected 0 B \(measured\)/);
  assert.match(projectionLine({ projection: { bytes: 0, free_after: 5, floor: 1 } })?.line ?? '', /\(no basis stated\)/);
  // A malformed projection is named, never dropped and never a zero.
  const bad = projectionLine({ projection: { bytes: -1, free_after: 1.5, floor: 1, refused: 'x' } });
  assert.deepEqual(bad, { line: "The plan's projection carries no whole byte count in bytes, free_after, so no disk figure is shown; refused: x.", refused: true, fault: true });
  assert.equal(projectionLine({ projection: 'big' })?.fault, true);
  assert.equal(projectionLine({ projection: [1] })?.fault, true);
});

test('bytes read in binary units, exact below a KiB', () => {
  assert.equal(bytesText(0), '0 B');
  assert.equal(bytesText(1023), '1023 B');
  assert.equal(bytesText(1024), '1.0 KiB');
  assert.equal(bytesText(1536 * 1024 * 1024), '1.5 GiB');
  assert.equal(bytesText(2 ** 53 - 1), '8.0 PiB');
});

// R4-01 repair round 1: free space after a plan BELOW ZERO is the plan not
// fitting, said with the shortfall — the reading R4-07's route is held to —
// never a malformed field and never a quiet line.
test('a plan larger than the free space reads as short by its shortfall', () => {
  const short = projectionLine({ projection: { bytes: 600 * GiB, free_after: -88 * GiB, floor: 75 * GiB, basis: 'extrapolation', refused: null } });
  assert.deepEqual(short, { line: 'projected 600.0 GiB (extrapolation); free after none, short by 88.0 GiB; floor 75.0 GiB; the plan does not fit the disk', refused: true, fault: false });
  const named = projectionLine({ projection: { bytes: 600 * GiB, free_after: -1, floor: 75 * GiB, basis: 'extrapolation', refused: true } });
  assert.equal(named?.line, 'projected 600.0 GiB (extrapolation); free after none, short by 1 B; floor 75.0 GiB; refused: free space after the plan (none, short by 1 B) would be below the floor (75.0 GiB)');
  assert.equal(projectionLine({ projection: { bytes: 1, free_after: -1.5, floor: 1 } })?.fault, true, 'a fraction is still malformed');
});

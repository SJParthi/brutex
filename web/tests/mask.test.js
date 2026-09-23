import { test } from 'node:test';
import assert from 'node:assert/strict';

import { decodeMaskWords } from '../src/lib/mask.js';

const zeros = () => ['0', '0', '0', '0', '0', '0'];

test('six canonical u64 words decode little-endian without Number rounding', () => {
  const words = zeros();
  words[0] = '9223372036854775809'; // bits 63 and 0
  words[5] = '18446744073709551615'; // every bit in the last word
  const result = decodeMaskWords(words);
  assert.equal(result.ok, true, result.why);
  assert.deepEqual(result.positions.slice(0, 2), [0, 63]);
  assert.deepEqual(result.positions.slice(2), Array.from({ length: 64 }, (_, bit) => 320 + bit));
  assert.equal(result.positions.at(-1), 383);
});

test('an empty canonical mask is valid and different from an undecodable mask', () => {
  assert.deepEqual(decodeMaskWords(zeros()), { ok: true, positions: [], why: '' });
  const bad = decodeMaskWords(['0']);
  assert.equal(bad.ok, false);
  assert.deepEqual(bad.positions, []);
  assert.match(bad.why, /exactly 6/);
});

test('mask decoding is all or nothing across length, syntax, type and u64 bounds', () => {
  const cases = [
    undefined,
    ['1', '0', '0', '0', '0'],
    ['1', '0', '0', '0', '0', '0', '0'],
    ['1', '-1', '0', '0', '0', '0'],
    ['1', '+1', '0', '0', '0', '0'],
    ['1', '01', '0', '0', '0', '0'],
    ['1', ' 1', '0', '0', '0', '0'],
    ['1', '1e3', '0', '0', '0', '0'],
    ['1', 1, '0', '0', '0', '0'],
    ['1', '18446744073709551616', '0', '0', '0', '0']
  ];
  for (const input of cases) {
    const result = decodeMaskWords(input);
    assert.equal(result.ok, false, JSON.stringify(input));
    assert.deepEqual(result.positions, [], 'no valid-looking prefix may survive');
    assert.ok(result.why.length > 0);
  }
});

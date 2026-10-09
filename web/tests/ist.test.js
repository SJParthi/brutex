// THE ONE WEB COPY OF THE IST OFFSET AND THE REGULAR SESSION, AND THE CANDLE
// GRID THE MARKETS PAGE DERIVES ON (D-3516; ONEAUTH-16, ONEAUTH-17).
//
// `$lib/ist.js` restates `pull::session`'s three numbers once. This file holds
// it to the Rust definitions it copies, holds its `bucketStart` to the grid
// `pull::fold` files the store's rungs on, drives the markets page's own
// `aggregate` through it, and refuses any other spelling of either number
// anywhere under `web/src`.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parse } from 'svelte/compiler';

import {
  IST_OFFSET_SECONDS,
  IST_OFFSET_MS,
  IST_OFFSET_MICROS,
  IST_OFFSET_MICROS_BIG,
  SESSION_OPEN_MINUTE,
  SESSION_MINUTES,
  bucketStart
} from '../src/lib/ist.js';
import { SESSION_MINUTES as PRESET_SESSION_MINUTES } from '../src/lib/presets.js';

const ROOT = fileURLToPath(new URL('../..', import.meta.url));
const SRC = fileURLToPath(new URL('../src', import.meta.url));

/**
 * The value of one `pub const NAME: T = <expr>;` line of a Rust file, where
 * `<expr>` is a sum of products of integer literals (the only shape the three
 * definitions use). Anything else is refused rather than guessed.
 *
 * @param {string} file repository-relative path
 * @param {string} name
 * @returns {number}
 */
function rustConst(file, name) {
  const text = readFileSync(join(ROOT, file), 'utf8');
  const found = [...text.matchAll(new RegExp(`^pub const ${name}: [iu](?:32|64) = ([0-9_ *+]+);$`, 'gm'))];
  assert.equal(found.length, 1, `${file}: exactly one definition of ${name} in the shape this reader reads`);
  return found[0][1]
    .split('+')
    .map((term) => term.split('*').map((factor) => Number(factor.trim().replaceAll('_', ''))).reduce((a, b) => a * b, 1))
    .reduce((a, b) => a + b, 0);
}

test('the web copy is the Rust definition it restates', () => {
  assert.equal(IST_OFFSET_SECONDS, rustConst('crates/pull/src/session.rs', 'IST_OFFSET_SECS'));
  assert.equal(SESSION_OPEN_MINUTE, rustConst('crates/pull/src/session.rs', 'SESSION_OPEN_MINUTE'));
  assert.equal(SESSION_MINUTES, rustConst('crates/pull/src/session.rs', 'BARS_PER_REGULAR_SESSION'));
  assert.equal(IST_OFFSET_SECONDS, 19_800);
  assert.equal(SESSION_OPEN_MINUTE, 555);
  assert.equal(SESSION_MINUTES, 375);
  assert.equal(IST_OFFSET_MS, 19_800_000);
  assert.equal(IST_OFFSET_MICROS, 19_800_000_000);
  assert.equal(IST_OFFSET_MICROS_BIG, 19_800_000_000n);
  assert.equal(PRESET_SESSION_MINUTES, SESSION_MINUTES, 'presets re-export the one copy');
});

// Monday 2026-08-03, 09:15 IST, in epoch seconds: 03:45 UTC.
const OPEN = Date.UTC(2026, 7, 3, 3, 45) / 1000;
const MIDNIGHT = OPEN - 555 * 60;
const DAY = 86_400;

test('an intraday candle is counted from the 09:15 open, as pull::fold files the rung', () => {
  // Every rung the store files below a day. The open-anchored grid inside a
  // session is `OPEN + floor(minute * 60 / width) * width`.
  for (const width of [60, 120, 180, 300, 600, 900, 1_800, 3_600]) {
    for (let minute = 0; minute < 375; minute += 1) {
      const t = OPEN + minute * 60;
      assert.equal(bucketStart(t, width), OPEN + Math.floor((minute * 60) / width) * width, `${width}s at minute ${minute}`);
    }
    // The first candle of the session starts AT the open, never before it.
    assert.equal(bucketStart(OPEN, width), OPEN, `${width}s first candle`);
    // And the same holds the next session and a session decades earlier.
    assert.equal(bucketStart(OPEN + DAY, width), OPEN + DAY);
    assert.equal(bucketStart(OPEN - 20_000 * DAY, width), OPEN - 20_000 * DAY);
  }
  // The midnight grid the markets page used put these first candles before the
  // open: 30 and 60 minutes at 09:00, 2 at 09:14, 10 at 09:10.
  assert.notEqual(bucketStart(OPEN, 1_800), Math.floor((OPEN + 19_800) / 1_800) * 1_800 - 19_800);
  assert.notEqual(bucketStart(OPEN, 3_600), Math.floor((OPEN + 19_800) / 3_600) * 3_600 - 19_800);
});

test('a width that does not divide a day still begins every session at 09:15', () => {
  for (const width of [420, 2_700, 4_500]) {
    for (const day of [0, 1, 7]) {
      const open = OPEN + day * DAY;
      assert.equal(bucketStart(open, width), open, `${width}s on day ${day}`);
      assert.equal(bucketStart(open + width - 1, width), open);
      assert.equal(bucketStart(open + width, width), open + width);
    }
  }
});

test('a day or wider is counted from IST midnight', () => {
  assert.equal(bucketStart(OPEN, DAY), MIDNIGHT);
  assert.equal(bucketStart(OPEN + 374 * 60, DAY), MIDNIGHT);
  assert.equal(bucketStart(MIDNIGHT, DAY), MIDNIGHT);
  assert.equal(bucketStart(MIDNIGHT - 1, DAY), MIDNIGHT - DAY);
  assert.equal(bucketStart(MIDNIGHT + DAY, DAY), MIDNIGHT + DAY);
});

test("the markets page's own aggregate draws the stored rung's candles", () => {
  const source = readFileSync(join(SRC, 'routes/markets/+page.svelte'), 'utf8');
  const found = parse(source).instance?.content.body.find(
    (/** @type {any} */ node) => node.type === 'FunctionDeclaration' && node.id?.name === 'aggregate'
  );
  assert.ok(found, 'markets: the aggregate function is required');
  const aggregate = new Function('bucketStart', `${source.slice(found.start, found.end)}\nreturn aggregate;`)(bucketStart);
  const minutes = [];
  for (let minute = 0; minute < 375; minute += 1) {
    minutes.push({ t: OPEN + minute * 60, o: 100 + minute, h: 200 + minute, l: 50 + minute, c: 101 + minute, v: 1 });
  }
  const half = aggregate(minutes, 1_800);
  // 375 minutes in 30-minute candles from 09:15: twelve full and one of 15.
  assert.equal(half.length, 13);
  assert.equal(half[0].t, OPEN);
  assert.deepEqual(half[0], { t: OPEN, o: 100, h: 229, l: 50, c: 130, v: 30 });
  assert.deepEqual(half[12], { t: OPEN + 360 * 60, o: 460, h: 574, l: 410, c: 475, v: 15 });
  const hour = aggregate(minutes, 3_600);
  assert.equal(hour.length, 7);
  assert.deepEqual(
    hour.map((/** @type {{ t: number }} */ bar) => (bar.t - OPEN) / 60),
    [0, 60, 120, 180, 240, 300, 360]
  );
  assert.deepEqual(aggregate(minutes, DAY).map((/** @type {{ t: number }} */ bar) => bar.t), [MIDNIGHT]);
  assert.deepEqual(aggregate(minutes, null), []);
});

/**
 * The code of one source file with its comments removed: `/* … *\/` blocks,
 * `<!-- … -->` blocks, and `//` to the end of a line unless a `:` precedes it
 * (a URL's scheme).
 *
 * @param {string} text
 * @returns {string}
 */
function codeOf(text) {
  return text
    .replace(/\/\*[\s\S]*?\*\//g, ' ')
    .replace(/<!--[\s\S]*?-->/g, ' ')
    .replace(/(^|[^:])\/\/.*$/gm, '$1');
}

/** Spellings of the offset in any unit, and of the session's open and length. */
const SPELLINGS = [
  /(?<![\w.,])19_?800(?:_?000)*n?(?!\w)/,
  /(?<![\w.,])(?:375|555)(?!\w)/
];

/**
 * Each `file:line: code` under `dir` that spells one of the numbers.
 *
 * @param {string} dir
 * @returns {string[]}
 */
function restatements(dir) {
  /** @type {string[]} */
  const out = [];
  for (const name of readdirSync(dir).sort()) {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) {
      out.push(...restatements(path));
      continue;
    }
    if (!/\.(?:js|ts|svelte)$/.test(name)) continue;
    const file = relative(SRC, path);
    if (file === join('lib', 'ist.js')) continue;
    codeOf(readFileSync(path, 'utf8'))
      .split('\n')
      .forEach((line, n) => {
        if (SPELLINGS.some((spelling) => spelling.test(line))) out.push(`${file}:${n + 1}: ${line.trim()}`);
      });
  }
  return out;
}

test('no other file under web/src spells the offset or the session', () => {
  assert.deepEqual(restatements(SRC), []);
});

test('the reader reads code and not comments', () => {
  for (const code of [
    'const IST = 19800;',
    'const IST_MS = 19_800_000;',
    'const IST = 19800000000n;',
    'x + 19_800_000_000',
    'const BARS = 375;',
    "{ per: 375 }",
    'minute - 555',
    'f(19800) // a trailing comment does not hide the code before it'
  ]) {
    assert.ok(SPELLINGS.some((spelling) => spelling.test(codeOf(code))), `passed: ${code}`);
  }
  for (const text of [
    '// const IST = 19800;',
    '/* 375 bars\n   and 19_800 seconds */',
    '<!-- 555 -->',
    '`11,07,375 rows`',
    'width: 375px;',
    'const x = 3750;',
    'const y = 1.375;'
  ]) {
    assert.ok(!SPELLINGS.some((spelling) => spelling.test(codeOf(text))), `refused: ${text}`);
  }
});

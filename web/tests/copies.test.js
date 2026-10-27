// CC-5: the extra-copies register as tables (/disk/copies.json). Every fixture
// is invented: no path, inode or count here was read from a real machine.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { ROUTE, copiesView, formatBytes, retirementFileName } from '../src/lib/copies.js';
import { answerLine } from '../src/lib/ingest-panels.js';

const SENTENCE = 'the operator removes these by hand; the app never does';
const NOT_UTF8 = '"/invented/queue/caf\\xE9.zip"';

const root = (over = {}) => ({
  path: '/invented/queue',
  state: 'walked',
  refused: null,
  files: 3,
  dirs: 1,
  links_not_followed: 0,
  others_not_opened: 0,
  bytes: 33,
  extra_bytes: 16,
  newest_mtime: '2026-10-24 19:00:00 IST',
  reader: 'none found',
  inside: null,
  proof: '/invented/queue/run/ALL_VERIFIED.txt',
  receipts: [],
  receipt_count: 1,
  twin: { proven: false, words: 'no proven twin (invented): these bytes may be the ONLY copy' },
  duplicates_live: true,
  linked_live: 1,
  linked_elsewhere: [],
  faults: [],
  skipped_roots: [],
  revisited: [],
  retirable: true,
  ...over
});
const hazard = (over = {}) => ({
  hazard: true,
  path: '/invented/queue/2020.p.zip.part',
  live: '/invented/Options/2020.zip',
  kind: 'nfo_archives',
  dev: 64769,
  ino: 4242,
  nlink: 2,
  bytes: 17,
  partial: true,
  ...over
});
const live = (over = {}) => ({ kind: 'nfo_archives', path: '/invented/Options', named_by: 'INVENTED_NFO_ARCHIVES', walked: true, why: null, files: 1, ...over });
const retire = (over = {}) => ({
  path: '/invented/queue',
  bytes: 16,
  files: 3,
  why: 'no configured root, setting or store step names it',
  twin: 'no proven twin (invented)',
  caution: null,
  hazard: false,
  ...over
});
const RETIREMENT_TEXT = `RETIREMENT LIST, walked 2026-10-24 19:50:00 IST: ${SENTENCE}.\n1. "/invented/queue"\n   bytes freed: 16 in 3 file(s)\n`;
const walked = (over = {}) => ({
  state: 'walked',
  age_ms: 2999,
  fresh: true,
  why: 'the first request since start',
  rewalking: false,
  key: 'disk.extra_copy_roots',
  walked_at: '2026-10-24 19:50:00 IST',
  walked_at_micros: 1761315600000000,
  walk_us: 1234,
  note: 'a link made deeper in a tree is seen at the next walk',
  roots_walked: ['/invented/queue', '/invented/Options'],
  whole: true,
  extra_copies_bytes: 16,
  roots: [root()],
  live: [live(), live({ kind: 'store', path: '/invented/store', named_by: 'the store root this process serves', walked: false, why: 'not walked for second names (invented)', files: 0 })],
  unset_live: ['cm_archive: invented reason'],
  hazards: [hazard()],
  hazard_count: 1,
  retirement_list: [
    retire({ caution: '1 name(s) in it are second names of live files' }),
    retire({ path: '/invented/queue/2020.p.zip.part', bytes: 0, files: 1, why: 'a second name of a live file', twin: 'frees no byte', caution: 'the live data stays', hazard: true })
  ],
  retirement_text: RETIREMENT_TEXT,
  retirement_sentence: SENTENCE,
  ...over
});
const waiting = (over = {}) => ({
  state: 'waiting',
  key: 'disk.extra_copy_roots',
  why: 'disk.extra_copy_roots is not set: invented words, never a default, so nothing is walked',
  settings_degraded: null,
  walked: false,
  ...over
});

/** The walked view of `body`, or a failure naming what came back instead. @param {any} body */
function walkedView(body) {
  const v = copiesView(body);
  if (v.fault !== null || v.state !== 'walked') throw new Error(`not a walked view: ${JSON.stringify(v)}`);
  return v;
}
/** The waiting view of `body`, or a failure naming what came back instead. @param {any} body */
function waitingView(body) {
  const v = copiesView(body);
  if (v.fault !== null || v.state !== 'waiting') throw new Error(`not a waiting view: ${JSON.stringify(v)}`);
  return v;
}

test('waiting: one line with the server why verbatim, and no folder invented', () => {
  const v = waitingView(waiting());
  assert.ok(v.line.endsWith(waiting().why), v.line);
  assert.match(v.line, /Nothing was walked/);
  assert.doesNotMatch(v.line, /\/invented|\//);
  assert.equal(v.degraded, null);
  assert.equal(waitingView(waiting({ settings_degraded: 'settings stopped: invented' })).degraded, 'settings stopped: invented');
  assert.equal(copiesView(waiting({ why: '' })).fault, '/disk/copies.json is waiting and says not why.');
  assert.equal(copiesView(waiting({ settings_degraded: 3 })).fault, '/disk/copies.json carries a settings_degraded that is not text.');
});

test('walking (HTTP 503) is the one named line answerLine makes from the server error', () => {
  const error = 'the first walk of the copy roots since start is running in another request; ask again in a moment';
  assert.equal(ROUTE, 'GET /disk/copies.json');
  assert.equal(answerLine(ROUTE, 503, error), `GET /disk/copies.json cannot answer now (HTTP 503): ${error}. Nothing here is a count until it does.`);
});

test('walked with hazards: every table is built from the body, nothing invented', () => {
  const v = walkedView(walked());
  assert.equal(v.walkedAt, '2026-10-24 19:50:00 IST');
  assert.equal(v.ageS, 2);
  assert.equal(v.walkMs, '1.234');
  assert.equal(v.fresh, true);
  assert.equal(v.rewalking, false);
  assert.equal(v.why, 'the first request since start');
  assert.equal(v.note, walked().note);
  assert.equal(v.hazardCount, 1);
  assert.deepEqual(v.hazards, [{ path: '/invented/queue/2020.p.zip.part', live: '/invented/Options/2020.zip', kind: 'nfo_archives', dev: 64769, ino: 4242, nlink: 2, bytes: 17, partial: 'partial-download name' }]);
  assert.equal(walkedView(walked({ hazards: [hazard({ partial: false })] })).hazards[0].partial, '');
  assert.deepEqual(v.roots, [
    {
      path: '/invented/queue',
      state: 'walked',
      refused: null,
      files: '3',
      bytes: '33 B (0.00 GiB)',
      extraBytes: '16 B (0.00 GiB)',
      reader: 'none found',
      proof: '/invented/queue/run/ALL_VERIFIED.txt',
      twin: 'no proven twin (invented): these bytes may be the ONLY copy',
      duplicatesLive: 'yes',
      inside: '',
      faults: 0,
      linkedElsewhere: 0
    }
  ]);
  assert.equal(v.live.length, 2);
  assert.deepEqual(v.live[1], { kind: 'store', path: '/invented/store', namedBy: 'the store root this process serves', walked: false, why: 'not walked for second names (invented)', files: 0 });
  assert.deepEqual(v.unsetLive, ['cm_archive: invented reason']);
  assert.equal(v.retirement.length, 2);
  assert.equal(v.retirement[1].hazard, true);
  assert.equal(v.retirement[0].caution, '1 name(s) in it are second names of live files');
  assert.equal(walkedView(walked({ retirement_list: [retire()] })).retirement[0].caution, '');
  assert.equal(v.extraCopiesBytes, '16 B (0.00 GiB)');
  assert.equal(v.whole, true);
  assert.equal(v.wholeLine, null);
});

test('a reader, an enclosing live root and the counts of faults and links are shown as served', () => {
  const v = walkedView(
    walked({
      roots: [
        root({
          reader: { kind: 'tick_store', path: '/invented/ticks', named_by: 'INVENTED_TICKS', how: 'opened by the invented reader' },
          inside: { kind: 'source_gdfl', path: '/invented/gdfl' },
          duplicates_live: false,
          faults: [{ path: '/invented/queue/x', why: 'invented' }],
          linked_elsewhere: ['/invented/a', '/invented/b'],
          bytes: 1073741824
        })
      ]
    })
  );
  const [r] = v.roots;
  assert.equal(r.reader, 'tick_store at /invented/ticks, named by INVENTED_TICKS: opened by the invented reader');
  assert.equal(r.inside, 'source_gdfl /invented/gdfl');
  assert.equal(r.duplicatesLive, 'no');
  assert.equal(r.faults, 1);
  assert.equal(r.linkedElsewhere, 2);
  assert.equal(r.bytes, '1,073,741,824 B (1.00 GiB)');
});

test('a refused root keeps its reason and is never counted as zero; not whole is said', () => {
  const refused = root({ path: '/invented/locked', state: 'refused', refused: 'cannot be read: invented', files: null, dirs: null, bytes: null, extra_bytes: null });
  const v = walkedView(walked({ roots: [root(), refused], whole: false }));
  assert.deepEqual(
    { state: v.roots[1].state, refused: v.roots[1].refused, files: v.roots[1].files, bytes: v.roots[1].bytes, extraBytes: v.roots[1].extraBytes },
    { state: 'refused', refused: 'cannot be read: invented', files: 'not measured', bytes: 'not measured', extraBytes: 'not measured' }
  );
  assert.equal(v.whole, false);
  assert.match(v.wholeLine ?? '', /^Not every root was walked whole/);
  assert.equal(walkedView(walked({ roots: [root({ state: 'partial' })] })).roots[0].state, 'partial');
  // A refused root with a count, or one with no reason, is a fault: never a 0.
  assert.equal(copiesView(walked({ roots: [{ ...refused, files: 0 }] })).fault, '/disk/copies.json root /invented/locked has no count for files.');
  assert.equal(copiesView(walked({ roots: [{ ...refused, refused: null }] })).fault, '/disk/copies.json root /invented/locked is refused and says not why.');
});

test('malformed bodies are named by what is wrong', () => {
  const fault = (/** @type {any} */ b) => copiesView(b).fault;
  assert.equal(fault(null), '/disk/copies.json did not answer an object.');
  assert.equal(fault([]), '/disk/copies.json did not answer an object.');
  assert.equal(fault({ state: 'walking' }), '/disk/copies.json answered a state this page does not know: "walking".');
  assert.equal(fault(walked({ walked_at: undefined })), '/disk/copies.json carries no walked_at.');
  assert.equal(fault(walked({ retirement_sentence: '' })), '/disk/copies.json carries no retirement_sentence.');
  assert.equal(fault(walked({ age_ms: -1 })), '/disk/copies.json has no count for age_ms.');
  assert.equal(fault(walked({ walk_us: 1.5 })), '/disk/copies.json has no count for walk_us.');
  assert.equal(fault(walked({ whole: 'yes' })), '/disk/copies.json does not say whether whole.');
  assert.equal(fault(walked({ hazards: {} })), '/disk/copies.json carries no hazards list.');
  assert.equal(fault(walked({ hazard_count: 2 })), '/disk/copies.json counts 2 hazard(s) and lists 1.');
  assert.equal(fault(walked({ hazards: [hazard({ ino: null })] })), '/disk/copies.json hazard /invented/queue/2020.p.zip.part has no count for ino.');
  assert.equal(fault(walked({ hazards: [hazard({ partial: 1 })] })), '/disk/copies.json hazard /invented/queue/2020.p.zip.part does not say whether it is partial.');
  assert.equal(fault(walked({ roots: [root({ state: 'gone' })] })), '/disk/copies.json root /invented/queue is in no known state.');
  assert.equal(fault(walked({ roots: [root({ reader: { kind: 'store' } })] })), '/disk/copies.json root /invented/queue names its reader in neither form.');
  assert.equal(fault(walked({ roots: [root({ reader: null })] })), '/disk/copies.json root /invented/queue names its reader in neither form.');
  assert.equal(fault(walked({ roots: [root({ proof: null })] })), '/disk/copies.json root /invented/queue names no proof, not even "none".');
  assert.equal(fault(walked({ roots: [root({ twin: {} })] })), '/disk/copies.json root /invented/queue carries no twin words.');
  assert.equal(fault(walked({ roots: [root({ inside: { kind: 'store' } })] })), '/disk/copies.json root /invented/queue names what it lies inside in no known form.');
  assert.equal(fault(walked({ roots: [root({ faults: null })] })), '/disk/copies.json root /invented/queue carries no faults or linked_elsewhere list.');
  assert.equal(fault(walked({ live: [live({ files: null })] })), '/disk/copies.json live root /invented/Options does not say whether it was walked, why, or how many files.');
  assert.equal(fault(walked({ unset_live: [3] })), '/disk/copies.json unset_live holds a line that is not text.');
  assert.equal(fault(walked({ retirement_list: [retire({ bytes: null })] })), '/disk/copies.json retirement row /invented/queue has no byte or file count.');
  assert.equal(fault(walked({ retirement_list: [retire({ hazard: null })] })), '/disk/copies.json retirement row /invented/queue carries no caution or hazard flag.');
});

test('the retirement text is passed through verbatim, keeps its sentence, and the page adds no command', () => {
  const v = walkedView(walked());
  assert.equal(v.retirementText, RETIREMENT_TEXT);
  assert.equal(v.retirementSentence, SENTENCE);
  assert.ok(v.retirementText.includes(SENTENCE));
  const command = /\b(rm|delete|unlink|mv|rename)\b/i;
  assert.doesNotMatch(v.retirementText, command);
  for (const file of ['../src/routes/ingest/panels/Copies.svelte', '../src/lib/CopiesTables.svelte', '../src/lib/copies.js']) {
    const src = readFileSync(new URL(file, import.meta.url), 'utf8');
    assert.doesNotMatch(src, command, file);
  }
});

test('a path that is not UTF-8 arrives escaped and quoted, and is shown exactly as given', () => {
  const v = walkedView(
    walked({
      roots: [root({ path: NOT_UTF8, proof: NOT_UTF8 })],
      hazards: [hazard({ path: NOT_UTF8, live: NOT_UTF8 })],
      live: [live({ path: NOT_UTF8 })],
      retirement_list: [retire({ path: NOT_UTF8 })]
    })
  );
  assert.equal(v.fault, null);
  assert.equal(v.roots[0].path, NOT_UTF8);
  assert.equal(v.roots[0].proof, NOT_UTF8);
  assert.equal(v.hazards[0].path, NOT_UTF8);
  assert.equal(v.hazards[0].live, NOT_UTF8);
  assert.equal(v.live[0].path, NOT_UTF8);
  assert.equal(v.retirement[0].path, NOT_UTF8);
});

test('the saved file is named by the walk, and a bad walk time is refused', () => {
  assert.equal(retirementFileName(1761315600000000), 'retirement-list-1761315600000000.txt');
  for (const bad of [-1, 1.5, '1', null]) assert.throws(() => retirementFileName(/** @type {any} */ (bad)), RangeError);
  assert.equal(formatBytes(0), '0 B (0.00 GiB)');
  assert.equal(formatBytes(1234567), '1,234,567 B (0.00 GiB)');
  assert.equal(formatBytes(16106127360), '16,106,127,360 B (15.00 GiB)');
});

test('the panel keeps the contract: order, title, its own poll, titled control, a scrolling table', () => {
  const src = readFileSync(new URL('../src/routes/ingest/panels/Copies.svelte', import.meta.url), 'utf8');
  assert.match(src, /export const order = 95;/);
  assert.match(src, /export const title = 'Extra copies: bytes, reader, proof, hard links to live archives';/);
  assert.match(src, /ask\('\/disk\/copies\.json'/);
  assert.match(src, /createPoller\(\{\s*every: 10_000/);
  assert.match(src, /poll\.stop\(\)/);
  assert.match(src, /answerLine\(ROUTE, r\.status, body\?\.error/);
  assert.match(src, /new Blob\(\[walked\.retirementText\], \{ type: 'text\/plain' \}\)/);
  for (const button of src.match(/<button[^>]*>/g) ?? []) assert.match(button, /title="[^"]+"/, button);
  assert.doesNotMatch(src, /width:\s*(3[89]\d|[4-9]\d\d|\d{4,})px/);
  // NOTHING OF IT ON /ingest's FIRST LOAD (R3-15's page budget): the reading
  // code and the tables are imported when needed, never statically, and a
  // part that fails to load is said with its reason.
  assert.doesNotMatch(src, /^\s*import [^(]*\$lib\/(copies\.js|CopiesTables\.svelte)/m);
  assert.match(src, /lib \?\?= await import\('\$lib\/copies\.js'\)/);
  assert.match(src, /Tables = \(await import\('\$lib\/CopiesTables\.svelte'\)\)\.default;/);
  assert.match(src, /reading code did not load \(asked again at the next poll\): \$\{words\(why\)\}/);
  assert.match(src, /tablesWhy = `\$\{words\(why\)\} \(asked again at the next poll\)`;/);
  assert.match(src, /\{:else if tablesWhy\}\s*<p class="line bad">The register's tables did not load: \{tablesWhy\}<\/p>/);
  // No `{#await}` block: its runtime would move into a chunk every page
  // loads (it grew /markets' first load past its budget when tried).
  assert.doesNotMatch(src, /\{#await/);
});

test('the tables hold no control, put the hazards first and fit 375 px', () => {
  const src = readFileSync(new URL('../src/lib/CopiesTables.svelte', import.meta.url), 'utf8');
  // The panel contract checks the controls of panels/*.svelte only, so the
  // tables it imports hold none: the panel's one control stays in the panel.
  assert.doesNotMatch(src, /<(button|input|select|textarea)\b/);
  assert.doesNotMatch(src, /\bfetch\(|setInterval\(|\bask\(/, 'the panel reads; the tables draw');
  // The hazards come first, before every other table.
  const labels = [...src.matchAll(/aria-label="([^"]+)"/g)].map((m) => m[1]);
  assert.deepEqual(labels, ['Hazards', 'Copy roots', 'Live roots', 'Retirement list']);
  // 375 px: tables scroll inside their wrapper and nothing is fixed wider.
  assert.match(src, /\.scroll \{\s*overflow-x: auto;/);
  assert.equal((src.match(/<table/g) ?? []).length, (src.match(/<div class="scroll">\s*<table/g) ?? []).length);
  assert.doesNotMatch(src, /width:\s*(3[89]\d|[4-9]\d\d|\d{4,})px/);
});

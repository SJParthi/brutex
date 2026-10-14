// R4-01: the /ingest panel registry and the contract every panel keeps, and
// the noscript link (RT-DB-13 item 4). Every fixture is invented.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { MIN_POLL_MS, answerLine, createPoller, panelFile, panelRegistry, refusedLine } from '../src/lib/ingest-panels.js';

const page = readFileSync(new URL('../src/routes/ingest/+page.svelte', import.meta.url), 'utf8');
const panelDir = new URL('../src/routes/ingest/panels/', import.meta.url);

test('panels_mount_in_order_and_a_bad_panel_is_named', () => {
  const C = () => {};
  const { panels, refused } = panelRegistry({
    './panels/Zeta.svelte': { order: 20, title: 'Zeta', default: C },
    './panels/Alpha.svelte': { order: 20, title: 'Alpha', default: C },
    './panels/First.svelte': { order: 10, title: 'First', default: C },
    './panels/NoOrder.svelte': { title: 'No order', default: C },
    './panels/NoTitle.svelte': { order: 1, title: '  ', default: C },
    './panels/NaNOrder.svelte': { order: Number.NaN, title: 'nan', default: C },
    './panels/NoComponent.svelte': { order: 5, title: 'Nothing to mount' }
  });
  // By order, ties by file name; a panel breaking the contract is mounted
  // LAST, never dropped.
  assert.deepEqual(panels.map((p) => p.file), [
    'First.svelte', 'Alpha.svelte', 'Zeta.svelte', 'NaNOrder.svelte', 'NoOrder.svelte', 'NoTitle.svelte'
  ]);
  assert.deepEqual(panels.find((p) => p.file === 'NoTitle.svelte')?.problems, ['exports no `title`']);
  assert.equal(panels.find((p) => p.file === 'NoTitle.svelte')?.title, 'NoTitle.svelte', 'titled by its file');
  assert.deepEqual(refused.map((r) => r.file), ['NaNOrder.svelte', 'NoComponent.svelte', 'NoOrder.svelte', 'NoTitle.svelte']);
  assert.match(refused.find((r) => r.file === 'NoComponent.svelte')?.why ?? '', /exports no component, so it cannot be mounted/);
  const line = refusedLine(refused);
  assert.match(line ?? '', /^4 \/ingest panel\(s\) break the panel contract/);
  for (const f of ['NaNOrder.svelte', 'NoComponent.svelte', 'NoOrder.svelte', 'NoTitle.svelte']) assert.ok(line?.includes(f), f);
  assert.equal(refusedLine([]), null, 'no breach, no line');
  assert.deepEqual(panelRegistry({}), { panels: [], refused: [] });
  assert.deepEqual(panelRegistry(/** @type {any} */ (null)), { panels: [], refused: [] });
  assert.equal(panelFile('./panels/x/Deep.svelte'), 'Deep.svelte');
});

test('the page mounts every panel file through the registry, with no per-panel edit', () => {
  assert.match(page, /panelRegistry\(import\.meta\.glob\('\.\/panels\/\*\.svelte', \{ eager: true \}\)\)/);
  assert.match(page, /\{#each panels\.panels as p \(p\.file\)\}\s+\{@const Panel = p\.component\}\s+<Panel ctx=\{panelCtx\} \/>/);
  assert.match(page, /for \(const r of panels\.refused\) console\.error\(`\/ingest panel \$\{r\.file\}: \$\{r\.why\}`\)/);
  assert.match(page, /\{#if panelsRefused\}<p class="warn mono panels-refused">\{panelsRefused\}<\/p>\{\/if\}/);
  assert.doesNotMatch(page, /import ZerodhaRepair/, 'a panel is a file, never an import of the page');
});

test('every panel file keeps the contract: order, title, a title on every control, no fast or hidden poll', () => {
  const files = readdirSync(panelDir).filter((f) => f.endsWith('.svelte'));
  assert.ok(files.includes('ZerodhaRepair.svelte'));
  for (const f of files) {
    const src = readFileSync(new URL(f, panelDir), 'utf8');
    assert.match(src, /<script module>[\s\S]*export const order = -?\d+(\.\d+)?;[\s\S]*export const title = '[^']+';[\s\S]*<\/script>/, `${f}: order and title`);
    // READ-03: every control carries a title.
    const controls = src.match(/<(button|input|select|textarea)\b[^>]*>/g) ?? [];
    assert.ok(controls.length > 0 || !/onclick/.test(src), `${f}: controls found`);
    for (const c of controls) assert.match(c, /\btitle=/, `${f}: ${c.slice(0, 80)} has no title`);
    // A panel polls through createPoller (at most once a second, never hidden), never setInterval.
    assert.doesNotMatch(src, /setInterval\(/, `${f}: no raw interval`);
    assert.doesNotMatch(src, /\bfetch\(/, `${f}: requests go through $lib/ask.js`);
  }
});

test('a poll never runs faster than once a second and never while hidden', async () => {
  /** @type {{ fn: () => void, ms: number }[]} */
  const timers = [];
  let hidden = false;
  let clock = 0;
  let runs = 0;
  const poll = createPoller({
    every: 200,
    run: () => { runs += 1; },
    hidden: () => hidden,
    setTimer: (fn, ms) => { timers.push({ fn, ms }); return timers.length; },
    clearTimer: () => {},
    now: () => clock
  });
  assert.equal(poll.ms, MIN_POLL_MS, 'a faster ask is clamped to a second');
  poll.start();
  assert.equal(timers.at(-1)?.ms, 1000);
  await timers.at(-1)?.fn();
  assert.equal(runs, 1);
  // Woken again inside the second: nothing is asked.
  clock = 500;
  poll.wake();
  assert.equal(runs, 1);
  // Hidden: the timer fires and asks nothing.
  hidden = true;
  clock = 2000;
  await timers.at(-1)?.fn();
  assert.equal(runs, 1);
  poll.wake();
  assert.equal(runs, 1);
  // Shown again, a second after the last ask: one ask at once.
  hidden = false;
  poll.wake();
  await Promise.resolve();
  assert.equal(runs, 2);
  poll.stop();
  const before = timers.length;
  poll.start();
  poll.start();
  assert.equal(timers.length, before + 1, 'start twice is one schedule');
  assert.equal(createPoller({ every: Number.NaN, run: () => {} }).ms, 1000);
  assert.equal(createPoller({ every: 5000, run: () => {} }).ms, 5000);
});

test('a 404 and a 503 are one named line each', () => {
  assert.equal(answerLine('GET /pull/repair.json', 404), 'GET /pull/repair.json is not served by this build (HTTP 404). The panel waits for a server that has it.');
  assert.equal(answerLine('GET /x.json', 503, 'census rebuilding'), 'GET /x.json cannot answer now (HTTP 503): census rebuilding. Nothing here is a count until it does.');
  assert.equal(answerLine('GET /x.json', 0, 'Failed to fetch'), 'GET /x.json did not answer: Failed to fetch.');
  assert.equal(answerLine('GET /x.json', 400, '  bad rung  '), 'GET /x.json answered HTTP 400: bad rung.');
});

test('the_noscript_bars_link_names_a_whole_selection (RT-DB-13 item 4)', () => {
  const html = readFileSync(new URL('../src/app.html', import.meta.url), 'utf8');
  const noscript = html.slice(html.indexOf('<noscript>'), html.indexOf('</noscript>'));
  const href = noscript.match(/href="(\/bars[^"]*)"/)?.[1] ?? '';
  // Before: href="/bars", which the server answers 400 (it renders one instrument-month).
  assert.notEqual(href, '/bars');
  const q = new URLSearchParams(href.replace(/&amp;/g, '&').split('?')[1] ?? '');
  for (const [k, v] of [['feed', 'zerodha'], ['exchange', 'NSE'], ['segment', 'INDEX'], ['symbol', 'NIFTY'], ['timeframe', '1day']]) {
    assert.equal(q.get(k), v, k);
  }
  assert.match(q.get('month') ?? '', /^\d{4}-\d{2}$/, 'a month, which /bars needs to answer 200');
});

test('the repair panel reads its plan on a press, never on every load, and plans for the selected feed', () => {
  const src = readFileSync(new URL('ZerodhaRepair.svelte', panelDir), 'utf8');
  const mount = src.slice(src.indexOf('onMount(() => {'), src.indexOf('</script>', src.indexOf('onMount(() => {')));
  assert.doesNotMatch(mount, /readPlan\(\)/, 'GET /pull/repair.json is O(days asked): not on every /ingest load');
  assert.match(src, /onsubmit=\{\(e\) => \{ e\.preventDefault\(\); void readPlan\(\); \}\}/);
  // No vendor named in the panel's logic: the feed is the page's, an archive is told by its own field.
  assert.match(src, /const archive = \$derived\(row\?\.archive != null\);/);
  assert.match(src, /planUrl\(\{ feed, instrument/);
  assert.doesNotMatch(src, /feed: 'zerodha'|feed=zerodha/);
  // One request per press goes through the gate; the poll is the contract's.
  assert.match(src, /const done = await gate\.press\(m\.body, reason, what\);/);
  assert.match(src, /createPoller\(\{\s+every: 1000,/);
});

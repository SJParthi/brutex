// R4-01: the /ingest panel registry and the contract every panel keeps, and
// the noscript link (RT-DB-13 item 4). Every fixture is invented.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { MIN_POLL_MS, answerLine, createPoller, panelFailedLine, panelFile, panelRegistry, refusedLine, reportPanelFailure } from '../src/lib/ingest-panels.js';
import { compile } from 'svelte/compiler';
import { render } from 'svelte/server';

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
  // EVERY PANEL THROUGH ITS OWN ERROR BOUNDARY (R4-01 repair round 1): the
  // bare `<Panel ctx={panelCtx} />` let one panel's throw blank the page.
  assert.match(page, /\{#each panels\.panels as p \(p\.file\)\}\s+<IngestPanelSlot panel=\{p\} ctx=\{panelCtx\} \/>\s+\{\/each\}/);
  assert.match(page, /import IngestPanelSlot from '\$lib\/IngestPanelSlot\.svelte';/);
  assert.doesNotMatch(page, /<Panel ctx=\{panelCtx\} \/>/, 'no panel is mounted outside the boundary');
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
  assert.match(src, /const done = await gate\.press\(m\.body, reason, what, m\.member\);/);
  assert.match(src, /\{#if m\.repair && m\.bodyFault !== null\}[\s\S]{0,200}<span class="warn mono" title=\{m\.bodyFault\}>refused<\/span>/, "a refused template shows no press");
  assert.match(src, /\{#if plan\.plan\.bodyFaultLine\}<p class="line bad">\{plan\.plan\.bodyFaultLine\}<\/p>\{\/if\}/);
  assert.match(src, /createPoller\(\{\s+every: 1000,/);
});

/* ─── R4-01 repair round 1: one panel's throw is one named line ───────────
   The slot the page mounts every panel through is compiled from disk and
   rendered with a panel that throws while it initialises (the checker's
   ZzThrows.svelte probe), between two healthy ones. Rendered on the server
   with `transformError`, which is how Svelte runs a boundary off the browser;
   in the browser the same `<svelte:boundary>` catches natively (checked in
   headless Chrome on the R4-01 shadow, see the package report). */

/** Compile Svelte source to a server module whose imports resolve from here. @param {string} src @param {string} name */
async function serverComponent(src, name) {
  const code = compile(src, { generate: 'server', filename: name }).js.code
    .replace(/from '(svelte[^']*)'/g, (_, spec) => `from '${import.meta.resolve(spec)}'`)
    .replace(/from '\.\/ingest-panels\.js'/g, `from '${new URL('../src/lib/ingest-panels.js', import.meta.url).href}'`);
  return (await import(`data:text/javascript,${encodeURIComponent(code)}`)).default;
}

test('a_panel_that_throws_is_one_named_line_and_the_page_keeps_working', async () => {
  const slotSrc = readFileSync(new URL('../src/lib/IngestPanelSlot.svelte', import.meta.url), 'utf8');
  assert.match(slotSrc, /<svelte:boundary onerror=\{\(error\) => reportPanelFailure\(panel\.file, error\)\}>/);
  assert.match(slotSrc, /\{#snippet failed\(error\)\}\s*<p class="warn mono panel-failed" role="alert" data-panel=\{panel\.file\}>\{panelFailedLine\(panel\.file, error\)\}<\/p>/);
  const Slot = await serverComponent(slotSrc, 'IngestPanelSlot.svelte');
  const Throws = await serverComponent(
    "<script module>export const order = 99; export const title = 'Throws';</script><script>let { ctx } = $props(); throw new Error('PROBE: panel init failure');</script><p>never shown</p>",
    'ZzThrows.svelte'
  );
  const Healthy = await serverComponent("<script>let { ctx } = $props();</script><p>healthy panel for {ctx.feed}</p>", 'Healthy.svelte');
  const Host = await serverComponent(
    "<script>let { Slot, panels } = $props();</script><section>{#each panels as p (p.file)}<Slot panel={p} ctx={{ feed: 'zerodha' }} />{/each}</section><p>the rest of /ingest</p>",
    'Host.svelte'
  );
  const { panels } = panelRegistry({
    './panels/A.svelte': { order: 1, title: 'A', default: Healthy },
    './panels/ZzThrows.svelte': { order: 2, title: 'Throws', default: Throws },
    './panels/Z.svelte': { order: 3, title: 'Z', default: Healthy }
  });
  const html = render(Host, { props: { Slot, panels }, transformError: (e) => e }).body;
  assert.match(html, /<p class="warn mono panel-failed[^"]*" role="alert" data-panel="ZzThrows\.svelte">\/ingest panel ZzThrows\.svelte failed and is not shown: PROBE: panel init failure\. Every other panel and the rest of \/ingest keep working/);
  assert.doesNotMatch(html, /never shown/);
  assert.equal(html.match(/healthy panel for zerodha/g)?.length, 2, 'the panels before and after it still render');
  assert.match(html, /the rest of \/ingest/);
  // A panel that does not throw renders as itself, with no failure line.
  const alone = render(Slot, { props: { panel: panels[0], ctx: { feed: 'gdfl' } }, transformError: (e) => e }).body;
  assert.match(alone, /healthy panel for gdfl/);
  assert.doesNotMatch(alone, /panel-failed/);
});

test('a failed panel is named on the console with its error', () => {
  /** @type {unknown[][]} */
  const logged = [];
  const err = new Error('PROBE: boom');
  const line = reportPanelFailure('Bad.svelte', err, (...a) => logged.push(a));
  assert.equal(line, '/ingest panel Bad.svelte failed and is not shown: PROBE: boom. Every other panel and the rest of /ingest keep working; the browser console holds the stack.');
  assert.deepEqual(logged, [[line, err]], 'once, with the error itself so the stack is kept');
  assert.match(panelFailedLine('X.svelte', 'a thrown string'), /failed and is not shown: a thrown string\./);
  assert.match(panelFailedLine('X.svelte', new Error('')), /failed and is not shown: an error with no message\./);
});

test('a poll whose run throws keeps polling and says what failed', async () => {
  /** @type {(() => void)[]} */
  const timers = [];
  let t = 0;
  let calls = 0;
  /** @type {unknown[]} */
  const reported = [];
  const poll = createPoller({
    every: 1000,
    run: async () => {
      calls += 1;
      if (calls <= 2) throw new Error(`PROBE: answer ${calls} failed`);
    },
    setTimer: (fn) => { timers.push(fn); return timers.length; },
    clearTimer: () => {},
    now: () => t,
    onError: (e) => {
      reported.push(e);
      if (reported.length === 2) throw new Error('the reporter itself failed');
    }
  });
  poll.start();
  const step = async () => { t += 1000; const fn = timers.shift(); assert.ok(fn, 'a next run is always scheduled'); fn(); await new Promise((r) => setTimeout(r, 0)); };
  await step();
  assert.equal(poll.failures, 1);
  assert.deepEqual(poll.failed, { message: 'PROBE: answer 1 failed', at: 1000 });
  assert.equal(timers.length, 1, 'the throw did not end the poll');
  await step();
  assert.equal(poll.failures, 2);
  assert.equal(timers.length, 1, 'a reporter that throws does not end it either');
  await step();
  assert.equal(calls, 3);
  assert.equal(poll.failed, null, 'a run that succeeds clears the failure');
  assert.equal(poll.failures, 2);
  assert.equal(reported.length, 2);
  poll.stop();
});

// conc18-3: the Pause/Resume POST did not revoke the in-flight 2 s poll. A GET
// the server answered before the control was applied, landing after the POST,
// reverted the state and the button and wrote a false transition into the trail.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { parse } from 'svelte/compiler';
import { watchVisible } from '../src/lib/page-requests.js';

const source = readFileSync(new URL('../src/routes/autopilot/+page.svelte', import.meta.url), 'utf8');
const ast = /** @type {any} */ (parse(source));
/** @param {string[]} names */
function functions(names) {
  return names.map((name) => {
    const found = ast.instance.content.body.find((/** @type {any} */ n) => n.type === 'FunctionDeclaration' && n.id?.name === name);
    assert.ok(found, `autopilot: actual ${name} function is required`);
    return source.slice(found.start, found.end);
  }).join('\n');
}
function deferred() {
  /** @type {(v?:any)=>void} */ let resolve = () => {};
  const promise = new Promise((yes) => { resolve = yes; });
  return { promise, resolve };
}
const flush = () => new Promise((done) => setImmediate(done));

test('a poll answered before the control cannot overwrite the state the control returned', async () => {
  /** @type {{url:string, reply:ReturnType<typeof deferred>}[]} */ const asks = [];
  const create = new Function('ask', 'watchVisible', `
    const CONTROL='/autopilot/control', TICK_MS=2000, performance={now:()=>0};
    const adopted=[]; let ap=null, link={kind:'probing',ms:0}, receipt=null, control={busy:false}, poller=null;
    const readState=(v)=>({ok:true,value:v}), adopt=(v)=>{adopted.push(v.state);ap=v;}, setLink=(v)=>{link=v;};
    const note=()=>{}, str=(v)=>typeof v==='string'?v:null, classify=()=>'ok';
    ${functions(['tick', 'send'])}
    let timers=[];
    poller=watchVisible(tick, TICK_MS, {visible:()=>true, listen:()=>()=>{}, schedule:(w)=>{timers.push(w);return timers.length;}, cancel:()=>{}});
    return {send, adopted, fire:()=>{const due=timers; timers=[]; for (const w of due) w();}, stop:()=>poller()};
  `);
  const app = create((/** @type {string} */ url) => { const reply = deferred(); asks.push({ url, reply }); return reply.promise; }, watchVisible);
  assert.equal(asks[0].url, '/autopilot.json');
  const json = (/** @type {any} */ body) => new Response(JSON.stringify(body), { headers: { 'content-type': 'application/json' } });
  const sending = app.send('pause');
  assert.equal(asks[1].url, '/autopilot/control');
  asks[1].reply.resolve(json({ accepted: true, why: 'paused by operator', status: { state: 'paused' } }));
  await sending;
  // The older GET, answered by the server before the pause, lands now.
  asks[0].reply.resolve(json({ state: 'running' }));
  await flush(); await flush();
  app.fire(); await flush();
  assert.deepEqual(app.adopted, ['paused'], 'the late poll must not revert the state');
  assert.equal(asks.at(-1)?.url, '/autopilot.json', 'a fresh poll is started after the control settles');
  app.stop();
});

test('the page keeps the poller handle the control refreshes', () => {
  assert.match(source, /poller = watching;/);
  assert.match(source, /control = \{ busy: false \};\s*(?:\/\/[^\n]*\n\s*)*poller\?\.refresh\(\);/);
});

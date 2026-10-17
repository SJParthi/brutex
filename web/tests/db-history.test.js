// /db's BACK AND FORWARD (R3-01; DB-077): every selection the operator makes
// is an entry, Back and Forward restore each one, and neither a seed nor a
// restore adds one. Driven through `dbHistory`, the object the page holds,
// over a model of the browser's history (entries and an index) and of the
// page (a selection that settles when told).
//
// Values are invented; nothing here is a vendor row.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import { dbHistory, encodeDb, decodeDb, addressNow, GESTURE_MS, isChoiceGesture, CHOICE_TARGET } from '../src/lib/urlstate.js';
import { dbSource } from './database-page-fixture.js';

/** What `applyAddress(search, true)` does: every field from the entry, every
 *  absent one back to the page's opening value. */
const OPENING = { feed: 'gdfl', seg: 'spot', sym: 'NIFTY', tf: '1s', page: 1, k: [], side: '', exp: '', rows: '', spot: 'exact' };

function browser(first = '', opening = OPENING) {
  /** @type {string[]} */
  const entries = [first];
  let at = 0;
  let now = 0;
  /** @type {Record<string, any>} */
  let page = { ...opening };
  /** @type {Array<'push' | 'replace'>} */
  const writes = [];
  const history = dbHistory({
    search: () => entries[at],
    write: (kind, q) => {
      writes.push(kind);
      if (kind === 'push') {
        entries.splice(at + 1);
        entries.push(q);
        at += 1;
      } else entries[at] = q;
    },
    apply: (search) => {
      page = { ...opening, ...decodeDb(search).sel };
    },
    clock: () => now
  });
  const settle = () => history.settled(encodeDb(page));
  return {
    entries,
    writes,
    get at() {
      return at;
    },
    get page() {
      return page;
    },
    /** The operator presses and the selection moves. @param {Record<string, any>} change */
    choose(change) {
      now += 1000;
      history.gesture();
      page = { ...page, ...change };
      return settle();
    },
    /** A seed or a clamp moves the selection with nobody pressing anything. @param {Record<string, any>} change */
    seed(change) {
      now += 1000;
      page = { ...page, ...change };
      return settle();
    },
    back() {
      at -= 1;
      history.popped(encodeDb(page));
      return settle();
    },
    forward() {
      at += 1;
      history.popped(encodeDb(page));
      return settle();
    },
    wait(/** @type {number} */ ms) {
      now += ms;
    },
    settle,
    history
  };
}

test('back_and_forward_restore_each_selection', () => {
  // /db entered from the top bar: a bare address.
  const b = browser('');
  assert.equal(b.settle(), 'replace', 'the opening completes the address, it is not an entry');
  assert.equal(b.entries.length, 1);
  const opened = b.entries[0];
  assert.equal(opened, 'feed=gdfl&seg=spot&sym=NIFTY&tf=1s');
  // Three selections by hand: three entries.
  assert.equal(b.choose({ from: '2020-03-02', to: '2020-03-31' }), 'push');
  assert.equal(b.choose({ sort: 'o', dir: 'asc' }), 'push');
  assert.equal(b.choose({ page: 2 }), 'push');
  assert.equal(b.entries.length, 4);
  const [a0, a1, a2, a3] = b.entries;
  // Back restores each selection, one step at a time, and adds nothing.
  assert.equal(b.back(), 'none');
  assert.equal(encodeDb(b.page), a2);
  assert.equal(b.page.page, 1);
  assert.equal(b.back(), 'none');
  assert.equal(encodeDb(b.page), a1);
  assert.equal(b.page.sort, undefined);
  assert.equal(b.back(), 'none');
  assert.equal(encodeDb(b.page), a0);
  assert.equal(b.page.from, undefined, 'a field the entry leaves out returns to the opening');
  // Forward walks them again, and the Forward entries are never erased.
  assert.equal(b.forward(), 'none');
  assert.equal(encodeDb(b.page), a1);
  assert.equal(b.forward(), 'none');
  assert.equal(b.forward(), 'none');
  assert.equal(encodeDb(b.page), a3);
  assert.equal(b.page.page, 2);
  assert.equal(b.entries.length, 4);
  assert.deepEqual(b.entries, [a0, a1, a2, a3]);
});

test('an option selection with two strikes, a side and a spot rule steps back field for field', () => {
  const opening = { ...OPENING, seg: 'options', exp: '2020-01-02' };
  const b = browser('feed=gdfl&seg=options&sym=NIFTY&tf=1s&exp=2020-01-02', opening);
  assert.equal(b.settle(), 'none', 'the address it opened on is already complete');
  assert.equal(b.choose({ k: ['1150000'] }), 'push');
  assert.equal(b.choose({ k: ['1150000', '1220000'], side: 'PE' }), 'push');
  assert.equal(b.choose({ spot: 'prev', rows: 25 }), 'push');
  assert.equal(b.page.spot, 'prev');
  b.back();
  assert.deepEqual(b.page.k, ['1150000', '1220000']);
  assert.equal(b.page.spot, 'exact', 'the spot rule returns to exact on the entry before it');
  assert.equal(b.page.rows, '');
  b.back();
  assert.deepEqual(b.page.k, ['1150000']);
  assert.equal(b.page.side, '');
  b.forward();
  b.forward();
  assert.equal(b.page.spot, 'prev');
  assert.equal(b.page.rows, 25);
});

test('a seed, a clamp and a restore correct the current entry and never add one', () => {
  const b = browser('');
  b.settle();
  // A seed with nobody pressing: a correction.
  assert.equal(b.seed({ to: '2026-09-30' }), 'replace');
  assert.equal(b.entries.length, 1);
  // One press passing through several settled states is ONE entry.
  assert.equal(b.choose({ sym: 'BANKNIFTY' }), 'push');
  assert.equal(b.seed({ tf: '1min' }), 'replace');
  assert.equal(b.entries.length, 2);
  // A press long ago is not the cause of a later write.
  b.history.gesture();
  b.wait(GESTURE_MS + 1);
  assert.equal(b.seed({ page: 5 }), 'replace');
  // A press that changes nothing writes nothing.
  b.history.gesture();
  assert.equal(b.settle(), 'none');
  // After Back, the restored entry's own correction is a replace: a push there
  // would erase every Forward entry.
  b.choose({ page: 6 });
  const before = b.entries.length;
  b.back();
  assert.equal(b.seed({ oow: true }), 'replace');
  assert.equal(b.entries.length, before, 'Forward is still there');
  // Popping onto an entry equal to the screen restores nothing.
  assert.equal(b.history.popped(encodeDb(b.page)), false);
});

test('a press off a control, or a key that chooses nothing, arms no Back entry', () => {
  // Second agent, 16 Oct: any pointerdown or keydown armed a push, so a seed
  // or a clamp settling within GESTURE_MS of a click on blank space became a
  // Back entry of its own.
  /** A target that is, or sits inside, a `CHOICE_TARGET` (tag) or nothing (null). */
  const on = (/** @type {string | null} */ tag) => ({
    closest: (/** @type {string} */ sel) => {
      assert.equal(sel, CHOICE_TARGET);
      return tag;
    }
  });
  const control = on('button');
  const blank = on(null);
  // The filter itself.
  assert.equal(isChoiceGesture({ type: 'pointerdown', button: 0, target: control }), true);
  assert.equal(isChoiceGesture({ type: 'keydown', key: 'Enter', target: control }), true);
  assert.equal(isChoiceGesture({ type: 'keydown', key: 'a', target: control }), true);
  assert.equal(isChoiceGesture({ type: 'keydown', key: 'ArrowDown', target: control }), true);
  for (const key of ['Tab', 'Shift', 'Meta', 'Control', 'Alt', 'Escape', 'CapsLock', 'F5', 'F12', '']) {
    assert.equal(isChoiceGesture({ type: 'keydown', key, target: control }), false, `${key || 'no key'} chooses nothing`);
  }
  assert.equal(isChoiceGesture({ type: 'pointerdown', button: 2, target: control }), false, 'a right press opens a menu');
  assert.equal(isChoiceGesture({ type: 'pointerdown', button: 0, target: blank }), false, 'blank space');
  assert.equal(isChoiceGesture({ type: 'keydown', key: 'Enter', target: blank }), false, 'a key on the page body');
  assert.equal(isChoiceGesture({ type: 'pointerdown', button: 0, target: {} }), false, 'a target that is not an element');
  assert.equal(isChoiceGesture({ type: 'pointerdown', button: 0, target: null }), false);
  assert.equal(isChoiceGesture({ type: 'click', target: control }), false, 'only the two events the page listens to');
  assert.equal(isChoiceGesture(null), false);
  for (const sel of ['input', 'select', 'button', 'a[href]', 'label', '[role="option"]', '[role="row"]', '[role="tab"]', '[tabindex]']) {
    assert.ok(CHOICE_TARGET.split(', ').includes(sel), `${sel} can make a choice`);
  }
  // Through the history rule: a press on blank space is not the cause of the
  // seed after it, a press on a control is.
  const b = browser('');
  b.settle();
  assert.equal(b.history.gesture({ type: 'pointerdown', button: 0, target: blank }), false);
  assert.equal(b.seed({ page: 3 }), 'replace');
  assert.equal(b.history.gesture({ type: 'keydown', key: 'Tab', target: control }), false);
  assert.equal(b.seed({ page: 4 }), 'replace');
  assert.equal(b.entries.length, 1, 'no entry was made by a press that chose nothing');
  assert.equal(b.history.gesture({ type: 'pointerdown', button: 0, target: control }), true);
  assert.equal(b.seed({ page: 5 }), 'push');
  assert.equal(b.entries.length, 2);
});

test('the page wires the rule: push and replace from SvelteKit, popstate within /db, presses and keys', () => {
  const source = dbSource('../src/lib/db/selection.svelte.js');
  assert.match(source, /import \{ replaceState, pushState \} from '\$app\/navigation';/);
  assert.match(source, /write: \(kind, q\) => \(kind === 'push' \? pushState : replaceState\)\(q \? `\?\$\{q\}` : location\.pathname, \{\}\)/);
  assert.match(source, /apply: \(search\) => db\.applyAddress\(search, true\)/);
  assert.match(source, /window\.addEventListener\('popstate', pop\);/);
  assert.match(source, /window\.addEventListener\('pointerdown', press, true\);/);
  assert.match(source, /window\.addEventListener\('keydown', press, true\);/);
  // The event goes to the rule, which keeps only a choice made on a control.
  assert.ok(source.includes('const press = (/** @type {Event} */ e) => void history.gesture(/** @type {any} */ (e));'));
  // A popstate onto ANOTHER path is the router's (leaving /db), not a restore.
  assert.match(source, /if \(location\.pathname !== page\.url\.pathname\) return;/);
  // The page leaves its title behind for a page that sets none (/terminal).
  const shell = dbSource('../src/routes/db/+page.svelte');
  assert.match(shell, /onDestroy\(\(\) => \{\n\s+if \(typeof document === 'undefined' \|\| document\.title !== OWN_TITLE\) return;/);
  // `addressNow` reads the browser's address, not a router copy.
  assert.equal(addressNow('?x=1'), typeof location === 'undefined' ? 'x=1' : location.search.replace(/^\?/, ''));
});

test('a restored entry with no day window leaves an unnarrowed window alone, so the page does not re-settle', () => {
  // MEASURED on the shadow, 13 Oct: Next, Next, sort by Open, then Back to
  // `…&page=3` showed rows 17-24 of 8 a page where page 3 had been 19-27 of
  // 9. Restoring an entry re-seeded the day window every time; the re-seed
  // un-settled the page, and the Fit re-measured while the sorted answer's
  // note line was still under the grid. Only a window applied NOW is undone.
  const source = dbSource('../src/lib/db/selection.svelte.js');
  const read = source.slice(source.indexOf('function applyAddress(search, entry) {'), source.indexOf('THE SELECTION FROM THE ADDRESS BAR'));
  const branch = read.slice(read.indexOf('} else if (entry) {'), read.indexOf('if (sel.tfrom) db.fromTime = sel.tfrom;'));
  assert.ok(branch.length > 0, 'the entry branch exists');
  assert.match(branch, /const seeded = seed\.from !== '' && seed\.from !== '\\u0000';/);
  // A seeded span is returned to only while a window narrows it, and the
  // seed itself is never replaced there.
  assert.match(branch, /if \(seeded\) \{\n\s+if \(db\.dayNarrowing\) \{\n\s+db\.fromDay = seed\.from;\n\s+db\.toDay = seed\.to;\n\s+\}\n\s+\}/);
  const seededPart = branch.slice(branch.indexOf('if (seeded) {'), branch.indexOf('} else if (seed.from'));
  assert.doesNotMatch(seededPart, /db\.daySeed =/);
  assert.doesNotMatch(read, /\} else if \(entry \|\| days\.notes\.length\) \{/, 'no unconditional re-seed');
});

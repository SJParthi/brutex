// WHICH FEED EVERY PAGE OPENS ON.
//
// This rule is not a preference, it is a bug fix, and `feeds.svelte.js` records
// the bug in its own words: "first ready feed" opened every page on Dhan —
// ready because a broker's credential proves entitlement, and holding nothing
// because it had never pulled — while Groww sat one dropdown away with 8,922
// bars. The operator saw an empty DB, an empty chart and "With data 0", which
// reads as a broken build rather than as an unselected feed.
//
// Nothing drove it: the rule lived in a module importing `$lib/ask.js`, an
// alias node cannot resolve.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import { pickDefaultFeed } from '../src/lib/pick.js';

test('THE REGRESSION: a ready-but-empty feed does not beat a full one', () => {
  // The exact shape of the original defect, with its real numbers.
  const held = [
    { wire: 'dhan', ready: true, bars: 0 },
    { wire: 'groww', ready: false, bars: 8_922 }
  ];
  assert.equal(pickDefaultFeed(held), 'groww');
});

test('the store decides: most bars wins', () => {
  const held = [
    { wire: 'a', ready: true, bars: 10 },
    { wire: 'b', ready: true, bars: 8_922 },
    { wire: 'c', ready: true, bars: 400 }
  ];
  assert.equal(pickDefaultFeed(held), 'b');
});

test('the credential is the FALLBACK, used only when nothing holds anything', () => {
  const held = [
    { wire: 'a', ready: false, bars: 0 },
    { wire: 'b', ready: true, bars: 0 }
  ];
  assert.equal(pickDefaultFeed(held), 'b');
});

test('with nothing held and nothing ready it still opens on something', () => {
  // A picker with no selection is a page that refuses to draw for a reason the
  // operator cannot see or act on. Opening on the first feed is at least a
  // state they can change.
  const held = [
    { wire: 'a', ready: false, bars: 0 },
    { wire: 'b', ready: false, bars: 0 }
  ];
  assert.equal(pickDefaultFeed(held), 'a');
});

test('no feeds is null, not a crash and not an invented name', () => {
  assert.equal(pickDefaultFeed([]), null);
  assert.equal(pickDefaultFeed(/** @type {any} */ (null)), null);
  assert.equal(pickDefaultFeed(/** @type {any} */ (undefined)), null);
});

test('a missing bars or ready field is absence, not a win', () => {
  // `surveyStores` fills these in, but a feed it has no entry for arrives with
  // the fields defaulted. Treating `undefined` as truthy or as a large number
  // would make the feed the store knows LEAST about the one every page opens on.
  const held = [{ wire: 'a' }, { wire: 'b', bars: 5 }];
  assert.equal(pickDefaultFeed(held), 'b');
});

test('it returns the wire name, never the display name', () => {
  // Every consumer keys on `wire`. A display name here would compare unequal to
  // `feeds.active` everywhere and quietly select nothing.
  const held = [{ wire: 'nse-dhan', ready: true, bars: 3 }];
  assert.equal(pickDefaultFeed(held), 'nse-dhan');
});

test('the choice is stable for the same input', () => {
  const held = [
    { wire: 'a', ready: true, bars: 100 },
    { wire: 'b', ready: true, bars: 100 }
  ];
  assert.equal(pickDefaultFeed(held), pickDefaultFeed(held), 'a tie must not alternate');
});

test('it does not reorder the caller’s array', () => {
  // The rule sorts to find the best. Sorting IN PLACE would leave the picker
  // rendering feeds in bar order rather than the order the server listed them,
  // which is a second, invisible consequence of asking this question.
  const held = [
    { wire: 'a', ready: true, bars: 1 },
    { wire: 'b', ready: true, bars: 9 }
  ];
  const before = held.map((f) => f.wire).join(',');
  pickDefaultFeed(held);
  assert.equal(held.map((f) => f.wire).join(','), before, 'the caller’s order survived');
});

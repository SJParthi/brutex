// TWO FRONT ENDS CARRY THE SAME O(1) INDEX, AND ONLY ONE HAS TESTS.
//
// `web/src/lib/prefix.js` is the SvelteKit app's instrument index, driven by
// `web/tests/prefix.test.js`. `web/typeahead.js` is the OTHER front end — a
// classic script served raw by `crates/api/src/assets.rs` and injected into
// every Rust-rendered page by `render.rs:1130` — and it carries its own copy of
// the same index: same bound, same Map, same build loop.
//
// THEY CANNOT SHARE CODE AS THINGS STAND. A classic script has no module scope
// to import into, and `assets.rs` serves exactly one file out of `web/` while
// its own doc says of `web/src`: "Nothing under it is ever served." Closing the
// duplication means changing how the Rust side serves the front end, which is a
// crate and a separate decision — see `docs/06-limits.md`.
//
// What can be done meanwhile is refuse to let them DRIFT. The bound is the
// number that must agree: raise it in one copy and the type-ahead on the
// Rust-rendered pages answers a different question from the one in the SPA,
// with nothing on either page saying so.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

import { MAX_PREFIX } from '../src/lib/prefix.js';

const web = join(dirname(fileURLToPath(import.meta.url)), '..');

test('both front ends index to the same prefix bound', () => {
  const script = readFileSync(join(web, 'typeahead.js'), 'utf8');
  const m = script.match(/const\s+MAX_PREFIX\s*=\s*(\d+)/);
  assert.ok(m, 'typeahead.js no longer declares MAX_PREFIX — has its index changed shape?');
  assert.equal(
    Number(m[1]),
    MAX_PREFIX,
    `typeahead.js indexes to ${m[1]} characters and $lib/prefix.js to ${MAX_PREFIX}. ` +
      'The two front ends would answer the same keystroke differently, and neither ' +
      'page would say so.'
  );
});

test('the duplicate is still a duplicate, and this test still has a reason to exist', () => {
  // If `typeahead.js` ever stops building its own Map — because the serving
  // question was settled and it imports the shared module — this assertion
  // fails and the whole file should be deleted rather than adjusted. A guard
  // that outlives the thing it guards is noise that reads as coverage.
  const script = readFileSync(join(web, 'typeahead.js'), 'utf8');
  assert.match(
    script,
    /byPrefix|new Map\(\)/,
    'typeahead.js no longer builds its own index — if it now shares $lib/prefix.js, ' +
      'delete this file: the drift it guards against cannot happen any more.'
  );
});

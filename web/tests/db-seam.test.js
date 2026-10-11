// THE /db SEAM WP-00 CUT, AND WHAT KEEPS IT A SEAM.
//
// `node --test web/tests/` — node's own runner and the Svelte compiler the
// fixture already uses; no new dependency.
//
// WP-00 split `routes/db/+page.svelte` (13,592 lines) into a shell plus one
// component or module per later work package, moving code only. Three things
// make that split usable by the packages after it, and each is checked here:
//
//   1. The shell stays a shell, so a later package never has to edit it.
//   2. Every placeholder is already MOUNTED, exactly once, and renders nothing
//      until its owner fills it — so the owner edits its own file and never
//      the file that mounts it. (DbCalendar was created and never mounted;
//      this is the test that found it.)
//   3. State crosses files as `db.<name>`, and `db` is untyped, so a name that
//      no factory publishes is `undefined` with no error from the compiler or
//      from svelte-check. Every name read is checked against what is published.
//
// AN OWNER FILLING A PLACEHOLDER removes its name from `PLACEHOLDERS` below in
// the same change: the "renders nothing" assertion is about the hand-over, not
// a rule that outlives it. The mount assertion stays.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readdirSync } from 'node:fs';
import { parse } from 'svelte/compiler';
import { DB_PAGE_FILES, dbSource } from './database-page-fixture.js';

const SHELL = '../src/routes/db/+page.svelte';
// DbExtremes WAS FILLED BY ITS OWNER, WP-52 (DB-G95), and left this list in
// the same change, as the note above asks; it stays mounted exactly once.
// DbDayFlags and DbCalendar WERE FILLED BY WP-54 (DB-025, DB-012) the same way,
// and DbProvenance and DbBasisBadge by WP-55 (DB-078, DB-A40, DB-017).
const PLACEHOLDERS = ['DbChart'];
const FILLED = ['DbExtremes', 'DbDayFlags', 'DbCalendar', 'DbProvenance', 'DbBasisBadge'];
const COMPONENTS = DB_PAGE_FILES.filter((path) => path.endsWith('.svelte'));

/** The number of times `<Name {db} />` is written across the page's components. */
const mounts = (/** @type {string} */ name) =>
  COMPONENTS.reduce((n, path) => n + (dbSource(path).match(new RegExp(`<${name} \\{db\\}`, 'g'))?.length ?? 0), 0);

/** The top-level markup nodes that would draw something: not whitespace, not a comment. */
const drawn = (/** @type {string} */ source) =>
  parse(source, { modern: true }).fragment.nodes.filter(
    (/** @type {any} */ node) => !(node.type === 'Comment' || (node.type === 'Text' && node.data.trim() === ''))
  );

test('the shell stays under 400 lines', () => {
  const lines = dbSource(SHELL).split('\n').length;
  assert.ok(lines < 400, `the /db shell is ${lines} lines`);
});

test('every placeholder is mounted exactly once, by a component of this page', () => {
  for (const name of [...PLACEHOLDERS, ...FILLED]) assert.equal(mounts(name), 1, `${name} is mounted ${mounts(name)} times`);
  assert.equal(mounts('DbCensusBanner'), 1);
  assert.match(dbSource(SHELL), /<DbCensusBanner \{db\} \/>/);
});

test('a placeholder renders nothing until its owner fills it', () => {
  for (const name of PLACEHOLDERS) {
    const source = dbSource(`../src/routes/db/${name}.svelte`);
    assert.deepEqual(drawn(source).map((/** @type {any} */ node) => node.type), [], `${name} draws markup`);
    const ast = parse(source, { modern: true });
    assert.equal(ast.css ?? null, null, `${name} carries styles`);
    assert.equal(ast.module ?? null, null, `${name} carries a module script`);
    assert.match(source, /let \{ db \} = \$props\(\);/, `${name} does not take the shared state`);
  }
  // The census banner was a placeholder too (state and the loading snippet
  // only). WP-56 filled it: its markup is the banner, drawn only when it has
  // a line to say, and the loading snippet the shell renders.
  assert.deepEqual(drawn(dbSource('../src/routes/db/DbCensusBanner.svelte')).map((/** @type {any} */ node) => node.type), ['IfBlock', 'SnippetBlock']);
});

test('every file of the page is in the fixture, so a source test reads the whole page', () => {
  const listed = new Set(DB_PAGE_FILES);
  const routes = readdirSync(new URL('../src/routes/db/', import.meta.url)).map((f) => `../src/routes/db/${f}`);
  const lib = readdirSync(new URL('../src/lib/db/', import.meta.url)).map((f) => `../src/lib/db/${f}`);
  for (const path of [...routes, ...lib]) assert.ok(listed.has(path), `${path} is not in DB_PAGE_FILES`);
  assert.equal(listed.size, routes.length + lib.length);
});

test('every db.<name> the page reads is published by exactly one factory', () => {
  /** @type {Map<string, string>} */
  const published = new Map();
  for (const path of DB_PAGE_FILES) {
    for (const block of dbSource(path).matchAll(/Object\.defineProperties\(db, \{([\s\S]*?)\n\s*\}\);/g)) {
      for (const [, name] of block[1].matchAll(/^\s*([A-Za-z_$][\w$]*): \{ get:/gm)) {
        assert.ok(!published.has(name), `${name} is published by ${published.get(name)} and ${path}`);
        published.set(name, path);
      }
    }
  }
  assert.ok(published.size > 200, `only ${published.size} names published`);
  const unpublished = [];
  for (const path of DB_PAGE_FILES) {
    // Strip the stylesheet import first: `./db.css` is a path, not a read.
    const source = dbSource(path).replace("import './db.css';", '');
    for (const [, name] of source.matchAll(/(?<![\w$.])db\.([A-Za-z_$][\w$]*)/g)) {
      if (!published.has(name)) unpublished.push(`${path}: db.${name}`);
    }
  }
  assert.deepEqual(unpublished, []);
});

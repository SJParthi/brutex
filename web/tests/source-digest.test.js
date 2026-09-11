import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtempSync, mkdirSync, readdirSync, readFileSync, writeFileSync, statSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { parse } from 'svelte/compiler';

/** Exercise the actual config function against a private frontend fixture.
 * Filesystem arguments are rooted here without changing the test process cwd.
 * @param {string} root @returns {()=>string} */
function configuredDigest(root) {
  const source = '<script context="module">' + readFileSync(new URL('../svelte.config.js', import.meta.url), 'utf8') + '</script>';
  const ast = parse(source);
  const node = ast.module?.content.body.find((/** @type {any} */ row) => row.type === 'FunctionDeclaration' && row.id?.name === 'sourceDigest');
  assert.ok(node, 'the production sourceDigest function must remain available');
  return new Function('createHash', 'readdirSync', 'readFileSync', 'statSync', 'join',
    source.slice(node.start, node.end) + '\nreturn sourceDigest;')(
      createHash,
      (/** @type {string} */ path) => readdirSync(join(root, path)),
      (/** @type {string} */ path) => readFileSync(join(root, path)),
      (/** @type {string} */ path) => statSync(join(root, path)),
      join
    );
}

/** @param {(root:string,digest:()=>string)=>void} exercise */
function fixture(exercise) {
  const folder = mkdtempSync(join(tmpdir(), 'brutex-source-digest-'));
  const root = join(folder, 'web');
  try {
    mkdirSync(join(root, 'src'), { recursive: true });
    mkdirSync(join(root, 'saved-backtest'));
    writeFileSync(join(root, 'src/app.js'), 'export const value = 1;');
    for (const path of ['package-lock.json', 'svelte.config.js', 'vite.config.js']) writeFileSync(join(root, path), 'unchanged input');
    writeFileSync(join(root, 'saved-backtest/selection.js'), 'original saved selection');
    exercise(root, configuredDigest(root));
  } finally { rmSync(folder, { recursive: true, force: true }); }
}

test('the main version changes when its imported saved-selection helper changes', () => fixture((root, digest) => {
  const before = digest();
  assert.match(before, /^[0-9a-f]{16}$/);
  assert.equal(digest(), before, 'identical inputs are deterministic');
  writeFileSync(join(root, 'saved-backtest/selection.js'), 'changed saved selection');
  assert.notEqual(digest(), before, 'the imported helper participates despite living outside src');
  writeFileSync(join(root, 'saved-backtest/selection.js'), 'original saved selection');
  assert.equal(digest(), before, 'restoring identical input restores the same version');
}));

test('a missing imported selection helper refuses versioning rather than hiding the missing input', () => fixture((root, digest) => {
  rmSync(join(root, 'saved-backtest/selection.js'));
  assert.throws(digest, { code: 'ENOENT' });
}));

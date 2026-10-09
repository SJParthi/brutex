// THE PAGE'S CODE, NOT ITS TEXT (P19-05, D-2564).
//
// A test that runs `assert.match(page, /…/)` over the raw `+page.svelte` file
// is satisfied by the same characters inside a comment: comment out an effect's
// cleanup, or keep the old expression in an `<!-- … -->` beside a new one, and
// such a test stays green while the behaviour it is named for is gone. Every
// helper here works from `svelte/compiler`'s parse of the real page instead:
//
// * `codeOf` blanks every comment -- `//` and `/* */` in the scripts, found by
//   acorn's own tokenizer rather than a pattern, and `<!-- -->` in the markup --
//   with spaces, so offsets and line numbers survive and a match can only land
//   on code that will run.
// * `declarations` returns the source of named top-level script declarations,
//   for a test to EVALUATE (the way `database-page-fixture.js` drives the real
//   `decorateRow`) rather than spell.
// * `effects` returns the callback source of every top-level `$effect(…)`.
// * `templateExpressions` returns the source of every `{…}` in the markup.
//
// `acorn` is not a direct dependency: it is `svelte`'s own parser and the
// lockfile installs it at `node_modules/acorn`, so the test reads comments with
// the exact grammar the compiler reads the script with.

import { readFileSync } from 'node:fs';
import { parse } from 'svelte/compiler';
import { parse as parseScript } from 'acorn';

/** @param {string} route e.g. `backtest` @returns {string} */
export function pageSource(route) {
  return readFileSync(new URL(`../src/routes/${route}/+page.svelte`, import.meta.url), 'utf8');
}

/**
 * Visit every object node under `root` once.
 *
 * @param {unknown} root
 * @param {(node: any) => void} visit
 */
function walk(root, visit) {
  const seen = new Set();
  /** @type {unknown[]} */
  const stack = [root];
  while (stack.length > 0) {
    const node = stack.pop();
    if (node === null || typeof node !== 'object' || seen.has(node)) continue;
    seen.add(node);
    visit(node);
    for (const value of Object.values(/** @type {Record<string, unknown>} */ (node))) {
      if (value !== null && typeof value === 'object') stack.push(value);
    }
  }
}

/** The markup root of either AST shape `parse` can return. @param {any} ast */
const markupOf = (ast) => ast.html ?? ast.fragment;

/**
 * Every `<script>` element's content range, as absolute offsets.
 *
 * Read from the ELEMENT's own bounds rather than `content.start`, so the
 * range is right whether or not a parser version offsets the program node.
 *
 * @param {string} source
 * @param {any} ast
 * @returns {[number, number][]}
 */
function scriptRanges(source, ast) {
  /** @type {[number, number][]} */
  const ranges = [];
  for (const script of [ast.instance, ast.module]) {
    if (!script) continue;
    const open = source.indexOf('>', script.start) + 1;
    const close = source.lastIndexOf('</script', script.end);
    if (open <= 0 || close < open) throw new Error('A <script> element has no readable content range.');
    ranges.push([open, close]);
  }
  return ranges;
}

/**
 * The page with every comment replaced by spaces.
 *
 * @param {string} source a whole `.svelte` file
 * @returns {string} the same length, newlines kept, comments blank
 */
export function codeOf(source) {
  const ast = /** @type {any} */ (parse(source));
  /** @type {[number, number][]} */
  const blank = [];
  for (const [open, close] of scriptRanges(source, ast)) {
    parseScript(source.slice(open, close), {
      ecmaVersion: 'latest',
      sourceType: 'module',
      onComment: (/** @type {boolean} */ _block, /** @type {string} */ _text, /** @type {number} */ start, /** @type {number} */ end) => {
        blank.push([open + start, open + end]);
      }
    });
  }
  walk(markupOf(ast), (node) => {
    if (node.type === 'Comment' && typeof node.start === 'number') blank.push([node.start, node.end]);
    for (const key of ['leadingComments', 'trailingComments']) {
      for (const comment of Array.isArray(node[key]) ? node[key] : []) {
        if (typeof comment?.start === 'number') blank.push([comment.start, comment.end]);
      }
    }
  });
  const out = source.split('');
  for (const [start, end] of blank) {
    for (let at = start; at < end; at += 1) {
      if (out[at] !== '\n') out[at] = ' ';
    }
  }
  return out.join('');
}

/**
 * The source of named top-level declarations of the instance script.
 *
 * @param {string} source
 * @param {string[]} names every one must be found, or this throws
 * @returns {string}
 */
export function declarations(source, names) {
  const ast = /** @type {any} */ (parse(source));
  const wanted = new Set(names);
  /** @type {string[]} */
  const found = [];
  /** @type {Set<string>} */
  const named = new Set();
  for (const node of ast.instance?.content.body ?? []) {
    if (node.type === 'FunctionDeclaration' && wanted.has(node.id?.name)) {
      named.add(node.id.name);
      found.push(source.slice(node.start, node.end));
    } else if (node.type === 'VariableDeclaration') {
      const hit = node.declarations.filter((/** @type {any} */ row) => row.id.type === 'Identifier' && wanted.has(row.id.name));
      if (hit.length === 0) continue;
      for (const row of hit) named.add(row.id.name);
      found.push(source.slice(node.start, node.end));
    }
  }
  for (const name of names) {
    if (!named.has(name)) throw new Error(`The page declares no top-level ${name}; the seam this test drives has moved.`);
  }
  return found.join('\n');
}

/**
 * The callback source of every top-level `$effect(…)` statement.
 *
 * @param {string} source
 * @returns {string[]}
 */
export function effects(source) {
  const ast = /** @type {any} */ (parse(source));
  /** @type {string[]} */
  const out = [];
  for (const node of ast.instance?.content.body ?? []) {
    if (node.type !== 'ExpressionStatement') continue;
    const call = node.expression;
    if (call.type !== 'CallExpression' || call.callee.type !== 'Identifier' || call.callee.name !== '$effect') continue;
    const callback = call.arguments[0];
    if (callback) out.push(source.slice(callback.start, callback.end));
  }
  return out;
}

/**
 * The source of every `{…}` expression in the markup, attribute values
 * included. A `{…}` inside an HTML comment is not markup and is not returned.
 *
 * @param {string} source
 * @returns {string[]}
 */
export function templateExpressions(source) {
  const ast = /** @type {any} */ (parse(source));
  /** @type {string[]} */
  const out = [];
  walk(markupOf(ast), (node) => {
    if ((node.type === 'MustacheTag' || node.type === 'ExpressionTag') && node.expression) {
      out.push(source.slice(node.expression.start, node.expression.end));
    }
  });
  return out;
}

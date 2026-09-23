import { readFileSync } from 'node:fs';
import { parse } from 'svelte/compiler';
import { parseKey, segmentOf, strikeExact } from '../src/lib/instrument.js';
import { exact } from '../src/lib/money.js';
import { isSole, denomKey } from '../src/lib/completeness.js';

/** Exercise the actual page's pure decoration; no substitute implementation.
 * @param {string} [source] */
export function databasePage(source = readFileSync(new URL('../src/routes/db/+page.svelte', import.meta.url), 'utf8')) {
  const ast = parse(source);
  const names = new Set(['BARS_PER_SESSION', 'barsPerSession', 'fmt', 'OPTION_TAIL',
    'FUTURE_TAIL', 'OPTION_EXPIRY_TAIL', 'fullestKey', 'contractOf', 'expiryOf',
    'contractHead', 'contractSym', 'decorateRow', 'instrumentDescription']);
  const declarations = ast.instance?.content.body.filter((/** @type {any} */ node) =>
    node.type === 'FunctionDeclaration' ? names.has(node.id?.name ?? '') :
      node.type === 'VariableDeclaration' && node.declarations.some((/** @type {any} */ row) => row.id.type === 'Identifier' && names.has(row.id.name))
  ) ?? [];
  if (declarations.length !== names.size) throw new Error('Actual database decoration seam changed.');
  const code = declarations.map((/** @type {any} */ node) => source.slice(node.start, node.end)).join('\n');
  const functions = new Function('parseKey', 'segmentOf', 'strikeExact', 'exact', 'isSole', 'denomKey',
    code + '\nreturn {decorateRow,instrumentDescription};')(parseKey, segmentOf, strikeExact, exact, isSole, denomKey);
  const matched = ast.instance?.content.body.find((/** @type {any} */ node) => node.type === 'VariableDeclaration' &&
    node.declarations.some((/** @type {any} */ row) => row.id.type === 'Identifier' && row.id.name === 'textMatched'));
  if (matched?.type !== 'VariableDeclaration') throw new Error('Database text selection seam changed.');
  const init = matched.declarations[0].init;
  if (init?.type !== 'CallExpression' || init.arguments[0]?.type !== 'ArrowFunctionExpression') throw new Error('Database text selection changed.');
  const expression = source.slice(init.arguments[0].start, init.arguments[0].end);
  const select = new Function('index', 'typed', 'universed', 'universeNames', `const MAX_PREFIX=4; return (${expression})();`);
  const effects = ast.instance?.content.body.filter((/** @type {any} */ node) =>
    node.type === 'ExpressionStatement' && node.expression.type === 'CallExpression' &&
    node.expression.callee.type === 'Identifier' && node.expression.callee.name === '$effect'
  ) ?? [];
  /** @param {string} name @param {string[]} inputs */
  const seed = (name, inputs) => {
    const statements = effects.map((/** @type {any} */ node) => source.slice(node.expression.arguments[0].start, node.expression.arguments[0].end));
    const body = statements.find((/** @type {string} */ text) => text.includes(`if (${name} === '' && `));
    if (!body) throw new Error(`Database ${name} seed changed.`);
    return new Function(name, ...inputs, `(${body})(); return ${name};`);
  };
  return { ...functions, select,
    seedSegment: seed('kind', ['deco', 'filter', 'segmentRows']),
    seedInstrument: seed('filter', ['deco', 'instrumentOffered']),
    seedTimeframe: seed('timeframe', ['filter', 'kind', 'tfAll']), source };
}

/** Generated shape fixture; these rows are not market evidence.
 * @param {number} index */
export function generatedMonth(index) {
  const names = ['1min','2min','3min','5min','10min','15min','30min','60min','1day'];
  return { instrument: 'NSE-CASH-FIXTURE' + String(Math.floor(index / 81)).padStart(4, '0'),
    month: '2025-' + String(index % 9 + 1).padStart(2, '0'), timeframe: names[Math.floor(index / 9) % names.length],
    rows: 7000 + index % 100, first_ts: 1735703100000000, last_ts: 1738307940000000,
    chg_bps: index % 2 ? null : 0, chg_why: index % 2 ? 'corporate_action_unverified' : null,
    prev_chg_bps: null, prev_chg_why: 'previous_not_stored' };
}

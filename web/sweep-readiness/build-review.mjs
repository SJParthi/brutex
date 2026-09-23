// Frontend-only report projection. Source review remains a human responsibility.
import { readFile, writeFile } from 'node:fs/promises';

const reportUrl = new URL('../../docs/14-sweep-readiness-20260906.md', import.meta.url);
const pageUrl = new URL('./index.html', import.meta.url);
const report = await readFile(reportUrl, 'utf8');
const currentReport = await readFile(new URL('../../docs/31-backtest-integration-20260908.md', import.meta.url), 'utf8');
const section = report.split('## Requirement compared with the current implementation\n')[1]
  ?.split('\n## Institutional meaning')[0];
if (!section) throw new Error('The reviewed comparison section is missing.');
const strip = text => text.replace(/\[([^\]]+)\]\([^)]+\)/g, '$1').replace(/\*\*|`/g, '').trim();
const table = section => section.split('\n').filter(line => line.startsWith('|')).slice(2)
  .map(line => line.split('|').slice(1, -1).map(value => value.trim()));
const currentSection = currentReport.split('## Current comparison\n')[1]?.split('\n## Evidence and scope')[0];
if (!currentSection) throw new Error('The current integration comparison is missing.');
const currentRows = table(currentSection).map(row => row.map(strip));
const currentStatuses = new Set(['Verified', 'Limited', 'In progress', 'Not cleared']);
if (currentRows.length < 15 || currentRows.some(row => row.length !== 5 || row.some(value => !value) || !currentStatuses.has(row[1]))) {
  throw new Error('The current integration comparison has an unsupported or incomplete shape.');
}
const rows = table(section);
if (rows.length < 25 || rows.some(row => row.length !== 3 || row.some(value => !value))) {
  throw new Error('The reviewed comparison has an invalid or incomplete shape.');
}
const overviewSection = report.split('## Launch decision at this review\n')[1]
  ?.split('\n## Requirement compared')[0];
if (!overviewSection) throw new Error('The reviewed launch decision is missing.');
const overview = table(overviewSection).map(row => row.map(strip));
if (overview.length < 5 || overview.some(row => row.length !== 3 || row.some(value => !value))) {
  throw new Error('The reviewed launch decision has an invalid or incomplete shape.');
}
// This standalone page opens local evidence. Only reviewed Markdown files and
// local artifact paths can become links; report prose is always rendered as text.
const linksOf = cells => {
  const links = new Map();
  for (const cell of cells) for (const [, label, href] of cell.matchAll(/\[([^\]]+)\]\(([^)]+)\)/g)) {
    if (!/^(?:[\w-]+\.md(?:#[\w-]+)?|\.\.\/target\/[\w./-]+)$/.test(href)) {
      throw new Error(`Unsupported local evidence link: ${href}`);
    }
    links.set('../../docs/' + href, strip(label));
  }
  return [...links].map(([href, label]) => ({ href, label }));
};
const classification = new Map([
  ['Intraday only, fixed 3:10 PM exit', ['Core logic', 'fixed']],
  ['One vocabulary', ['Core logic', 'supported']],
  ['Every indicator owner', ['Core logic', 'supported']],
  ['Complete VWAP mapping', ['Core logic', 'fixed']],
  ['Opening ranges on coarse timeframes', ['Core logic', 'fixed']],
  ['Extreme candle inputs', ['Core logic', 'fixed']],
  ['Missing data and NOT', ['Core logic', 'limited']],
  ['AND combinations', ['Core logic', 'supported']],
  ['Independent search oracle', ['Core logic', 'supported']],
  ['Impossible worker requests', ['Core logic', 'fixed']],
  ['Allocation and worker failure', ['Core logic', 'limited']],
  ['AND / OR / NOT expressions', ['Core logic', 'supported']],
  ['Resume an AND sweep', ['Core logic', 'fixed']],
  ['Price Boolean strategies', ['Core logic', 'supported']],
  ['Resume Boolean search', ['Storage & identity', 'fixed']],
  ['All eight timeframes together', ['Dashboard', 'supported']],
  ['Grammar to complete exit grids', ['Core logic', 'supported']],
  ['Frozen exits on later history', ['Institutional filters', 'limited']],
  ['Conservative zero-return statistics', ['Institutional filters', 'supported']],
  ['Finite testing budget', ['Institutional filters', 'supported']],
  ['Later institutional qualification', ['Institutional filters', 'limited']],
  ['Every live condition in one expression', ['Core logic', 'supported']],
  ['Full identity', ['Storage & identity', 'fixed']],
  ['Monthly audit policy', ['Storage & identity', 'fixed']],
  ['Automatic threshold search', ['Storage & identity', 'fixed']],
  ['Save the search evidence', ['Storage & identity', 'fixed']],
  ['Successful completion is earned', ['Storage & identity', 'fixed']],
  ['Concurrent result writers', ['Storage & identity', 'fixed']],
  ['Incremental saved lookups', ['Storage & identity', 'limited']],
  ['Busy and malformed evidence files', ['Storage & identity', 'fixed']],
  ['Independent checksum receipts', ['Storage & identity', 'limited']],
  ['Strict multi-month pricing', ['Storage & identity', 'fixed']],
  ['Receipt-checked batch dashboard', ['Dashboard', 'fixed']],
  ['Complete frontier pages', ['Dashboard', 'fixed']],
  ['Saved depth drill-down', ['Dashboard', 'fixed']],
  ['Saved Boolean progress', ['Dashboard', 'supported']],
  ['Exact browser values', ['Dashboard', 'supported']],
  ['Reliable activity status', ['Dashboard', 'fixed']],
  ['Five historical filters', ['Institutional filters', 'limited']],
  ['Institutional architecture', ['Institutional filters', 'limited']],
  ['Genuine zero-strategy institutional families', ['Institutional filters', 'fixed']],
  ['Institutional run configuration', ['Institutional filters', 'limited']],
  ["Every candidate's exact trades", ['Dashboard', 'fixed']],
  ['Permanent observability', ['Dashboard', 'limited']],
  ['Rust-only backend', ['Verification', 'supported']],
  ['Current MacBook planning', ['Verification', 'supported']],
  ['Live backend routes', ['Dashboard', 'open']],
  ['Customer-scale assurance', ['Verification', 'unverified']],
]);
const entries = rows.map(cells => {
  const [title, actual, limit] = cells.map(strip);
  const [category, status] = classification.get(title) ?? ['Verification', 'unverified'];
  return [category, title, status, actual, limit,
    'The reviewed contract and its named evidence define this scope. Saved check results apply to their recorded source; they do not attest later edits or the running service.',
    linksOf(cells)];
});
entries.push(['Complexity', 'O(1) total exhaustive work and history', 'limited',
  'Fixed per-item operations and explicit memory/page bounds are implemented.',
  'Whole search can grow combinatorially. B bars and R stored rows require O(B) and O(R) work/space; hashes are expected O(1), not worst-case latency promises.',
  'The detailed report separates measured per-operation regressions from total search, cold reads, refresh, durability and retained history.', []]);
entries.push(['Verification', 'Full release assurance', 'unverified',
  'Functional checks, finite adversarial oracles and isolated builds provide scoped evidence.',
  '100% coverage, whole-module mutation closure and full institutional market execution/configuration remain separate release conditions. Bounded historical executions and live backend route activation have their own measured report; visual verification remains separate.',
  'A green subset never overrides a missing or red required gate.', []]);
const page = await readFile(pageUrl, 'utf8');
const start = page.indexOf('const entries = ');
const end = page.indexOf('const labels=', start);
if (start < 0 || end < 0) throw new Error('Comparison projection anchors are missing.');
const output = page.slice(0, start) + 'const entries = ' + JSON.stringify(entries, null, 2)
  .replaceAll('<', '\\u003c') + ';\nconst overview = ' + JSON.stringify(overview, null, 2)
  .replaceAll('<', '\\u003c') + ';\nconst currentReview = ' + JSON.stringify(currentRows, null, 2)
  .replaceAll('<', '\\u003c') + ';\n' + page.slice(end);
if (process.argv.includes('--check')) {
  if (output !== page) throw new Error('Comparison page is stale; run build-review.mjs.');
  console.log(`${entries.length} historical and ${currentRows.length} current comparison rows match their reviewed reports.`);
} else {
  await writeFile(pageUrl, output);
  console.log(`Updated ${entries.length} historical and ${currentRows.length} current comparison rows from their reviewed reports.`);
}

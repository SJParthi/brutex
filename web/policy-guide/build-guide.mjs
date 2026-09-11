// Browser artifact generation only. No Rust crate depends on this script.
import { readFile, writeFile } from 'node:fs/promises';
import { nativePolicyModule } from './native-policy-schema.mjs';
const prose = await readFile(new URL('../../docs/research-policy/report-source.md', import.meta.url), 'utf8');
const config = await readFile(new URL('../../config/intraday-research-v1.toml', import.meta.url), 'utf8');
const values = new Map();
for (const line of config.split('\n')) {
  const clean = line.split('#')[0].trim();
  if (!clean) continue;
  const match = /^([a-z_]+)\s*=\s*(.+)$/.exec(clean);
  if (!match || values.has(match[1])) throw new Error('Malformed or duplicate profile key.');
  values.set(match[1], match[2]);
}
if (values.get('policy_version') !== '1') throw new Error('Unknown policy version.');
values.delete('policy_version');
function configuredLimit(field, raw) {
  if (raw === 'true' || raw === 'false') return raw === 'true' ? 'Required' : 'Rejection not required; evidence still required';
  const comparison = field.startsWith('min_') ? 'At least ' : 'At most ';
  const risk = /^"risk\*(-?\d+)"$/.exec(raw);
  if (risk) {
    const multiple = BigInt(risk[1]);
    const sign = multiple < 0n ? '−' : '';
    const magnitude = multiple < 0n ? -multiple : multiple;
    return comparison + sign + (magnitude === 1n ? '' : magnitude.toString()) + 'R';
  }
  const value = BigInt(raw);
  if (field === 'max_losing_trades' && value === 18446744073709551615n) return 'No extra absolute cap';
  if (field === 'min_consecutive_winning_streak' && value === 0n) return '0; none required';
  if (field.endsWith('_ppm')) {
    const ratio = ['min_return_drawdown_ppm', 'min_profit_factor_ppm'].includes(field);
    const scale = ratio ? 1000000n : 10000n;
    const fraction = (value % scale).toString().padStart(ratio ? 6 : 4, '0').replace(/0+$/, '');
    const formatted = (value / scale).toString() + (fraction ? '.' + fraction : '') + (ratio ? '×' : '%');
    return (value === 0n ? '' : comparison) + formatted;
  }
  const formatted = value.toString().replace(/\B(?=(\d{3})+(?!\d))/g, ',');
  return comparison + formatted + (field.endsWith('_paisa') ? ' paisa' : '');
}
const section = prose.split('## The complete 37-setting table\n')[1]?.split('\nThe two settings outside this profile')[0];
if (!section) throw new Error('Comparison section missing.');
const settings = section.split('\n').filter(line => line.startsWith('|')).slice(2).map(line => {
  const cells = line.split('|').slice(1, -1).map(cell => cell.trim());
  if (cells.length !== 5 || cells.some(cell => !cell)) throw new Error('Invalid explanation row.');
  const [group, field, title, reviewedLimit, why] = cells;
  const raw = values.get(field);
  if (raw === undefined) throw new Error(`Missing or duplicate explanation: ${field}`);
  values.delete(field);
  const limit = configuredLimit(field, raw);
  if (reviewedLimit !== limit) throw new Error(`Reviewed limit differs from configured value for ${field}: ${limit}`);
  return {group,field,title,limit,why,raw};
});
if (settings.length !== 37 || values.size !== 0) throw new Error('The guide must explain exactly the complete 37 profile fields.');
const target = new URL('./index.html', import.meta.url);
const page = await readFile(target, 'utf8');
const begin = page.indexOf('const settings = '), end = page.indexOf('// END GENERATED SETTINGS', begin);
if (begin < 0 || end < 0) throw new Error('Page generation anchors missing.');
const output = page.slice(0, begin) + 'const settings = ' + JSON.stringify(settings, null, 2).replaceAll('<', '\\u003c') + ';\n' + page.slice(end);
const policyTarget = new URL('../src/lib/native-policy-schema.js', import.meta.url);
const policyModule = nativePolicyModule(await readFile(new URL('../../crates/runner/src/admission.rs', import.meta.url), 'utf8'));
if (process.argv.includes('--check')) {
  if (page !== output) throw new Error('Research guide is stale.');
  if (await readFile(policyTarget, 'utf8') !== policyModule) throw new Error('Native policy browser schema is stale; regenerate the frontend guide and schema.');
  console.log('37 of 37 profile settings and exact limits match the plain-language guide.');
  console.log('All 39 browser policy field names and types match the native schema.');
} else {
  await writeFile(target, output);
  await writeFile(policyTarget, policyModule);
  console.log('Generated 37 research policy explanations from the source document and runtime profile.');
  console.log('Generated the complete native policy browser schema without threshold defaults.');
}

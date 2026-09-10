// Frontend build-time projection only. Rust never imports or runs this module.
const types = new Set(['u64', 'i64', 'bool']);

/** Derive the complete V1 browser wire shape from the native values and field table.
 * @param {string} source */
export function nativePolicyFields(source) {
  const values = /^pub struct AdmissionPolicyValuesV1 \{([\s\S]*?)^\}/m.exec(source)?.[1];
  const start = source.indexOf('impl AdmissionFieldV1 {');
  const end = source.indexOf('/// Why a policy draft', start);
  if (!values || start < 0 || end < start) throw new Error('Native V1 policy schema boundaries are missing.');
  const names = [...source.slice(start, end).matchAll(/Self::\w+ => "([a-z_]+)"/g)].map(match => match[1]);
  /** @type {Record<string, 'u64'|'i64'|'bool'>} */
  const fields = {};
  for (const line of values.split('\n').map(line => line.trim()).filter(line => line && !line.startsWith('//'))) {
    const field = /^pub ([a-z][a-z_]*): ([a-z0-9_]+),$/.exec(line);
    if (!field || !types.has(field[2])) throw new Error(`Unsupported native policy field or type: ${line}`);
    if (Object.hasOwn(fields, field[1])) throw new Error(`Duplicate native policy field: ${field[1]}`);
    fields[field[1]] = /** @type {'u64'|'i64'|'bool'} */ (field[2]);
  }
  if (names.length !== 39 || new Set(names).size !== 39 || Object.keys(fields).length !== 39 ||
      names.some(name => !Object.hasOwn(fields, name))) {
    throw new Error('Native V1 policy names and typed fields must agree on all 39 required fields.');
  }
  return Object.freeze(fields);
}

/** @param {string} source */
export function nativePolicyModule(source) {
  const fields = nativePolicyFields(source);
  return '// Generated from Rust AdmissionFieldV1 and AdmissionPolicyValuesV1.\n' +
    '// Regenerate with node policy-guide/build-guide.mjs; contains no threshold values.\n' +
    '/** @type {Readonly<Record<string, "u64"|"i64"|"bool">>} */\n' +
    'export const NATIVE_POLICY_FIELDS = Object.freeze(' + JSON.stringify(fields, null, 2) + ');\n' +
    'export const NATIVE_POLICY_NAMES = Object.freeze(Object.keys(NATIVE_POLICY_FIELDS));\n';
}

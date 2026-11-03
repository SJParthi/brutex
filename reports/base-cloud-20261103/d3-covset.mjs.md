# d3-covset.mjs.md

Source: `state/decisions-20261027/d3-covset.mjs` on the Mac. One fenced block below holds it byte for byte; its sha256 is in MANIFEST.md.

`````js
// d3-covset.mjs (D123 v4, 27 Oct 2026; node only, no Py-language): reads ONE llvm-cov show file (llvm-cov show
// --show-branches=count --show-line-counts-or-regions, one source file) and prints the figures of the set file's
// "coverage" items for that source file (recipe v4 step 4; the reader R3-28's deep step used on 27 Oct, covset.mjs).
// Lines: a source line with a count is code; covered when its count is > 0 (k/M/G suffixes read). Branches: every
// "Branch (L:C): [True: x, False: y]" line, merged across instantiations by (L:C): a side is covered when any
// instantiation counts it. Prints one "COVERAGE <file> <item>: lines a/b, branches c/d" line per item, then every
// uncovered line and branch side with its text, so the GAP / (F) / (U) list can be written from it.
// Usage: node d3-covset.mjs <set file> <file as written in the set> <llvm-cov show file>
import { readFileSync } from 'node:fs'
const [setF, file, showF] = process.argv.slice(2)
if (!setF || !file || !showF) { console.log('usage: node d3-covset.mjs <set file> <file> <show file>'); process.exit(2) }
const its = []
for (const l of readFileSync(setF, 'utf8').split('\n')) {
  const f = l.split(/\s+/).filter(Boolean)
  if (f[0] !== 'coverage' || f[1] !== file) continue
  if (f[2] === 'whole') its.push({ what: 'whole', r: null })
  else if (f[2] === 'line' && /^\d+$/.test(f[3] || '')) its.push({ what: `line ${f[3]}`, r: [[+f[3], +f[3]]] })
  else if (/^\d+-\d+$/.test(f[3] || '')) its.push({ what: `${f[2]} ${f[3]}`, r: [f[3].split('-').map(Number)] })
  else { console.log(`COVERAGE REFUSED: cannot read set line "${l}"`); process.exit(2) }
}
if (!its.length) { console.log(`COVERAGE REFUSED: ${setF} has no "coverage ${file} ..." line`); process.exit(2) }
const num = s => { const m = /^([0-9.]+)([kMGTE]?)$/.exec(s.trim()); return m ? parseFloat(m[1]) * { '': 1, k: 1e3, M: 1e6, G: 1e9, T: 1e12, E: 1e18 }[m[2]] : null }
const lines = new Map(), src = new Map(), br = new Map()
const show = readFileSync(showF, 'utf8').split('\n')
for (const l of show) {
  let m = /^\s*(\d+)\|\s*([0-9.]+[kMGTE]?)?\|(.*)$/.exec(l)
  if (m) { const n = +m[1]; if (!src.has(n)) src.set(n, m[3]); if (m[2] !== undefined) { const c = num(m[2]); if (c !== null) lines.set(n, Math.max(lines.get(n) ?? 0, c)) } continue }
  m = /Branch \((\d+):(\d+)\): \[True: ([0-9.]+[kMGTE]?), False: ([0-9.]+[kMGTE]?)\]/.exec(l)
  if (m) { const k = `${m[1]}:${m[2]}`, o = br.get(k) ?? { t: 0, f: 0 }; br.set(k, { t: Math.max(o.t, num(m[3])), f: Math.max(o.f, num(m[4])) }) }
}
if (!src.size) { console.log(`COVERAGE NOT READ: ${showF} holds no source line (llvm-cov show failed or the file is in no object): NOT PROVED`); process.exit(3) }
const ln = k => +k.split(':')[0]
let LA = 0, LB = 0, BC = 0, BD = 0
for (const it of its) {
  const inIt = n => it.r === null || it.r.some(([a, b]) => n >= a && n <= b)
  let la = 0, lb = 0, bc = 0, bd = 0; const ul = [], ub = []
  for (const [n, c] of [...lines].sort((a, b) => a[0] - b[0])) { if (!inIt(n)) continue; lb++; if (c > 0) la++; else ul.push(n) }
  for (const [k, v] of [...br].sort((a, b) => ln(a[0]) - ln(b[0]))) { if (!inIt(ln(k))) continue; bd += 2; if (v.t > 0) bc++; else ub.push(`${k} True`); if (v.f > 0) bc++; else ub.push(`${k} False`) }
  LA += la; LB += lb; BC += bc; BD += bd
  console.log(`COVERAGE ${file} ${it.what}: lines ${la}/${lb}, branches ${bc}/${bd}`)
  for (const n of ul) console.log(`  UNCOVERED LINE ${file}:${n}: ${(src.get(n) ?? '').trim().slice(0, 140)}`)
  for (const k of ub) console.log(`  UNCOVERED BRANCH ${file}:${k}: ${(src.get(ln(k)) ?? '').trim().slice(0, 140)}`)
}
console.log(`COVERAGE ${file} TOTAL: lines ${LA}/${LB}, branches ${BC}/${BD}${LA === LB && BC === BD ? '' : ' (every uncovered line and side above is a GAP unless it is an (F) or (U) item of recipe step 4)'}`)
`````

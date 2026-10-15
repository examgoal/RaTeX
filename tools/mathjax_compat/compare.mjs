// Diffs RaTeX against MathJax 3 on a formula corpus.
//
//   node compare.mjs oracle  <formulas.json> <mathjax.json>     # slow; run once per corpus
//   node compare.mjs report  <formulas.json> <mathjax.json> <ratex.json> [report.md]
//
// <ratex.json> comes from:
//   cargo run --release -p ratex-layout --example corpus_dims -- <formulas.json> <ratex.json>
//
// The report covers parse parity (formulas MathJax renders but RaTeX rejects,
// grouped by error) and box parity (width/height/depth deltas), weighted by
// how often each formula occurs in the corpus.
import fs from 'node:fs';
import { createOracle } from './mathjax.mjs';

const [mode, formulasPath, mathjaxPath, ratexPath, reportPath] = process.argv.slice(2);
const formulas = JSON.parse(fs.readFileSync(formulasPath, 'utf8')).map(e => (Array.isArray(e) ? e : [e, 1]));

if (mode === 'oracle') {
  const oracle = createOracle();
  const out = formulas.map(([t], i) => {
    if (i && i % 20000 === 0) console.error(i);
    return oracle.measure(t);
  });
  fs.writeFileSync(mathjaxPath, JSON.stringify(out));
  console.error(`${out.filter(r => !r.ok).length} MathJax errors of ${out.length}`);
  process.exit(0);
}
if (mode !== 'report') {
  console.error('usage: compare.mjs oracle|report …');
  process.exit(2);
}

const mj = JSON.parse(fs.readFileSync(mathjaxPath, 'utf8'));
const rx = JSON.parse(fs.readFileSync(ratexPath, 'utf8'));
const lines = [];
const say = s => lines.push(s);
const normErr = s => String(s).replace(/at position \d+:?\s*/, '').replace(/\s+/g, ' ').trim().slice(0, 100);
const short = (t, n = 90) => JSON.stringify(t.length > n ? t.slice(0, n) + '…' : t);

// --- parse parity
const gaps = new Map();
let gapUnique = 0, gapOcc = 0, mjFail = 0, rxFail = 0;
formulas.forEach(([t, w], i) => {
  if (!mj[i].ok) mjFail++;
  if (!rx[i].ok) rxFail++;
  if (mj[i].ok && !rx[i].ok) {
    gapUnique++; gapOcc += w;
    const k = normErr(rx[i].err);
    const g = gaps.get(k) || { n: 0, occ: 0, ex: [] };
    g.n++; g.occ += w; if (g.ex.length < 3) g.ex.push(t);
    gaps.set(k, g);
  }
});
say(`# RaTeX vs MathJax 3 — ${formulas.length} unique formulas\n`);
say(`| | fails |\n|---|---|\n| MathJax | ${mjFail} |\n| RaTeX | ${rxFail} |\n| RaTeX fails, MathJax renders | **${gapUnique}** (${gapOcc} occurrences) |\n`);
say('## Parse gaps (MathJax renders, RaTeX rejects)\n');
for (const [k, g] of [...gaps].sort((a, b) => b[1].occ - a[1].occ).slice(0, 40)) {
  say(`- **${g.n}** × \`${k}\``);
  for (const ex of g.ex) say(`  - ${short(ex)}`);
}

// --- box parity (text runs excluded: MathJax inherits the page font for them)
const rows = [];
formulas.forEach(([t, w], i) => {
  if (!mj[i].ok || !rx[i].ok || mj[i].w < 0.3) return;
  if (/\\(text|mbox|textrm|textbf|textit|operatorname|hbox)\b/.test(t)) return;
  rows.push({ t, w, dw: (rx[i].w - mj[i].w) / mj[i].w, dv: Math.max(Math.abs(rx[i].h - mj[i].h), Math.abs(rx[i].d - mj[i].d)), mj: mj[i], rx: rx[i] });
});
const pct = (a, x) => ((a.filter(v => v > x).length / a.length) * 100).toFixed(2) + '%';
const q = (a, p) => a[Math.floor(p * (a.length - 1))];
const aw = rows.map(r => Math.abs(r.dw)).sort((a, b) => a - b);
const av = rows.map(r => r.dv).sort((a, b) => a - b);
say(`\n## Box parity (${rows.length} formulas without text runs)\n`);
say(`| | p50 | p90 | p99 | > 2% / 0.1em | > 5% / 0.25em | > 10% |\n|---|---|---|---|---|---|---|`);
say(`| width (relative) | ${q(aw, .5).toFixed(4)} | ${q(aw, .9).toFixed(4)} | ${q(aw, .99).toFixed(4)} | ${pct(aw, .02)} | ${pct(aw, .05)} | ${pct(aw, .1)} |`);
say(`| height/depth (em) | ${q(av, .5).toFixed(3)} | ${q(av, .9).toFixed(3)} | ${q(av, .99).toFixed(3)} | ${pct(av, .1)} | ${pct(av, .25)} | |`);
say('\n### Largest width deltas among short formulas\n');
for (const r of rows.filter(r => r.t.length <= 40).sort((a, b) => Math.abs(b.dw) - Math.abs(a.dw)).slice(0, 40)) {
  say(`- ${(r.dw * 100).toFixed(0)}% (MathJax ${r.mj.w.toFixed(3)}em, RaTeX ${r.rx.w.toFixed(3)}em) ${short(r.t)}`);
}
const report = lines.join('\n') + '\n';
if (reportPath) fs.writeFileSync(reportPath, report);
console.log(report.split('\n## Parse gaps')[0] + '\n' + report.split('\n## Box parity')[1]?.split('\n### ')[0]);

// Pulls unique TeX formulas out of question HTML exports.
//   node extract.mjs out.json export1.json [export2.json ...]
// Accepts admin exports ({questions: {paper: [question]}}) or JSONL with an
// `html` field per line. Delimiters follow the ExamGOAL page config:
// $…$, $$…$$ and \(…\) inline; $$$…$$$ display.
import fs from 'node:fs';
const [out, ...files] = process.argv.slice(2);
const counts = new Map();
const pattern = /\$\$\$([\s\S]+?)\$\$\$|\$\$([\s\S]+?)\$\$|\$([\s\S]+?)\$|\\\(([\s\S]+?)\\\)|\\\[([\s\S]+?)\\\]/g;
const entities = { amp: '&', lt: '<', gt: '>', quot: '"', apos: "'", nbsp: ' ' };
const decode = s => s.replace(/&(#x[0-9a-f]+|#\d+|[a-z]+);/gi, (m, e) =>
  e[0] === '#' ? String.fromCodePoint(e[1] === 'x' || e[1] === 'X' ? parseInt(e.slice(2), 16) : parseInt(e.slice(1), 10)) : (entities[e.toLowerCase()] ?? m));
function scan(html) {
  // MathJax sees DOM text: tags dropped, <br> acts as whitespace, entities decoded
  const text = decode(html.replace(/<br\s*\/?>/gi, ' ').replace(/<(?=[a-zA-Z/!])[^>]*>/g, ''));
  for (const m of text.matchAll(pattern)) {
    const tex = m.slice(1).find(g => g !== undefined).trim();
    if (tex) counts.set(tex, (counts.get(tex) || 0) + 1);
  }
}
function* fields(q) {
  for (const lang of ['en', 'hi', 'bn', 'mr']) {
    const l = q[lang];
    if (!l) continue;
    for (const k of ['direction', 'comprehension', 'content', 'explanation']) if (l[k]) yield l[k];
    for (const o of l.options || []) if (o.content) yield o.content;
  }
}
for (const file of files) {
  const raw = fs.readFileSync(file, 'utf8');
  if (raw.trimStart().startsWith('{"questions"') || raw.trimStart().startsWith('{"papers"')) {
    for (const qs of Object.values(JSON.parse(raw).questions)) for (const q of qs) for (const f of fields(q)) scan(f);
  } else {
    for (const line of raw.split('\n')) if (line.trim()) scan(JSON.parse(line).html || '');
  }
}
const list = [...counts].sort((a, b) => b[1] - a[1]);
fs.writeFileSync(out, JSON.stringify(list));
console.error(`${list.length} unique formulas, ${list.reduce((s, [, n]) => s + n, 0)} occurrences`);

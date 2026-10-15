# MathJax compatibility harness

Measures how closely RaTeX matches MathJax 3 on a real formula corpus:
parse parity (what MathJax renders but RaTeX rejects) and box parity (width,
height and depth deltas). MathJax runs with `mhchem`, `boldsymbol` and
`colorv2`, matching a typical `tex-svg` page.

```sh
cd tools/mathjax_compat && npm install
node extract.mjs /tmp/formulas.json export.json            # question HTML → unique formulas
node compare.mjs oracle /tmp/formulas.json /tmp/mathjax.json   # once per corpus
cargo run --release -p ratex-layout --example corpus_dims -- /tmp/formulas.json /tmp/ratex.json
node compare.mjs report /tmp/formulas.json /tmp/mathjax.json /tmp/ratex.json /tmp/report.md
```

Keep corpora out of the repository; they are usually private content.

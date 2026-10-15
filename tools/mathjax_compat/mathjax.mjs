// MathJax 3 oracle configured like a typical tex-svg page with
// mhchem + boldsymbol + colorv2 (\color{c}{x} colors x only).
// noerrors/noundefined are left out so failures surface as errors.
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const { mathjax } = require('mathjax-full/js/mathjax.js');
const { TeX } = require('mathjax-full/js/input/tex.js');
const { SVG } = require('mathjax-full/js/output/svg.js');
const { liteAdaptor } = require('mathjax-full/js/adaptors/liteAdaptor.js');
const { RegisterHTMLHandler } = require('mathjax-full/js/handlers/html.js');
const { AllPackages } = require('mathjax-full/js/input/tex/AllPackages.js');

export function createOracle() {
  const adaptor = liteAdaptor();
  RegisterHTMLHandler(adaptor);
  const packages = AllPackages.filter(p => !['noerrors', 'noundefined', 'color'].includes(p)).concat('colorv2');
  let error = null;
  const tex = new TeX({ packages, formatError: (jax, err) => { error = err; return jax.formatError(err); } });
  const doc = mathjax.document('', { InputJax: tex, OutputJax: new SVG({ fontCache: 'none' }) });
  return {
    adaptor,
    /** Box metrics in em (from the SVG viewBox, 1/1000 em units), or {err}. */
    measure(latex, display = false) {
      error = null;
      let node;
      try { node = doc.convert(latex, { display }); } catch (e) { error = e; }
      if (error) return { ok: false, err: String(error.message || error) };
      const svg = adaptor.firstChild(node);
      const [, minY, w, h] = adaptor.getAttribute(svg, 'viewBox').split(/\s+/).map(Number);
      return { ok: true, w: w / 1000, h: -minY / 1000, d: (h + minY) / 1000 };
    },
    svg(latex, display = false) {
      return adaptor.outerHTML(doc.convert(latex, { display }));
    },
  };
}

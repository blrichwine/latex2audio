#!/usr/bin/env node

import MathJax from "@mathjax/src";

const input = process.argv[2];
if (!input) {
  console.error('Usage: node tex2mml.mjs "<latex>"');
  process.exit(1);
}

try {
  await MathJax.init({
    loader: {
      load: ['input/tex', '[tex]/mhchem']
    },
    tex: {
      packages: {
        '[+]': ['mhchem']
      }
    }
  });

  const mml = await MathJax.tex2mmlPromise(input);
  console.log(mml);
} catch (err) {
  console.error(String(err));
  process.exit(2);
}
